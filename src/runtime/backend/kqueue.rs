use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
use hashbrown::HashMap;

pub struct KqueueBackend {
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    kq: i32,
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fd_map: HashMap<u64, i32>,
    initialized: bool,
}

impl KqueueBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(any(target_os = "macos", target_os = "freebsd"))]
            kq: -1,
            #[cfg(any(target_os = "macos", target_os = "freebsd"))]
            fd_map: HashMap::new(),
            initialized: false,
        }
    }
}

impl Default for KqueueBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBackend for KqueueBackend {
    fn name(&self) -> &'static str {
        "kqueue"
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        self.kq = fd;
        self.initialized = true;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        Err(RuntimeError::BackendUnavailable {
            backend: String::from("kqueue"),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn register(&mut self, token: u64, fd: i32, interest: FdInterest) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut changes: Vec<libc::kevent> = Vec::with_capacity(2);

        if interest.readable {
            let mut ev: libc::kevent = unsafe { std::mem::zeroed() };
            ev.ident = fd as libc::uintptr_t;
            ev.filter = libc::EVFILT_READ;
            ev.flags = libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR;
            ev.udata = token as *mut libc::c_void;
            changes.push(ev);
        }

        if interest.writable {
            let mut ev: libc::kevent = unsafe { std::mem::zeroed() };
            ev.ident = fd as libc::uintptr_t;
            ev.filter = libc::EVFILT_WRITE;
            ev.flags = libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR;
            ev.udata = token as *mut libc::c_void;
            changes.push(ev);
        }

        if !changes.is_empty() {
            let result = unsafe {
                libc::kevent(
                    self.kq,
                    changes.as_ptr(),
                    changes.len() as libc::c_int,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            if result < 0 {
                return Err(RuntimeError::EventLoopStartFailed);
            }
        }

        self.fd_map.insert(token, fd);
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    fn register(&mut self, _token: u64, _fd: i32, _interest: FdInterest) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn deregister(&mut self, token: u64) -> Result<(), RuntimeError> {
        if let Some(fd) = self.fd_map.remove(&token) {
            let mut changes: [libc::kevent; 2] = unsafe { std::mem::zeroed() };

            changes[0].ident = fd as libc::uintptr_t;
            changes[0].filter = libc::EVFILT_READ;
            changes[0].flags = libc::EV_DELETE;

            changes[1].ident = fd as libc::uintptr_t;
            changes[1].filter = libc::EVFILT_WRITE;
            changes[1].flags = libc::EV_DELETE;

            unsafe {
                libc::kevent(
                    self.kq,
                    changes.as_ptr(),
                    2,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                );
            }
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    fn deregister(&mut self, _token: u64) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn poll(&mut self, events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut event_buf: [libc::kevent; 64] = unsafe { std::mem::zeroed() };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let count = unsafe {
            libc::kevent(
                self.kq,
                std::ptr::null(),
                0,
                event_buf.as_mut_ptr(),
                64,
                &timeout,
            )
        };

        if count < 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(0);
            if errno == libc::EINTR {
                return Ok(());
            }
            return Err(RuntimeError::EventLoopStartFailed);
        }

        for ev in &event_buf[..count as usize] {

            if ev.flags & libc::EV_ERROR != 0 {
                continue;
            }

            let token = ev.udata as u64;
            events.push(ReadyEvent {
                token,
                readable: ev.filter == libc::EVFILT_READ,
                writable: ev.filter == libc::EVFILT_WRITE,
            });
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    fn poll(&mut self, _events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        Err(RuntimeError::EventLoopStartFailed)
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
impl Drop for KqueueBackend {
    fn drop(&mut self) {
        if self.kq >= 0 {
            unsafe {
                libc::close(self.kq);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::kqueue::KqueueBackend;

    #[test]
    fn kqueue_initialization_is_target_gated() {
        let mut backend = KqueueBackend::new();
        let mut events = Vec::new();

        let init_result = backend.initialize();
        if cfg!(target_os = "macos") || cfg!(target_os = "freebsd") {
            assert!(init_result.is_ok());
            assert!(backend.poll(&mut events).is_ok());
            assert!(events.is_empty());
        } else {
            assert!(init_result.is_err());
        }
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    #[test]
    fn kqueue_wakes_on_pipe_read() {
        use crate::runtime::backend::FdInterest;

        let mut fds = [0i32; 2];
        let pipe_result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(0, pipe_result);

        let read_fd = fds[0];
        let write_fd = fds[1];

        let mut backend = KqueueBackend::new();
        backend.initialize().expect("init kqueue");

        backend
            .register(
                42,
                read_fd,
                FdInterest {
                    readable: true,
                    writable: false,
                },
            )
            .expect("register");

        let data = [0xABu8; 4];
        let written = unsafe { libc::write(write_fd, data.as_ptr().cast(), data.len()) };
        assert_eq!(4, written);

        let mut events = Vec::new();
        backend.poll(&mut events).expect("poll");

        assert_eq!(1, events.len());
        assert_eq!(42, events[0].token);
        assert!(events[0].readable);
        assert!(!events[0].writable);

        backend.deregister(42).expect("deregister");

        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }
}
