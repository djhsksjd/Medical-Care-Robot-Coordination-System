use std::env;
use std::io::{self, Write};
use std::process;

use COMP_2432project::api::{AppState, TaskPriority};
use COMP_2432project::coordinator::builder::{effective_demo_task_count, effective_worker_count};
use COMP_2432project::types::config::{Config, SchedulerKind};

const SEP: &str = "========================================================================";
const DASH: &str = "------------------------------------------------------------------------";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    let app = AppState::new();

    // If no arguments are provided, enter interactive mode.
    if args.is_empty() {
        run_interactive(&app);
        return;
    }

    let config = match parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!();
            print_usage();
            process::exit(1);
        }
    };

    app.apply_config(config.clone());
    print_report(&app, &config);
}

// ── Helpers ─────────────────────────────────────────────────────────

fn scheduler_label(kind: SchedulerKind) -> &'static str {
    match kind {
        SchedulerKind::Fifo => "Fifo",
        SchedulerKind::Priority => "Priority",
        SchedulerKind::RoundRobin => "RoundRobin",
        SchedulerKind::Srt => "Srt",
    }
}

fn bool_label(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}

fn zone_label(id: u64) -> String {
    match id {
        1 => "ICU".into(),
        2 => "Ward".into(),
        3 => "OR".into(),
        other => format!("Zone({other})"),
    }
}

fn priority_label(p: &TaskPriority) -> &'static str {
    match p {
        TaskPriority::Low => "Low",
        TaskPriority::Normal => "Normal",
        TaskPriority::High => "High",
    }
}

fn parse_scheduler(input: &str) -> Option<SchedulerKind> {
    match input.trim().to_ascii_lowercase().as_str() {
        "fifo" => Some(SchedulerKind::Fifo),
        "priority" => Some(SchedulerKind::Priority),
        "roundrobin" | "rr" => Some(SchedulerKind::RoundRobin),
        "srt" => Some(SchedulerKind::Srt),
        _ => None,
    }
}

fn parse_bool(input: &str) -> Option<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn read_line(prompt: &str) -> Option<String> {
    print!("{prompt}");
    io::stdout().flush().ok()?;

    let mut s = String::new();
    io::stdin().read_line(&mut s).ok()?;
    Some(s.trim().to_string())
}

fn is_quit(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "q" | "quit" | "exit")
}

fn run_interactive(app: &AppState) {
    println!();
    println!("{SEP}");
    println!("  CLI Scheduling Report (Interactive Mode)");
    println!("{SEP}");
    println!("  Enter values to generate a report.");
    println!("  Press Enter to keep the default.");
    println!("  Type 'q' to quit at any prompt.");
    println!();

    loop {
        let mut config = Config::default();

        // Scheduler
        let default_sched = scheduler_label(config.scheduler).to_ascii_lowercase();
        let s = match read_line(&format!(
            "Scheduler [fifo|priority|roundrobin|rr|srt] (default: {default_sched}): "
        )) {
            Some(v) => v,
            None => return,
        };
        if is_quit(&s) {
            return;
        }
        if !s.is_empty() {
            config.scheduler = parse_scheduler(&s).unwrap_or(config.scheduler);
        }

        // Stress preset (overrides workers/tasks in report via effective_* helpers)
        let s = match read_line("Stress preset? [y/n] (default: n): ") {
            Some(v) => v,
            None => return,
        };
        if is_quit(&s) {
            return;
        }
        if !s.is_empty() {
            config.use_stress_preset = parse_bool(&s).unwrap_or(false);
        }

        // Workers + tasks only matter when stress preset is OFF
        if !config.use_stress_preset {
            let s = match read_line(&format!(
                "Worker count (default: {}): ",
                config.worker_count
            )) {
                Some(v) => v,
                None => return,
            };
            if is_quit(&s) {
                return;
            }
            if !s.is_empty() {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 {
                        config.worker_count = n;
                    }
                }
            }

            let s = match read_line(&format!(
                "Demo task count (default: {}): ",
                config.demo_task_count
            )) {
                Some(v) => v,
                None => return,
            };
            if is_quit(&s) {
                return;
            }
            if !s.is_empty() {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 {
                        config.demo_task_count = n;
                    }
                }
            }
        } else {
            println!("(Stress preset ON: using 12 workers, 108 tasks)");
        }

        // Work stealing
        let s = match read_line("Work stealing? [y/n] (default: n): ") {
            Some(v) => v,
            None => return,
        };
        if is_quit(&s) {
            return;
        }
        if !s.is_empty() {
            config.use_work_stealing = parse_bool(&s).unwrap_or(false);
        }

        app.apply_config(config.clone());
        print_report(app, &config);

        let s = match read_line("Run another report? [Enter=yes, q=quit]: ") {
            Some(v) => v,
            None => return,
        };
        if is_quit(&s) {
            return;
        }
        println!();
    }
}

