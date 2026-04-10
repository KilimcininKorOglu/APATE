use crate::RuntimeError;
use crate::runtime::backend::RuntimeBackend;

#[derive(Debug, Default)]
pub struct KqueueBackend {
    initialized: bool,
}

impl KqueueBackend {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl RuntimeBackend for KqueueBackend {
    fn name(&self) -> &'static str {
        "kqueue"
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
    use crate::runtime::backend::kqueue::KqueueBackend;

    #[test]
    fn kqueue_requires_initialization_before_poll() {
        let mut backend = KqueueBackend::new();
        assert!(backend.poll().is_err());

        assert!(backend.initialize().is_ok());
        assert_eq!(Ok(0), backend.poll());
    }
}
