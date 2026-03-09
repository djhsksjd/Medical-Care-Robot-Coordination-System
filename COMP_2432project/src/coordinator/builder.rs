//! Builder for assembling a configured coordinator instance.
//! 负责根据 `Config` 构造出一个可运行的 `Coordinator`，
//! 包括：初始化调度器、预填充 Demo 任务、按 `worker_count` 创建多台 Robot。

use crate::coordinator::lifecycle::Coordinator;
use crate::coordinator::task_table::TaskTable;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::scheduler::SchedulerStrategy;
use crate::types::config::Config;
use crate::types::robot::Robot;
use crate::types::task::{Task, TaskPriority};
use crate::util::id_generator::next_task_id;
use std::time::Duration;

/// Fluent builder for configuring and creating a coordinator.
pub struct CoordinatorBuilder {
    config: Config,
}

impl CoordinatorBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn with_demo_defaults() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Build a coordinator and seed the central task structures.
    ///
    /// - Tasks are first inserted into a strategy scheduler so we can honor
    ///   FIFO vs priority policies.
    /// - The resulting ordering is then pushed into the shared `task_queue`
    ///   that workers consume from.
    /// - All task metadata is stored in `task_table` for API and monitoring.
    pub fn build(
        self,
        task_table: &TaskTable,
        task_queue: &ThreadSafeTaskQueue,
    ) -> Coordinator {
        // Set up the configured scheduler and seed it with demo tasks.
        let mut scheduler = SchedulerStrategy::new(self.config.scheduler);

        // 每个任务模拟执行约 30 秒（用于演示多机器人调度与监控）
        let task_duration = Duration::from_secs(30);
        for index in 0..self.config.demo_task_count {
            let mut task =
                Task::new(next_task_id(), format!("demo-task-{index}"), task_duration);
            task.priority = match index % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Normal,
                _ => TaskPriority::Low,
            };

            // Insert into the central task table so lifecycle / API can observe
            // real per-task state and timestamps.
            task_table.insert(task.clone());

            // Also feed into the logical scheduler so we can derive the order
            // tasks will be enqueued to workers.
            scheduler.submit(task);
        }

        // Create multiple robots based on worker_count.
        let mut robots = Vec::with_capacity(self.config.worker_count);
        for i in 0..self.config.worker_count {
            robots.push(Robot::new(i as u64 + 1, format!("robot-{}", i + 1)));
        }

        // Drain the scheduler into the shared task queue in the chosen order.
        while !scheduler.is_empty() {
            if let Ok(task) = scheduler.next_task() {
                if !task_queue.push(task.id) {
                    break;
                }
            } else {
                break;
            }
        }

        Coordinator::new(self.config, robots)
    }
}
