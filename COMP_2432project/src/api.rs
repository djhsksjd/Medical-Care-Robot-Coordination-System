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
use crate::coordinator::lifecycle::Coordinator;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;
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
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Pending,
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

#[derive(Debug)]
struct SharedState {
    coord: Coordinator,
    heartbeats: HeartbeatRegistry,
    metrics: MetricsRegistry,
    system_status: SystemStatus,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<SharedState>>,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::default();
        let builder = CoordinatorBuilder::new(config.clone());
        let coord = builder.build();
        let heartbeats = HeartbeatRegistry::new();
        let metrics = MetricsRegistry::new();
        Self {
            inner: Arc::new(Mutex::new(SharedState {
                coord,
                heartbeats,
                metrics,
                system_status: SystemStatus::Stopped,
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
    let robot_ids: Vec<RobotId> = guard.coord.robots.iter().map(|r| r.id).collect();
    let report: SystemReport =
        build_report(&guard.heartbeats, &guard.metrics, &robot_ids, hb_timeout);

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

    // Build a simple demo task list derived from config and metrics.
    let demo_count = guard.coord.config.demo_task_count as u64;
    let completed = throughput.min(demo_count);
    let tasks: Vec<Task> = (0..demo_count)
        .map(|id| Task {
            id,
            name: format!("demo-task-{id}"),
            priority: if id % 3 == 0 {
                TaskPriority::High
            } else if id % 3 == 1 {
                TaskPriority::Normal
            } else {
                TaskPriority::Low
            },
            status: if id < completed {
                TaskStatus::Finished
            } else {
                TaskStatus::Pending
            },
            robot_id: None,
            zone_id: Some(1),
            expected_duration_ms: 30_000, // 与后端实际执行时间一致（约 30 秒）
            started_at: None,
            finished_at: None,
        })
        .collect();

    // Single demo zone derived from config and robot count.
    let zones: Vec<Zone> = vec![Zone {
        id: 1,
        name: "zone-1".to_string(),
        capacity: demo_count as u32,
        current_tasks: (demo_count - completed) as u32,
        active_robots: robots.len() as u32,
        health: ZoneHealth::Normal,
    }];

    let state = SystemState {
        tasks,
        robots,
        zones,
        config: guard.coord.config.clone(),
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
    // Rebuild coordinator with new configuration.
    let builder = CoordinatorBuilder::new(new_config.clone());
    guard.coord = builder.build();
    guard.system_status = SystemStatus::Stopped;
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
        }
        ControlAction::Pause => {
            guard.system_status = SystemStatus::Paused;
        }
        ControlAction::Stop => {
            guard.system_status = SystemStatus::Stopped;
        }
        ControlAction::RunDemo => {
            // For now, RunDemo just runs the demo synchronously in-process and
            // lets monitoring capture the activity.
            guard.system_status = SystemStatus::Running;
            let config = guard.coord.config.clone();
            let coord = std::mem::replace(&mut guard.coord, CoordinatorBuilder::new(config).build());
            drop(guard); // release lock while running demo
            coord.run_demo();
        }
    }
    StatusCode::NO_CONTENT
}

