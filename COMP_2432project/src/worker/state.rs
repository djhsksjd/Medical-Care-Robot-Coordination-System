//! Worker state machine definitions.
//! 描述每个 RobotWorker 的生命周期状态，用于简化调度与监控逻辑。

/// High-level worker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Idle,
    Busy,
    Stopped,
}

impl WorkerState {
    pub fn is_active(self) -> bool {
        matches!(self, WorkerState::Idle | WorkerState::Busy)
    }
}
