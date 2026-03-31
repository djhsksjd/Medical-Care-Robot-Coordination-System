//! Robot-related type definitions.
//! In this project, each Robot is analogous to a physical CPU or execution core.

/// Unique identifier for a robot.
pub type RobotId = u64;

/// Minimal robot representation used by workers.
#[derive(Debug, Clone)]
pub struct Robot {
    pub id: RobotId,
    pub name: String,
}

impl Robot {
    pub fn new(id: RobotId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}
