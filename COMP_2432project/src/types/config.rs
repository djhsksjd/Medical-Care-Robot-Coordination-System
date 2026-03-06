//! Configuration types for tuning the system.
// Version 1 only exposes a minimal demo configuration.

/// Scheduler type for the system. Only `Fifo` is implemented in V1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerKind {
    Fifo,
}

/// Global configuration used by the coordinator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub scheduler: SchedulerKind,
    pub worker_count: usize,
    pub demo_task_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scheduler: SchedulerKind::Fifo,
            worker_count: 2, // 多机器人：默认 2 台 Robot 并发从队列取任务
            demo_task_count: 5,
        }
    }
}
