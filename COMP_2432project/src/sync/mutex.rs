//! Mutex abstraction.
//! Thin wrapper around `std::sync::Mutex` for easier swapping/instrumentation later.

use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard};

pub type Mutex<T> = StdMutex<T>;
pub type MutexGuard<'a, T> = StdMutexGuard<'a, T>;
