//! System lifecycle management for the coordinator.
//! Wires together scheduler, workers, and monitoring for demo runs.
//!
//! 可以把 `Coordinator` 看成是本项目里的「内核核心」：
//! - 由 builder 负责初始化调度器、Robot 集合与配置
//! - 在 `run_demo` 中把调度器 + WorkerPool + 监控子系统按顺序串起来
//! - 对外只暴露一个简单的入口，方便 HTTP API 或 examples 直接调用

use std::sync::Arc;

use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::config::Config;
use crate::types::robot::Robot;
use crate::util::logger::log_info;
use crate::worker::lifecycle::PauseController;
use crate::worker::pool::WorkerPool;

/// High-level orchestrator for the demo system.
#[derive(Debug)]
pub struct Coordinator {
    pub config: Config,
    pub robots: Vec<Robot>,
}

impl Coordinator {
    pub fn new(config: Config, robots: Vec<Robot>) -> Self {
        Self { config, robots }
    }

    /// Run a simple end-to-end demo with real multi-threaded workers.
    ///
    /// - Tasks have been inserted into `task_table` and their IDs pushed into `task_queue`.
    /// - One OS thread is spawned per robot, all sharing the same registries and zone manager.
    pub fn run_demo(
        &mut self,
        heartbeats: Arc<HeartbeatRegistry>,
        metrics: Arc<MetricsRegistry>,
        task_table: Arc<TaskTable>,
        task_queue: Arc<ThreadSafeTaskQueue>,
        zone_manager: Arc<ZoneManager>,
        shutdown: Arc<AtomicBool>,
        pause: Arc<PauseController>,
    ) {
        log_info("Starting coordinator demo run");

        // 确保标志位处于运行状态。
        shutdown.store(false, Ordering::SeqCst);
        pause.resume();

        let pool = WorkerPool::new(
            self.robots.clone(),
            task_queue,
            task_table,
            zone_manager,
            heartbeats,
            metrics,
            shutdown,
            pause,
        );
        pool.run_blocking();

        log_info("Coordinator demo run finished");
    }
}
