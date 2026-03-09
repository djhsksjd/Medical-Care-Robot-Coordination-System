//! Core zone allocator responsible for distributing resources.
//! Provides a simple ZoneManager that assigns zones to tasks.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::mm::allocation_table::AllocationTable;
use crate::types::task::TaskId;
use crate::types::zone::{Zone, ZoneId};

#[derive(Debug)]
pub struct ZoneManager {
    zones: Vec<Zone>,
    allocations: AllocationTable,
    next_index: AtomicUsize,
}

impl ZoneManager {
    pub fn new(zones: Vec<Zone>) -> Self {
        Self {
            zones,
            allocations: AllocationTable::new(),
            next_index: AtomicUsize::new(0),
        }
    }

    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    /// Simple round-robin assignment of zones to tasks.
    pub fn allocate_for_task(&self, task_id: TaskId) -> ZoneId {
        let idx = self
            .next_index
            .fetch_add(1, Ordering::Relaxed)
            % self.zones.len();
        let zone = &self.zones[idx];
        self.allocations.assign(task_id, zone.id);
        zone.id
    }

    pub fn release_for_task(&self, task_id: TaskId) {
        self.allocations.release(task_id);
    }

    pub fn zone_for_task(&self, task_id: TaskId) -> Option<ZoneId> {
        self.allocations.zone_for_task(task_id)
    }

    pub fn allocations(&self) -> &AllocationTable {
        &self.allocations
    }
}

