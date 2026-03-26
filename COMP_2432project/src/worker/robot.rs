//! Robot worker implementation.
//! Executes tasks on behalf of a robot and reports heartbeats/metrics.
//!
//! 可以把 `RobotWorker` 理解成「绑定到某个 CPU 的内核线程」：
//! - 从全局任务队列中阻塞式领取 TaskId
//! - 通过 TaskTable 查找真实 Task 并更新状态
//! - 向 ZoneManager 申请 / 释放区域
//! - 在执行前后上报心跳与统计数据，供监控子系统使用

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::robot::Robot;
use crate::types::task::TaskId;
use crate::util::logger::log_info;

/// Worker bound to a single robot instance.
#[derive(Debug)]
pub struct RobotWorker {
    pub robot: Robot,
    task_queue: Arc<ThreadSafeTaskQueue>,
    task_table: Arc<TaskTable>,
    zone_manager: Arc<ZoneManager>,
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    shutdown: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
}

impl RobotWorker {
    pub fn new(
        robot: Robot,
        task_queue: Arc<ThreadSafeTaskQueue>,
        task_table: Arc<TaskTable>,
        zone_manager: Arc<ZoneManager>,
        heartbeats: Arc<HeartbeatRegistry>,
        metrics: Arc<MetricsRegistry>,
        shutdown: Arc<AtomicBool>,
        pause: Arc<AtomicBool>,
    ) -> Self {
        Self {
            robot,
            task_queue,
            task_table,
            zone_manager,
            heartbeats,
            metrics,
            shutdown,
            pause,
        }
    }

    /// Main worker loop: keep fetching and executing tasks until shutdown or queue is closed.
    pub fn run(self) {
        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                log_info(format!("Robot {} received shutdown signal", self.robot.id));
                break;
            }

            if self.pause.load(Ordering::SeqCst) {
                // 简单的暂停实现：短暂休眠后重试
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let Some(task_id) = self.task_queue.pop_blocking() else {
                // Queue closed and empty.
                log_info(format!(
                    "Robot {} exiting: task queue closed",
                    self.robot.id
                ));
                break;
            };

            self.run_single_task(task_id);
        }
    }

    fn run_single_task(&self, task_id: TaskId) {
        let required_zone = self.task_table.required_zone(task_id);
        let zone_id = self.zone_manager.allocate_for_task(task_id, required_zone);
        let expected = self
            .task_table
            .start_task(task_id, self.robot.id, zone_id)
            .unwrap_or_else(|| Duration::from_secs(30));

        self.heartbeats.touch(self.robot.id);

        log_info(format!(
            "Robot {} starting task {} in zone {}",
            self.robot.id, task_id, zone_id
        ));

        let start = Instant::now();
        // 按任务声明的执行时间模拟工作负载（约 30s），上限 60s 防止配置错误导致过长阻塞
        let sleep_secs = expected.as_secs().min(60);
        thread::sleep(Duration::from_secs(sleep_secs));
        let exec_time = start.elapsed();

        self.task_table.set_finished(task_id);
        self.zone_manager.release_for_task(task_id);
        self.heartbeats.touch(self.robot.id);
        self.metrics.record_completion(self.robot.id, exec_time);

        log_info(format!(
            "Robot {} finished task {} in {:?}",
            self.robot.id, task_id, exec_time
        ));
    }
}
