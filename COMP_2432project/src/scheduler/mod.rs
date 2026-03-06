//! Scheduler interfaces and strategy registration.
//! 当前实现了 FIFO 调度器，并预留了优先级调度 / 时间片轮转等扩展位。
//! 从类 OS 的角度看，这里对应 Linux 的 `kernel/sched/` 子系统。

pub mod queue;
pub mod fifo;
pub mod priority;
pub mod round_robin;
pub mod stats;
