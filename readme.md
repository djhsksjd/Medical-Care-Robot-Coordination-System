# Medical Care Robot Coordination System

This repository is a COMP2432 Operating Systems course project. It demonstrates OS-style ideas in a medical-robot scenario, focusing on **concurrency**, **synchronization**, and **task scheduling**.

## Overview

The project has two parts:
- **Frontend (React)**: optional visualization for system state (dashboard).
- **Backend (Rust)**: the “kernel-like” core that implements coordination logic (scheduling, worker robots, zone mutual exclusion, monitoring/metrics) and exposes an HTTP API.

During runtime, you can demonstrate the system in two ways:
- **Run the CLI report**: no frontend required; use the terminal to show scheduling analysis and key metrics.
- **Run the backend service (main)**: pair it with the frontend page to visualize live system state.

## Prerequisites

- **Rust toolchain** (stable) with Cargo installed.
- (Optional) **Node.js** + package manager (npm/pnpm/yarn) for the React frontend.

## Option A (recommended): Run the CLI report (no frontend needed)

```bash
cd COMP_2432project
```

### Interactive mode (no need to retype commands)

Run without arguments. The program will prompt you for parameters. Type `q` to exit at any prompt.

```bash
cargo run --bin cli
```

### One-shot mode (pass parameters once)

Important: when using `cargo run`, add `--` before the CLI arguments.

```bash
cargo run --bin cli -- -s priority -w 6 -t 18
```

### CLI parameters

- **Scheduler**: `-s, --scheduler <KIND>`
  - `fifo` | `priority` | `roundrobin` (or `rr`) | `srt`
- **Worker robots**: `-w, --workers <N>`
- **Demo tasks**: `-t, --tasks <N>`
- **Work stealing**: `--work-stealing` (or `--ws`)
- **Stress preset**: `--stress` (forces 12 workers, 108 tasks)

Examples:

```bash
# Stress preset
cargo run --bin cli -- --stress

# SRT + work stealing
cargo run --bin cli -- -s srt --work-stealing
```

## Option B: Run backend service + frontend dashboard

### Start the backend (Rust)

```bash
cd COMP_2432project
cargo run
```

### Start the frontend (React)

Locate the frontend folder (the one containing `package.json`), then run:

```bash
npm install
npm run dev
```

Then open the dev server URL shown in the terminal.

## Build & test (backend)

```bash
cd COMP_2432project
cargo build --release
cargo test
```

## Troubleshooting

- **Cargo says “unexpected argument”**: you likely forgot the `--` separator.
  - Correct: `cargo run --bin cli -- -s priority`
- **PowerShell usage**: don’t type commas like `-s,` — the comma only appears in help text to indicate aliases.
*** End of File
# Medical Care Robot Coordination System

This repository is a COMP2432 Operating Systems course project. It demonstrates OS-style ideas in a medical-robot scenario, focusing on **concurrency**, **synchronization**, and **task scheduling**.

## Overview

The project has two parts:
- **Frontend (React)**: optional visualization for system state (dashboard).
- **Backend (Rust)**: the “kernel-like” core that implements coordination logic (scheduling, worker robots, zone mutual exclusion, monitoring/metrics) and exposes an HTTP API.

During runtime, you can demonstrate the system in two ways:
- **Run the CLI report**: no frontend required; use the terminal to show scheduling analysis and key metrics.
- **Run the backend service (main)**: pair it with the frontend page to visualize live system state.

## Prerequisites

- **Rust toolchain** (stable) with Cargo installed.
- (Optional) **Node.js** + package manager (npm/pnpm/yarn) for the React frontend.

## Option A (recommended): Run the CLI report (no frontend needed)

```bash
cd COMP_2432project
```

### Interactive mode (no need to retype commands)

Run without arguments. The program will prompt you for parameters. Type `q` to exit at any prompt.

```bash
cargo run --bin cli
```

### One-shot mode (pass parameters once)

Important: when using `cargo run`, add `--` before the CLI arguments.

