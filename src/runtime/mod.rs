pub mod executor;
pub mod reactor;
pub mod timer;

use crate::RuntimeError;
use crate::runtime::executor::Executor;
use crate::runtime::reactor::Reactor;
use crate::runtime::timer::TimerWheel;

#[derive(Debug)]
pub struct Runtime {
    pub reactor: Reactor,
    pub executor: Executor,
    pub timer_wheel: TimerWheel,
    running: bool,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            reactor: Reactor::new(),
            executor: Executor::new(),
            timer_wheel: TimerWheel::new(),
            running: false,
        }
    }

    pub fn start(&mut self) -> Result<(), RuntimeError> {
        if self.running {
            return Ok(());
        }

        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
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

    #[test]
    fn runtime_start_stop_cycle() {
        let mut runtime = Runtime::new();
        assert!(!runtime.is_running());

        assert!(runtime.start().is_ok());
        assert!(runtime.is_running());

        runtime.stop();
        assert!(!runtime.is_running());
    }
}
