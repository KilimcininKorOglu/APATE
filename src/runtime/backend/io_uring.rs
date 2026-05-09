use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, ReadyEvent, RuntimeBackend};

#[cfg(target_os = "linux")]
mod ring {
    use hashbrown::HashMap;

    pub const IORING_OP_POLL_ADD: u8 = 6;
    pub const IORING_ENTER_GETEVENTS: u32 = 1;
    pub const IORING_OFF_SQ_RING: i64 = 0;
    pub const IORING_OFF_CQ_RING: i64 = 0x8000000;
    pub const IORING_OFF_SQES: i64 = 0x10000000;

    const QUEUE_DEPTH: u32 = 32;

    #[repr(C)]
    #[derive(Default)]
    pub struct IoUringParams {
        pub sq_entries: u32,
        pub cq_entries: u32,
        pub flags: u32,
        pub sq_thread_cpu: u32,
        pub sq_thread_idle: u32,
        pub features: u32,
        pub wq_fd: u32,
        pub resv: [u32; 3],
        pub sq_off: SqRingOffsets,
        pub cq_off: CqRingOffsets,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct SqRingOffsets {
        pub head: u32,
        pub tail: u32,
        pub ring_mask: u32,
        pub ring_entries: u32,
        pub flags: u32,
        pub dropped: u32,
        pub array: u32,
        pub resv1: u32,
        pub user_addr: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct CqRingOffsets {
        pub head: u32,
        pub tail: u32,
        pub ring_mask: u32,
        pub ring_entries: u32,
        pub overflow: u32,
        pub cqes: u32,
        pub flags: u32,
        pub resv1: u32,
        pub user_addr: u64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct IoUringSqe {
        pub opcode: u8,
        pub flags: u8,
        pub ioprio: u16,
        pub fd: i32,
        pub off_addr2: u64,
        pub addr_splice: u64,
        pub len: u32,
        pub op_flags: u32,
        pub user_data: u64,
        pub buf_index: u16,
        pub personality: u16,
        pub splice_fd_in: i32,
        pub addr3: u64,
        pub pad2: [u64; 1],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct IoUringCqe {
        pub user_data: u64,
        pub res: i32,
        pub flags: u32,
    }

    pub struct IoUringState {
        pub ring_fd: i32,
        pub sq_ring_ptr: *mut u8,
        pub cq_ring_ptr: *mut u8,
        pub sqes_ptr: *mut IoUringSqe,
        pub sq_ring_sz: usize,
        pub cq_ring_sz: usize,
        pub sqes_sz: usize,
        pub sq_mask: u32,
        pub cq_mask: u32,
        pub sq_head: *mut u32,
        pub sq_tail: *mut u32,
        pub sq_array: *mut u32,
        pub cq_head: *mut u32,
        pub cq_tail: *mut u32,
        pub cqes: *mut IoUringCqe,
        pub fd_map: HashMap<u64, i32>,
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    impl IoUringState {
        pub unsafe fn setup() -> Result<Self, ()> {
            let mut params: IoUringParams = std::mem::zeroed();

            let fd = libc::syscall(
                libc::SYS_io_uring_setup,
                QUEUE_DEPTH as libc::c_int,
                &mut params as *mut IoUringParams,
            ) as i32;

            if fd < 0 {
                return Err(());
            }

            let sq_ring_sz = (params.sq_off.array + params.sq_entries * 4) as usize;
            let cq_ring_sz = (params.cq_off.cqes
                + params.cq_entries * std::mem::size_of::<IoUringCqe>() as u32)
                as usize;
            let sqes_sz = params.sq_entries as usize * std::mem::size_of::<IoUringSqe>();

            let sq_ring_ptr = libc::mmap(
                std::ptr::null_mut(),
                sq_ring_sz,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                IORING_OFF_SQ_RING,
            ) as *mut u8;

            if sq_ring_ptr == libc::MAP_FAILED as *mut u8 {
                libc::close(fd);
                return Err(());
            }

            let cq_ring_ptr = libc::mmap(
                std::ptr::null_mut(),
                cq_ring_sz,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                IORING_OFF_CQ_RING,
            ) as *mut u8;

            if cq_ring_ptr == libc::MAP_FAILED as *mut u8 {
                libc::munmap(sq_ring_ptr.cast(), sq_ring_sz);
                libc::close(fd);
                return Err(());
            }

            let sqes_ptr = libc::mmap(
                std::ptr::null_mut(),
                sqes_sz,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                IORING_OFF_SQES,
            ) as *mut IoUringSqe;

            if sqes_ptr == libc::MAP_FAILED as *mut IoUringSqe {
                libc::munmap(cq_ring_ptr.cast(), cq_ring_sz);
                libc::munmap(sq_ring_ptr.cast(), sq_ring_sz);
                libc::close(fd);
                return Err(());
            }

            let sq_head = sq_ring_ptr.add(params.sq_off.head as usize).cast::<u32>();
            let sq_tail = sq_ring_ptr.add(params.sq_off.tail as usize).cast::<u32>();
            let sq_array = sq_ring_ptr.add(params.sq_off.array as usize).cast::<u32>();
            let sq_mask = sq_ring_ptr
                .add(params.sq_off.ring_mask as usize)
                .cast::<u32>()
                .read_volatile();

            let cq_head = cq_ring_ptr.add(params.cq_off.head as usize).cast::<u32>();
            let cq_tail = cq_ring_ptr.add(params.cq_off.tail as usize).cast::<u32>();
            let cq_mask = cq_ring_ptr
                .add(params.cq_off.ring_mask as usize)
                .cast::<u32>()
                .read_volatile();
            let cqes = cq_ring_ptr
                .add(params.cq_off.cqes as usize)
                .cast::<IoUringCqe>();

            Ok(Self {
                ring_fd: fd,
                sq_ring_ptr,
                cq_ring_ptr,
                sqes_ptr,
                sq_ring_sz,
                cq_ring_sz,
                sqes_sz,
                sq_mask,
                cq_mask,
                sq_head,
                sq_tail,
                sq_array,
                cq_head,
                cq_tail,
                cqes,
                fd_map: HashMap::new(),
            })
        }

        pub unsafe fn submit_poll(&mut self, fd: i32, token: u64, poll_mask: u32) -> bool {
            let tail = self.sq_tail.read_volatile();
            let head = self.sq_head.read_volatile();

            if tail.wrapping_sub(head) >= self.sq_mask + 1 {
                return false;
            }

            let index = tail & self.sq_mask;
            let sqe = &mut *self.sqes_ptr.add(index as usize);
            *sqe = std::mem::zeroed();
            sqe.opcode = IORING_OP_POLL_ADD;
            sqe.fd = fd;
            sqe.op_flags = poll_mask;
            sqe.user_data = token;

            self.sq_array.add(index as usize).write_volatile(index);

            std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
            self.sq_tail.write_volatile(tail.wrapping_add(1));

            true
        }

        pub unsafe fn enter_and_reap(&mut self, events: &mut Vec<super::ReadyEvent>) {
            libc::syscall(
                libc::SYS_io_uring_enter,
                self.ring_fd,
                1u32,
                0u32,
                IORING_ENTER_GETEVENTS,
                std::ptr::null::<libc::c_void>(),
                0usize,
            );

            let mut head = self.cq_head.read_volatile();
            std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
            let tail = self.cq_tail.read_volatile();

            while head != tail {
                let index = head & self.cq_mask;
                let cqe = &*self.cqes.add(index as usize);

                let token = cqe.user_data;
                let res = cqe.res;

                if res >= 0 {
                    let mask = res as u32;
                    events.push(super::ReadyEvent {
                        token,
                        readable: mask & libc::POLLIN as u32 != 0,
                        writable: mask & libc::POLLOUT as u32 != 0,
                    });
                }

                head = head.wrapping_add(1);
            }

            self.cq_head.write_volatile(head);
        }

        pub unsafe fn cleanup(&mut self) {
            if !self.sqes_ptr.is_null() {
                libc::munmap(self.sqes_ptr.cast(), self.sqes_sz);
            }
            if !self.cq_ring_ptr.is_null() {
                libc::munmap(self.cq_ring_ptr.cast(), self.cq_ring_sz);
            }
            if !self.sq_ring_ptr.is_null() {
                libc::munmap(self.sq_ring_ptr.cast(), self.sq_ring_sz);
            }
            if self.ring_fd >= 0 {
                libc::close(self.ring_fd);
            }
        }
    }
}

pub struct IoUringBackend {
    #[cfg(target_os = "linux")]
    state: Option<ring::IoUringState>,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    initialized: bool,
}

impl IoUringBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            state: None,
            initialized: false,
        }
    }
}

impl Default for IoUringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBackend for IoUringBackend {
    fn name(&self) -> &'static str {
        "io_uring"
    }

