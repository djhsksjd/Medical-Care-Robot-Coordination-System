//! Builder for assembling a configured coordinator instance.
//! 负责根据 `Config` 构造出一个可运行的 `Coordinator`，
//! 包括：初始化调度器、预填充 Demo 任务、按 `worker_count` 创建多台 Robot。

use crate::coordinator::lifecycle::Coordinator;
use crate::coordinator::task_table::TaskTable;
use crate::scheduler::SchedulerStrategy;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::types::config::Config;
use crate::types::robot::Robot;
use crate::types::task::{Task, TaskPriority};
use crate::types::zone::ZoneId;
use crate::util::id_generator::next_task_id;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DemoTaskPlan {
    pub sequence: u64,
    pub name: String,
    pub priority: TaskPriority,
    pub expected_duration: Duration,
    pub description: String,
    pub required_zone: Option<ZoneId>,
}

const BASE_DEMO_TASKS: [(&str, TaskPriority, u64, &str); 18] = [
    (
        "Emergency ICU medicine delivery",
        TaskPriority::High,
        4,
        "Urgent medication shipment to ICU beds.",
    ),
    (
        "Bulk linen transfer - west ward",
        TaskPriority::Low,
        12,
        "Large but non-urgent transport workload.",
    ),
    (
        "STAT blood sample dispatch",
        TaskPriority::High,
        3,
        "Time-sensitive specimen trip to the lab.",
    ),
    (
        "Waste removal batch A",
        TaskPriority::Low,
        9,
        "Scheduled sanitation pickup with long travel path.",
    ),
    (
        "Ward meal delivery - north",
        TaskPriority::Normal,
        5,
        "Routine meal drop-off across patient rooms.",
    ),
    (
        "OR sterilization prep",
        TaskPriority::High,
        6,
        "Pre-op cleaning tools must arrive before surgery starts.",
    ),
    (
        "Supply restock - pediatrics",
        TaskPriority::Normal,
        7,
        "Restock consumables for the pediatrics station.",
    ),
    (
        "Deep clean corridor east",
        TaskPriority::Low,
        14,
        "Long-running cleaning route with low urgency.",
    ),
    (
        "Urgent pharmacy pickup",
        TaskPriority::High,
        4,
        "Prescription needs to reach nurses quickly.",
    ),
    (
        "Discharge document transport",
        TaskPriority::Normal,
        5,
        "Deliver paperwork to the discharge desk.",
    ),
    (
        "Lab specimen shuttle",
        TaskPriority::High,
        3,
        "Fast sample handoff before testing window closes.",
    ),
    (
        "Waste removal batch B",
        TaskPriority::Low,
        10,
        "Second sanitation sweep for the lower floors.",
    ),
    (
        "Infusion pump delivery",
        TaskPriority::High,
        4,
        "Critical device delivery for patient treatment.",
    ),
    (
        "UV disinfection - ward 3",
        TaskPriority::Normal,
        8,
        "Medium-priority hygiene cycle.",
    ),
    (
        "Laundry return - south wing",
        TaskPriority::Low,
        9,
        "Bulky transport with no immediate deadline.",
    ),
    (
        "Emergency oxygen cylinder",
        TaskPriority::High,
        6,
        "Urgent respiratory support delivery.",
    ),
    (
        "Night medicine refill",
        TaskPriority::Normal,
        7,
        "Routine refill before the next medication round.",
    ),
    (
        "Terminal cleaning - ICU annex",
        TaskPriority::Low,
        12,
        "Long final cleaning sweep for the annex.",
    ),
];

pub fn demo_task_plans(count: usize) -> Vec<DemoTaskPlan> {
    let count = count.max(1);

    (0..count)
        .map(|index| {
            let sequence = index as u64 + 1;
            let cycle = index / BASE_DEMO_TASKS.len();
            let (name, priority, duration_secs, description) =
                BASE_DEMO_TASKS[index % BASE_DEMO_TASKS.len()];

            DemoTaskPlan {
                sequence,
                name: if cycle == 0 {
                    name.to_string()
                } else {
                    format!("{name} (wave {})", cycle + 1)
                },
                priority,
                expected_duration: Duration::from_secs(duration_secs),
                description: if cycle == 0 {
                    description.to_string()
                } else {
                    format!("{description} Repeated workload wave {}.", cycle + 1)
                },
                required_zone: None,
            }
        })
        .collect()
}

/// Fluent builder for configuring and creating a coordinator.
pub struct CoordinatorBuilder {
    config: Config,
}

impl CoordinatorBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn with_demo_defaults() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Build a coordinator and seed the central task structures.
    ///
    /// - Tasks are first inserted into a strategy scheduler so we can honor
    ///   FIFO vs priority policies.
    /// - The resulting ordering is then pushed into the shared `task_queue`
    ///   that workers consume from.
    /// - All task metadata is stored in `task_table` for API and monitoring.
    pub fn build(self, task_table: &TaskTable, task_queue: &ThreadSafeTaskQueue) -> Coordinator {
        // Set up the configured scheduler and seed it with demo tasks.
        let mut scheduler = SchedulerStrategy::new(self.config.scheduler);

        for plan in demo_task_plans(self.config.demo_task_count) {
            let mut task = Task::new(next_task_id(), plan.name, plan.expected_duration);
            task.priority = plan.priority;
            task.required_zone = plan.required_zone;

            // Insert into the central task table so lifecycle / API can observe
            // real per-task state and timestamps.
            task_table.insert(task.clone());

            // Also feed into the logical scheduler so we can derive the order
            // tasks will be enqueued to workers.
            scheduler.submit(task);
        }

        // Create multiple robots based on worker_count.
        let mut robots = Vec::with_capacity(self.config.worker_count);
        for i in 0..self.config.worker_count {
            robots.push(Robot::new(i as u64 + 1, format!("robot-{}", i + 1)));
        }

        // Drain the scheduler into the shared task queue in the chosen order.
        while !scheduler.is_empty() {
            if let Ok(task) = scheduler.next_task() {
                if !task_queue.push(task.id) {
                    break;
                }
            } else {
                break;
            }
        }

        Coordinator::new(self.config, robots)
    }
}
