## Project Blaze – Rust Backend Design

### 1. Goals and Scope

- **Course context**: COMP2432 Operating Systems – focus on concurrency, synchronization, coordination.
- **Domain**: multi-robot hospital scenario (delivery / disinfection / assistance).
- **Scope of this design**:
  - The **Rust backend crate** `COMP_2432project`.
  - Models tasks, robots, zones and monitoring using OS-like abstractions.
  - Exposes a simple HTTP API that the React dashboard consumes.

High-level goals:

- **OS-inspired architecture**: treat Tasks like processes, Robots like CPUs, Zones like resources.
- **Safe concurrency**: use Rust primitives to avoid data races and inconsistent state.
- **Observability**: first-class monitoring (heartbeats, metrics, health evaluation).
- **Configurability**: allow tuning worker count, number of tasks, and future scheduling strategies.

---

### 2. Top-Level Architecture

The project is a standard Cargo crate with the following Rust-side structure:

- `src/lib.rs` – library root; exports all subsystems.
- `src/main.rs` – binary entry; starts the HTTP API server.
- Subsystems (modules):
  - `types/` – core domain types (Task, Robot, Zone, Config, Error).
  - `scheduler/` – task queues and scheduling policies (FIFO now, hooks for more).
  - `worker/` – Robot workers and WorkerPool (execution side).
  - `monitor/` – heartbeats, metrics, health-checker, reporting.
  - `mm/` – memory/resource management (zones, allocation table; mostly interfaces now).
  - `sync/` – thin wrappers around `Mutex`, `RwLock`, atomics, etc.
  - `util/` – logging, timers, ID generators, random helpers.
  - `coordinator/` – “kernel core” that wires scheduler + workers + monitor together.
  - `api/` – HTTP API layer (axum) that exposes state and controls.
  - `prelude.rs` – convenience re-exports for commonly used items.

Conceptually, the **Coordinator** behaves like a small OS kernel:

- Owns a scheduler and the set of Robots.
- Uses a WorkerPool to execute Tasks on Robots.
- Uses the Monitor subsystem to track heartbeats and metrics.
- Is driven by the HTTP API (e.g. “run demo”, change config).

---

### 3. Domain Model

#### 3.1 Tasks (Processes)

File: `src/types/task.rs`

- `TaskId = u64`
- `TaskPriority = Low | Normal | High`
- `TaskStatus = Pending | Running | Finished`
- `Task`:
  - `id: TaskId`
  - `name: String`
  - `priority: TaskPriority`
  - `expected_duration: Duration` (simulation runtime)
  - `status: TaskStatus`

Design notes:

- **Analogy**: a Task is the unit of work like a process or thread in an OS.
- The current scheduler is FIFO and ignores `priority`, but the type is ready for priority scheduling in future iterations.
- `expected_duration` is used by workers to simulate execution time; in the demo it is set to about 30 seconds.

#### 3.2 Robots (CPUs)

File: `src/types/robot.rs`

- `RobotId = u64`
- `Robot`:
  - `id: RobotId`
  - `name: String` (e.g. `"robot-1"`)

Design notes:

- Each Robot represents an execution core (like a CPU core).
- The `WorkerPool` and `RobotWorker` bind to `Robot` values to simulate multi-CPU scheduling.

#### 3.3 Zones (Resource Domains)

File: `src/types/zone.rs`

- `ZoneId = u64`
- `Zone`:
  - `id: ZoneId`
  - `name: String`
  - `capacity: u32`

Design notes:

- Zones represent hospital areas or resource domains (e.g. ICU, corridor, OR).
- They are the natural place to enforce **mutual exclusion**: ensure no two robots occupy the same zone concurrently.
- The `mm` subsystem will own and evolve the zone management logic.

#### 3.4 Config and Error Types

File: `src/types/config.rs`

- `SchedulerKind`:
  - `Fifo` (future: `Priority`, `RoundRobin`).
- `Config` (serde, camelCase):
  - `scheduler: SchedulerKind`
  - `worker_count: usize` – how many Robots to create.
  - `demo_task_count: usize` – how many demo tasks to seed in the scheduler.
- `Default` config:
  - `scheduler = Fifo`
  - `worker_count = 2` (multi-robot by default)
  - `demo_task_count = 5`

File: `src/types/error.rs`

- `Error` enum:
  - `SchedulerEmpty`, `WorkerStopped`, `ZoneUnavailable`, `Other(String)`.
- `Result<T> = std::result::Result<T, Error>`.

Design notes:

- `Config` is the central knob for tuning experiments and demos.
- The error type expresses OS-like error conditions in a lightweight way.

---

### 4. Scheduling Subsystem

Directory: `src/scheduler/`

#### 4.1 TaskQueue

File: `scheduler/queue.rs`

- `TaskQueue`:
  - Internal `VecDeque<Task>`.
  - API:
    - `push(task: Task)`
    - `pop() -> Option<Task>`
    - `is_empty() -> bool`

Design notes:

