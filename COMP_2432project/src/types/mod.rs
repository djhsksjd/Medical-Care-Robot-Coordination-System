//! Type definitions module, similar to Linux `include/`.
//! 这里集中放置整个系统中会被广泛使用的基础类型定义（Task / Robot / Zone / Config / Error 等），
//! 类比 Linux 内核源码树中的 `include/` 目录。

pub mod task;
pub mod robot;
pub mod zone;
pub mod config;
pub mod error;
