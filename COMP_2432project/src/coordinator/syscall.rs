//! System call style interface layer.
// Version 1 exposes only a very small API.

use std::sync::Arc;

use crate::coordinator::lifecycle::Coordinator;
use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::AtomicBool;
use crate::worker::lifecycle::PauseController;

/// Run the built-in demo scenario through the coordinator.
pub fn run_demo(
    coordinator: &mut Coordinator,
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    task_table: Arc<TaskTable>,
    task_queue: Arc<ThreadSafeTaskQueue>,
    zone_manager: Arc<ZoneManager>,
    shutdown: Arc<AtomicBool>,
    pause: Arc<PauseController>,
) {
    coordinator.run_demo(
        heartbeats,
        metrics,
        task_table,
        task_queue,
        zone_manager,
        shutdown,
        pause,
    );
}