- A minimal **non-thread-safe** FIFO queue.
- Suitable for single-threaded scheduling; later can be replaced or wrapped for multi-threaded use (e.g. `Mutex<VecDeque> + Condvar` to support blocking pops).

#### 4.2 FIFO Scheduler

File: `scheduler/fifo.rs`

- `FifoScheduler`:
  - Holds a `TaskQueue`.
  - `new() -> FifoScheduler`
  - `submit(task: Task)` – enqueue task.
  - `next_task() -> Result<Task>` – dequeue front or `Error::SchedulerEmpty`.
  - `is_empty() -> bool`

Design notes:

- Strict FIFO (First-In-First-Out) with no timeslicing and no preemption.
- Models a simple **cooperative** scheduler that drains tasks in arrival order.
- Future schedulers (priority-based, round-robin) can share the same `Task` type and coexist behind a trait.

#### 4.3 Future Work

- Implement a `Scheduler` trait with methods like `submit`, `next_task`, `is_empty`.
- Provide:
  - `PriorityScheduler` that reorders by `TaskPriority`.
  - `RoundRobinScheduler` with explicit timeslices and fairness.

---

### 5. Worker Subsystem

Directory: `src/worker/`

#### 5.1 Worker State

File: `worker/state.rs`

- `WorkerState`:
  - `Idle`, `Busy`, `Stopped`.
- `is_active()` returns true for `Idle` or `Busy`.

Design notes:

- Separates **lifecycle state** of a RobotWorker from task state.
- Used to short-circuit execution if a worker is logically stopped.

#### 5.2 RobotWorker

File: `worker/robot.rs`

- `RobotWorker<'a>`:
  - `robot: Robot`
  - `state: WorkerState`
  - `scheduler: &'a mut FifoScheduler`
  - `heartbeats: &'a HeartbeatRegistry`
  - `metrics: &'a MetricsRegistry`
- `new(robot, scheduler, heartbeats, metrics)`:
  - Binds a Robot to the shared scheduler and monitoring registries.
- `run_once() -> Result<()>`:
  - If worker is `Stopped`, return `Error::WorkerStopped`.
  - Call `scheduler.next_task()` to get the next Task.
  - Mark worker as `Busy`, touch heartbeat.
  - Log start event.
  - Mark Task as `Running`.
  - Sleep for `min(expected_duration, 60s)` (≈30s in current demo).
  - Compute `exec_time`, mark Task as `Finished`, worker as `Idle`.
  - Touch heartbeat again, record completion metrics.
  - Log finish event.
- `stop()`:
  - Set `state = Stopped`.

Design notes:

- Models an execution context pinned to a specific Robot (CPU).
- Uses `expected_duration` purely for simulation; no real work is done.
- Reporting to `MetricsRegistry` and `HeartbeatRegistry` enables the monitoring subsystem.

#### 5.3 WorkerPool

File: `worker/pool.rs`

- `WorkerPool<'a>`:
  - `robots: Vec<Robot>`
  - `scheduler: &'a mut FifoScheduler`
  - `heartbeats: &'a HeartbeatRegistry`
  - `metrics: &'a MetricsRegistry`
- `new(robots, scheduler, heartbeats, metrics)`:
  - Clones robots into the pool; all share the same scheduler and monitors.
- `run_until_empty()`:
  - Repeatedly:
    - Create a `RobotWorker` for each robot (one by one).
    - Call `run_once()`; if it succeeds for any robot, mark `made_progress = true`.
    - If no robot made progress in this pass (no tasks), break.

Design notes:

- This is a **single OS-thread** simulation of a multi-CPU system:
  - Multiple robots compete for tasks from the same scheduler.
  - In the future this can be converted into real threads (`std::thread::spawn`) plus a thread-safe queue.

---

### 6. Monitoring Subsystem

Directory: `src/monitor/`

#### 6.1 Heartbeats

File: `monitor/heartbeat.rs`

- `HeartbeatRegistry`:
  - `inner: RwLock<HashMap<RobotId, Instant>>`.
  - `touch(robot_id)`: update last-seen timestamp.
  - `last_seen(robot_id) -> Option<Instant>`.
  - `stale_robots(timeout) -> Vec<RobotId>`.

Design notes:

- This is analogous to a **watchdog** or liveness tracking for every CPU.
- RobotWorkers call `touch()` before and after running tasks.

#### 6.2 Metrics

File: `monitor/metrics.rs`

- `RobotMetrics`:
  - `completed_tasks: u64`
  - `total_exec_time: Duration`
  - `avg_exec_time() -> Option<Duration>`
- `GlobalMetrics`:
  - Same fields aggregated across all robots.
- `MetricsRegistry`:
  - `robots: Mutex<HashMap<RobotId, RobotMetrics>>`
  - `global: Mutex<GlobalMetrics>`
  - `record_completion(robot_id, exec_time)`: updates both maps.
  - `snapshot() -> (GlobalMetrics, HashMap<RobotId, RobotMetrics>)`.

Design notes:

- Can be thought of as a `/proc`-style statistics source.
- The HTTP API compresses this into a simpler DTO for the frontend.

