use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEntry {
    pub timer_id: u64,
    pub deadline_tick: u64,
}

#[derive(Debug, Default)]
pub struct TimerWheel {
    next_timer_id: u64,
    by_deadline: BTreeMap<u64, Vec<u64>>,
}

impl TimerWheel {
    pub fn new() -> Self {
        Self {
            next_timer_id: 1,
            by_deadline: BTreeMap::new(),
        }
    }

    pub fn schedule(&mut self, deadline_tick: u64) -> u64 {
        let timer_id = self.next_timer_id;
        self.next_timer_id = self.next_timer_id.saturating_add(1);
        self.by_deadline
            .entry(deadline_tick)
            .or_default()
            .push(timer_id);
        timer_id
    }

    pub fn drain_expired(&mut self, now_tick: u64) -> Vec<TimerEntry> {
        let mut expired = Vec::new();

        loop {
            let next = self.by_deadline.first_key_value().map(|(k, _v)| *k);
            let Some(deadline) = next else {
                break;
            };
            if deadline > now_tick {
                break;
            }

            if let Some(timer_ids) = self.by_deadline.remove(&deadline) {
                for timer_id in timer_ids {
                    expired.push(TimerEntry {
                        timer_id,
                        deadline_tick: deadline,
                    });
                }
            }
        }

        expired
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::timer::TimerWheel;

    #[test]
    fn timer_wheel_drains_expired_in_deadline_order() {
        let mut timers = TimerWheel::new();
        let first = timers.schedule(10);
        let second = timers.schedule(20);

        let early = timers.drain_expired(9);
        assert!(early.is_empty());

        let at_ten = timers.drain_expired(10);
        assert_eq!(1, at_ten.len());
        assert_eq!(first, at_ten[0].timer_id);

        let at_twenty = timers.drain_expired(20);
        assert_eq!(1, at_twenty.len());
        assert_eq!(second, at_twenty[0].timer_id);
    }
}
