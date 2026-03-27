//! Work-stealing infrastructure for robot workers.
//!
//! Each robot owns a **local deque** of task IDs. When a non-blocking zone
//! allocation fails, the task is pushed to the *back* of the local deque so
//! the robot can try other tasks first. When both the local deque and the
//! global queue are empty, a robot may **steal** from a peer's deque.
//!
//! This mirrors the scheduling model used by Go (GMP), Tokio, and Java's
//! ForkJoinPool — adapted here for zone-aware medical robot coordination.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::types::task::TaskId;

// ─── Per-robot local queue ───────────────────────────────────────────

/// Per-robot local task queue. The owner pops from the **front**; thieves
/// steal from the **back**, minimising contention on the same end.
#[derive(Debug)]
pub struct LocalTaskQueue {
    inner: Mutex<VecDeque<TaskId>>,
}

impl LocalTaskQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
        }
    }

    /// Owner pushes a task that cannot execute right now to the back.
    pub fn push_back(&self, task_id: TaskId) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(task_id);
    }

    /// Owner takes the next task to attempt.
    pub fn pop_front(&self) -> Option<TaskId> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    /// A peer steals from the opposite end to reduce contention.
    pub fn steal(&self) -> Option<TaskId> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_back()
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drain all tasks for zone-aware scanning. The caller inspects each task
    /// and pushes back the ones it cannot execute right now.
    pub fn drain_all(&self) -> Vec<TaskId> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect()
    }
}

// ─── Shared context across all workers ───────────────────────────────

/// Shared context that enables work-stealing across all robot workers.
///
/// Holds every worker's local queue plus a global counter of tasks that
/// have not yet finished. Workers exit when `pending_tasks` reaches zero.
#[derive(Debug)]
pub struct WorkStealingContext {
    local_queues: Vec<Arc<LocalTaskQueue>>,
    pending_tasks: AtomicUsize,
}

impl WorkStealingContext {
    pub fn new(worker_count: usize, total_tasks: usize) -> Self {
        let local_queues = (0..worker_count)
            .map(|_| Arc::new(LocalTaskQueue::new()))
            .collect();
        Self {
            local_queues,
            pending_tasks: AtomicUsize::new(total_tasks),
        }
    }

    pub fn local_queue(&self, worker_index: usize) -> &Arc<LocalTaskQueue> {
        &self.local_queues[worker_index]
    }

    /// Called after a task finishes execution.
    pub fn task_completed(&self) {
        self.pending_tasks.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn pending_tasks(&self) -> usize {
        self.pending_tasks.load(Ordering::SeqCst)
    }

    /// Try to steal a task from any peer's local queue, checking each peer
    /// in round-robin order starting after `my_index`.
    pub fn steal_from_peers(&self, my_index: usize) -> Option<TaskId> {
        let count = self.local_queues.len();
        for offset in 1..count {
            let peer = (my_index + offset) % count;
            if let Some(task_id) = self.local_queues[peer].steal() {
                return Some(task_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_queue_fifo_order() {
        let q = LocalTaskQueue::new();
        q.push_back(1);
        q.push_back(2);
        q.push_back(3);
        assert_eq!(q.pop_front(), Some(1));
        assert_eq!(q.pop_front(), Some(2));
        assert_eq!(q.pop_front(), Some(3));
        assert_eq!(q.pop_front(), None);
    }

    #[test]
    fn steal_takes_from_back() {
        let q = LocalTaskQueue::new();
        q.push_back(1);
        q.push_back(2);
        q.push_back(3);
        assert_eq!(q.steal(), Some(3));
        assert_eq!(q.pop_front(), Some(1));
        assert_eq!(q.steal(), Some(2));
        assert!(q.is_empty());
    }

    #[test]
    fn steal_from_peers_skips_self() {
        let ctx = WorkStealingContext::new(3, 10);
        ctx.local_queue(0).push_back(100);
        ctx.local_queue(1).push_back(200);

        let stolen = ctx.steal_from_peers(0);
        assert_eq!(stolen, Some(200), "should steal from peer 1, not self");
        assert_eq!(ctx.local_queue(0).len(), 1, "own queue untouched");
    }

    #[test]
    fn pending_tasks_counts_down() {
        let ctx = WorkStealingContext::new(2, 5);
        assert_eq!(ctx.pending_tasks(), 5);
        ctx.task_completed();
        ctx.task_completed();
        assert_eq!(ctx.pending_tasks(), 3);
    }
}
