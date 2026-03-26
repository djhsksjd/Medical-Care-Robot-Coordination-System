//! Worker module representing user-space style workers.
//! 从概念上对应「用户态进程/线程」，在本项目中由 RobotWorker + WorkerPool 组成：
//! - robot：绑定到单个 Robot 的执行单元
//! - pool：管理多台 RobotWorker 的调度循环
//! - state / lifecycle：抽象 Worker 的状态与生命周期管理

pub mod lifecycle;
pub mod pool;
pub mod robot;
pub mod state;
