//! Common prelude for conveniently importing core types and traits.
// Keep this list explicit to avoid ambiguous glob re-exports.

pub use crate::api::{AppState, build_router};
pub use crate::coordinator::builder::CoordinatorBuilder;
pub use crate::coordinator::lifecycle::Coordinator;
pub use crate::coordinator::task_table::{TaskSnapshot, TaskTable};
pub use crate::mm::zone_allocator::{ZoneLease, ZoneManager};
pub use crate::monitor::heartbeat::HeartbeatRegistry;
pub use crate::monitor::metrics::{GlobalMetrics, MetricsRegistry, RobotMetrics};
pub use crate::scheduler::SchedulerStrategy;
pub use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
pub use crate::sync::atomic::{AtomicBool, AtomicU64, Ordering};
pub use crate::types::config::{Config, SchedulerKind};
pub use crate::types::error::{Error, Result};
pub use crate::types::robot::{Robot, RobotId};
pub use crate::types::task::{Task, TaskId, TaskPriority, TaskStatus};
pub use crate::types::zone::{Zone, ZoneId};
