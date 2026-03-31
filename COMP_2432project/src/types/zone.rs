//! Zone type definitions for resource or physical areas.
//! Represents controlled physical areas in a hospital, analogous to memory zones / NUMA nodes in an OS.

/// Unique identifier for a zone.
pub type ZoneId = u64;

/// Simple zone with a name and capacity hint.
#[derive(Debug, Clone)]
pub struct Zone {
    pub id: ZoneId,
    pub name: String,
    pub capacity: u32,
}

impl Zone {
    pub fn new(id: ZoneId, name: impl Into<String>, capacity: u32) -> Self {
        Self {
            id,
            name: name.into(),
            capacity,
        }
    }
}
