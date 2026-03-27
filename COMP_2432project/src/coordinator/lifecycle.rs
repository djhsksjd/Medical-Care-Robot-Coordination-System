//! System lifecycle management for the coordinator.
//! Wires together scheduler, workers, and monitoring for demo runs.
//!
//! 可以把 `Coordinator` 看成是本项目里的「内核核心」：
//! - 由 builder 负责初始化调度器、Robot 集合与配置
//! - 在 `run_demo` 中把调度器 + WorkerPool + 监控子系统按顺序串起来
//! - 对外只暴露一个简单的入口，方便 HTTP API 或 examples 直接调用

use std::sync::Arc;

use crate::coordinator::builder::{effective_demo_task_count, effective_worker_count};
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
        let mode = if self.config.use_work_stealing {
            "work-stealing"
        } else {
            "classic"
        };
        let worker_count = effective_worker_count(&self.config);
        let task_count = effective_demo_task_count(&self.config);
        log_info(format!(
            "Starting coordinator demo run: mode={mode}, scheduler={:?}, workers={}, tasks={}",
            self.config.scheduler, worker_count, task_count
        ));

        // 确保标志位处于运行状态。
        shutdown.store(false, Ordering::SeqCst);
        pause.resume();

        metrics.mark_demo_start();

        let pool = WorkerPool::new(
            self.robots.clone(),
            task_queue,
            task_table,
            zone_manager,
            heartbeats,
            Arc::clone(&metrics),
            shutdown,
            pause,
            effective_demo_task_count(&self.config),
            self.config.use_work_stealing,
        );
        pool.run_blocking();

        metrics.mark_demo_end();
        let makespan_ms = metrics.makespan_ms();
        let (global_metrics, _) = metrics.snapshot();

        log_info(format!(
            "Coordinator demo run finished: mode={mode}, makespan_ms={makespan_ms}, completed_tasks={}",
            global_metrics.completed_tasks
        ));
    }
}
