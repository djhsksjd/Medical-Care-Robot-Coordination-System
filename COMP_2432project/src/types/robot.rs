//! Robot-related type definitions.
//! 在本项目中，每个 Robot 可以类比为一颗物理 CPU 或一个执行核心。

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
