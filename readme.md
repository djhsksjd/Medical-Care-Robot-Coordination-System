# Project Blaze: Medical Care Robot Coordination System — Requirements Document
**Course**: PolyU COMP2432 Operating Systems (2025–2026)
**Core Focus**: OS concurrency, synchronization, and safe coordination concepts
**Implementation Scope**: Lightweight OS kernel; minimalist design — no preemption, deadlock prevention, or complex scheduling policies; focused on concurrency correctness

## 1. Project Overview (Current Implementation Progress)

This project implements a lightweight task scheduling and monitoring kernel for medical care robots, analogous to a small, purpose-built OS kernel:

- **Task ≈ Process**: Represents a care / transport / disinfection task to be executed, with fields such as `id / name / priority / status / expected_duration`.
- **Robot ≈ CPU/Core**: Represents a physical robot (or a logical execution unit); the scheduler assigns Tasks to different Robots for execution.
- **Scheduler**: Currently implements a FIFO scheduler; priority scheduling and Round Robin will be added later.
- **WorkerPool**: Creates multiple RobotWorkers based on the configured `worker_count`, simulating multi-CPU task contention.
- **Monitor**: Continuously collects heartbeat, execution metrics, and health status from each Robot — similar to `/proc` statistics and a watchdog.
- **Coordinator**: Acts as the "kernel core", integrating the scheduler, worker threads, resource management, and monitoring, and exposing a unified view via an HTTP API.

The project also provides a React-based frontend Dashboard that polls `/api/state` in real time for system status (task list, robot states, zone utilization, configuration, and metrics) and allows parameter adjustments and demo runs via `/api/config` and `/api/system/control`.

## CLI Scheduling Report (Rust-only submission friendly)

If you can only submit Rust source files, you can still demonstrate the “frontend” information using the built-in **CLI scheduling report** binary (`cli`). It prints:
- The chosen configuration (scheduler / workers / task count / work-stealing / stress preset)
- The demo input task list
- Per-scheduler strategy metrics (makespan, average completion, urgent completion, worker load)
- A final comparison summary table

### How to run

From the `COMP_2432project/` directory:

```bash
cargo run --bin cli
```

This starts **interactive mode** (the program will prompt for parameters, print the report, and allow quitting with `q`).

### Parameters (same semantics as the frontend config)

- **Scheduler**: `-s, --scheduler <KIND>`
  - `fifo` | `priority` | `roundrobin` (or `rr`) | `srt`
- **Worker robots**: `-w, --workers <N>`
- **Demo tasks**: `-t, --tasks <N>`
- **Work stealing mode**: `--work-stealing` (or `--ws`)
- **Stress preset**: `--stress` (forces 12 workers, 108 tasks)

### Examples

```bash
# Default: FIFO, 9 workers, 20 demo tasks
cargo run --bin cli

# Priority scheduler, 6 workers, 18 tasks
cargo run --bin cli -- -s priority -w 6 -t 18

# Stress preset
cargo run --bin cli -- --stress

# SRT with work stealing enabled
cargo run --bin cli -- -s srt --work-stealing
```

---

## System Control Semantics (Stop / Pause / Reset)

To ensure **repeatability** and **predictable state** during demonstrations, the control actions are intentionally simplified (closer to a "teaching kernel demo" than a production system):

- **Pause**: Suspends the worker's task-fetch/execution loop. Implemented via condition variable blocking (not spin-waiting); workers wake immediately upon Resume.
- **Start/Resume**: Clears the paused state and lets workers continue running.
- **Stop**: Signals workers and the monitor to stop, and **closes the task queue** to unblock waiting threads; then resets runtime state including the task table, queue, heartbeats, and metrics.
- **Update Config**: Also resets runtime state to prevent stale tasks/metrics from mixing with the new configuration and interfering with demo observation.

Design trade-offs:
- **Pros**: Every demo starts from a consistent initial state, making it easy to showcase concurrency/synchronization effects and compare scheduling strategies.
- **Cons**: Historical tasks and metrics are not preserved after Stop/Config updates (supporting traceable history would require persistent or run-id-segmented storage for tasks and metrics).

