//! Builder for assembling a configured coordinator instance.
//! 负责根据 `Config` 构造出一个可运行的 `Coordinator`，
//! 包括：预填充 Demo 任务、按 `worker_count` 创建多台 Robot，
//! 并将任务 ID 推入线程安全的全局任务队列。

use crate::coordinator::lifecycle::Coordinator;
use crate::coordinator::task_table::TaskTable;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::types::config::Config;
use crate::types::robot::Robot;
use crate::types::task::Task;
use crate::util::id_generator::next_task_id;
use std::sync::Arc;
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

    pub fn build(
        self,
        task_table: &Arc<TaskTable>,
        task_queue: &Arc<ThreadSafeTaskQueue>,
    ) -> Coordinator {
        // 每个任务模拟执行约 30 秒（用于演示多机器人调度与监控）
        let task_duration = Duration::from_secs(30);
        for _ in 0..self.config.demo_task_count {
            let task = Task::new(next_task_id(), "demo-task", task_duration);
            let id = task.id;
            task_table.insert(task);
            // 将 TaskId 推入线程安全队列，供工作线程并发消费。
            let _ = task_queue.push(id);
        }

        // Create multiple robots based on worker_count.
        let mut robots = Vec::with_capacity(self.config.worker_count);
        for i in 0..self.config.worker_count {
            robots.push(Robot::new(i as u64 + 1, format!("robot-{}", i + 1)));
        }

        Coordinator::new(self.config, robots)
    }
}
