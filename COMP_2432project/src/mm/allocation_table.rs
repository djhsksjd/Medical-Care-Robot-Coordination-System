//! Allocation table tracking resource ownership.
//! Maintains the mapping from tasks to zones for reporting and control.

use std::collections::HashMap;

use crate::sync::mutex::Mutex;
use crate::types::task::TaskId;
use crate::types::zone::ZoneId;

#[derive(Debug, Default)]
pub struct AllocationTable {
    inner: Mutex<HashMap<TaskId, ZoneId>>,
}

impl AllocationTable {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn assign(&self, task_id: TaskId, zone_id: ZoneId) {
        let mut inner = self.inner.lock().expect("allocation table lock");
        inner.insert(task_id, zone_id);
    }

    pub fn release(&self, task_id: TaskId) {
        let mut inner = self.inner.lock().expect("allocation table lock");
        inner.remove(&task_id);
    }

    pub fn zone_for_task(&self, task_id: TaskId) -> Option<ZoneId> {
        let inner = self.inner.lock().expect("allocation table lock");
        inner.get(&task_id).cloned()
    }

    pub fn all(&self) -> HashMap<TaskId, ZoneId> {
        let inner = self.inner.lock().expect("allocation table lock");
        inner.clone()
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().expect("allocation table lock");
        inner.clear();
    }
}


