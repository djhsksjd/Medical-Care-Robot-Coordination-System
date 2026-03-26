//! Core coordinator module analogous to a kernel core.
//! Coordinate scheduling, workers, and monitoring into a cohesive system.

pub mod builder;
pub mod lifecycle;
pub mod syscall;
pub mod task_table;
