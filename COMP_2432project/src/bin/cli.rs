use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use COMP_2432project::api::{AppState, ControlAction, SystemStatus};
use COMP_2432project::types::config::{Config, SchedulerKind};

fn main() {
    let app = AppState::new();

    println!("Project Blaze CLI");
    println!("Type `help` to see commands.");

    let mut line = String::new();
    loop {
        print!("blaze> ");
        let _ = io::stdout().flush();
        line.clear();

        if io::stdin().read_line(&mut line).is_err() {
            println!("Failed to read input.");
            continue;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let mut parts = input.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "help" => print_help(),
            "exit" | "quit" => break,
            "state" => print_state(&app),
            "run-demo" => {
                let status = app.control(ControlAction::RunDemo);
                println!("RunDemo -> {status}");
            }
            "pause" => {
                let status = app.control(ControlAction::Pause);
                println!("Pause -> {status}");
            }
            "resume" | "start" => {
                let status = app.control(ControlAction::Start);
                println!("Start/Resume -> {status}");
            }
            "stop" => {
                let status = app.control(ControlAction::Stop);
                println!("Stop -> {status}");
            }
            "watch" => {
                let secs: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                watch_state(&app, Duration::from_secs(secs));
            }
            "config" => match parts.next() {
                Some("show") | None => print_config(&app),
                Some("set") => handle_config_set(&app, parts.collect()),
                Some(other) => println!(
                    "Unknown config subcommand: {other}. Try `config show` or `config set ...`."
                ),
            },
            other => println!("Unknown command: {other}. Type `help`."),
        }
    }

    println!("Bye.");
}

fn print_help() {
    println!(
        r#"Commands:
  help                      Show this help
  state                     Print a concise system snapshot
  watch [interval_secs]     Poll and print state repeatedly (default 1s)

  run-demo                  Start a demo run in background
  pause                     Pause workers (blocking, non-busy-wait)
  resume|start              Resume workers
  stop                      Stop workers and reset runtime state

  config show               Show current config
  config set workers <n>    Set worker_count
  config set tasks <n>      Set demo_task_count
  config set scheduler <k>  Set scheduler: fifo|priority|roundrobin|srt

  exit|quit                 Exit CLI
"#
    );
}

fn print_config(app: &AppState) {
    let s = app.snapshot_state();
    println!(
        "Config: scheduler={:?}, worker_count={}, demo_task_count={}",
        s.config.scheduler, s.config.worker_count, s.config.demo_task_count
    );
}

fn handle_config_set(app: &AppState, args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: config set <workers|tasks|scheduler> <value>");
        return;
    }

    let key = args[0];
    let value = args[1];

    let mut cfg: Config = app.snapshot_state().config;
    match key {
        "workers" => match value.parse::<usize>() {
            Ok(n) if n >= 1 => cfg.worker_count = n,
            _ => {
                println!("Invalid worker count: {value}");
                return;
            }
        },
        "tasks" => match value.parse::<usize>() {
            Ok(n) if n >= 1 => cfg.demo_task_count = n,
            _ => {
                println!("Invalid task count: {value}");
                return;
            }
        },
        "scheduler" => match value.to_ascii_lowercase().as_str() {
            "fifo" => cfg.scheduler = SchedulerKind::Fifo,
            "priority" => cfg.scheduler = SchedulerKind::Priority,
            "roundrobin" | "rr" => cfg.scheduler = SchedulerKind::RoundRobin,
            "srt" => cfg.scheduler = SchedulerKind::Srt,
            _ => {
                println!("Invalid scheduler kind: {value}");
                return;
            }
        },
        _ => {
            println!("Unknown config key: {key}");
            return;
        }
    }

    app.apply_config(cfg);
    println!("Config updated (runtime state reset).");
}

fn print_state(app: &AppState) {
    let s = app.snapshot_state();

    let mut pending = 0;
    let mut running = 0;
    let mut finished = 0;
    let mut failed = 0;
    for t in &s.tasks {
        match t.status {
            COMP_2432project::api::TaskStatus::Pending => pending += 1,
            COMP_2432project::api::TaskStatus::Running => running += 1,
            COMP_2432project::api::TaskStatus::Finished => finished += 1,
            COMP_2432project::api::TaskStatus::Failed => failed += 1,
        }
    }

    println!(
        "Status: {:?} | tasks: total={} (pending={}, running={}, finished={}, failed={}) | throughput={} | avgLatencyMs={}",
        s.system_status,
        s.tasks.len(),
        pending,
        running,
        finished,
        failed,
        s.metrics.throughput,
        s.metrics.avg_latency_ms
    );

    if !s.zones.is_empty() {
        println!("Zones:");
        for z in &s.zones {
            println!(
                "  - {} (id={}): {}/{} activeRobots={} health={:?}",
                z.name, z.id, z.current_tasks, z.capacity, z.active_robots, z.health
            );
        }
    }
}

fn watch_state(app: &AppState, interval: Duration) {
    loop {
        let s = app.snapshot_state();
        print_state(app);
        if matches!(s.system_status, SystemStatus::Stopped) {
            break;
        }
        thread::sleep(interval);
    }
}
