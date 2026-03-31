//! Status reporting utilities.
//! Aggregate metrics and health into a compact snapshot for external consumers.
//!
//! Assembles internal monitoring data (heartbeats + metrics + health) into a compact snapshot
//! for the HTTP API / frontend dashboard, avoiding direct dependency on complex internal structures.

use std::collections::HashMap;
use std::time::Duration;

use crate::monitor::health_checker::{RobotHealth, SystemHealth, evaluate_health};
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::{GlobalMetrics, MetricsRegistry, RobotMetrics};
use crate::types::robot::RobotId;

/// Per-robot monitoring information consumed by the frontend or API.
#[derive(Debug, Clone)]
pub struct RobotReport {
    pub robot_id: RobotId,
    pub health: RobotHealth,
    pub metrics: RobotMetrics,
}

/// System-wide monitoring snapshot.
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
