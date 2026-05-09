use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[derive(Debug, Default)]
pub struct EpollBackend {
    initialized: bool,
}

impl EpollBackend {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl RuntimeBackend for EpollBackend {
    fn name(&self) -> &'static str {
        "epoll"
    }

    fn initialize(&mut self) -> Result<(), RuntimeError> {
        self.initialized = true;
        Ok(())
    }

    fn register(&mut self, _token: u64, _fd: i32, _interest: FdInterest) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn deregister(&mut self, _token: u64) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn poll(&mut self, _events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::epoll::EpollBackend;

    #[test]
    fn epoll_requires_initialization_before_poll() {
        let mut backend = EpollBackend::new();
        let mut events = Vec::new();
        assert!(backend.poll(&mut events).is_err());

        assert!(backend.initialize().is_ok());
        assert!(backend.poll(&mut events).is_ok());
        assert!(events.is_empty());
    }
}
