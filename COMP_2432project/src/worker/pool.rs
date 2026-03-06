//! Worker pool and thread management.
//! For now we provide a simple single-threaded pool that iterates robots.
//!
//! 这里相当于一个非常简化的「调度循环」：
//! - 持有一组逻辑 CPU（Robot）
//! - 在一个循环里依次让每个 RobotWorker 从调度器里领取任务并执行
//! - 直到本轮所有 Robot 都拿不到任务（没有进度），认为调度队列已空，循环结束

use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::scheduler::fifo::FifoScheduler;
use crate::types::robot::Robot;
use crate::worker::robot::RobotWorker;

/// Simple worker pool managing multiple robot workers.
pub struct WorkerPool<'a> {
    robots: Vec<Robot>,
    scheduler: &'a mut FifoScheduler,
    heartbeats: &'a HeartbeatRegistry,
    metrics: &'a MetricsRegistry,
}

impl<'a> WorkerPool<'a> {
    pub fn new(
        robots: Vec<Robot>,
        scheduler: &'a mut FifoScheduler,
        heartbeats: &'a HeartbeatRegistry,
        metrics: &'a MetricsRegistry,
    ) -> Self {
        Self {
            robots,
            scheduler,
            heartbeats,
            metrics,
        }
    }

    /// Run all workers until the scheduler queue is empty.
    /// This is a cooperative, single-threaded loop to keep the model simple.
    pub fn run_until_empty(&mut self) {
        loop {
            let mut made_progress = false;
            for robot in &self.robots {
                let mut worker = RobotWorker::new(
                    robot.clone(),
                    self.scheduler,
                    self.heartbeats,
                    self.metrics,
                );
                if worker.run_once().is_ok() {
                    made_progress = true;
                }
            }
            if !made_progress {
                break;
            }
        }
    }
}


