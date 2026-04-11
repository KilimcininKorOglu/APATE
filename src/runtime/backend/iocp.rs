use crate::RuntimeError;
use crate::runtime::backend::RuntimeBackend;

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
    use crate::runtime::backend::iocp::IocpBackend;

    #[test]
    fn iocp_backend_reports_platform_support() {
        let mut backend = IocpBackend::new();

        let init_result = backend.initialize();
        if cfg!(target_os = "windows") {
            assert!(init_result.is_ok());
            assert_eq!(Ok(0), backend.poll());
        } else {
            assert!(init_result.is_err());
        }
    }
}
