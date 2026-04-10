use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactorEvent {
    Readable { token: u64 },
    Writable { token: u64 },
    TimerFired { timer_id: u64 },
}

#[derive(Debug, Default)]
pub struct Reactor {
    pending_events: VecDeque<ReactorEvent>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            pending_events: VecDeque::new(),
        }
    }

    pub fn push_event(&mut self, event: ReactorEvent) {
        self.pending_events.push_back(event);
    }

    pub fn pop_event(&mut self) -> Option<ReactorEvent> {
        self.pending_events.pop_front()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::reactor::{Reactor, ReactorEvent};

    #[test]
    fn reactor_behaves_fifo() {
        let mut reactor = Reactor::new();
        reactor.push_event(ReactorEvent::Readable { token: 1 });
        reactor.push_event(ReactorEvent::Writable { token: 2 });

        assert_eq!(2, reactor.pending_count());
        assert_eq!(
            Some(ReactorEvent::Readable { token: 1 }),
            reactor.pop_event()
        );
        assert_eq!(
            Some(ReactorEvent::Writable { token: 2 }),
            reactor.pop_event()
        );
        assert_eq!(None, reactor.pop_event());
    }
}