> Current status: The core scheduling path (Task → Scheduler → RobotWorker → Metrics/Heartbeat) is fully operational, supporting multi-robot concurrent demo task execution with real-time frontend visualization.

---

## 2. Core Business Requirements
Based on the scenario of autonomous hospital robots (delivery, disinfection, surgical assistance), the system implements **three core functional modules** to ensure safe coordination during multi-robot concurrent operation:

### 2.1 Task Queue Module
- Store and enqueue robot tasks;
- Support concurrent, safe task retrieval by multiple robots (FIFO principle);
- Guarantee race-condition-free access to the shared task queue with consistent state.

### 2.2 Zone Access Control Module
- Implement mutual exclusion for hospital zones — **no two robots may occupy the same zone simultaneously**;
- Support zone request and release operations with concurrency safety.

### 2.3 Health Monitoring Module
- Track real-time heartbeat status of all robots;
- Allow robots to actively report heartbeats;
- Detect heartbeat timeouts and automatically mark timed-out robots as offline.

## 3. Demonstration Requirements
The system must complete **three demonstration scenarios** that clearly illustrate core OS concurrency and synchronization concepts. Demo results can be presented via terminal output or a visual interface:
1. Multiple robot threads concurrently request and acquire tasks without conflicts;
2. Multiple robots requesting shared zones demonstrate strict mutual exclusion with no concurrent occupancy;
3. Simulate single/multiple robot heartbeat timeouts, with the system successfully marking them as offline and displaying the updated status.

## 4. Technical Implementation Requirements

### 4.1 Language and Project Standards
- Developed in **Rust**, submitted as a standard Cargo project;
- Must compile successfully with `cargo build --release`;
- Write tests with reasonable coverage; all tests must pass via `cargo test`;
- Clear project structure with well-organized modules and a traceable Git commit history reflecting development progress.

### 4.2 Core Technical Requirements
- **Concurrency Control**: Use threads to enable multi-robot parallel execution, ensuring safe access to shared state;
- **Synchronization**: Properly use synchronization primitives (Mutex, RwLock, Condvar, etc.) to completely prevent race conditions and shared-state inconsistency;
- **Thread Coordination**: Multi-worker (robot) thread organization with clear ownership; correct inter-thread interaction logic with no unexpected blocking or crashes;
- **Code Quality**: Readable code following idiomatic Rust conventions; rigorous error handling; no unsafe or dangerous patterns (e.g., indiscriminate `unwrap`).

## 5. Deliverables
Three categories of deliverables are required: **source code, written report, and video demonstration**, each with strict format, content, and technical requirements.

### 5.1 Source Code Deliverables
- Complete implementation of all three core modules, robot simulation, and the system coordination layer;
- Unit tests (per module) and integration tests (multi-robot concurrency scenarios);
- Well-structured project directory with decoupled modules and clear comments on core logic;
- No unnecessary third-party dependencies; prefer Rust standard library for implementation.

### 5.2 Written Report Deliverables
The report must follow a specified structure with each section meeting **word/character count limits**, and must be complete, logically coherent, and academically sound:

| Report Section | Word/Character Limit | Key Content |
|---|---|---|
| Abstract | 800–1100 characters | Concise project summary, core challenges and solutions, key implementation results |
| Introduction | 300–500 words | Problem statement and motivation, core project objectives, implementation approach overview |
| Related Work | 400–700 words | Survey of concurrency control/synchronization techniques, task queue scheduling methods, resource mutual exclusion patterns; comparison with own implementation |
| Implementation | 700–1000 words | System architecture diagram; detailed design and implementation of the three modules, synchronization primitive choices; core critical-section code snippets |
| Benchmarks | 500–700 words | Testing methodology; system correctness verification; performance metrics analysis (task throughput, zone access latency, CPU usage); scalability/stress test results with charts/tables |
| Discussion | ≥500 words | Analysis of test results; design trade-offs; system limitations and performance bottlenecks; comparison of different synchronization primitives; concurrency programming lessons learned |
| Conclusion & Future Work | ≥300 words | Project outcomes summary; how objectives were achieved; potential improvements; future feature roadmap |
| References | — | At least 20 references in APA format; must include Rust official documentation, relevant academic papers, and technical articles |

