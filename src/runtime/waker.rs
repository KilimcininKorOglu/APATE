use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct WakerRegistry {
    ready_tokens: HashSet<u64>,
}

impl WakerRegistry {
    pub fn new() -> Self {
        Self {
            ready_tokens: HashSet::new(),
        }
    }

    pub fn wake(&mut self, token: u64) {
        self.ready_tokens.insert(token);
    }

    pub fn drain_ready(&mut self) -> Vec<u64> {
        let mut drained = self.ready_tokens.iter().copied().collect::<Vec<_>>();
        drained.sort_unstable();
        self.ready_tokens.clear();
        drained
    }

    pub fn ready_count(&self) -> usize {
        self.ready_tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::waker::WakerRegistry;

    #[test]
    fn registry_wake_and_drain() {
        let mut registry = WakerRegistry::new();
        registry.wake(11);
        registry.wake(7);
        registry.wake(11);

        assert_eq!(2, registry.ready_count());
        assert_eq!(vec![7, 11], registry.drain_ready());
        assert_eq!(0, registry.ready_count());
    }
}
