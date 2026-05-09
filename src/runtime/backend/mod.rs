pub mod epoll;
#[cfg(any(target_os = "linux", test))]
pub mod io_uring;
#[cfg(any(target_os = "windows", test))]
pub mod iocp;
pub mod kqueue;

use crate::RuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyEvent {
    pub token: u64,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FdInterest {
    pub readable: bool,
    pub writable: bool,
}

pub trait RuntimeBackend {
    fn name(&self) -> &'static str;
    fn initialize(&mut self) -> Result<(), RuntimeError>;
    fn register(&mut self, token: u64, fd: i32, interest: FdInterest) -> Result<(), RuntimeError>;
    fn deregister(&mut self, token: u64) -> Result<(), RuntimeError>;
    fn poll(&mut self, events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Epoll,
    IoUring,
    Kqueue,
    Iocp,
}

pub fn select_backend() -> Box<dyn RuntimeBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(epoll::EpollBackend::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(kqueue::KqueueBackend::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(iocp::IocpBackend::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(epoll::EpollBackend::new())
    }
}
