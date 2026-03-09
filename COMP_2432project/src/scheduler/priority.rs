//! Priority-based scheduling policy.
//! 当前实现为「非抢占式优先级调度」：
//! - High 优先于 Normal，Normal 优先于 Low
//! - 同一优先级内部保持 FIFO，便于行为可预测
//! - 预留后续扩展为真正的抢占式调度

use crate::scheduler::queue::TaskQueue;
use crate::types::error::{Error, Result};
use crate::types::task::{Task, TaskPriority};

/// Non-preemptive priority scheduler with FIFO ordering inside each band.
#[derive(Debug, Default)]
pub struct PriorityScheduler {
	high: TaskQueue,
	normal: TaskQueue,
	low: TaskQueue,
}

impl PriorityScheduler {
	pub fn new() -> Self {
		Self {
			high: TaskQueue::new(),
			normal: TaskQueue::new(),
			low: TaskQueue::new(),
		}
	}

	/// Submit a task into the queue that matches its declared priority.
	pub fn submit(&mut self, task: Task) {
		self.queue_for_priority(task.priority).push(task);
	}

	/// Fetch the next task to run, preferring higher priorities first.
	pub fn next_task(&mut self) -> Result<Task> {
		self.high
			.pop()
			.or_else(|| self.normal.pop())
			.or_else(|| self.low.pop())
			.ok_or(Error::SchedulerEmpty)
	}

	pub fn is_empty(&self) -> bool {
		self.high.is_empty() && self.normal.is_empty() && self.low.is_empty()
	}

	fn queue_for_priority(&mut self, priority: TaskPriority) -> &mut TaskQueue {
		match priority {
			TaskPriority::High => &mut self.high,
			TaskPriority::Normal => &mut self.normal,
			TaskPriority::Low => &mut self.low,
		}
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use super::PriorityScheduler;
	use crate::types::task::{Task, TaskPriority};

	fn make_task(id: u64, priority: TaskPriority) -> Task {
		let mut task = Task::new(id, format!("task-{id}"), Duration::from_secs(1));
		task.priority = priority;
		task
	}

	#[test]
	fn schedules_higher_priority_before_lower_priority() {
		let mut scheduler = PriorityScheduler::new();
		scheduler.submit(make_task(1, TaskPriority::Low));
		scheduler.submit(make_task(2, TaskPriority::Normal));
		scheduler.submit(make_task(3, TaskPriority::High));

		assert_eq!(scheduler.next_task().expect("high task").id, 3);
		assert_eq!(scheduler.next_task().expect("normal task").id, 2);
		assert_eq!(scheduler.next_task().expect("low task").id, 1);
	}

	#[test]
	fn preserves_fifo_within_same_priority() {
		let mut scheduler = PriorityScheduler::new();
		scheduler.submit(make_task(10, TaskPriority::High));
		scheduler.submit(make_task(11, TaskPriority::High));
		scheduler.submit(make_task(12, TaskPriority::High));

		assert_eq!(scheduler.next_task().expect("first task").id, 10);
		assert_eq!(scheduler.next_task().expect("second task").id, 11);
		assert_eq!(scheduler.next_task().expect("third task").id, 12);
	}

	#[test]
	fn reports_empty_after_draining_all_queues() {
		let mut scheduler = PriorityScheduler::new();
		scheduler.submit(make_task(1, TaskPriority::Normal));

		assert!(!scheduler.is_empty());
		let _ = scheduler.next_task().expect("queued task");
		assert!(scheduler.is_empty());
		assert!(scheduler.next_task().is_err());
	}
}
