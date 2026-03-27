//! Robot worker implementation with optional zone-aware work stealing.
//!
//! Two execution modes controlled by `use_work_stealing`:
//!
//! **Innovative mode** (work stealing ON):
//! 1. Takes a task from its **local queue** first (cheapest path).
//! 2. Falls back to the **global queue** (non-blocking) if local is empty.
//! 3. **Steals** from a peer's local queue as a last resort.
//! 4. Attempts a **non-blocking** zone allocation.
//!    - Success → execute the task.
//!    - Failure → push the task to the *back* of the local queue.
//!
//! **Classic mode** (work stealing OFF):
//! 1. Blocking pop from the global task queue.
//! 2. Blocking zone allocation (waits on Condvar).
//! 3. Zone failure → task marked failed immediately.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::{ZoneLease, ZoneManager};
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::scheduler::work_stealing::WorkStealingContext;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::Robot;
use crate::types::task::TaskId;
use crate::util::logger::log_info;
use crate::worker::lifecycle::PauseController;

const IDLE_BACKOFF: Duration = Duration::from_millis(50);

/// Worker bound to a single robot instance.
#[derive(Debug)]
pub struct RobotWorker {
    pub robot: Robot,
    worker_index: usize,
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

impl RobotWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        robot: Robot,
        worker_index: usize,
        task_queue: Arc<ThreadSafeTaskQueue>,
        task_table: Arc<TaskTable>,
        zone_manager: Arc<ZoneManager>,
        heartbeats: Arc<HeartbeatRegistry>,
        metrics: Arc<MetricsRegistry>,
        shutdown: Arc<AtomicBool>,
        pause: Arc<PauseController>,
        ws_context: Arc<WorkStealingContext>,
        use_work_stealing: bool,
    ) -> Self {
        Self {
            robot,
            worker_index,
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

    /// Entry point: dispatch to the matching execution mode.
    pub fn run(self) {
        if self.use_work_stealing {
            self.run_work_stealing();
        } else {
            self.run_classic();
        }
    }

    /// Classic blocking mode (baseline for comparison).
    fn run_classic(self) {
        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                log_info(format!("Robot {} received shutdown signal", self.robot.id));
                break;
            }

            if self.pause.is_paused() {
                self.pause.wait_while_paused();
                continue;
            }

            let Some(task_id) = self.task_queue.pop_blocking() else {
                log_info(format!(
                    "Robot {} exiting: task queue closed",
                    self.robot.id
                ));
                break;
            };

            let required_zone = self.task_table.required_zone(task_id);
            match self.zone_manager.lease_for_task(task_id, required_zone) {
                Ok(lease) => {
                    self.execute_task(task_id, lease);
                }
                Err(err) => {
                    log_info(format!(
                        "Robot {} failed to allocate zone for task {}: {err}",
                        self.robot.id, task_id
                    ));
                    self.task_table.set_failed(task_id);
                    self.ws_context.task_completed();
                }
            }
        }
    }

    /// Innovative mode: zone-aware work stealing + non-blocking allocation.
    ///
    /// Unlike the classic path which blocks on a single zone, this mode
    /// **scans the entire local queue** to find a task whose target zone has
    /// available capacity. This prevents a robot from stalling on a full
    /// zone while tasks for other zones sit idle in its queue.
    fn run_work_stealing(self) {
        let local_queue = Arc::clone(self.ws_context.local_queue(self.worker_index));

        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                log_info(format!("Robot {} received shutdown signal", self.robot.id));
                break;
            }

            if self.pause.is_paused() {
                self.pause.wait_while_paused();
                continue;
            }

            // ── Phase 1: Zone-aware scan of local queue ───────────────
            // Drain all tasks, try non-blocking zone allocation for each,
            // execute the first one that succeeds, push the rest back.
            let mut found_local = None;
            {
                let tasks = local_queue.drain_all();
                if !tasks.is_empty() {
                    let mut remaining = Vec::with_capacity(tasks.len());
                    for task_id in tasks {
                        if found_local.is_some() {
                            remaining.push(task_id);
                            continue;
                        }
                        let required_zone = self.task_table.required_zone(task_id);
                        if let Some(lease) =
                            self.zone_manager.try_lease_for_task(task_id, required_zone)
                        {
                            found_local = Some((task_id, lease));
                        } else {
                            remaining.push(task_id);
                        }
                    }
                    for task_id in remaining {
                        local_queue.push_back(task_id);
                    }
                }
            }

            if let Some((task_id, lease)) = found_local {
                self.execute_task(task_id, lease);
                continue;
            }

            // ── Phase 2: Try global queue ─────────────────────────────
            if let Some(task_id) = self.task_queue.try_pop() {
                let required_zone = self.task_table.required_zone(task_id);
                if let Some(lease) =
                    self.zone_manager.try_lease_for_task(task_id, required_zone)
                {
                    self.execute_task(task_id, lease);
                } else {
                    local_queue.push_back(task_id);
                }
                continue;
            }

            // ── Phase 3: Steal from a peer ────────────────────────────
            if let Some(task_id) = self.ws_context.steal_from_peers(self.worker_index) {
                let required_zone = self.task_table.required_zone(task_id);
                if let Some(lease) =
                    self.zone_manager.try_lease_for_task(task_id, required_zone)
                {
                    self.execute_task(task_id, lease);
                } else {
                    local_queue.push_back(task_id);
                }
                continue;
            }

            // ── Nothing available anywhere ────────────────────────────
            if self.ws_context.pending_tasks() == 0 {
                log_info(format!(
                    "Robot {} exiting: all tasks completed",
                    self.robot.id
                ));
                break;
            }
            thread::sleep(IDLE_BACKOFF);
        }
    }

    fn execute_task(&self, task_id: TaskId, lease: ZoneLease) {
        let expected = self
            .task_table
            .start_task(task_id, self.robot.id, lease.zone_id)
            .unwrap_or_else(|| Duration::from_secs(30));

        self.metrics.record_zone_execution(self.robot.id, lease.zone_id);
        self.heartbeats.touch(self.robot.id);

        log_info(format!(
            "Robot {} starting task {} in zone {}",
            self.robot.id, task_id, lease.zone_id
        ));

        let start = Instant::now();
        let sleep_secs = expected.as_secs().min(60);
        thread::sleep(Duration::from_secs(sleep_secs));
        let exec_time = start.elapsed();

        self.task_table.set_finished(task_id);
        lease.release();
        self.ws_context.task_completed();
        self.heartbeats.touch(self.robot.id);
        self.metrics.record_completion(self.robot.id, exec_time);

        log_info(format!(
            "Robot {} finished task {} in {:?}",
            self.robot.id, task_id, exec_time
        ));
    }
}
