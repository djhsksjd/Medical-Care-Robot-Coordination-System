//! Core zone allocator responsible for distributing resources.
//! Provides a zone-aware manager that enforces per-zone capacity limits.

use std::collections::HashMap;
use std::sync::Condvar;

use crate::mm::allocation_table::AllocationTable;
use crate::sync::mutex::Mutex;
use crate::types::task::TaskId;
use crate::types::zone::{Zone, ZoneId};

#[derive(Debug, Default)]
struct ZoneRuntimeState {
    next_index: usize,
    active_counts: HashMap<ZoneId, usize>,
}

#[derive(Debug)]
pub struct ZoneManager {
    zones: Vec<Zone>,
    allocations: AllocationTable,
    state: Mutex<ZoneRuntimeState>,
    zone_available: Condvar,
}

impl ZoneManager {
    pub fn new(zones: Vec<Zone>) -> Self {
        Self {
            zones,
            allocations: AllocationTable::new(),
            state: Mutex::new(ZoneRuntimeState::default()),
            zone_available: Condvar::new(),
        }
    }

    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    /// Allocate a zone for a task while respecting each zone's capacity limit.
    ///
    /// When `required_zone` is `Some(id)`, the task is pinned to that specific
    /// zone and will block until capacity becomes available there — even if
    /// other zones have free slots.
    ///
    /// When `required_zone` is `None`, zones are tried in round-robin order and
    /// the call blocks only when **all** zones are full.
    pub fn allocate_for_task(&self, task_id: TaskId, required_zone: Option<ZoneId>) -> ZoneId {
        assert!(!self.zones.is_empty(), "zone manager requires at least one zone");

        let mut state = self.state.lock().expect("zone manager lock");

        loop {
            if let Some(target_id) = required_zone {
                if let Some(zone) = self.zones.iter().find(|z| z.id == target_id) {
                    let active = state.active_counts.get(&zone.id).copied().unwrap_or(0);
                    if active < zone.capacity as usize {
                        state.active_counts.insert(zone.id, active + 1);
                        drop(state);
                        self.allocations.assign(task_id, zone.id);
                        return zone.id;
                    }
                } else {
                    panic!("required_zone {} does not exist in zone manager", target_id);
                }
            } else {
                let zone_count = self.zones.len();
                let start_index = state.next_index % zone_count;

                for offset in 0..zone_count {
                    let idx = (start_index + offset) % zone_count;
                    let zone = &self.zones[idx];
                    let active = state.active_counts.get(&zone.id).copied().unwrap_or(0);

                    if active < zone.capacity as usize {
                        state.active_counts.insert(zone.id, active + 1);
                        state.next_index = (idx + 1) % zone_count;
                        drop(state);

                        self.allocations.assign(task_id, zone.id);
                        return zone.id;
                    }
                }
            }

            state = self
                .zone_available
                .wait(state)
                .expect("zone manager condvar wait");
        }
    }

    pub fn release_for_task(&self, task_id: TaskId) {
        let Some(zone_id) = self.allocations.zone_for_task(task_id) else {
            return;
        };

        {
            let mut state = self.state.lock().expect("zone manager lock");
            if let Some(active) = state.active_counts.get_mut(&zone_id) {
                *active = active.saturating_sub(1);
                if *active == 0 {
                    state.active_counts.remove(&zone_id);
                }
            }
        }

        self.allocations.release(task_id);
        self.zone_available.notify_all();
    }

    pub fn zone_for_task(&self, task_id: TaskId) -> Option<ZoneId> {
        self.allocations.zone_for_task(task_id)
    }

    pub fn allocations(&self) -> &AllocationTable {
        &self.allocations
    }

    pub fn active_tasks_in_zone(&self, zone_id: ZoneId) -> usize {
        let state = self.state.lock().expect("zone manager lock");
        state.active_counts.get(&zone_id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::ZoneManager;
    use crate::types::zone::Zone;

    #[test]
    fn uses_another_zone_when_first_choice_is_full() {
        let manager = ZoneManager::new(vec![
            Zone::new(1, "ICU", 1),
            Zone::new(2, "Ward", 1),
        ]);

        let first = manager.allocate_for_task(1, None);
        let second = manager.allocate_for_task(2, None);

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(manager.active_tasks_in_zone(1), 1);
        assert_eq!(manager.active_tasks_in_zone(2), 1);
    }

    #[test]
    fn waits_until_zone_capacity_is_available() {
        let manager = Arc::new(ZoneManager::new(vec![Zone::new(1, "ICU", 1)]));
        let first_zone = manager.allocate_for_task(1, None);
        assert_eq!(first_zone, 1);
        assert_eq!(manager.active_tasks_in_zone(1), 1);

        let manager_for_thread = Arc::clone(&manager);
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            started_tx.send(()).expect("signal thread start");
            let zone = manager_for_thread.allocate_for_task(2, None);
            acquired_tx.send(zone).expect("signal acquisition");
        });

        started_rx.recv().expect("thread should start");
        assert!(
            acquired_rx.recv_timeout(Duration::from_millis(150)).is_err(),
            "second task should wait while the zone is full"
        );

        manager.release_for_task(1);

        let second_zone = acquired_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second task should acquire the released zone");
        assert_eq!(second_zone, 1);
        assert_eq!(manager.active_tasks_in_zone(1), 1);

        manager.release_for_task(2);
        handle.join().expect("thread should finish");
        assert_eq!(manager.active_tasks_in_zone(1), 0);
    }

    #[test]
    fn required_zone_pins_task_to_specific_zone() {
        let manager = ZoneManager::new(vec![
            Zone::new(1, "ICU", 1),
            Zone::new(2, "Ward", 2),
        ]);

        let zone = manager.allocate_for_task(1, Some(2));
        assert_eq!(zone, 2, "task should be placed in the required zone");
        assert_eq!(manager.active_tasks_in_zone(2), 1);
        assert_eq!(manager.active_tasks_in_zone(1), 0);
    }

    #[test]
    fn required_zone_blocks_when_target_is_full() {
        let manager = Arc::new(ZoneManager::new(vec![
            Zone::new(1, "ICU", 1),
            Zone::new(2, "Ward", 2),
        ]));

        let zone = manager.allocate_for_task(1, Some(1));
        assert_eq!(zone, 1);

        let manager_for_thread = Arc::clone(&manager);
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            started_tx.send(()).expect("signal thread start");
            let z = manager_for_thread.allocate_for_task(2, Some(1));
            acquired_tx.send(z).expect("signal acquisition");
        });

        started_rx.recv().expect("thread should start");
        assert!(
            acquired_rx.recv_timeout(Duration::from_millis(150)).is_err(),
            "task should block even though Ward has capacity, because it requires ICU"
        );

        manager.release_for_task(1);

        let acquired_zone = acquired_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("task should acquire ICU after release");
        assert_eq!(acquired_zone, 1);

        manager.release_for_task(2);
        handle.join().expect("thread should finish");
    }
}

