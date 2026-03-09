//! HTTP API 层，充当「内核 ↔ 前端」之间的胶水层。
//! - 通过 `AppState` 持有 Coordinator + 监控子系统
//! - 对外暴露 /api/state, /api/config, /api/system/control 等端点，供前端 Dashboard 调用

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::coordinator::builder::CoordinatorBuilder;
use crate::coordinator::task_table::TaskTable;
use crate::mm::zone_allocator::ZoneManager;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
use crate::monitor::monitor_thread::spawn_monitor_thread;
use crate::scheduler::thread_safe_queue::ThreadSafeTaskQueue;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::monitor::reporter::{build_report, SystemReport};
use crate::types::config::Config;
use crate::types::robot::RobotId;

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
pub struct SystemState {
    pub tasks: Vec<Task>,
    pub robots: Vec<Robot>,
    pub zones: Vec<Zone>,
    pub config: Config,
    pub metrics: Metrics,
    pub system_status: SystemStatus,
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
            state: match rr.health.status {
                crate::monitor::health_checker::RobotHealthStatus::Healthy => WorkerState::Idle,
                crate::monitor::health_checker::RobotHealthStatus::Degraded => WorkerState::Busy,
                crate::monitor::health_checker::RobotHealthStatus::Unreachable => {
                    WorkerState::Stopped
                }
            },
            current_task_id: None,
            recent_completed: rr.metrics.completed_tasks,
        })
        .collect();

    // Build task list from the central TaskTable so we reflect real statuses.
    let task_snapshots = guard.task_table.all();
    let tasks: Vec<Task> = task_snapshots
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
    };

    Json(state)
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

