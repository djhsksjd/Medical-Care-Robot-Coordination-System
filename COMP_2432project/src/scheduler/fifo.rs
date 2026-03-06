//! FIFO scheduling policy.
//! 最简单的调度策略：严格先来先服务，不考虑优先级与时间片。

use crate::scheduler::queue::TaskQueue;
use crate::types::error::{Error, Result};
use crate::types::task::Task;

/// Simple FIFO scheduler that owns a task queue.
#[derive(Debug, Default)]
pub struct FifoScheduler {
    queue: TaskQueue,
}

impl FifoScheduler {
    pub fn new() -> Self {
        Self {
            queue: TaskQueue::new(),
        }
    }

    /// Submit a task into the scheduler.
    pub fn submit(&mut self, task: Task) {
        self.queue.push(task);
    }

    /// Fetch the next task to run.
    pub fn next_task(&mut self) -> Result<Task> {
        self.queue.pop().ok_or(Error::SchedulerEmpty)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
