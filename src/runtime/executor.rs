use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Executor {
    ready_queue: VecDeque<u64>,
    next_task_id: u64,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            next_task_id: 1,
        }
    }

    pub fn spawn_task(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        self.ready_queue.push_back(task_id);
        task_id
    }

    pub fn schedule_ready(&mut self, token: u64) {
        if !self.ready_queue.contains(&token) {
            self.ready_queue.push_back(token);
        }
    }

    pub fn poll_ready_task(&mut self) -> Option<u64> {
        self.ready_queue.pop_front()
    }

    pub fn pending_tasks(&self) -> usize {
        self.ready_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::executor::Executor;

    #[test]
    fn executor_returns_tasks_in_insert_order() {
        let mut executor = Executor::new();
        let first = executor.spawn_task();
        let second = executor.spawn_task();

        assert_eq!(Some(first), executor.poll_ready_task());
        assert_eq!(Some(second), executor.poll_ready_task());
        assert_eq!(None, executor.poll_ready_task());
    }

    #[test]
    fn schedule_ready_adds_token_to_queue() {
        let mut executor = Executor::new();
        executor.schedule_ready(42);
        executor.schedule_ready(43);
        executor.schedule_ready(42);

        assert_eq!(2, executor.pending_tasks());
        assert_eq!(Some(42), executor.poll_ready_task());
        assert_eq!(Some(43), executor.poll_ready_task());
    }
}