```bash
cargo run --bin cli -- -s priority -w 6 -t 18
```

### CLI parameters

- **Scheduler**: `-s, --scheduler <KIND>`
  - `fifo` | `priority` | `roundrobin` (or `rr`) | `srt`
- **Worker robots**: `-w, --workers <N>`
- **Demo tasks**: `-t, --tasks <N>`
- **Work stealing**: `--work-stealing` (or `--ws`)
- **Stress preset**: `--stress` (forces 12 workers, 108 tasks)

Examples:

```bash
# Stress preset
cargo run --bin cli -- --stress

# SRT + work stealing
cargo run --bin cli -- -s srt --work-stealing
```

## Option B: Run backend service + frontend dashboard

### Start the backend (Rust)

```bash
cd COMP_2432project
cargo run
```

### Start the frontend (React)

Locate the frontend folder (the one containing `package.json`), then run:

```bash
npm install
npm run dev
```

Then open the dev server URL shown in the terminal.

## Build & test (backend)

```bash
cd COMP_2432project
cargo build --release
cargo test
```

## Troubleshooting

- **Cargo says “unexpected argument”**: you likely forgot the `--` separator.
  - Correct: `cargo run --bin cli -- -s priority`
- **PowerShell usage**: don’t type commas like `-s,` — the comma only appears in help text to indicate aliases.

# Medical Care Robot Coordination System

This repository is a COMP2432 Operating Systems course project. It demonstrates OS-style ideas in a medical-robot scenario, focusing on **concurrency**, **synchronization**, and **task scheduling**.

## Overview

The project has two parts:
- **Frontend (React)**: optional visualization for system state (dashboard).
- **Backend (Rust)**: the “kernel-like” core that implements coordination logic (scheduling, worker robots, zone mutual exclusion, monitoring/metrics) and exposes an HTTP API.

During runtime, you can demonstrate the system in two ways:
- **Run the CLI report**: no frontend required; use the terminal to show scheduling analysis and key metrics.
- **Run the backend service (main)**: pair it with the frontend page to visualize live system state.

## Prerequisites

- **Rust toolchain** (stable) with Cargo installed.
- (Optional) **Node.js** + package manager (npm/pnpm/yarn) for the React frontend.

## Option A (recommended): Run the CLI report (no frontend needed)

Go to the backend crate directory:

```bash
cd COMP_2432project
```

### Interactive mode (no need to retype commands)

Run without arguments. The program will prompt you for parameters. Type `q` to exit at any prompt.

```bash
cargo run --bin cli
```

### One-shot mode (pass parameters once)

Important: when using `cargo run`, add `--` before the CLI arguments.

```bash
cargo run --bin cli -- -s priority -w 6 -t 18
```

### CLI parameters

- **Scheduler**: `-s, --scheduler <KIND>`
  - `fifo` | `priority` | `roundrobin` (or `rr`) | `srt`
- **Worker robots**: `-w, --workers <N>`
- **Demo tasks**: `-t, --tasks <N>`
- **Work stealing**: `--work-stealing` (or `--ws`)
- **Stress preset**: `--stress` (forces 12 workers, 108 tasks)

Examples:

```bash
# Stress preset
cargo run --bin cli -- --stress

# SRT + work stealing
cargo run --bin cli -- -s srt --work-stealing
```

## Option B: Run backend service + frontend dashboard

### Start the backend (Rust)

```bash
cd COMP_2432project
cargo run
```

### Start the frontend (React)

Locate the frontend folder (the one containing `package.json`), then run:

```bash
npm install
npm run dev
```

Then open the dev server URL shown in the terminal.

## Build & test (backend)

```bash
cd COMP_2432project
cargo build --release
cargo test
```

## Troubleshooting

- **Cargo says “unexpected argument”**: you likely forgot the `--` separator.
  - Correct: `cargo run --bin cli -- -s priority`
- **PowerShell usage**: don’t type commas like `-s,` — the comma only appears in help text to indicate aliases.

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
