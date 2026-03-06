//! Memory and resource management module, similar to Linux `mm/`.
// TODO: Define abstractions for zones, allocation, and deadlock handling.

pub mod zone_allocator;
pub mod lock_guard;
pub mod deadlock_detector;
pub mod allocation_table;