    #[cfg(target_os = "linux")]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        match unsafe { ring::IoUringState::setup() } {
            Ok(state) => {
                self.state = Some(state);
                self.initialized = true;
                Ok(())
            }
            Err(()) => Err(RuntimeError::BackendUnavailable {
                backend: String::from("io_uring"),
            }),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn initialize(&mut self) -> Result<(), RuntimeError> {
        Err(RuntimeError::BackendUnavailable {
            backend: String::from("io_uring"),
        })
    }

    #[cfg(target_os = "linux")]
    fn register(&mut self, token: u64, fd: i32, interest: FdInterest) -> Result<(), RuntimeError> {
        let state = self
            .state
            .as_mut()
            .ok_or(RuntimeError::EventLoopStartFailed)?;

        let mut poll_mask: u32 = 0;
        if interest.readable {
            poll_mask |= libc::POLLIN as u32;
        }
        if interest.writable {
            poll_mask |= libc::POLLOUT as u32;
        }

        let submitted = unsafe { state.submit_poll(fd, token, poll_mask) };
        if !submitted {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        state.fd_map.insert(token, fd);
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn register(
        &mut self,
        _token: u64,
        _fd: i32,
        _interest: FdInterest,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn deregister(&mut self, token: u64) -> Result<(), RuntimeError> {
        if let Some(state) = self.state.as_mut() {
            state.fd_map.remove(&token);
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn deregister(&mut self, _token: u64) -> Result<(), RuntimeError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn poll(&mut self, events: &mut Vec<ReadyEvent>) -> Result<(), RuntimeError> {
        let state = self
            .state
            .as_mut()
            .ok_or(RuntimeError::EventLoopStartFailed)?;

        unsafe {
            state.enter_and_reap(events);
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
impl Drop for IoUringBackend {
    fn drop(&mut self) {
        if let Some(ref mut state) = self.state {
            unsafe {
                state.cleanup();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::backend::RuntimeBackend;
    use crate::runtime::backend::io_uring::IoUringBackend;

    #[test]
    fn io_uring_platform_gated() {
        let mut backend = IoUringBackend::new();
        let result = backend.initialize();

        if cfg!(target_os = "linux") {
            // may succeed or fail depending on kernel version
            let _ = result;
        } else {
            assert!(result.is_err());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn io_uring_wakes_on_pipe_read() {
        use crate::runtime::backend::FdInterest;

        let mut backend = IoUringBackend::new();
        if backend.initialize().is_err() {
            // kernel too old for io_uring
            return;
        }

        let mut fds = [0i32; 2];
        let pipe_result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(0, pipe_result);

        let read_fd = fds[0];
        let write_fd = fds[1];

        backend
            .register(
                99,
                read_fd,
                FdInterest {
                    readable: true,
                    writable: false,
                },
            )
            .expect("register");

        let data = [0xEFu8; 4];
        let written = unsafe { libc::write(write_fd, data.as_ptr().cast(), data.len()) };
        assert_eq!(4, written);

        let mut events = Vec::new();
        backend.poll(&mut events).expect("poll");

        assert!(!events.is_empty());
        assert_eq!(99, events[0].token);
        assert!(events[0].readable);

        backend.deregister(99).expect("deregister");

        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }
}
