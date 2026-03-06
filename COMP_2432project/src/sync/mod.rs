//! Synchronization primitives module, similar to Linux `kernel/locking/`.
//! 对标准库中的锁、原子等进行轻量封装，便于后续替换或加入额外监控。
//! 从概念上对应 Linux 内核源码中的 `kernel/locking/`。

pub mod mutex;
pub mod rwlock;
pub mod atomic;
pub mod channel;
pub mod barrier;
