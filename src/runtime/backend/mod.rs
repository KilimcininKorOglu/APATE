pub mod epoll;
pub mod kqueue;

use crate::RuntimeError;

pub trait RuntimeBackend {
    fn name(&self) -> &'static str;
    fn initialize(&mut self) -> Result<(), RuntimeError>;
    fn poll(&mut self) -> Result<usize, RuntimeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Epoll,
    Kqueue,
}
