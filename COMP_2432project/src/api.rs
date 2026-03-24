//! HTTP API 层，充当「内核 ↔ 前端」之间的胶水层。
//! - 通过 `AppState` 持有 Coordinator + 监控子系统
//! - 对外暴露 /api/state, /api/config, /api/system/control 等端点，供前端 Dashboard 调用

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::coordinator::builder::{demo_task_plans, CoordinatorBuilder, DemoTaskPlan};
use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::monitor::monitor_thread::spawn_monitor_thread;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::scheduler::SchedulerStrategy;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::monitor::reporter::{build_report, SystemReport};
use crate::types::config::{Config, SchedulerKind};
use crate::types::robot::RobotId;
use crate::types::task::{Task as CoreTask, TaskPriority as CoreTaskPriority};

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

#[derive()]
struct SharedState {
    config: Config,
    heartbeats: Arc<HeartbeatRegistry>,
    metrics: Arc<MetricsRegistry>,
    task_table: Arc<TaskTable>,
    task_queue: Arc<ThreadSafeTaskQueue>,
    zone_manager: Arc<ZoneManager>,
    worker_shutdown: Arc<AtomicBool>,
    worker_pause: Arc<AtomicBool>,
    monitor_shutdown: Arc<AtomicBool>,
    system_status: SystemStatus,
    run_generation: u64,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<SharedState>>,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::default();
        let heartbeats = Arc::new(HeartbeatRegistry::new());
        let metrics = Arc::new(MetricsRegistry::new());
        let task_table = Arc::new(TaskTable::new());
        let task_queue = Arc::new(ThreadSafeTaskQueue::new());
        let zones = vec![
            crate::types::zone::Zone::new(1, "ICU".to_string(), 2),
            crate::types::zone::Zone::new(2, "Ward".to_string(), 4),
            crate::types::zone::Zone::new(3, "OR".to_string(), 1),
        ];
        let zone_manager = Arc::new(ZoneManager::new(zones));
        let worker_shutdown = Arc::new(AtomicBool::new(false));
        let worker_pause = Arc::new(AtomicBool::new(false));
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ControlAction {
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
    let guard = app.inner.lock().expect("lock poisoned");
    let hb_timeout = Duration::from_secs(5);
    let demo_plans = demo_task_plans(guard.config.demo_task_count);
    let scheduling_analysis = build_scheduling_analysis(&demo_plans, guard.config.worker_count);
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
    let robot_ids: Vec<RobotId> = (1..=guard.config.worker_count as u64).collect();
    let report: SystemReport =
        build_report(guard.heartbeats.as_ref(), guard.metrics.as_ref(), &robot_ids, hb_timeout);

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
                    crate::monitor::health_checker::RobotHealthStatus::Healthy => {
                        WorkerState::Idle
                    }
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
            },
            robot_id: snap.robot_id.map(|id| id as u64),
            zone_id: snap.zone_id.map(|id| id as u64),
            expected_duration_ms: snap.task.expected_duration.as_secs() as u64 * 1000,
            started_at: snap
                .started_at
                .map(|t| format!("{t:?}")),
            finished_at: snap
                .finished_at
                .map(|t| format!("{t:?}")),
        })
        .collect();
    tasks.sort_by_key(|task| task.id);

    // Compute zone statistics based on ZoneManager allocations and task table.
    let mut zones: Vec<Zone> = Vec::new();
    for z in guard.zone_manager.zones() {
        let current_tasks = tasks
            .iter()
            .filter(|t| {
                t.zone_id == Some(z.id)
                    && matches!(t.status, TaskStatus::Pending | TaskStatus::Running)
            })
            .count() as u32;

        // 简单地认为有任务分配到该区域的机器人都是 active。
        let active_robots = robots.len() as u32;

        let health = if current_tasks > z.capacity {
            ZoneHealth::HighLoad
        } else {
            ZoneHealth::Normal
        };

        zones.push(Zone {
            id: z.id,
            name: z.name.clone(),
            capacity: z.capacity,
            current_tasks,
            active_robots,
            health,
        });
    }

    let state = SystemState {
        tasks,
        robots,
        zones,
        config: guard.config.clone(),
        metrics: Metrics {
            throughput,
            avg_latency_ms,
        },
        system_status: guard.system_status,
        scheduling_analysis,
    };

    Json(state)
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
        })
        .collect::<Vec<_>>();

    let worker_count = worker_count.max(1);
    let mut strategies = vec![
        simulate_strategy(SchedulerKind::Fifo, demo_plans, worker_count),
        simulate_strategy(SchedulerKind::Priority, demo_plans, worker_count),
    ];

    let fifo_makespan = strategies
        .iter()
        .find(|summary| matches!(summary.scheduler, SchedulerKind::Fifo))
        .map(|summary| summary.makespan_ms)
        .unwrap_or(0);

    for summary in &mut strategies {
        summary.speedup_vs_fifo_pct = if fifo_makespan == 0 {
            0.0
        } else {
            ((fifo_makespan as f64 - summary.makespan_ms as f64) / fifo_makespan as f64) * 100.0
        };
    }

    SchedulingAnalysis {
        input_tasks,
        strategies,
        worker_count,
    }
}

