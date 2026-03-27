//! Scheduler interfaces and strategy registration.
//! 当前实现了 FIFO 调度器，并预留了优先级调度 / 时间片轮转等扩展位。
//! 从类 OS 的角度看，这里对应 Linux 的 `kernel/sched/` 子系统。

pub mod fifo;
pub mod priority;
pub mod queue;
pub mod round_robin;
pub mod srt;
pub mod thread_safe_queue;
pub mod work_stealing;

use crate::scheduler::fifo::FifoScheduler;
use crate::scheduler::priority::PriorityScheduler;
use crate::scheduler::round_robin::RoundRobinScheduler;
use crate::scheduler::srt::SrtScheduler;
use crate::types::config::SchedulerKind;
use crate::types::error::Result;
use crate::types::task::Task;

/// Unified scheduler wrapper so the rest of the system can switch strategies
/// without caring about the concrete queue implementation.
#[derive(Debug)]
pub enum SchedulerStrategy {
    Fifo(FifoScheduler),
    Priority(PriorityScheduler),
    RoundRobin(RoundRobinScheduler),
    Srt(SrtScheduler),
}

impl SchedulerStrategy {
    pub fn new(kind: SchedulerKind) -> Self {
        match kind {
            SchedulerKind::Fifo => Self::Fifo(FifoScheduler::new()),
            SchedulerKind::Priority => Self::Priority(PriorityScheduler::new()),
            SchedulerKind::RoundRobin => Self::RoundRobin(RoundRobinScheduler::new()),
            SchedulerKind::Srt => Self::Srt(SrtScheduler::new()),
        }
    }

    pub fn submit(&mut self, task: Task) {
        match self {
            Self::Fifo(scheduler) => scheduler.submit(task),
            Self::Priority(scheduler) => scheduler.submit(task),
            Self::RoundRobin(scheduler) => scheduler.submit(task),
            Self::Srt(scheduler) => scheduler.submit(task),
        }
    }

    pub fn next_task(&mut self) -> Result<Task> {
        match self {
            Self::Fifo(scheduler) => scheduler.next_task(),
            Self::Priority(scheduler) => scheduler.next_task(),
            Self::RoundRobin(scheduler) => scheduler.next_task(),
            Self::Srt(scheduler) => scheduler.next_task(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Fifo(scheduler) => scheduler.is_empty(),
            Self::Priority(scheduler) => scheduler.is_empty(),
            Self::RoundRobin(scheduler) => scheduler.is_empty(),
            Self::Srt(scheduler) => scheduler.is_empty(),
        }
    }
}
