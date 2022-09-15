// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use anyhow::Context;
use futures::pin_mut;
use futures::select;
use futures::FutureExt;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use base::info;
use base::CloseNotifier;
use base::ReadNotifier;
use base::Tube;
use cros_async::EventAsync;
use cros_async::Executor;
use vmm_vhost::connection::TubeEndpoint;
use vmm_vhost::message::MasterReq;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::Master;
use vmm_vhost::MasterReqHandler;
use vmm_vhost::VhostUserMaster;

use crate::virtio::vhost::user::vmm::handler::BackendReqHandler;
use crate::virtio::vhost::user::vmm::handler::BackendReqHandlerImpl;
use crate::virtio::vhost::user::vmm::handler::VhostUserHandler;
use crate::virtio::vhost::user::vmm::Error;
use crate::virtio::vhost::user::vmm::Result as VhostResult;

// TODO(rizhang): upstream CL so SocketMaster is renamed to EndpointMaster to make it more cross
// platform.
pub(in crate::virtio::vhost::user::vmm::handler) type SocketMaster =
    Master<TubeEndpoint<MasterReq>>;

impl VhostUserHandler {
    /// Creates a `VhostUserHandler` instance attached to the provided Tube
    /// with features and protocol features initialized.
    pub fn new_from_tube(
        tube: Tube,
        max_queue_num: u64,
        allow_features: u64,
        init_features: u64,
        allow_protocol_features: VhostUserProtocolFeatures,
    ) -> VhostResult<Self> {
        let backend_pid = tube.target_pid();
        Self::new(
            SocketMaster::from_stream(tube, max_queue_num),
            allow_features,
            init_features,
            allow_protocol_features,
            backend_pid,
        )
    }

    pub fn initialize_backend_req_handler(&mut self, h: BackendReqHandlerImpl) -> VhostResult<()> {
        let backend_pid = self
            .backend_pid
            .expect("tube needs target pid for backend requests");
        let mut handler = MasterReqHandler::with_tube(Arc::new(Mutex::new(h)), backend_pid)
            .map_err(Error::CreateShmemMapperError)?;
        self.vu
            .set_slave_request_fd(&handler.take_tx_descriptor())
            .map_err(Error::SetDeviceRequestChannel)?;
        self.backend_req_handler = Some(handler);
        Ok(())
    }
}

pub async fn run_backend_request_handler(
    handler: Option<BackendReqHandler>,
    ex: &Executor,
) -> Result<()> {
    let mut handler = match handler {
        Some(h) => h,
        None => std::future::pending().await,
    };

    let read_notifier = handler.get_read_notifier();
    let close_notifier = handler.get_close_notifier();

    let read_event =
        EventAsync::clone_raw(read_notifier, ex).context("failed to create an async event")?;
    let close_event =
        EventAsync::clone_raw(close_notifier, ex).context("failed to create an async event")?;

    let read_event_fut = read_event.next_val().fuse();
    let close_event_fut = close_event.next_val().fuse();
    pin_mut!(read_event_fut);
    pin_mut!(close_event_fut);

    loop {
        select! {
            _read_res = read_event_fut => {
                handler
                    .handle_request()
                    .context("failed to handle a vhost-user request")?;
                read_event_fut.set(read_event.next_val().fuse());
            }
            // Tube closed event.
            _close_res = close_event_fut => {
                info!("exit run loop: got close event");
                return Ok(())
            }
        }
    }
}
