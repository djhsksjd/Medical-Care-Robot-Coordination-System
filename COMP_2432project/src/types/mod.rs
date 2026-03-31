//! Type definitions module, similar to Linux `include/`.
//! Centralizes fundamental type definitions (Task / Robot / Zone / Config / Error, etc.)
//! used throughout the system, analogous to the `include/` directory in the Linux kernel source tree.

pub mod config;
pub mod error;
pub mod robot;
pub mod task;
pub mod zone;
