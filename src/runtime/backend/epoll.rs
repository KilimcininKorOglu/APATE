use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[cfg(target_os = "linux")]
use hashbrown::HashMap;

pub struct EpollBackend {
    #[cfg(target_os = "linux")]
    epfd: i32,
    #[cfg(target_os = "linux")]
    fd_map: HashMap<u64, i32>,
    initialized: bool,
}

impl EpollBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            epfd: -1,
            #[cfg(target_os = "linux")]
            fd_map: HashMap::new(),
            initialized: false,
        }
    }
}

impl Default for EpollBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBackend for EpollBackend {
    fn name(&self) -> &'static str {
        "epoll"
    }

    #[cfg(target_os = "linux")]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        let fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if fd < 0 {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        self.epfd = fd;
        self.initialized = true;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        self.initialized = true;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn register(&mut self, token: u64, fd: i32, interest: FdInterest) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut events: u32 = libc::EPOLLET as u32;
        if interest.readable {
            events |= libc::EPOLLIN as u32;
        }
        if interest.writable {
            events |= libc::EPOLLOUT as u32;
        }

        let mut ev = libc::epoll_event { events, u64: token };

        let result = unsafe {
            libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_ADD, fd, &mut ev)
        };
        if result < 0 {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        self.fd_map.insert(token, fd);
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn register(&mut self, _token: u64, _fd: i32, _interest: FdInterest) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn deregister(&mut self, token: u64) -> Result<(), RuntimeError> {
        if let Some(fd) = self.fd_map.remove(&token) {
            unsafe {
                libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn deregister(&mut self, _token: u64) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn poll(&mut self, events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut event_buf: [libc::epoll_event; 64] = unsafe { std::mem::zeroed() };

        let count = unsafe {
            libc::epoll_wait(self.epfd, event_buf.as_mut_ptr(), 64, 0)
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
            events.push(ReadyEvent {
                token: ev.u64,
                readable: ev.events & libc::EPOLLIN as u32 != 0,
                writable: ev.events & libc::EPOLLOUT as u32 != 0,
            });
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn poll(&mut self, _events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for EpollBackend {
    fn drop(&mut self) {
        if self.epfd >= 0 {
            unsafe {
                libc::close(self.epfd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::epoll::EpollBackend;

    #[test]
    fn epoll_initializes_successfully() {
        let mut backend = EpollBackend::new();
        assert!(backend.initialize().is_ok());

        let mut events = Vec::new();
        assert!(backend.poll(&mut events).is_ok());
        assert!(events.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn epoll_wakes_on_pipe_read() {
        use crate::runtime::backend::FdInterest;

        let mut fds = [0i32; 2];
        let pipe_result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(0, pipe_result);

        let read_fd = fds[0];
        let write_fd = fds[1];

        let mut backend = EpollBackend::new();
        backend.initialize().expect("init epoll");

        backend
            .register(
                77,
                read_fd,
                FdInterest {
                    readable: true,
                    writable: false,
                },
            )
            .expect("register");

        let data = [0xCDu8; 4];
        let written = unsafe { libc::write(write_fd, data.as_ptr().cast(), data.len()) };
        assert_eq!(4, written);

        let mut events = Vec::new();
        backend.poll(&mut events).expect("poll");

        assert_eq!(1, events.len());
        assert_eq!(77, events[0].token);
        assert!(events[0].readable);
        assert!(!events[0].writable);

        backend.deregister(77).expect("deregister");

        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }
}