fn fmt_dur(ms: u64) -> String {
    if ms >= 1000 {
        if ms % 1000 == 0 {
            format!("{} s", ms / 1000)
        } else {
            format!("{:.1} s", ms as f64 / 1000.0)
        }
    } else {
        format!("{ms} ms")
    }
}

fn fmt_delta(ms: i64) -> String {
    let abs = fmt_dur(ms.unsigned_abs());
    if ms > 0 {
        format!("{abs} faster")
    } else if ms < 0 {
        format!("{abs} slower")
    } else {
        "same as FIFO".into()
    }
}

// ── Argument parsing (mirrors frontend Config fields exactly) ───────

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut config = Config::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scheduler" | "-s" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("Missing value for --scheduler")?;
                config.scheduler = match val.to_ascii_lowercase().as_str() {
                    "fifo" => SchedulerKind::Fifo,
                    "priority" => SchedulerKind::Priority,
                    "roundrobin" | "rr" => SchedulerKind::RoundRobin,
                    "srt" => SchedulerKind::Srt,
                    _ => return Err(format!("Unknown scheduler: {val}. Use: fifo, priority, roundrobin, srt")),
                };
            }
            "--workers" | "-w" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("Missing value for --workers")?;
                config.worker_count = val
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid worker count: {val}"))?;
                if config.worker_count < 1 {
                    return Err("Worker count must be >= 1".into());
                }
            }
            "--tasks" | "-t" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("Missing value for --tasks")?;
                config.demo_task_count = val
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid task count: {val}"))?;
                if config.demo_task_count < 1 {
                    return Err("Task count must be >= 1".into());
                }
            }
            "--work-stealing" | "--ws" => {
                config.use_work_stealing = true;
            }
            "--stress" => {
                config.use_stress_preset = true;
            }
            other => {
                return Err(format!("Unknown argument: {other}"));
            }
        }
        i += 1;
    }
    Ok(config)
}

fn print_usage() {
    println!(
        r#"Usage: cli [OPTIONS]

Interactive mode:
  cli

Options:
  -s, --scheduler <KIND>  Scheduling algorithm: fifo | priority | roundrobin | srt
                          (default: fifo)
  -w, --workers <N>       Number of worker robots (default: 9)
  -t, --tasks <N>         Number of demo tasks (default: 20)
      --work-stealing     Enable work-stealing + non-blocking zone allocation
      --stress            Use stress test preset (12 workers, 108 tasks)
  -h, --help              Show this help message

Examples:
  cli                              Default config (Fifo, 9 workers, 20 tasks)
  cli -s priority -w 6 -t 18      Priority scheduler, 6 workers, 18 tasks
  cli --stress                     Stress test preset
  cli -s srt --work-stealing       SRT scheduler with work-stealing enabled"#
    );
}

