//! Robot worker implementation.
//! Executes tasks on behalf of a robot and reports heartbeats/metrics.
//!
//! 可以把 `RobotWorker` 理解成「绑定到某个 CPU 的内核线程」：
//! - 从调度器里领取 Task（类似进程/线程调度）
//! - 执行模拟工作负载（sleep）
//! - 在执行前后上报心跳与统计数据，供监控子系统使用

use std::thread;
use std::time::{Duration, Instant};

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::SchedulerStrategy;
use crate::types::error::{Error, Result};
use crate::types::robot::Robot;
use crate::types::task::{Task, TaskStatus};
use crate::worker::state::WorkerState;
use crate::util::logger::log_info;

/// Worker bound to a single robot instance.
#[derive(Debug)]
pub struct RobotWorker<'a> {
    pub robot: Robot,
    pub state: WorkerState,
    scheduler: &'a mut SchedulerStrategy,
    heartbeats: &'a HeartbeatRegistry,
    metrics: &'a MetricsRegistry,
}

impl<'a> RobotWorker<'a> {
    pub fn new(
        robot: Robot,
        scheduler: &'a mut SchedulerStrategy,
        heartbeats: &'a HeartbeatRegistry,
        metrics: &'a MetricsRegistry,
    ) -> Self {
        Self {
            robot,
            state: WorkerState::Idle,
            scheduler,
            heartbeats,
            metrics,
        }
    }

    /// Run a single task if available.
    pub fn run_once(&mut self) -> Result<()> {
        if !self.state.is_active() {
            return Err(Error::WorkerStopped);
        }

        let mut task: Task = self.scheduler.next_task()?;
        self.state = WorkerState::Busy;
        self.heartbeats.touch(self.robot.id);

        log_info(format!(
            "Robot {} starting task {} ({})",
            self.robot.id, task.id, task.name
        ));

        task.status = TaskStatus::Running;

        let start = Instant::now();
        // 按任务声明的执行时间模拟工作负载（约 30s），上限 60s 防止配置错误导致过长阻塞
        let sleep_secs = task.expected_duration.as_secs().min(60);
        thread::sleep(Duration::from_secs(sleep_secs));
        let exec_time = start.elapsed();

        task.status = TaskStatus::Finished;
        self.state = WorkerState::Idle;
        self.heartbeats.touch(self.robot.id);
        self.metrics.record_completion(self.robot.id, exec_time);

        log_info(format!(
            "Robot {} finished task {} in {:?}",
            self.robot.id, task.id, exec_time
        ));
        Ok(())
    }

    pub fn stop(&mut self) {
        self.state = WorkerState::Stopped;
    }
}
