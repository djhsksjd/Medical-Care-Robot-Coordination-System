//! Worker state machine definitions.
//! Describes each RobotWorker's lifecycle states to simplify scheduling and monitoring logic.

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
