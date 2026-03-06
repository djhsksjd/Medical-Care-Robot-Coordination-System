//! Read-write lock abstraction.
//! Thin wrapper around `std::sync::RwLock`.

use std::sync::{RwLock as StdRwLock, RwLockReadGuard, RwLockWriteGuard};

pub type RwLock<T> = StdRwLock<T>;
pub type RwLockRead<'a, T> = RwLockReadGuard<'a, T>;
pub type RwLockWrite<'a, T> = RwLockWriteGuard<'a, T>;