fn print_report(app: &AppState, config: &Config) {
    let state = app.snapshot_state();
    let analysis = &state.scheduling_analysis;

    let ew = effective_worker_count(config);
    let et = effective_demo_task_count(config);

    // ── Header ──────────────────────────────────────────────────────
    println!();
    println!("{SEP}");
    println!("    Medical Care Robot Coordination System - Scheduling Report");
    println!("{SEP}");
    println!();

    // ── Configuration (same fields as frontend Config panel) ────────
    println!("  Scheduler           = {}", scheduler_label(config.scheduler));
    println!("  Worker Count        = {ew}");
    println!("  Demo Task Count     = {et}");
    println!(
        "  Work Stealing       = {}",
        bool_label(config.use_work_stealing)
    );
    println!(
        "  Stress Preset       = {}",
        bool_label(config.use_stress_preset)
    );
    println!();

    // ── Input Tasks (same as frontend "Explicit long demo input") ───
    println!("{SEP}");
    println!("                       Test Data (Input Tasks)");
    println!("{SEP}");
    println!(
        "  {:<4} {:<40} {:<8} {:<8} {}",
        "#", "Task Name", "Priority", "Duration", "Zone"
    );
    println!("  {}", "-".repeat(70));
    for task in &analysis.input_tasks {
        let zone = task
            .required_zone_id
            .map(zone_label)
            .unwrap_or_else(|| "-".into());
        println!(
            "  {:<4} {:<40} {:<8} {:<8} {}",
            task.id,
            &task.name,
            priority_label(&task.priority),
            fmt_dur(task.expected_duration_ms),
            zone,
        );
    }
    println!();

    // ── Per-strategy details (same metrics as frontend StrategyCard) ─
    for strategy in &analysis.strategies {
        let is_current = strategy.scheduler == config.scheduler;
        let tag = if is_current { "  * Current *" } else { "" };

        println!("{SEP}");
        println!(
            "                    Strategy: {}{tag}",
            scheduler_label(strategy.scheduler)
        );
        println!("{SEP}");

        println!(
            "  Makespan                           = {}",
            fmt_dur(strategy.makespan_ms)
        );
        println!(
            "  Avg Completion                     = {}",
            fmt_dur(strategy.avg_completion_ms)
        );
        println!(
            "  Avg Wait                           = {}",
            fmt_dur(strategy.avg_wait_ms)
        );
        println!(
            "  Urgent Avg Finish                  = {}",
            fmt_dur(strategy.avg_high_priority_completion_ms)
        );

        if matches!(strategy.scheduler, SchedulerKind::Fifo) {
            println!("  Vs FIFO Avg Completion             = baseline");
            println!("  Vs FIFO Urgent                     = baseline");
            println!("  Speedup vs FIFO                    = baseline");
        } else {
            println!(
                "  Vs FIFO Avg Completion             = {}",
                fmt_delta(strategy.avg_completion_improvement_vs_fifo_ms)
            );
            println!(
                "  Vs FIFO Urgent                     = {}",
                fmt_delta(strategy.avg_high_priority_improvement_vs_fifo_ms)
            );
            let sign = if strategy.speedup_vs_fifo_pct >= 0.0 {
                "+"
            } else {
                ""
            };
            println!(
                "  Speedup vs FIFO                    = {sign}{:.1}%",
                strategy.speedup_vs_fifo_pct
            );
        }

        println!("  {DASH}");
        println!("  Worker Load:");
        for (i, &busy) in strategy.worker_busy_ms.iter().enumerate() {
            println!(
                "    Robot {:<3}                        = {}",
                i + 1,
                fmt_dur(busy)
            );
        }
        println!();
    }

    // ── Final Summary (same comparison as frontend metric charts) ───
    println!("{SEP}");
    println!("                           Final Summary");
    println!("{SEP}");
    println!(
        "  {:<12} {:<10} {:<12} {:<13} {}",
        "Strategy", "Makespan", "Avg Compl.", "Urgent Fin.", "Speedup vs FIFO"
    );
    println!("  {}", "-".repeat(66));
    for strategy in &analysis.strategies {
        let speedup = if matches!(strategy.scheduler, SchedulerKind::Fifo) {
            "baseline".into()
        } else {
            let sign = if strategy.speedup_vs_fifo_pct >= 0.0 {
                "+"
            } else {
                ""
            };
            format!("{sign}{:.1}%", strategy.speedup_vs_fifo_pct)
        };
        println!(
            "  {:<12} {:<10} {:<12} {:<13} {}",
            scheduler_label(strategy.scheduler),
            fmt_dur(strategy.makespan_ms),
            fmt_dur(strategy.avg_completion_ms),
            fmt_dur(strategy.avg_high_priority_completion_ms),
            speedup,
        );
    }
    println!("{SEP}");
    println!();
}
