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
    /// When `true`, workers use the innovative work-stealing + non-blocking
    /// zone allocation path. When `false`, the classic blocking path is used.
    /// This toggle exists for A/B comparison experiments.
    #[serde(default = "default_use_work_stealing")]
    pub use_work_stealing: bool,
    /// When `true`, run with backend-defined stress-test preset values.
    /// Effective worker/task counts are resolved in coordinator builder.
    #[serde(default = "default_use_stress_preset")]
    pub use_stress_preset: bool,
}

fn default_use_work_stealing() -> bool {
    false
}

fn default_use_stress_preset() -> bool {
    false
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scheduler: SchedulerKind::Fifo,
            worker_count: 9,
            demo_task_count: 20,
            // Default to classic mode so the frontend can explicitly compare both modes via toggle.
            use_work_stealing: false,
            use_stress_preset: false,
        }
    }
}
