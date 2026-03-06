//! Zone type definitions for resource or physical areas.
//! 可以理解为医院中受控的物理区域，类似 OS 里的「内存分区 / NUMA node」等资源域。

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
