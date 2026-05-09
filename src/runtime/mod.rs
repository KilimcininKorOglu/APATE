pub mod backend;
pub mod executor;
pub mod reactor;
pub mod timer;
pub mod waker;

use crate::RuntimeError;
use crate::runtime::backend::{FdInterest, RuntimeBackend, select_backend};
use crate::runtime::executor::Executor;
use crate::runtime::reactor::{Reactor, ReactorEvent};
use crate::runtime::timer::TimerWheel;
use crate::runtime::waker::WakerRegistry;

pub struct Runtime {
    pub reactor: Reactor,
    pub executor: Executor,
    pub timer_wheel: TimerWheel,
    pub waker: WakerRegistry,
    backend: Box<dyn RuntimeBackend>,
    running: bool,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            reactor: Reactor::new(),
            executor: Executor::new(),
            timer_wheel: TimerWheel::new(),
            waker: WakerRegistry::new(),
            backend: select_backend(),
            running: false,
        }
    }

    pub fn start(&mut self) -> Result<(), RuntimeError> {
        if self.running {
            return Ok(());
        }

        self.backend.initialize()?;
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    pub fn register_fd(
        &mut self,
        token: u64,
        fd: i32,
        interest: FdInterest,
    ) -> Result<(), RuntimeError> {
        self.backend.register(token, fd, interest)
    }

    pub fn deregister_fd(&mut self, token: u64) -> Result<(), RuntimeError> {
        self.backend.deregister(token)
    }

    pub fn tick(&mut self) -> Result<(), RuntimeError> {
        let now = self.timer_wheel.now_ms();
        self.run_once(now)
    }

    pub fn run_once(&mut self, now_tick: u64) -> Result<(), RuntimeError> {
        if !self.running {
            return Err(RuntimeError::EventLoopStartFailed);
        }

        let mut events = Vec::new();
        self.backend.poll(&mut events)?;
        for ev in events {
            if ev.readable {
                self.reactor
                    .push_event(ReactorEvent::Readable { token: ev.token });
            }
            if ev.writable {
                self.reactor
                    .push_event(ReactorEvent::Writable { token: ev.token });
            }
        }

        for entry in self.timer_wheel.drain_expired(now_tick) {
            self.reactor.push_event(ReactorEvent::TimerFired {
                timer_id: entry.timer_id,
            });
        }

        while let Some(event) = self.reactor.pop_event() {
            let token = match event {
                ReactorEvent::Readable { token } | ReactorEvent::Writable { token } => token,
                ReactorEvent::TimerFired { timer_id } => timer_id,
            };
            self.waker.wake(token);
        }

        for token in self.waker.drain_ready() {
            self.executor.schedule_ready(token);
        }

        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use crate::runtime::reactor::ReactorEvent;

    #[test]
    fn runtime_start_stop_cycle() {
        let mut runtime = Runtime::new();
        assert!(!runtime.is_running());

        assert!(runtime.start().is_ok());
        assert!(runtime.is_running());

        runtime.stop();
        assert!(!runtime.is_running());
    }

    #[test]
    fn run_once_requires_running_state() {
        let mut runtime = Runtime::new();
        assert!(runtime.run_once(0).is_err());
    }

    #[test]
    fn run_once_drains_timers_into_executor() {
        let mut runtime = Runtime::new();
        runtime.start().expect("start");

        let timer_id = runtime.timer_wheel.schedule(10);
        runtime.run_once(10).expect("tick");

        assert_eq!(Some(timer_id), runtime.executor.poll_ready_task());
    }

    #[test]
    fn run_once_drains_reactor_events_into_executor() {
        let mut runtime = Runtime::new();
        runtime.start().expect("start");

        runtime
            .reactor
            .push_event(ReactorEvent::Readable { token: 99 });
        runtime.run_once(0).expect("tick");

        assert_eq!(Some(99), runtime.executor.poll_ready_task());
    }

    #[test]
    fn backend_name_returns_platform_backend() {
        let runtime = Runtime::new();
        let name = runtime.backend_name();
        assert!(!name.is_empty());
    }
}
