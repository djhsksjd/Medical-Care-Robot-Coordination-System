//! Thread-safe task ID queue for multi-threaded workers.
//! Uses a Mutex + Condvar and a closed flag to support blocking pops and graceful shutdown.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use crate::types::task::TaskId;

#[derive(Debug, Default)]
struct Inner {
    queue: VecDeque<TaskId>,
    closed: bool,
}

#[derive(Debug, Default)]
pub struct ThreadSafeTaskQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
}

impl ThreadSafeTaskQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            cv: Condvar::new(),
        }
    }

    /// Push a task ID into the queue.
    /// Returns false if the queue has been closed and the task was discarded.
    pub fn push(&self, task_id: TaskId) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.closed {
            return false;
        }
        inner.queue.push_back(task_id);
        self.cv.notify_one();
        true
    }

    /// Pop the next task ID, blocking until either:
    /// - a task arrives, or
    /// - the queue is closed and empty (then returns None).
    pub fn pop_blocking(&self) -> Option<TaskId> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(id) = inner.queue.pop_front() {
                return Some(id);
            }
            if inner.closed {
                return None;
            }
            inner = self.cv.wait(inner).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Non-blocking pop: returns `Some(id)` if a task is available, `None` otherwise.
    pub fn try_pop(&self) -> Option<TaskId> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.queue.pop_front()
    }

    /// Close the queue, waking up all waiting workers.
    /// After this, pushes will be ignored and pops will eventually return None.
    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.closed = true;
        self.cv.notify_all();
    }
}
