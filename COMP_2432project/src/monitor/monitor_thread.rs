//! Background monitoring thread.
//! 周期性地对系统健康状态进行评估，并输出日志，模拟 watchdog / systemd 风格的守护进程。

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::monitor::reporter::build_report;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::RobotId;
use crate::util::logger::log_info;

/// 启动一个长期运行的监控线程。
///
/// - `shutdown` 为 true 时线程会在下一轮循环安全退出。
/// - 线程每 `interval` 秒钟采集一次快照并输出到日志。
pub fn spawn_monitor_thread(
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    shutdown: Arc<AtomicBool>,
    interval: Duration,
    robot_ids: Vec<RobotId>,
) {
    thread::spawn(move || {
        while !shutdown.load(Ordering::SeqCst) {
            let report =
                build_report(heartbeats.as_ref(), metrics.as_ref(), &robot_ids, Duration::from_secs(5));

            log_info(format!(
                "Monitor snapshot: system={:?}, completed_tasks={}",
                report.health.status,
                report.global_metrics.completed_tasks
            ));

            thread::sleep(interval);
        }

        log_info("Monitor thread exiting due to shutdown signal");
    });
}

