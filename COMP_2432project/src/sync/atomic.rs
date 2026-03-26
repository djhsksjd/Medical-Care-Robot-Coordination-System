//! Atomic operation helpers.
//! Re-export a few commonly used atomic types.

pub use std::sync::atomic::{AtomicBool, AtomicU64, Ordering, fence};
