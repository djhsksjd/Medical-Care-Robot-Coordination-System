//! Utility helpers shared across the project.
//! 提供通用的小工具模块，例如：
//! - logger：统一的日志输出接口
//! - timer：基于 `Instant` 的简单计时封装
//! - id_generator：全局唯一 ID 生成器
//! - rand：随机数相关工具（后续可按需扩展）

pub mod id_generator;
pub mod logger;
pub mod rand;
pub mod timer;
