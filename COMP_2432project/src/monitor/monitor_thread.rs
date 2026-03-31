//! Background monitoring thread.
//! Periodically evaluates system health and logs results, simulating a watchdog/systemd-style daemon.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::monitor::reporter::build_report;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::RobotId;
use crate::util::logger::log_info;

/// Spawn a long-running monitoring thread.
///
/// - The thread exits safely on the next loop iteration when `shutdown` is set to true.
/// - Takes a snapshot and logs it every `interval`.
pub fn spawn_monitor_thread(
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    shutdown: Arc<AtomicBool>,
    interval: Duration,
    robot_ids: Vec<RobotId>,
) {
    thread::spawn(move || {
        while !shutdown.load(Ordering::SeqCst) {
            let report = build_report(
                heartbeats.as_ref(),
                metrics.as_ref(),
                &robot_ids,
                Duration::from_secs(5),
            );

            log_info(format!(
                "Monitor snapshot: system={:?}, completed_tasks={}",
                report.health.status, report.global_metrics.completed_tasks
            ));

            thread::sleep(interval);
        }

        log_info("Monitor thread exiting due to shutdown signal");
    });
}
