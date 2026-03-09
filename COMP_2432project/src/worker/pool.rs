//! Worker pool and thread management.
//!
//! 在多线程版本中，WorkerPool 会为每个 Robot 启动一个 OS 线程，
//! 线程内部循环从 `ThreadSafeTaskQueue` 中阻塞式获取任务并执行，直到：
//! - 任务队列被关闭且为空，或
//! - 收到全局 shutdown 信号。

use std::sync::Arc;
use std::thread;

use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::Robot;
use crate::worker::robot::RobotWorker;

/// Worker pool managing multiple robot workers running in parallel threads.
pub struct WorkerPool {
    robots: Vec<Robot>,
    task_queue: Arc<ThreadSafeTaskQueue>,
    task_table: Arc<TaskTable>,
    zone_manager: Arc<ZoneManager>,
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    shutdown: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
}

impl WorkerPool {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        robots: Vec<Robot>,
        task_queue: Arc<ThreadSafeTaskQueue>,
        task_table: Arc<TaskTable>,
        zone_manager: Arc<ZoneManager>,
        heartbeats: Arc<HeartbeatRegistry>,
        metrics: Arc<MetricsRegistry>,
        shutdown: Arc<AtomicBool>,
        pause: Arc<AtomicBool>,
    ) -> Self {
        Self {
            robots,
            task_queue,
            task_table,
            zone_manager,
            heartbeats,
            metrics,
            shutdown,
            pause,
        }
    }

    /// Spawn one OS thread per robot and block until all workers finish.
    pub fn run_blocking(&self) {
        let mut handles = Vec::with_capacity(self.robots.len());

        for robot in &self.robots {
            let worker = RobotWorker::new(
                robot.clone(),
                Arc::clone(&self.task_queue),
                Arc::clone(&self.task_table),
                Arc::clone(&self.zone_manager),
                Arc::clone(&self.heartbeats),
                Arc::clone(&self.metrics),
                Arc::clone(&self.shutdown),
                Arc::clone(&self.pause),
            );

            let handle = thread::spawn(move || {
                worker.run();
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Err(err) = handle.join() {
                eprintln!("Worker thread panicked: {:?}", err);
            }
        }

        // 确保所有工作线程结束后，关闭队列，避免悬挂的阻塞调用。
        if !self.shutdown.load(Ordering::SeqCst) {
            self.task_queue.close();
        }
    }
}
