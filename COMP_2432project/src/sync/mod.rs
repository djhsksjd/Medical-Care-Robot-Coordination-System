//! Synchronization primitives module, similar to Linux `kernel/locking/`.
// TODO: Wrap standard synchronization types and expose channels and barriers.

pub mod mutex;
pub mod rwlock;
pub mod atomic;
pub mod channel;
pub mod barrier;
