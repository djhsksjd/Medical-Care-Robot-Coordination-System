//! Worker pool and thread management.
//!
//! `WorkerPool` spawns one OS thread per robot. Each worker participates in
//! a **work-stealing** scheme: tasks flow from the global queue into per-robot
//! local deques, and idle robots steal from busy peers.

use std::sync::Arc;
use std::thread;

use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::scheduler::work_stealing::WorkStealingContext;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::Robot;
use crate::worker::lifecycle::PauseController;
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
    pause: Arc<PauseController>,
    ws_context: Arc<WorkStealingContext>,
    use_work_stealing: bool,
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
        pause: Arc<PauseController>,
        total_tasks: usize,
        use_work_stealing: bool,
    ) -> Self {
        let ws_context = Arc::new(WorkStealingContext::new(robots.len(), total_tasks));
        Self {
            robots,
            task_queue,
            task_table,
            zone_manager,
            heartbeats,
            metrics,
            shutdown,
            pause,
            ws_context,
            use_work_stealing,
        }
    }

    /// Spawn one OS thread per robot and block until all workers finish.
    pub fn run_blocking(&self) {
        let mut handles = Vec::with_capacity(self.robots.len());

        for (index, robot) in self.robots.iter().enumerate() {
            let worker = RobotWorker::new(
                robot.clone(),
                index,
                Arc::clone(&self.task_queue),
                Arc::clone(&self.task_table),
                Arc::clone(&self.zone_manager),
                Arc::clone(&self.heartbeats),
                Arc::clone(&self.metrics),
                Arc::clone(&self.shutdown),
                Arc::clone(&self.pause),
                Arc::clone(&self.ws_context),
                self.use_work_stealing,
            );

            let handle = thread::spawn(move || {
                worker.run();
            });
            handles.push(handle);
        }

        // Close the global queue so workers know no new tasks will arrive.
        // They rely on `pending_tasks == 0` for the actual exit condition.
        if !self.shutdown.load(Ordering::SeqCst) {
            self.task_queue.close();
        }

        for handle in handles {
            if let Err(err) = handle.join() {
                eprintln!("Worker thread panicked: {:?}", err);
            }
        }
    }
}
