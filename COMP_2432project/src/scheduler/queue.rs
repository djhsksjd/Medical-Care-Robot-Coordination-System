//! Core task queue implementation for scheduling.
//! 这是最基础的任务队列结构，为调度器提供 FIFO 语义。

use std::collections::VecDeque;

use crate::types::task::Task;

/// Simple FIFO queue for tasks.
#[derive(Debug, Default)]
pub struct TaskQueue {
    inner: VecDeque<Task>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    /// Push a task to the back of the queue.
    pub fn push(&mut self, task: Task) {
        self.inner.push_back(task);
    }

    /// Pop the next task from the front of the queue.
    pub fn pop(&mut self) -> Option<Task> {
        self.inner.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
