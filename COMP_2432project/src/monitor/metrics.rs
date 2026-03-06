//! Metrics collection for monitoring.
//! Track basic per-robot and global scheduler metrics.
//!
//! 在类操作系统语义下，可以把这里看成是 /proc 里导出的统计信息：
//! - RobotMetrics ≈ 每个 CPU 的运行统计（完成任务数、累计运行时间）
//! - GlobalMetrics ≈ 整个系统级别的吞吐与平均延迟

use std::collections::HashMap;
use std::time::Duration;

use crate::sync::mutex::Mutex;
use crate::types::robot::RobotId;

/// 每个机器人的本地统计信息，类似「某个 CPU 上完成了多少个进程」。
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

/// 全局统计信息，聚合所有机器人执行情况。
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

#[derive(Debug, Default)]
pub struct MetricsRegistry {
    robots: Mutex<HashMap<RobotId, RobotMetrics>>,
    global: Mutex<GlobalMetrics>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            robots: Mutex::new(HashMap::new()),
            global: Mutex::new(GlobalMetrics::default()),
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

    pub fn snapshot(&self) -> (GlobalMetrics, HashMap<RobotId, RobotMetrics>) {
        let global = self.global.lock().expect("metrics global lock").clone();
        let robots = self.robots.lock().expect("metrics robots lock").clone();
        (global, robots)
    }
}
