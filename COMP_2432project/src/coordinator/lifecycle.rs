//! System lifecycle management for the coordinator.
//! Wires together scheduler, workers, and monitoring for demo runs.
//!
//! 可以把 `Coordinator` 看成是本项目里的「内核核心」：
//! - 由 builder 负责初始化调度器、Robot 集合与配置
//! - 在 `run_demo` 中把调度器 + WorkerPool + 监控子系统按顺序串起来
//! - 对外只暴露一个简单的入口，方便 HTTP API 或 examples 直接调用

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::fifo::FifoScheduler;
use crate::types::config::Config;
use crate::types::robot::Robot;
use crate::util::logger::log_info;
use crate::worker::pool::WorkerPool;

/// High-level orchestrator for the demo system.
#[derive(Debug)]
pub struct Coordinator {
    pub config: Config,
    pub scheduler: FifoScheduler,
    pub robots: Vec<Robot>,
}

impl Coordinator {
    pub fn new(config: Config, scheduler: FifoScheduler, robots: Vec<Robot>) -> Self {
        Self {
            config,
            scheduler,
            robots,
        }
    }

    /// Run a simple end-to-end demo:
    /// - Seeded tasks are already in the scheduler (via builder).
    /// - Workers run tasks until the queue is empty.
    pub fn run_demo(mut self) {
        log_info("Starting coordinator demo run");

        let heartbeats = HeartbeatRegistry::new();
        let metrics = MetricsRegistry::new();

        {
            let mut pool = WorkerPool::new(
                self.robots.clone(),
                &mut self.scheduler,
                &heartbeats,
                &metrics,
            );
            pool.run_until_empty();
        }

        log_info("Coordinator demo run finished");
    }
}

