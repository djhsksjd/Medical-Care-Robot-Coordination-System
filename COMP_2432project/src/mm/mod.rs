//! Memory and resource management module, similar to Linux `mm/`.
//! 主要负责「区域（Zone）」和资源分配相关的抽象：
//! - zone_allocator：任务到区域的分配策略
//! - allocation_table：记录 Task / Robot 与 Zone 的映射关系
//! - lock_guard / deadlock_detector：为后续更复杂的锁管理与死锁检测预留接口

pub mod zone_allocator;
pub mod lock_guard;
pub mod deadlock_detector;
pub mod allocation_table;
