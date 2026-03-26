//! Smallest remaining time scheduling policy.
//! 当前实现为基于任务预计执行时间的非抢占式最短剩余时间优先：
//! - 预计剩余时间更短的任务优先执行
//! - 若时间相同，则保持较小任务 id 在前，保证行为稳定可预测

use crate::types::error::{Error, Result};
use crate::types::task::Task;

/// Non-preemptive shortest-remaining-time scheduler.
#[derive(Debug, Default)]
pub struct SrtScheduler {
    tasks: Vec<Task>,
}

impl SrtScheduler {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Submit a task into the scheduler.
    pub fn submit(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Fetch the task with the smallest expected remaining duration.
    pub fn next_task(&mut self) -> Result<Task> {
        let Some((index, _)) = self
            .tasks
            .iter()
            .enumerate()
            .min_by_key(|(_, task)| (task.expected_duration, task.id))
        else {
            return Err(Error::SchedulerEmpty);
        };

        Ok(self.tasks.remove(index))
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::SrtScheduler;
    use crate::types::task::{Task, TaskPriority};

    fn make_task(id: u64, secs: u64, priority: TaskPriority) -> Task {
        let mut task = Task::new(id, format!("task-{id}"), Duration::from_secs(secs));
        task.priority = priority;
        task
    }

    #[test]
    fn schedules_shorter_tasks_first() {
        let mut scheduler = SrtScheduler::new();
        scheduler.submit(make_task(1, 10, TaskPriority::Low));
        scheduler.submit(make_task(2, 3, TaskPriority::High));
        scheduler.submit(make_task(3, 6, TaskPriority::Normal));

        assert_eq!(scheduler.next_task().expect("first task").id, 2);
        assert_eq!(scheduler.next_task().expect("second task").id, 3);
        assert_eq!(scheduler.next_task().expect("third task").id, 1);
    }

    #[test]
    fn breaks_ties_by_task_id() {
        let mut scheduler = SrtScheduler::new();
        scheduler.submit(make_task(11, 5, TaskPriority::High));
        scheduler.submit(make_task(10, 5, TaskPriority::Low));

        assert_eq!(scheduler.next_task().expect("first tie task").id, 10);
        assert_eq!(scheduler.next_task().expect("second tie task").id, 11);
    }
}
