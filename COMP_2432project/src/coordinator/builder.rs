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

pub const STRESS_TEST_WORKER_COUNT: usize = 12;
pub const STRESS_TEST_TASK_COUNT: usize = 108;

pub fn effective_worker_count(config: &Config) -> usize {
    if config.use_stress_preset {
        STRESS_TEST_WORKER_COUNT
    } else {
        config.worker_count
    }
}

pub fn effective_demo_task_count(config: &Config) -> usize {
    if config.use_stress_preset {
        STRESS_TEST_TASK_COUNT
    } else {
        config.demo_task_count
    }
}

#[derive(Debug, Clone)]
pub struct DemoTaskPlan {
    pub sequence: u64,
    pub name: String,
    pub priority: TaskPriority,
    pub expected_duration: Duration,
    pub description: String,
    pub required_zone: Option<ZoneId>,
}

// Zone IDs: ICU = 1, Ward = 2, OR = 3
//
// Design rationale: 10/18 tasks target ICU (cap 2) to create a severe
// bottleneck there, while Ward (cap 2) and OR (cap 1) remain lighter.
// In classic mode a worker blocks when its task's zone is full; in
// work-stealing mode the worker skips the blocked task and picks one
// whose zone has capacity — this is where the measurable difference
// comes from.  Duration variance (2 s – 16 s) amplifies the effect.
const BASE_DEMO_TASKS: [(&str, TaskPriority, u64, &str, Option<ZoneId>); 18] = [
    (
        "Emergency ICU medicine delivery",
        TaskPriority::High,
        3,
        "Urgent medication shipment to ICU beds.",
        Some(1), // ICU
    ),
    (
        "Bulk linen transfer - west ward",
        TaskPriority::Low,
        14,
        "Large but non-urgent transport workload.",
        Some(2), // Ward
    ),
    (
        "STAT blood sample dispatch",
        TaskPriority::High,
        2,
        "Time-sensitive specimen trip to the lab.",
        Some(3), // OR
    ),
    (
        "ICU critical-care equipment setup",
        TaskPriority::High,
        5,
        "Assembling and calibrating bedside monitors.",
        Some(1), // ICU
    ),
    (
        "Ward meal delivery - north",
        TaskPriority::Normal,
        4,
        "Routine meal drop-off across patient rooms.",
        Some(2), // Ward
    ),
    (
        "OR sterilization prep",
        TaskPriority::High,
        6,
        "Pre-op cleaning tools must arrive before surgery starts.",
        Some(3), // OR
    ),
    (
        "ICU supply restock - pediatrics",
        TaskPriority::Normal,
        8,
        "Restock consumables for the pediatrics ICU station.",
        Some(1), // ICU
    ),
    (
        "Deep clean ICU corridor east",
        TaskPriority::Low,
        16,
        "Long-running cleaning route with low urgency (ICU wing).",
        Some(1), // ICU
    ),
    (
        "Urgent pharmacy pickup - ICU",
        TaskPriority::High,
        3,
        "Prescription needs to reach ICU nurses quickly.",
        Some(1), // ICU
    ),
    (
        "Discharge document transport",
        TaskPriority::Normal,
        4,
        "Deliver paperwork to the discharge desk.",
        Some(2), // Ward
    ),
    (
        "Lab specimen shuttle",
        TaskPriority::High,
        2,
        "Fast sample handoff before testing window closes.",
        Some(3), // OR
    ),
    (
        "ICU infusion pump delivery",
        TaskPriority::High,
        3,
        "Critical device delivery for ICU patient treatment.",
        Some(1), // ICU
    ),
    (
        "Ward UV disinfection - ward 3",
        TaskPriority::Normal,
        7,
        "Medium-priority hygiene cycle for ward section.",
        Some(2), // Ward
    ),
    (
        "Emergency ICU oxygen cylinder",
        TaskPriority::High,
        5,
        "Urgent respiratory support delivery to ICU.",
        Some(1), // ICU
    ),
    (
        "Ward laundry return - south wing",
        TaskPriority::Low,
        9,
        "Bulky transport with no immediate deadline.",
        Some(2), // Ward
    ),
    (
        "ICU monitoring setup - annex",
        TaskPriority::Normal,
        10,
        "Install and configure monitoring in ICU annex beds.",
        Some(1), // ICU
    ),
    (
        "Night medicine refill - ICU",
        TaskPriority::Normal,
        6,
        "Routine refill before the next ICU medication round.",
        Some(1), // ICU
    ),
    (
        "Terminal cleaning - ICU annex",
        TaskPriority::Low,
        15,
        "Long final cleaning sweep for the ICU annex.",
        Some(1), // ICU
    ),
];

pub fn demo_task_plans(count: usize) -> Vec<DemoTaskPlan> {
    let count = count.max(1);

    (0..count)
        .map(|index| {
            let sequence = index as u64 + 1;
            let cycle = index / BASE_DEMO_TASKS.len();
            let (name, priority, duration_secs, description, required_zone) =
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
                required_zone,
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
        let effective_task_count = effective_demo_task_count(&self.config);
        let effective_worker_count = effective_worker_count(&self.config);

        for plan in demo_task_plans(effective_task_count) {
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
        let mut robots = Vec::with_capacity(effective_worker_count);
        for i in 0..effective_worker_count {
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
