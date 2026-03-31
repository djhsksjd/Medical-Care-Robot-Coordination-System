//! Round-robin scheduling policy.
//! The runtime implementation is still constrained by the non-preemptive worker model,
//! so this provides a queue-level RR entry point; the actual time-slice comparison is simulated in the API layer.

use crate::scheduler::queue::TaskQueue;
use crate::types::error::{Error, Result};
use crate::types::task::Task;

/// Queue-level round-robin scheduler.
///
/// With the current non-preemptive worker design, tasks are still dequeued
/// one-by-one, so this runtime scheduler behaves like FIFO. The richer RR
/// time-slice behavior is simulated in the API comparison layer.
#[derive(Debug, Default)]
pub struct RoundRobinScheduler {
    queue: TaskQueue,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            queue: TaskQueue::new(),
        }
    }

    pub fn submit(&mut self, task: Task) {
        self.queue.push(task);
    }

    pub fn next_task(&mut self) -> Result<Task> {
        self.queue.pop().ok_or(Error::SchedulerEmpty)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RoundRobinScheduler;
    use crate::types::task::Task;

    #[test]
    fn preserves_fifo_order_for_runtime_queueing() {
        let mut scheduler = RoundRobinScheduler::new();
        scheduler.submit(Task::new(1, "task-1", Duration::from_secs(5)));
        scheduler.submit(Task::new(2, "task-2", Duration::from_secs(3)));

        assert_eq!(scheduler.next_task().expect("first task").id, 1);
        assert_eq!(scheduler.next_task().expect("second task").id, 2);
        assert!(scheduler.is_empty());
    }
}
