//! Core coordinator module analogous to a kernel core.
//! Coordinate scheduling, workers, and monitoring into a cohesive system.

pub mod builder;
pub mod syscall;
pub mod lifecycle;
pub mod task_table;
