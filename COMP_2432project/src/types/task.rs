//! Task type definitions representing scheduled work units.
//! 在类 OS 里可以认为是「进程 / 线程」的抽象，后续可以继续扩展字段以模拟更多状态。

use std::time::Duration;

use crate::types::zone::ZoneId;

/// Unique identifier for a task.
pub type TaskId = u64;

/// Simple priority levels. V1 mostly uses `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

/// Execution status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Finished,
}

/// Basic task structure scheduled and executed by workers.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub priority: TaskPriority,
    pub expected_duration: Duration,
    pub status: TaskStatus,
    /// When set, the task can only execute in this specific zone.
    /// The robot will block until capacity is available in this zone.
    pub required_zone: Option<ZoneId>,
}

impl Task {
    /// Create a new pending task with the given id and name.
    pub fn new(id: TaskId, name: impl Into<String>, expected_duration: Duration) -> Self {
        Self {
            id,
            name: name.into(),
            priority: TaskPriority::Normal,
            expected_duration,
            status: TaskStatus::Pending,
            required_zone: None,
        }
    }
}
