//! Status reporting utilities.
//! Aggregate metrics and health into a compact snapshot for external consumers.
//!
//! 这一层主要负责把内部的监控数据（心跳 + 指标 + 健康状态）整理成一个「拍照快照」，
//! 供上层 HTTP API / 前端 Dashboard 使用，避免前端直接依赖内部复杂的数据结构。

use std::collections::HashMap;
use std::time::Duration;

use crate::monitor::health_checker::{RobotHealth, SystemHealth, evaluate_health};
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::{GlobalMetrics, MetricsRegistry, RobotMetrics};
use crate::types::robot::RobotId;

/// 前端或 API 所关心的「单个机器人」监控信息。
#[derive(Debug, Clone)]
pub struct RobotReport {
    pub robot_id: RobotId,
    pub health: RobotHealth,
    pub metrics: RobotMetrics,
}

/// 整个系统级别的监控快照。
#[derive(Debug, Clone)]
pub struct SystemReport {
    pub health: SystemHealth,
    pub global_metrics: GlobalMetrics,
    pub robots: Vec<RobotReport>,
}

pub fn build_report(
    heartbeats: &HeartbeatRegistry,
    metrics: &MetricsRegistry,
    robot_ids: &[RobotId],
    heartbeat_timeout: Duration,
) -> SystemReport {
    let (global, per_robot) = metrics.snapshot();
    let health = evaluate_health(heartbeats, metrics, robot_ids, heartbeat_timeout);

    let mut robots = Vec::new();
    let per_robot_map: HashMap<RobotId, RobotMetrics> = per_robot;

    for rh in &health.robots {
        let rm = per_robot_map.get(&rh.robot_id).cloned().unwrap_or_default();
        robots.push(RobotReport {
            robot_id: rh.robot_id,
            health: rh.clone(),
            metrics: rm,
        });
    }

    SystemReport {
        health,
        global_metrics: global,
        robots,
    }
}
