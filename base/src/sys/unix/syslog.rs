// Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub use super::target_os::syslog::PlatformSyslog;
pub use super::RawDescriptor;

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::fs::File;
    use std::io::Read;
    use std::io::Seek;
    use std::io::SeekFrom;
    use std::os::unix::io::FromRawFd;

    cfg_if::cfg_if! {
        // ANDROID: b/228881485
        if #[cfg(not(target_os = "android"))] {
            use libc::shm_open;
            use libc::shm_unlink;
            use libc::O_CREAT;
            use libc::O_EXCL;
            use libc::O_RDWR;
        }
    }

    use crate::syslog::*;

    #[test]
    fn fds() {
        ensure_inited().unwrap();
        let mut fds = Vec::new();
        push_descriptors(&mut fds);
        assert!(!fds.is_empty());
        for fd in fds {
            assert!(fd >= 0);
        }
    }

    #[test]
    #[cfg(not(target_os = "android"))] // ANDROID: b/228881485
    fn syslog_file() {
        ensure_inited().unwrap();
        let shm_name = CStr::from_bytes_with_nul(b"/crosvm_shm\0").unwrap();
        let mut file = unsafe {
            shm_unlink(shm_name.as_ptr());
            let fd = shm_open(shm_name.as_ptr(), O_RDWR | O_CREAT | O_EXCL, 0o666);
            assert!(fd >= 0, "error creating shared memory;");
            shm_unlink(shm_name.as_ptr());
            File::from_raw_fd(fd)
        };

        let syslog_file = file.try_clone().expect("error cloning shared memory file");
        let state = State::new(LogConfig {
            pipe: Some(Box::new(syslog_file)),
            ..Default::default()
        })
        .unwrap();

        const TEST_STR: &str = "hello shared memory file";
        state.log(
            &log::RecordBuilder::new()
                .level(Level::Error)
                .args(format_args!("{}", TEST_STR))
                .build(),
        );

        file.seek(SeekFrom::Start(0))
            .expect("error seeking shared memory file");
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .expect("error reading shared memory file");
        assert!(buf.contains(TEST_STR));
    }
}
