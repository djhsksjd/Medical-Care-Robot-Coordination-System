//! HTTP API layer acting as the glue between the kernel and the frontend.
//! - Holds the Coordinator and monitoring subsystems via `AppState`
//! - Exposes endpoints such as /api/state, /api/config, /api/system/control for the frontend dashboard

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::coordinator::builder::{
    CoordinatorBuilder, DemoTaskPlan, demo_task_plans, effective_demo_task_count,
    effective_worker_count,
};
use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::monitor::monitor_thread::spawn_monitor_thread;
use crate::monitor::reporter::{SystemReport, build_report};
use crate::scheduler::SchedulerStrategy;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::types::config::{Config, SchedulerKind};
use crate::types::robot::RobotId;
use crate::types::task::{Task as CoreTask, TaskPriority as CoreTaskPriority};
use crate::worker::lifecycle::PauseController;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SystemStatus {
    Running,
    Paused,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub throughput: u64,
    pub avg_latency_ms: u64,
    /// Wall-clock time from demo start to demo end (milliseconds).
    pub makespan_ms: u64,
    /// Total zone-switch events across all robots (work-stealing only).
    pub total_zone_switches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TaskStatus {
    Pending,
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WorkerState {
    Idle,
    Busy,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Robot {
    pub id: RobotId,
    pub name: String,
    pub state: WorkerState,
    pub current_task_id: Option<u64>,
    pub recent_completed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ZoneHealth {
    Normal,
    HighLoad,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub id: u64,
    pub name: String,
    pub capacity: u32,
    pub current_tasks: u32,
    pub active_robots: u32,
    pub health: ZoneHealth,
    /// Number of times a robot switched *out of* this zone to execute in another.
    pub zone_switches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: u64,
    pub name: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub robot_id: Option<u64>,
    pub zone_id: Option<u64>,
    pub required_zone_id: Option<u64>,
    pub expected_duration_ms: u64,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoInputTask {
    pub id: u64,
    pub name: String,
    pub priority: TaskPriority,
    pub expected_duration_ms: u64,
    pub description: String,
    pub required_zone_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyTaskTiming {
    pub task_id: u64,
    pub task_name: String,
    pub priority: TaskPriority,
    pub worker_id: u64,
    pub start_ms: u64,
    pub finish_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategySummary {
    pub scheduler: SchedulerKind,
    pub makespan_ms: u64,
    pub avg_completion_ms: u64,
    pub avg_wait_ms: u64,
    pub avg_high_priority_completion_ms: u64,
    pub avg_completion_improvement_vs_fifo_ms: i64,
    pub avg_high_priority_improvement_vs_fifo_ms: i64,
    pub worker_busy_ms: Vec<u64>,
    pub speedup_vs_fifo_pct: f64,
    pub task_timings: Vec<StrategyTaskTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingAnalysis {
    pub input_tasks: Vec<DemoInputTask>,
    pub strategies: Vec<StrategySummary>,
    pub worker_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemState {
    pub tasks: Vec<Task>,
    pub robots: Vec<Robot>,
    pub zones: Vec<Zone>,
    pub config: Config,
    pub metrics: Metrics,
    pub system_status: SystemStatus,
    pub scheduling_analysis: SchedulingAnalysis,
}

#[derive(Debug)]
struct SharedState {
    config: Config,
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    task_table: Arc<TaskTable>,
    task_queue: Arc<ThreadSafeTaskQueue>,
    zone_manager: Arc<ZoneManager>,
    worker_shutdown: Arc<AtomicBool>,
    worker_pause: Arc<PauseController>,
    monitor_shutdown: Arc<AtomicBool>,
    system_status: SystemStatus,
    run_generation: u64,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<SharedState>>,
}

fn default_zones() -> Vec<crate::types::zone::Zone> {
    vec![
        crate::types::zone::Zone::new(1, "ICU".to_string(), 2),
        crate::types::zone::Zone::new(2, "Ward".to_string(), 2),
        crate::types::zone::Zone::new(3, "OR".to_string(), 1),
    ]
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::default();
        let heartbeats = Arc::new(HeartbeatRegistry::new());
        let metrics = Arc::new(MetricsRegistry::new());
        let task_table = Arc::new(TaskTable::new());
        let task_queue = Arc::new(ThreadSafeTaskQueue::new());
        let zone_manager = Arc::new(ZoneManager::new(default_zones()));
        let worker_shutdown = Arc::new(AtomicBool::new(false));
        let worker_pause = Arc::new(PauseController::new());
        let monitor_shutdown = Arc::new(AtomicBool::new(false));

        Self {
            inner: Arc::new(Mutex::new(SharedState {
                config,
                heartbeats,
                metrics,
                task_table,
                task_queue,
                zone_manager,
                worker_shutdown,
                worker_pause,
                monitor_shutdown,
                system_status: SystemStatus::Stopped,
                run_generation: 0,
            })),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlAction {
    Start,
    Pause,
    Stop,
    RunDemo,
}

#[derive(Debug, Deserialize)]
struct ControlRequest {
    action: ControlAction,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/state", get(get_state))
        .route("/api/config", put(update_config))
        .route("/api/system/control", post(control_system))
        .with_state(state)
}

async fn get_state(State(app): State<AppState>) -> Json<SystemState> {
    Json(snapshot_state_inner(&app))
}

impl AppState {
    /// Synchronous snapshot of the current system state (same payload as `GET /api/state`).
    pub fn snapshot_state(&self) -> SystemState {
        snapshot_state_inner(self)
    }

    /// Apply a new config (same semantics as `PUT /api/config`).
    pub fn apply_config(&self, new_config: Config) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        // Signal OLD monitor/worker threads to stop (they hold clones of
        // the current Arcs). Then replace with fresh Arcs so the next run
        // gets independent flags.
        guard.monitor_shutdown.store(true, Ordering::SeqCst);
        guard.worker_shutdown.store(true, Ordering::SeqCst);
        guard.task_queue.close();

        guard.config = new_config;
        guard.heartbeats = Arc::new(HeartbeatRegistry::new());
        guard.metrics = Arc::new(MetricsRegistry::new());
        guard.task_table = Arc::new(TaskTable::new());
        guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
        guard.zone_manager = Arc::new(ZoneManager::new(default_zones()));
        guard.worker_shutdown = Arc::new(AtomicBool::new(false));
        guard.worker_pause.resume();
        guard.monitor_shutdown = Arc::new(AtomicBool::new(false));
        guard.system_status = SystemStatus::Stopped;
        guard.run_generation += 1;
    }

    /// Control the running system (same semantics as `POST /api/system/control`).
    pub fn control(&self, action: ControlAction) -> StatusCode {
        control_inner(self, action)
    }
}

fn snapshot_state_inner(app: &AppState) -> SystemState {
    let guard = app.inner.lock().unwrap_or_else(|e| e.into_inner());
    let hb_timeout = Duration::from_secs(5);
    let effective_tasks = effective_demo_task_count(&guard.config);
    let effective_workers = effective_worker_count(&guard.config);
    let demo_plans = demo_task_plans(effective_tasks);
    let scheduling_analysis = build_scheduling_analysis(&demo_plans, effective_workers);
    let task_snapshots = guard.task_table.all();

    let running_tasks_by_robot: HashMap<RobotId, u64> = task_snapshots
        .iter()
        .filter_map(|snap| match (snap.task.status, snap.robot_id) {
            (crate::types::task::TaskStatus::Running, Some(robot_id)) => {
                Some((robot_id, snap.task.id))
            }
            _ => None,
        })
        .collect();

    // Build monitoring report for all robots that coordinator knows about.
    let robot_ids: Vec<RobotId> = (1..=effective_workers as u64).collect();
    let report: SystemReport = build_report(
        guard.heartbeats.as_ref(),
        guard.metrics.as_ref(),
        &robot_ids,
        hb_timeout,
    );

    // Map internal types to API DTO.
    let (throughput, avg_latency_ms) = {
        let gm = &report.global_metrics;
        let avg_ms = gm
            .avg_exec_time()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        (gm.completed_tasks, avg_ms)
    };

    let robots: Vec<Robot> = report
        .robots
        .iter()
        .map(|rr| Robot {
            id: rr.robot_id,
            name: format!("robot-{}", rr.robot_id),
            state: if running_tasks_by_robot.contains_key(&rr.robot_id) {
                WorkerState::Busy
            } else {
                match rr.health.status {
                    crate::monitor::health_checker::RobotHealthStatus::Healthy => WorkerState::Idle,
                    crate::monitor::health_checker::RobotHealthStatus::Degraded => {
                        WorkerState::Busy
                    }
                    crate::monitor::health_checker::RobotHealthStatus::Unreachable => {
                        WorkerState::Stopped
                    }
                }
            },
            current_task_id: running_tasks_by_robot.get(&rr.robot_id).copied(),
            recent_completed: rr.metrics.completed_tasks,
        })
        .collect();

    let zone_switch_map = guard.metrics.zone_switch_snapshot();
    let total_zone_switches: u64 = zone_switch_map.values().sum();
    let makespan_ms = guard.metrics.makespan_ms();

    // Build task list from the central TaskTable so we reflect real statuses.
    let mut tasks: Vec<Task> = task_snapshots
        .into_iter()
        .map(|snap| Task {
            id: snap.task.id,
            name: snap.task.name.clone(),
            priority: match snap.task.priority {
                crate::types::task::TaskPriority::Low => TaskPriority::Low,
                crate::types::task::TaskPriority::Normal => TaskPriority::Normal,
                crate::types::task::TaskPriority::High => TaskPriority::High,
            },
            status: match snap.task.status {
                crate::types::task::TaskStatus::Pending => TaskStatus::Pending,
                crate::types::task::TaskStatus::Running => TaskStatus::Running,
                crate::types::task::TaskStatus::Finished => TaskStatus::Finished,
                crate::types::task::TaskStatus::Failed => TaskStatus::Failed,
            },
            robot_id: snap.robot_id,
            zone_id: snap.zone_id,
            required_zone_id: snap.task.required_zone,
            expected_duration_ms: snap.task.expected_duration.as_secs() * 1000,
            started_at: snap.started_at.map(|t| format!("{t:?}")),
            finished_at: snap.finished_at.map(|t| format!("{t:?}")),
        })
        .collect();
    tasks.sort_by_key(|task| task.id);

    // Compute zone statistics based on live running tasks in each zone.
    let mut zones: Vec<Zone> = Vec::new();
    for z in guard.zone_manager.zones() {
        let current_tasks = tasks
            .iter()
            .filter(|t| t.zone_id == Some(z.id) && matches!(t.status, TaskStatus::Running))
            .count() as u32;

        let active_robots = tasks
            .iter()
            .filter(|t| t.zone_id == Some(z.id) && matches!(t.status, TaskStatus::Running))
            .filter_map(|t| t.robot_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len() as u32;

        let health = if current_tasks >= z.capacity {
            ZoneHealth::HighLoad
        } else {
            ZoneHealth::Normal
        };

        let zs = zone_switch_map.get(&z.id).copied().unwrap_or(0);
        zones.push(Zone {
            id: z.id,
            name: z.name.clone(),
            capacity: z.capacity,
            current_tasks,
            active_robots,
            health,
            zone_switches: zs,
        });
    }

    SystemState {
        tasks,
        robots,
        zones,
        config: guard.config.clone(),
        metrics: Metrics {
            throughput,
            avg_latency_ms,
            makespan_ms,
            total_zone_switches,
        },
        system_status: guard.system_status,
        scheduling_analysis,
    }
}

fn map_priority(priority: CoreTaskPriority) -> TaskPriority {
    match priority {
        CoreTaskPriority::Low => TaskPriority::Low,
        CoreTaskPriority::Normal => TaskPriority::Normal,
        CoreTaskPriority::High => TaskPriority::High,
    }
}

fn build_scheduling_analysis(
    demo_plans: &[DemoTaskPlan],
    worker_count: usize,
) -> SchedulingAnalysis {
    let input_tasks = demo_plans
        .iter()
        .map(|plan| DemoInputTask {
            id: plan.sequence,
            name: plan.name.clone(),
            priority: map_priority(plan.priority),
            expected_duration_ms: plan.expected_duration.as_millis() as u64,
            description: plan.description.clone(),
            required_zone_id: plan.required_zone,
        })
        .collect::<Vec<_>>();

    let worker_count = worker_count.max(1);
    let mut strategies = vec![
        simulate_strategy(SchedulerKind::Fifo, demo_plans, worker_count),
        simulate_strategy(SchedulerKind::Priority, demo_plans, worker_count),
        simulate_strategy(SchedulerKind::RoundRobin, demo_plans, worker_count),
        simulate_strategy(SchedulerKind::Srt, demo_plans, worker_count),
    ];

    let fifo_summary = strategies
        .iter()
        .find(|summary| matches!(summary.scheduler, SchedulerKind::Fifo))
        .cloned();

    let fifo_makespan = fifo_summary
        .as_ref()
        .map(|summary| summary.makespan_ms)
        .unwrap_or(0);
    let fifo_avg_completion = fifo_summary
        .as_ref()
        .map(|summary| summary.avg_completion_ms)
        .unwrap_or(0);
    let fifo_avg_high = fifo_summary
        .as_ref()
        .map(|summary| summary.avg_high_priority_completion_ms)
        .unwrap_or(0);

    for summary in &mut strategies {
        summary.speedup_vs_fifo_pct = if fifo_makespan == 0 {
            0.0
        } else {
            ((fifo_makespan as f64 - summary.makespan_ms as f64) / fifo_makespan as f64) * 100.0
        };
        summary.avg_completion_improvement_vs_fifo_ms =
            fifo_avg_completion as i64 - summary.avg_completion_ms as i64;
        summary.avg_high_priority_improvement_vs_fifo_ms =
            fifo_avg_high as i64 - summary.avg_high_priority_completion_ms as i64;
    }

    SchedulingAnalysis {
        input_tasks,
        strategies,
        worker_count,
    }
}

/// Default zone capacities used by the simulation to mirror the live system.
const SIM_ZONE_CAPS: [(u64, u32); 3] = [
    (1, 2), // ICU
    (2, 2), // Ward
    (3, 1), // OR
];

/// Event used by the zone-aware discrete-event simulation.
#[derive(Debug, Clone, Copy)]
struct SimEvent {
    finish_ms: u64,
    zone_id: u64,
}

fn simulate_strategy(
    scheduler_kind: SchedulerKind,
    demo_plans: &[DemoTaskPlan],
    worker_count: usize,
) -> StrategySummary {
    if matches!(scheduler_kind, SchedulerKind::RoundRobin) {
        return simulate_round_robin_strategy(demo_plans, worker_count);
    }

    let mut scheduler = SchedulerStrategy::new(scheduler_kind);

    for plan in demo_plans {
        let mut task = CoreTask::new(plan.sequence, plan.name.clone(), plan.expected_duration);
        task.priority = plan.priority;
        task.required_zone = plan.required_zone;
        scheduler.submit(task);
    }

    struct OrderedTask {
        id: u64,
        name: String,
        priority: CoreTaskPriority,
        duration_ms: u64,
        required_zone: Option<u64>,
    }

    let mut ordered_tasks: Vec<OrderedTask> = Vec::with_capacity(demo_plans.len());
    while !scheduler.is_empty() {
        if let Ok(task) = scheduler.next_task() {
            ordered_tasks.push(OrderedTask {
                id: task.id,
                name: task.name,
                priority: task.priority,
                duration_ms: task.expected_duration.as_millis() as u64,
                required_zone: task.required_zone,
            });
        }
    }

    let worker_count = worker_count.max(1);
    let mut worker_available_ms = vec![0_u64; worker_count];
    let mut worker_busy_ms = vec![0_u64; worker_count];
    let mut task_timings = Vec::with_capacity(ordered_tasks.len());
    let mut total_completion_ms = 0_u64;
    let mut total_wait_ms = 0_u64;
    let mut high_completion_ms = 0_u64;
    let mut high_count = 0_u64;

    let zone_caps: HashMap<u64, u32> = SIM_ZONE_CAPS.iter().copied().collect();
    // (finish_ms, zone_id) for every in-flight task
    let mut active_events: Vec<SimEvent> = Vec::new();
    let mut zone_active: HashMap<u64, u32> = HashMap::new();

    for task in ordered_tasks {
        let worker_index = worker_available_ms
            .iter()
            .enumerate()
            .min_by_key(|(index, available_at)| (**available_at, *index))
            .map(|(index, _)| index)
            .unwrap_or(0);

        let mut earliest_start = worker_available_ms[worker_index];

        if let Some(target_zone) = task.required_zone {
            let cap = zone_caps.get(&target_zone).copied().unwrap_or(u32::MAX);

            loop {
                let active = zone_active.get(&target_zone).copied().unwrap_or(0);
                if active < cap {
                    break;
                }
                // Find earliest event that frees a slot in the target zone.
                if let Some(free_at) = active_events
                    .iter()
                    .filter(|e| e.zone_id == target_zone)
                    .map(|e| e.finish_ms)
                    .min()
                {
                    if free_at > earliest_start {
                        earliest_start = free_at;
                    }
                    // Drain all events that finish at or before earliest_start.
                    active_events.retain(|e| {
                        if e.finish_ms <= earliest_start {
                            *zone_active.entry(e.zone_id).or_insert(1) =
                                zone_active.get(&e.zone_id).copied().unwrap_or(1).saturating_sub(1);
                            false
                        } else {
                            true
                        }
                    });
                } else {
                    break;
                }
            }
        }

        let start_ms = earliest_start;
        let finish_ms = start_ms + task.duration_ms;
        worker_available_ms[worker_index] = finish_ms;
        worker_busy_ms[worker_index] += task.duration_ms;

        if let Some(zone_id) = task.required_zone {
            *zone_active.entry(zone_id).or_insert(0) += 1;
            active_events.push(SimEvent { finish_ms, zone_id });
        }

        total_completion_ms += finish_ms;
        total_wait_ms += start_ms;

        if matches!(task.priority, CoreTaskPriority::High) {
            high_completion_ms += finish_ms;
            high_count += 1;
        }

        task_timings.push(StrategyTaskTiming {
            task_id: task.id,
            task_name: task.name,
            priority: map_priority(task.priority),
            worker_id: worker_index as u64 + 1,
            start_ms,
            finish_ms,
            duration_ms: task.duration_ms,
        });
    }

    let task_count = task_timings.len() as u64;
    let makespan_ms = worker_available_ms.into_iter().max().unwrap_or(0);
    let avg_completion_ms = if task_count == 0 {
        0
    } else {
        total_completion_ms / task_count
    };
    let avg_wait_ms = if task_count == 0 {
        0
    } else {
        total_wait_ms / task_count
    };
    let avg_high_priority_completion_ms = if high_count == 0 {
        0
    } else {
        high_completion_ms / high_count
    };

    StrategySummary {
        scheduler: scheduler_kind,
        makespan_ms,
        avg_completion_ms,
        avg_wait_ms,
        avg_high_priority_completion_ms,
        avg_completion_improvement_vs_fifo_ms: 0,
        avg_high_priority_improvement_vs_fifo_ms: 0,
        worker_busy_ms,
        speedup_vs_fifo_pct: 0.0,
        task_timings,
    }
}

fn simulate_round_robin_strategy(
    demo_plans: &[DemoTaskPlan],
    worker_count: usize,
) -> StrategySummary {
    const QUANTUM_MS: u64 = 4_000;

    #[derive(Debug)]
    struct RoundRobinTask {
        id: u64,
        name: String,
        priority: CoreTaskPriority,
        total_duration_ms: u64,
        remaining_ms: u64,
        required_zone: Option<u64>,
    }

    let mut queue: VecDeque<RoundRobinTask> = demo_plans
        .iter()
        .map(|plan| RoundRobinTask {
            id: plan.sequence,
            name: plan.name.clone(),
            priority: plan.priority,
            total_duration_ms: plan.expected_duration.as_millis() as u64,
            remaining_ms: plan.expected_duration.as_millis() as u64,
            required_zone: plan.required_zone,
        })
        .collect();

    let worker_count = worker_count.max(1);
    let mut worker_available_ms = vec![0_u64; worker_count];
    let mut worker_busy_ms = vec![0_u64; worker_count];
    let mut task_timings = Vec::new();
    let mut completion_ms_by_task = HashMap::with_capacity(demo_plans.len());
    let mut task_duration_ms = HashMap::with_capacity(demo_plans.len());
    let mut task_priority = HashMap::with_capacity(demo_plans.len());

    let zone_caps: HashMap<u64, u32> = SIM_ZONE_CAPS.iter().copied().collect();
    let mut active_events: Vec<SimEvent> = Vec::new();
    let mut zone_active: HashMap<u64, u32> = HashMap::new();

    while let Some(mut task) = queue.pop_front() {
        let worker_index = worker_available_ms
            .iter()
            .enumerate()
            .min_by_key(|(index, available_at)| (**available_at, *index))
            .map(|(index, _)| index)
            .unwrap_or(0);

        let mut earliest_start = worker_available_ms[worker_index];

        if let Some(target_zone) = task.required_zone {
            let cap = zone_caps.get(&target_zone).copied().unwrap_or(u32::MAX);

            loop {
                let active = zone_active.get(&target_zone).copied().unwrap_or(0);
                if active < cap {
                    break;
                }
                if let Some(free_at) = active_events
                    .iter()
                    .filter(|e| e.zone_id == target_zone)
                    .map(|e| e.finish_ms)
                    .min()
                {
                    if free_at > earliest_start {
                        earliest_start = free_at;
                    }
                    active_events.retain(|e| {
                        if e.finish_ms <= earliest_start {
                            *zone_active.entry(e.zone_id).or_insert(1) =
                                zone_active.get(&e.zone_id).copied().unwrap_or(1).saturating_sub(1);
                            false
                        } else {
                            true
                        }
                    });
                } else {
                    break;
                }
            }
        }

        let start_ms = earliest_start;
        let slice_ms = task.remaining_ms.min(QUANTUM_MS);
        let finish_ms = start_ms + slice_ms;

        worker_available_ms[worker_index] = finish_ms;
        worker_busy_ms[worker_index] += slice_ms;

        if let Some(zone_id) = task.required_zone {
            *zone_active.entry(zone_id).or_insert(0) += 1;
            active_events.push(SimEvent { finish_ms, zone_id });
        }

        task_duration_ms.insert(task.id, task.total_duration_ms);
        task_priority.insert(task.id, task.priority);

        task_timings.push(StrategyTaskTiming {
            task_id: task.id,
            task_name: task.name.clone(),
            priority: map_priority(task.priority),
            worker_id: worker_index as u64 + 1,
            start_ms,
            finish_ms,
            duration_ms: slice_ms,
        });

        task.remaining_ms -= slice_ms;
        if task.remaining_ms == 0 {
            completion_ms_by_task.insert(task.id, finish_ms);
        } else {
            queue.push_back(task);
        }
    }

    let task_count = completion_ms_by_task.len() as u64;
    let total_completion_ms = completion_ms_by_task.values().copied().sum::<u64>();
    let total_wait_ms = completion_ms_by_task
        .iter()
        .map(|(task_id, completion_ms)| {
            completion_ms.saturating_sub(*task_duration_ms.get(task_id).unwrap_or(&0))
        })
        .sum::<u64>();
    let (high_completion_ms, high_count) = completion_ms_by_task.iter().fold(
        (0_u64, 0_u64),
        |(sum, count), (task_id, completion_ms)| {
            if matches!(task_priority.get(task_id), Some(CoreTaskPriority::High)) {
                (sum + completion_ms, count + 1)
            } else {
                (sum, count)
            }
        },
    );

    let makespan_ms = worker_available_ms.into_iter().max().unwrap_or(0);
    let avg_completion_ms = if task_count == 0 {
        0
    } else {
        total_completion_ms / task_count
    };
    let avg_wait_ms = if task_count == 0 {
        0
    } else {
        total_wait_ms / task_count
    };
    let avg_high_priority_completion_ms = if high_count == 0 {
        0
    } else {
        high_completion_ms / high_count
    };

    StrategySummary {
        scheduler: SchedulerKind::RoundRobin,
        makespan_ms,
        avg_completion_ms,
        avg_wait_ms,
        avg_high_priority_completion_ms,
        avg_completion_improvement_vs_fifo_ms: 0,
        avg_high_priority_improvement_vs_fifo_ms: 0,
        worker_busy_ms,
        speedup_vs_fifo_pct: 0.0,
        task_timings,
    }
}

async fn update_config(State(app): State<AppState>, Json(new_config): Json<Config>) -> StatusCode {
    app.apply_config(new_config);
    StatusCode::NO_CONTENT
}

async fn control_system(
    State(app): State<AppState>,
    Json(body): Json<ControlRequest>,
) -> StatusCode {
    app.control(body.action)
}

fn control_inner(app: &AppState, action: ControlAction) -> StatusCode {
    let mut guard = app.inner.lock().unwrap_or_else(|e| e.into_inner());
    match action {
        ControlAction::Start => {
            guard.system_status = SystemStatus::Running;
            guard.worker_pause.resume();
            StatusCode::NO_CONTENT
        }
        ControlAction::Pause => {
            guard.system_status = SystemStatus::Paused;
            guard.worker_pause.pause();
            StatusCode::NO_CONTENT
        }
        ControlAction::Stop => {
            guard.run_generation += 1;

            // Signal old threads to stop, then replace with fresh flags.
            guard.monitor_shutdown.store(true, Ordering::SeqCst);
            guard.worker_shutdown.store(true, Ordering::SeqCst);
            guard.task_queue.close();

            guard.heartbeats = Arc::new(HeartbeatRegistry::new());
            guard.metrics = Arc::new(MetricsRegistry::new());
            guard.task_table = Arc::new(TaskTable::new());
            guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
            guard.worker_shutdown = Arc::new(AtomicBool::new(false));
            guard.monitor_shutdown = Arc::new(AtomicBool::new(false));
            guard.system_status = SystemStatus::Stopped;
            StatusCode::NO_CONTENT
        }
        ControlAction::RunDemo => {
            guard.run_generation += 1;
            let run_generation = guard.run_generation;

            // Stop lingering threads from any previous run before
            // creating fresh state for this one.
            guard.monitor_shutdown.store(true, Ordering::SeqCst);
            guard.worker_shutdown.store(true, Ordering::SeqCst);
            guard.task_queue.close();

            guard.system_status = SystemStatus::Running;
            guard.heartbeats = Arc::new(HeartbeatRegistry::new());
            guard.metrics = Arc::new(MetricsRegistry::new());
            guard.task_table = Arc::new(TaskTable::new());
            guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
            guard.zone_manager = Arc::new(ZoneManager::new(default_zones()));
            guard.worker_shutdown = Arc::new(AtomicBool::new(false));
            guard.worker_pause.resume();
            guard.monitor_shutdown = Arc::new(AtomicBool::new(false));

            let config = guard.config.clone();
            let heartbeats = Arc::clone(&guard.heartbeats);
            let metrics = Arc::clone(&guard.metrics);
            let task_table = Arc::clone(&guard.task_table);
            let task_queue = Arc::clone(&guard.task_queue);
            let zone_manager = Arc::clone(&guard.zone_manager);
            let worker_shutdown = Arc::clone(&guard.worker_shutdown);
            let worker_pause = Arc::clone(&guard.worker_pause);
            let monitor_shutdown = Arc::clone(&guard.monitor_shutdown);
            let inner = Arc::clone(&app.inner);
            drop(guard);

            std::thread::spawn(move || {
                let mut coord = CoordinatorBuilder::new(config).build(&task_table, &task_queue);
                let robot_ids: Vec<RobotId> = (1..=coord.robots.len() as u64).collect();
                spawn_monitor_thread(
                    Arc::clone(&heartbeats),
                    Arc::clone(&metrics),
                    Arc::clone(&monitor_shutdown),
                    Duration::from_secs(2),
                    robot_ids,
                );

                coord.run_demo(
                    heartbeats,
                    metrics,
                    task_table,
                    task_queue,
                    zone_manager,
                    worker_shutdown,
                    worker_pause,
                );

                let mut guard = inner.lock().unwrap_or_else(|e| e.into_inner());
                if guard.run_generation == run_generation {
                    guard.system_status = SystemStatus::Stopped;
                }
            });

            StatusCode::ACCEPTED
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_scheduling_analysis;
    use crate::coordinator::builder::demo_task_plans;
    use crate::types::config::SchedulerKind;

    #[test]
    fn priority_finishes_urgent_work_faster_than_fifo_on_long_demo_input() {
        let analysis = build_scheduling_analysis(&demo_task_plans(18), 3);
        let fifo = analysis
            .strategies
            .iter()
            .find(|summary| matches!(summary.scheduler, SchedulerKind::Fifo))
            .expect("fifo summary");
        let priority = analysis
            .strategies
            .iter()
            .find(|summary| matches!(summary.scheduler, SchedulerKind::Priority))
            .expect("priority summary");

        assert!(
            priority.avg_high_priority_completion_ms < fifo.avg_high_priority_completion_ms,
            "priority scheduling should finish urgent tasks sooner"
        );
    }

    #[test]
    fn srt_improves_average_completion_time_over_fifo() {
        let analysis = build_scheduling_analysis(&demo_task_plans(18), 3);
        let fifo = analysis
            .strategies
            .iter()
            .find(|summary| matches!(summary.scheduler, SchedulerKind::Fifo))
            .expect("fifo summary");
        let srt = analysis
            .strategies
            .iter()
            .find(|summary| matches!(summary.scheduler, SchedulerKind::Srt))
            .expect("srt summary");

        assert!(
            srt.avg_completion_ms < fifo.avg_completion_ms,
            "SRT should reduce average completion time compared with FIFO"
        );
    }

    #[test]
    fn round_robin_strategy_is_included_in_analysis() {
        let analysis = build_scheduling_analysis(&demo_task_plans(18), 3);
        let round_robin = analysis
            .strategies
            .iter()
            .find(|summary| matches!(summary.scheduler, SchedulerKind::RoundRobin))
            .expect("round robin summary");

        assert!(
            round_robin.task_timings.len() > 18,
            "Round Robin should emit multiple time slices for longer tasks"
        );
    }
}