- Outstanding reports may have opportunities for open publication; sufficient technical depth, clear writing, and comprehensive evaluation are required.

### 5.3 Video Demonstration Deliverables
Video length: **maximum 3 minutes**. Must clearly demonstrate system functionality and core OS concepts, meeting all technical and content requirements:

#### Content Requirements (Suggested Structure, Optional)
1. System Overview (30s): Brief explanation of system architecture, showing running system components;
2. Live Demo (2 min): Demonstrate multi-robot concurrent operation, verify safe coordination of shared resources;
3. Code Walkthrough (30s): Highlight core synchronization code, explain one key design decision.

#### Technical Requirements
- File size **≤ 50MB**; oversized files may be submitted as an unlisted YouTube video with a valid link;
- MP4 or other common video format, clear and audible audio without noise;
- Screen recording showing terminal output, system execution, and core operations;
- Narration required — clear, logical, and aligned with the demo content.

## 6. Grading Criteria
Total score: 100%, assessed across **5 dimensions** with explicit weights and core criteria. **Synchronization is the highest-weighted dimension**:

| Dimension | Weight | Core Criteria |
|---|---|---|
| Learning Outcome A: Concurrency | 25% | Multi-threaded safe execution with zero race conditions; shared state access meets concurrency safety requirements |
| Learning Outcome B: Synchronization | 40% | All shared state (task queue, zones, health monitoring) remains consistent with no data corruption or state anomalies; synchronization primitives are used correctly and efficiently |
| Learning Outcome C: Coordination | 10% | Correct multi-robot thread interaction logic; all three demo scenarios complete successfully without errors; no unexpected blocking or crashes |
| Code Quality | 5% | Clear, readable code structure; idiomatic Rust style; rigorous error handling; reasonable test coverage |
| Report & Demo | 20% | Well-structured and complete written report with clear logic; clear video demonstration with thorough feature verification and effective narration |

================================================================================

PROJECT BLAZE — Complete Project File Structure

Designed with reference to Linux kernel and modern Rust best practices

================================================================================

project-blaze/

├── Cargo.toml                      # Rust project configuration

├── Cargo.lock                      # Dependency lock file

├── README.md                       # Project documentation

├── DESIGN.md                       # Design document

├── .gitignore                      # Git ignore file

│

├── src/                            # Source code directory

│   ├── main.rs                     # Application entry point, demo scenarios

│   ├── lib.rs                      # Library root, public API exports

│   │

│   ├── types/                      # Type definitions module (similar to Linux include/)

│   │   ├── mod.rs                  # Module root

│   │   ├── task.rs                 # Task type definition

│   │   ├── robot.rs                # Robot type definition

│   │   ├── zone.rs                 # Zone type definition

│   │   ├── config.rs               # Config type definition

│   │   └── error.rs                # Error type definition

│   │

│   ├── scheduler/                  # Scheduler module (similar to Linux kernel/sched/)

│   │   ├── mod.rs                  # Scheduler interface definition

│   │   ├── queue.rs                # Task queue core implementation

│   │   ├── fifo.rs                 # FIFO scheduling policy

│   │   ├── priority.rs             # Priority scheduling (extension)

│   │   ├── round_robin.rs          # Round Robin (extension)

│   │   └── stats.rs                # Scheduling statistics

│   │

│   ├── mm/                         # Resource management module (similar to Linux mm/)

│   │   ├── mod.rs                  # Resource management interface

│   │   ├── zone_allocator.rs      # Zone allocator core

│   │   ├── lock_guard.rs           # RAII lock guard

│   │   ├── deadlock_detector.rs   # Deadlock detection (extension)

