//! Configuration types for tuning the system.
// Version 1 now exposes FIFO and priority scheduling choices.

/// Scheduler type for the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerKind {
    Fifo,
    Priority,
    RoundRobin,
    Srt,
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
            worker_count: 9, // 多机器人压力测试：默认 9 台 Robot 并发从队列取任务
            demo_task_count: 63,
        }
    }
}
