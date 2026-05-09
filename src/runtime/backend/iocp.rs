use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[cfg(target_os = "windows")]
use hashbrown::HashMap;

pub struct IocpBackend {
    #[cfg(target_os = "windows")]
    iocp_handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(target_os = "windows")]
    fd_map: HashMap<u64, i32>,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    initialized: bool,
}

impl IocpBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            iocp_handle: windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
            #[cfg(target_os = "windows")]
            fd_map: HashMap::new(),
            initialized: false,
        }
    }
}

impl Default for IocpBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBackend for IocpBackend {
    fn name(&self) -> &'static str {
        "iocp"
    }

    #[cfg(target_os = "windows")]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::System::IO::CreateIoCompletionPort;

        let handle = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, 0, 0, 1) };
        if handle == 0 {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        self.iocp_handle = handle;
        self.initialized = true;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        Err(RuntimeError::BackendUnavailable {
            backend: String::from("iocp"),
        })
    }

    #[cfg(target_os = "windows")]
    fn register(&mut self, token: u64, fd: i32, _interest: FdInterest) -> Result<(), RuntimeError> {
        use windows_sys::Win32::System::IO::CreateIoCompletionPort;

        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let handle = fd as isize;
        let result = unsafe { CreateIoCompletionPort(handle, self.iocp_handle, token as usize, 0) };
        if result == 0 {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        self.fd_map.insert(token, fd);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn register(
        &mut self,
        _token: u64,
        _fd: i32,
        _interest: FdInterest,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn deregister(&mut self, token: u64) -> Result<(), RuntimeError> {
        self.fd_map.remove(&token);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn deregister(&mut self, _token: u64) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn poll(&mut self, events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        use windows_sys::Win32::System::IO::{GetQueuedCompletionStatusEx, OVERLAPPED_ENTRY};

        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut entries: [OVERLAPPED_ENTRY; 64] = unsafe { std::mem::zeroed() };
        let mut count: u32 = 0;

        let success = unsafe {
            GetQueuedCompletionStatusEx(
                self.iocp_handle,
                entries.as_mut_ptr(),
                64,
                &mut count,
                0,
                0,
            )
        };

        if success == 0 {
            return Ok(());
        }

        for entry in &entries[..count as usize] {
            let token = entry.lpCompletionKey as u64;
            events.push(ReadyEvent {
                token,
                readable: true,
                writable: false,
            });
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn poll(&mut self, _events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        if !self.initialized {
            return Err(RuntimeError::EventLoopStartFailed);
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Drop for IocpBackend {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        if self.iocp_handle != INVALID_HANDLE_VALUE && self.iocp_handle != 0 {
            unsafe {
                CloseHandle(self.iocp_handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::iocp::IocpBackend;

    #[test]
    fn iocp_platform_gated() {
        let mut backend = IocpBackend::new();
        let result = backend.initialize();

        if cfg!(target_os = "windows") {
            assert!(result.is_ok());
            let mut events = Vec::new();
            assert!(backend.poll(&mut events).is_ok());
            assert!(events.is_empty());
        } else {
            assert!(result.is_err());
        }
    }
}