│   │   └── allocation_table.rs    # Resource allocation table

│   │

│   ├── monitor/                    # Monitoring module (similar to systemd/watchdog)

│   │   ├── mod.rs                  # Monitor interface definition

│   │   ├── heartbeat.rs            # Heartbeat monitoring implementation

│   │   ├── health_checker.rs      # Health checker

│   │   ├── reporter.rs             # Status reporter

│   │   └── metrics.rs              # Monitoring metrics collection

│   │

│   ├── worker/                     # Worker thread module (similar to userspace processes)

│   │   ├── mod.rs                  # Worker trait definition

│   │   ├── robot.rs                # Robot Worker implementation

│   │   ├── pool.rs                 # Worker Pool (thread pool)

│   │   ├── state.rs                # Worker state machine

│   │   └── lifecycle.rs            # Lifecycle management

│   │

│   ├── coordinator/                # Coordinator module (similar to kernel core)

│   │   ├── mod.rs                  # Coordinator implementation

│   │   ├── builder.rs              # Builder pattern constructor

│   │   ├── syscall.rs              # System call interface layer

│   │   └── lifecycle.rs            # System lifecycle management

│   │

│   ├── sync/                       # Synchronization primitives module (similar to Linux kernel/locking/)

│   │   ├── mod.rs                  # Synchronization primitives exports

│   │   ├── mutex.rs                # Mutex wrapper

│   │   ├── rwlock.rs               # RwLock wrapper

│   │   ├── atomic.rs               # Atomic operations wrapper

│   │   ├── channel.rs              # Channel communication

│   │   └── barrier.rs              # Barrier synchronization

│   │

│   ├── util/                       # Utility module

│   │   ├── mod.rs                  # Utility function exports

│   │   ├── logger.rs               # Logging system

│   │   ├── timer.rs                # Timer

│   │   ├── id_generator.rs         # ID generator

│   │   └── rand.rs                 # Random number generation

│   │

│   └── prelude.rs                  # Common imports prelude

│

├── tests/                          # Integration tests directory

│   ├── common/                     # Test common code

│   │   ├── mod.rs                  # Test helper functions

│   │   └── fixtures.rs             # Test data fixtures

│   │

│   ├── test_scheduler.rs           # Scheduler integration tests

│   ├── test_zone_control.rs        # Zone control tests

│   ├── test_monitor.rs             # Monitor module tests

│   ├── test_concurrency.rs         # Concurrency safety tests

│   ├── test_demo_scenarios.rs      # Demo scenario tests

│   └── test_stress.rs              # Stress tests

│

├── benches/                        # Performance benchmarks (using criterion)

│   ├── scheduler_bench.rs          # Scheduler performance tests

│   ├── zone_lock_bench.rs          # Lock contention performance tests

│   ├── heartbeat_bench.rs          # Heartbeat detection performance tests

│   └── throughput_bench.rs         # System throughput tests

│

├── examples/                       # Example programs

│   ├── basic_demo.rs               # Basic demonstration

│   ├── priority_scheduling.rs     # Priority scheduling example

│   ├── deadlock_demo.rs            # Deadlock detection example

│   └── high_load.rs                # High-load test

│

├── docs/                           # Documentation directory

│   ├── architecture.md             # Architecture design document

│   ├── api.md                      # API usage document

│   ├── benchmarks.md               # Performance test report

│   ├── images/                     # Documentation images

│   │   ├── architecture.png        # System architecture diagram

│   │   └── execution_flow.png      # Execution flow diagram

│   └── report_template.md          # Project report template

│

├── scripts/                        # Utility scripts

│   ├── run_demo.sh                 # Run demo script

│   ├── run_tests.sh                # Run tests script

│   ├── generate_report.sh          # Generate report script

│   └── benchmark.sh                # Performance benchmark script

│

└── config/                         # Configuration files directory

    ├── default.toml                # Default configuration

    ├── demo.toml                   # Demo configuration

    └── stress.toml                 # Stress test configuration
