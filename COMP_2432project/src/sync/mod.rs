//! Synchronization primitives module, similar to Linux `kernel/locking/`.
//! Thin wrappers around std locks and atomics for easy replacement or additional monitoring.
//! Conceptually corresponds to `kernel/locking/` in the Linux kernel source.

pub mod atomic;
pub mod mutex;
pub mod rwlock;
