//! Memory and resource management module, similar to Linux `mm/`.
//! Handles zone and resource allocation abstractions:
//! - zone_allocator: task-to-zone allocation strategy
//! - allocation_table: tracks Task / Robot / Zone mappings
//! - (optional extension) more complex lock management and deadlock detection

pub mod allocation_table;
pub mod zone_allocator;
