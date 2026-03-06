//! Atomic operation helpers.
//! Re-export a few commonly used atomic types.

pub use std::sync::atomic::{
    fence,
    AtomicBool,
    AtomicU64,
    Ordering,
};