fn simulate_strategy(
    scheduler_kind: SchedulerKind,
    demo_plans: &[DemoTaskPlan],
    worker_count: usize,
) -> StrategySummary {
    let mut scheduler = SchedulerStrategy::new(scheduler_kind);

    for plan in demo_plans {
        let mut task = CoreTask::new(plan.sequence, plan.name.clone(), plan.expected_duration);
        task.priority = plan.priority;
        scheduler.submit(task);
    }

    let mut ordered_tasks = Vec::with_capacity(demo_plans.len());
    while !scheduler.is_empty() {
        if let Ok(task) = scheduler.next_task() {
            ordered_tasks.push(task);
        }
    }

    let mut worker_available_ms = vec![0_u64; worker_count.max(1)];
    let mut worker_busy_ms = vec![0_u64; worker_count.max(1)];
    let mut task_timings = Vec::with_capacity(ordered_tasks.len());
    let mut total_completion_ms = 0_u64;
    let mut total_wait_ms = 0_u64;
    let mut high_completion_ms = 0_u64;
    let mut high_count = 0_u64;

    for task in ordered_tasks {
        let duration_ms = task.expected_duration.as_millis() as u64;
        let worker_index = worker_available_ms
            .iter()
            .enumerate()
            .min_by_key(|(index, available_at)| (**available_at, *index))
            .map(|(index, _)| index)
            .unwrap_or(0);

        let start_ms = worker_available_ms[worker_index];
        let finish_ms = start_ms + duration_ms;
        worker_available_ms[worker_index] = finish_ms;
        worker_busy_ms[worker_index] += duration_ms;

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
            duration_ms,
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
        worker_busy_ms,
        speedup_vs_fifo_pct: 0.0,
        task_timings,
    }
}

async fn update_config(
    State(app): State<AppState>,
    Json(new_config): Json<Config>,
) -> StatusCode {
    let mut guard = app.inner.lock().expect("lock poisoned");
    guard.config = new_config;
    guard.heartbeats = Arc::new(HeartbeatRegistry::new());
    guard.metrics = Arc::new(MetricsRegistry::new());
    guard.task_table = Arc::new(TaskTable::new());
    guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
    guard.system_status = SystemStatus::Stopped;
    guard.worker_shutdown.store(false, Ordering::SeqCst);
    guard.worker_pause.store(false, Ordering::SeqCst);
    guard.monitor_shutdown.store(false, Ordering::SeqCst);
    guard.run_generation += 1;
    StatusCode::NO_CONTENT
}

async fn control_system(
    State(app): State<AppState>,
    Json(body): Json<ControlRequest>,
) -> StatusCode {
    let mut guard = app.inner.lock().expect("lock poisoned");
    match body.action {
        ControlAction::Start => {
            guard.system_status = SystemStatus::Running;
            // Start simply clears pause flag; workers will already be running for demo runs.
            guard.worker_pause.store(false, Ordering::SeqCst);
            StatusCode::NO_CONTENT
        }
        ControlAction::Pause => {
            guard.system_status = SystemStatus::Paused;
            guard.worker_pause.store(true, Ordering::SeqCst);
            StatusCode::NO_CONTENT
        }
        ControlAction::Stop => {
            guard.run_generation += 1;
            // Signal shutdown to workers and monitor, and close the task queue to wake any waiters.
            guard.worker_shutdown.store(true, Ordering::SeqCst);
            guard.monitor_shutdown.store(true, Ordering::SeqCst);
            guard.task_queue.close();

            guard.heartbeats = Arc::new(HeartbeatRegistry::new());
            guard.metrics = Arc::new(MetricsRegistry::new());
            guard.task_table = Arc::new(TaskTable::new());
            guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
            guard.system_status = SystemStatus::Stopped;
            StatusCode::NO_CONTENT
        }
        ControlAction::RunDemo => {
            guard.run_generation += 1;
            let run_generation = guard.run_generation;
            guard.system_status = SystemStatus::Running;
            guard.heartbeats = Arc::new(HeartbeatRegistry::new());
            guard.metrics = Arc::new(MetricsRegistry::new());
            guard.task_table = Arc::new(TaskTable::new());
            guard.task_queue = Arc::new(ThreadSafeTaskQueue::new());
            guard.worker_shutdown.store(false, Ordering::SeqCst);
            guard.worker_pause.store(false, Ordering::SeqCst);
            guard.monitor_shutdown.store(false, Ordering::SeqCst);

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
                let mut coord =
                    CoordinatorBuilder::new(config).build(&task_table, &task_queue);
                // 启动监控线程：定期评估健康状态并打印日志。
                let robot_ids: Vec<RobotId> =
                    (1..=coord.config.worker_count as u64).collect();
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

                let mut guard = inner.lock().expect("lock poisoned");
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
}

