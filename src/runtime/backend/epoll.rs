use crate::RuntimeError;
use crate::runtime::backend::RuntimeBackend;

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

    fn poll(&mut self) -> Result<usize, RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::epoll::EpollBackend;

    #[test]
    fn epoll_requires_initialization_before_poll() {
        let mut backend = EpollBackend::new();
        assert!(backend.poll().is_err());

        assert!(backend.initialize().is_ok());
        assert_eq!(Ok(0), backend.poll());
    }
}
