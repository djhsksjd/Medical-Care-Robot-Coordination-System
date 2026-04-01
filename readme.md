# Project B

This is the COMP2432 Operating Systems course project. It demonstrates OS-style ideas in a medical-robot scenario, focusing on **concurrency**, **synchronization**, and **task scheduling**.

## Overview

The project has two parts:
- **Frontend (React)**: optional visualization for system state (dashboard).
- **Backend (Rust)**: the “kernel-like” core that implements coordination logic (scheduling, worker robots, zone mutual exclusion, monitoring/metrics) and exposes an HTTP API.

During runtime, you can demonstrate the system in two ways:
- **Run the CLI report**: no frontend required; use the terminal to show scheduling analysis and key metrics.
- **Run the backend service (main)**: pair it with the frontend page to visualize live system state.

## Project structure

```text
sourcecode/
├─ COMP_2432project/              # Rust backend (Cargo crate)
│  ├─ src/
│  │  ├─ main.rs                  # Backend entry (HTTP API service)
│  │  ├─ api.rs                   # API types + router
│  │  ├─ bin/
│  │  │  └─ cli.rs                # CLI scheduling report (interactive + one-shot)
│  │  ├─ coordinator/             # Coordinator + builder + lifecycle
│  │  ├─ scheduler/               # Scheduling strategies + queues
│  │  ├─ worker/                  # Robot workers + thread pool
│  │  ├─ mm/                      # Zone/resource management
│  │  ├─ monitor/                 # Heartbeat + metrics + reporting
│  │  ├─ sync/                    # Sync primitives/wrappers
│  │  ├─ types/                   # Core types (task/robot/zone/config)
│  │  └─ util/                    # Utilities (logger/id generator/etc.)
│  └─ Cargo.toml
├─ frontend/                      # React frontend (contains package.json)
├─ readme.md
└─ .gitignore
```

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
