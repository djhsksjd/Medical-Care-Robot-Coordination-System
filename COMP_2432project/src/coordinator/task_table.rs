//! Central task table for tracking real task status and timestamps.
//! This allows the API and monitoring code to see the true lifecycle
//! of each task instead of inferring from aggregate metrics.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use crate::types::robot::RobotId;
use crate::types::task::{Task, TaskId, TaskStatus};
use crate::types::zone::ZoneId;

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub task: Task,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub robot_id: Option<RobotId>,
    pub zone_id: Option<ZoneId>,
}

#[derive(Debug)]
struct Entry {
    task: Task,
    started_at: Option<SystemTime>,
    finished_at: Option<SystemTime>,
    robot_id: Option<RobotId>,
    zone_id: Option<ZoneId>,
}

#[derive(Debug)]
pub struct TaskTable {
    inner: Mutex<HashMap<TaskId, Entry>>,
}

impl TaskTable {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Insert a new task into the table in Pending state.
    pub fn insert(&self, task: Task) {
        let mut inner = self.inner.lock().expect("task table lock");
        let id = task.id;
        inner.insert(
            id,
            Entry {
                task,
                started_at: None,
                finished_at: None,
                robot_id: None,
                zone_id: None,
            },
        );
    }

    /// Mark a task as running on a specific robot and zone.
    /// Returns the expected duration of the task if it exists.
    pub fn start_task(
        &self,
        id: TaskId,
        robot_id: RobotId,
        zone_id: ZoneId,
    ) -> Option<Duration> {
        let mut inner = self.inner.lock().expect("task table lock");
        let entry = inner.get_mut(&id)?;
        entry.task.status = TaskStatus::Running;
        if entry.started_at.is_none() {
            entry.started_at = Some(SystemTime::now());
        }
        entry.robot_id = Some(robot_id);
        entry.zone_id = Some(zone_id);
        Some(entry.task.expected_duration)
    }

    /// Mark a task as running and record its start time if not already set.
    pub fn set_running(&self, id: TaskId) {
        let mut inner = self.inner.lock().expect("task table lock");
        if let Some(entry) = inner.get_mut(&id) {
            entry.task.status = TaskStatus::Running;
            if entry.started_at.is_none() {
                entry.started_at = Some(SystemTime::now());
            }
        }
    }

    /// Mark a task as finished and record its finish time.
    pub fn set_finished(&self, id: TaskId) {
        let mut inner = self.inner.lock().expect("task table lock");
        if let Some(entry) = inner.get_mut(&id) {
            entry.task.status = TaskStatus::Finished;
            entry.finished_at = Some(SystemTime::now());
        }
    }

    /// Return a snapshot of all tasks and their timestamps.
    pub fn all(&self) -> Vec<TaskSnapshot> {
        let inner = self.inner.lock().expect("task table lock");
        inner
            .values()
            .map(|e| TaskSnapshot {
                task: e.task.clone(),
                started_at: e.started_at,
                finished_at: e.finished_at,
                robot_id: e.robot_id,
                zone_id: e.zone_id,
            })
            .collect()
    }

    /// Remove all tasks (used when resetting the system).
    pub fn clear(&self) {
        let mut inner = self.inner.lock().expect("task table lock");
        inner.clear();
    }
}

