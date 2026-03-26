//! Health checker for system components.
//! Evaluate per-robot and global health based on heartbeats and metrics.
//!
//! 可以把这一层理解成 OS 中「调度器 / 监控守护进程」对 CPU 和系统整体健康状态的判定：
//! - 结合 `HeartbeatRegistry` 判断某个 Robot 是否长时间无心跳（Unreachable）
//! - 结合 `MetricsRegistry` 的全局统计，在没有任何执行数据时把系统视为 Degraded

use std::time::Duration;

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::types::robot::RobotId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct RobotHealth {
    pub robot_id: RobotId,
    pub status: RobotHealthStatus,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub status: SystemHealthStatus,
    pub robots: Vec<RobotHealth>,
}

pub fn evaluate_health(
    heartbeats: &HeartbeatRegistry,
    metrics: &MetricsRegistry,
    robot_ids: &[RobotId],
    heartbeat_timeout: Duration,
) -> SystemHealth {
    let (global, _robot_metrics) = metrics.snapshot();
    let mut robots_health = Vec::new();
    let mut has_unreachable = false;
    let mut has_degraded = false;

    for &id in robot_ids {
        let last_seen = heartbeats.last_seen(id);
        let (status, reason) = match last_seen {
            None => (
                RobotHealthStatus::Unreachable,
                "no heartbeat recorded".to_string(),
            ),
            Some(ts) => {
                let age = ts.elapsed();
                if age > heartbeat_timeout {
                    (
                        RobotHealthStatus::Unreachable,
                        format!("last heartbeat {age:?} ago"),
                    )
                } else {
                    // For now we treat recent heartbeat as healthy; metrics-based
                    // degradation can be added later.
                    (RobotHealthStatus::Healthy, "heartbeat ok".to_string())
                }
            }
        };
        if status == RobotHealthStatus::Unreachable {
            has_unreachable = true;
        } else if status == RobotHealthStatus::Degraded {
            has_degraded = true;
        }
        robots_health.push(RobotHealth {
            robot_id: id,
            status,
            reason,
        });
    }

    let system_status = if has_unreachable {
        SystemHealthStatus::Unhealthy
    } else if has_degraded {
        SystemHealthStatus::Degraded
    } else if global.completed_tasks == 0 {
        // No data yet; treat as degraded rather than healthy.
        SystemHealthStatus::Degraded
    } else {
        SystemHealthStatus::Healthy
    };

    SystemHealth {
        status: system_status,
        robots: robots_health,
    }
}
