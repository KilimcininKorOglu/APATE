use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[derive(Debug, Default)]
pub struct IocpBackend {
    initialized: bool,
    platform_supported: bool,
}

impl IocpBackend {
    pub fn new() -> Self {
        Self {
            initialized: false,
            platform_supported: cfg!(target_os = "windows"),
        }
    }
}

impl RuntimeBackend for IocpBackend {
    fn name(&self) -> &'static str {
        "iocp"
    }

    fn initialize(&mut self) -> Result<(), RuntimeError> {
        if !self.platform_supported {
            return Err(RuntimeError::BackendUnavailable {
                backend: String::from("iocp"),
            });
        }

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
    use crate::runtime::backend::iocp::IocpBackend;

    #[test]
    fn iocp_backend_reports_platform_support() {
        let mut backend = IocpBackend::new();
        let mut events = Vec::new();

        let init_result = backend.initialize();
        if cfg!(target_os = "windows") {
            assert!(init_result.is_ok());
            assert!(backend.poll(&mut events).is_ok());
            assert!(events.is_empty());
        } else {
            assert!(init_result.is_err());
        }
    }
}
