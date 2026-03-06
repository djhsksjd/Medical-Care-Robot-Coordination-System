//! Heartbeat monitoring implementation.
//! Track per-robot liveness timestamps for health checking.
//!
//! 在类操作系统的类比下，相当于为每个「CPU / Robot」维护一个心跳时间戳，
//! 便于上层的健康检查逻辑判断该机器人是否「离线」或「卡死」。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::sync::rwlock::RwLock;
use crate::types::robot::RobotId;

#[derive(Debug, Default)]
pub struct HeartbeatRegistry {
    inner: RwLock<HashMap<RobotId, Instant>>,
}

impl HeartbeatRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Mark a robot as alive "now".
    pub fn touch(&self, robot_id: RobotId) {
        let mut guard = self.inner.write().expect("heartbeat write lock");
        guard.insert(robot_id, Instant::now());
    }

    /// Get the last-seen timestamp for a robot, if any.
    pub fn last_seen(&self, robot_id: RobotId) -> Option<Instant> {
        let guard = self.inner.read().expect("heartbeat read lock");
        guard.get(&robot_id).cloned()
    }

    /// Return all robots considered stale given a timeout.
    pub fn stale_robots(&self, timeout: Duration) -> Vec<RobotId> {
        let now = Instant::now();
        let guard = self.inner.read().expect("heartbeat read lock");
        guard
            .iter()
            .filter_map(|(id, ts)| {
                if now.duration_since(*ts) > timeout {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }
}
