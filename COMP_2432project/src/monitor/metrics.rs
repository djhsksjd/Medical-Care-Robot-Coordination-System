//! Metrics collection for monitoring.
//! Track basic per-robot and global scheduler metrics.
//!
//! In OS-like semantics, think of this as the statistics exported via /proc:
//! - RobotMetrics ~ per-CPU execution stats (completed tasks, cumulative run time)
//! - GlobalMetrics ~ system-wide throughput and average latency

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::sync::mutex::Mutex;
use crate::types::robot::RobotId;
use crate::types::zone::ZoneId;

/// Per-robot local statistics, analogous to how many processes a given CPU has completed.
#[derive(Debug, Default, Clone)]
pub struct RobotMetrics {
    pub completed_tasks: u64,
    pub total_exec_time: Duration,
}

impl RobotMetrics {
    pub fn record_completion(&mut self, exec_time: Duration) {
        self.completed_tasks += 1;
        self.total_exec_time += exec_time;
    }

    pub fn avg_exec_time(&self) -> Option<Duration> {
        if self.completed_tasks == 0 {
            None
        } else {
            Some(self.total_exec_time / (self.completed_tasks as u32))
        }
    }
}

/// Global statistics aggregating execution data from all robots.
#[derive(Debug, Default, Clone)]
pub struct GlobalMetrics {
    pub completed_tasks: u64,
    pub total_exec_time: Duration,
}

impl GlobalMetrics {
    pub fn record_completion(&mut self, exec_time: Duration) {
        self.completed_tasks += 1;
        self.total_exec_time += exec_time;
    }

    pub fn avg_exec_time(&self) -> Option<Duration> {
        if self.completed_tasks == 0 {
            None
        } else {
            Some(self.total_exec_time / (self.completed_tasks as u32))
        }
    }
}

#[derive(Debug)]
pub struct MetricsRegistry {
    robots: Mutex<HashMap<RobotId, RobotMetrics>>,
    global: Mutex<GlobalMetrics>,
    demo_start: Mutex<Option<Instant>>,
    demo_end: Mutex<Option<Instant>>,
    /// Tracks each robot's last executed zone for zone-switch detection.
    last_zone: Mutex<HashMap<RobotId, ZoneId>>,
    /// Per-zone count of robots switching *out* (robot was here, then went elsewhere).
    zone_switches: Mutex<HashMap<ZoneId, u64>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            robots: Mutex::new(HashMap::new()),
            global: Mutex::new(GlobalMetrics::default()),
            demo_start: Mutex::new(None),
            demo_end: Mutex::new(None),
            last_zone: Mutex::new(HashMap::new()),
            zone_switches: Mutex::new(HashMap::new()),
        }
    }

    pub fn mark_demo_start(&self) {
        *self.demo_start.lock().expect("demo_start lock") = Some(Instant::now());
    }

    pub fn mark_demo_end(&self) {
        *self.demo_end.lock().expect("demo_end lock") = Some(Instant::now());
    }

    /// Wall-clock duration of the demo run in milliseconds, or 0 if not yet
    /// started / still running.
    pub fn makespan_ms(&self) -> u64 {
        let start = *self.demo_start.lock().expect("demo_start lock");
        let end = *self.demo_end.lock().expect("demo_end lock");
        match (start, end) {
            (Some(s), Some(e)) => e.duration_since(s).as_millis() as u64,
            (Some(s), None) => s.elapsed().as_millis() as u64,
            _ => 0,
        }
    }

    pub fn record_completion(&self, robot_id: RobotId, exec_time: Duration) {
        {
            let mut robots = self.robots.lock().expect("metrics robots lock");
            let entry = robots.entry(robot_id).or_default();
            entry.record_completion(exec_time);
        }
        {
            let mut global = self.global.lock().expect("metrics global lock");
            global.record_completion(exec_time);
        }
    }

    /// Record that a robot is executing a task in `zone_id`. If the robot's
    /// previous task was in a *different* zone, count a zone-switch-out for
    /// the **previous** zone.
    pub fn record_zone_execution(&self, robot_id: RobotId, zone_id: ZoneId) {
        let mut last = self.last_zone.lock().expect("last_zone lock");
        if let Some(&prev_zone) = last.get(&robot_id)
            && prev_zone != zone_id
        {
            let mut zs = self.zone_switches.lock().expect("zone_switches lock");
            *zs.entry(prev_zone).or_insert(0) += 1;
        }
        last.insert(robot_id, zone_id);
    }

    pub fn zone_switch_snapshot(&self) -> HashMap<ZoneId, u64> {
        self.zone_switches.lock().expect("zone_switches lock").clone()
    }

    pub fn snapshot(&self) -> (GlobalMetrics, HashMap<RobotId, RobotMetrics>) {
        let global = self.global.lock().expect("metrics global lock").clone();
        let robots = self.robots.lock().expect("metrics robots lock").clone();
        (global, robots)
    }
}
