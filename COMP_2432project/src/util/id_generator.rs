//! ID generation utilities.
//! Global monotonic ID generators used across the system.

use crate::sync::atomic::{AtomicU64, Ordering};

static TASK_ID_GEN: AtomicU64 = AtomicU64::new(1);
static ROBOT_ID_GEN: AtomicU64 = AtomicU64::new(1);
static ZONE_ID_GEN: AtomicU64 = AtomicU64::new(1);

pub fn next_task_id() -> u64 {
    TASK_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

pub fn next_robot_id() -> u64 {
    ROBOT_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

pub fn next_zone_id() -> u64 {
    ZONE_ID_GEN.fetch_add(1, Ordering::Relaxed)
}
