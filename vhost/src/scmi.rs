use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use base::{AsRawDescriptor, RawDescriptor};

use super::{Error, Result, Vhost};

/// Handle for running VHOST_SCMI ioctls.
pub struct Scmi {
    descriptor: File,
}

impl Scmi {
    /// Open a handle to a new VHOST_SCMI instance.
    pub fn new(vhost_scmi_device_path: &Path) -> Result<Scmi> {
        Ok(Scmi {
            descriptor: OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
                .open(vhost_scmi_device_path)
                .map_err(Error::VhostOpen)?,
        })
    }
}

impl Vhost for Scmi {}

impl AsRawDescriptor for Scmi {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.descriptor.as_raw_descriptor()
    }
}