#### 6.3 Health Checker and Reporter

File: `monitor/health_checker.rs`

- Classifies each robot as:
  - `Healthy`, `Degraded`, `Unreachable` based on heartbeat and metrics.
- Produces `SystemHealth` with an overall `SystemHealthStatus`.

File: `monitor/reporter.rs`

- Combines `HeartbeatRegistry` and `MetricsRegistry` into a `SystemReport`:
  - `GlobalMetrics` plus per-robot metrics and health.
- This is consumed by the HTTP API to build a frontend-friendly `SystemState`.

---

### 7. Coordinator (Kernel Core)

Directory: `src/coordinator/`

#### 7.1 Coordinator Structure

File: `coordinator/lifecycle.rs`

- `Coordinator`:
  - `config: Config`
  - `scheduler: FifoScheduler`
  - `robots: Vec<Robot>`

#### 7.2 CoordinatorBuilder

File: `coordinator/builder.rs`

- Input: `Config`.
- Behavior:
  - Create a `FifoScheduler`.
  - Seed `demo_task_count` demo tasks:
    - `Task::new(next_task_id(), "demo-task", Duration::from_secs(30))`.
  - Create `worker_count` Robots:
    - IDs `1..=worker_count`, names `"robot-1"`, `"robot-2"`, etc.
  - Return `Coordinator::new(config, scheduler, robots)`.

#### 7.3 Running a Demo

`Coordinator::run_demo(self)`:

- Logs the start of a demo run.
- Instantiates `HeartbeatRegistry` and `MetricsRegistry`.
- Builds a `WorkerPool` with cloned robots and a mutable scheduler reference.
- Runs `WorkerPool::run_until_empty()`:
  - RobotWorkers consume all tasks from the scheduler.
  - Each execution updates heartbeats and metrics.
- Logs completion of the demo.

Design notes:

- This method currently runs **synchronously** in a single OS thread.
- Future improvement is to spawn multiple OS threads and let each run a RobotWorker loop in parallel.

---

### 8. HTTP API and Frontend Integration

Directory: `src/api.rs`, entry in `src/main.rs`

#### 8.1 SharedState and AppState

- `SharedState`:
  - `coord: Coordinator`
  - `heartbeats: HeartbeatRegistry`
  - `metrics: MetricsRegistry`
  - `system_status: SystemStatus`
- `AppState`:
  - Wraps `SharedState` in `Arc<Mutex<_>>`.
  - `AppState::new()` builds default config, coordinator and monitoring registries.

#### 8.2 API Endpoints

Using `axum`, the project exposes:

- `GET /api/state`:
  - Reads monitoring snapshot and coordinator config.
  - Maps internal types to DTOs:
    - `robots`: per-robot metrics and health.
    - `tasks`: synthesized from `demo_task_count` and global throughput; each has `expected_duration_ms ≈ 30000`.
    - `zones`: currently a single demo zone derived from config and robot count.
    - `metrics`: throughput and average latency (in ms).
    - `system_status`.
- `PUT /api/config`:
  - Accepts a new `Config` JSON.
  - Rebuilds `Coordinator` with the new config, resets `system_status` to `Stopped`.
- `POST /api/system/control`:
  - `Start` / `Pause` / `Stop`: update `system_status`.
  - `RunDemo`: rebuild and run `Coordinator::run_demo()` synchronously to execute the demo scenario.

#### 8.3 HTTP Server Entry

File: `src/main.rs`

- Uses `#[tokio::main]` to run an async server.
- Builds a router with `build_router(AppState::new())`.
- Applies a permissive CORS layer (`tower_http::cors::CorsLayer`).
- Binds to `0.0.0.0:3000` and serves requests.

---

### 9. Concurrency Model and Future Evolution

Current concurrency model:

- Internally:
  - Single OS thread runs `Coordinator::run_demo()` and the WorkerPool.
  - `MetricsRegistry` and `HeartbeatRegistry` are thread-safe and could be shared across threads.
- From outside:
  - Multiple HTTP calls can be serialized through `Mutex<SharedState>`; the API itself is asynchronous but state access is synchronized.

Planned improvements (future versions):

- Introduce a **thread-safe task queue** (e.g. `Mutex<VecDeque<Task>> + Condvar`) and a `ThreadSafeScheduler` wrapper.
- Spawn real OS threads for each RobotWorker, each running:
  - `loop { pop_blocking(); run Task; update metrics/heartbeat; }`.
- Run a dedicated **monitor thread** that periodically:
  - Reads `HeartbeatRegistry` and `MetricsRegistry`.
  - Calls `health_checker::evaluate_health`.
  - Logs warnings or updates a shared `SystemHealth` state for the API.
- Extend `mm` to:
  - Implement zone allocation and mutual exclusion on zones.
  - Surface real per-zone load and conflicts (or deadlock detection) through the API.

This design keeps the codebase small and understandable, while clearly mapping back to core OS topics: **process/task abstraction, CPU (Robot) scheduling, resource management, and system monitoring**.

