//! Monitoring and watchdog module.
//! Collects and aggregates runtime state for systemd/watchdog-style health monitoring:
//! - heartbeat: records per-robot heartbeat timestamps
//! - metrics: tracks per-robot and global execution statistics
//! - health_checker: evaluates health based on heartbeats and metrics
//! - reporter: generates consumer-friendly report structures for the API / frontend
//! - monitor_thread: background thread that periodically evaluates health and logs results

pub mod health_checker;
pub mod heartbeat;
pub mod metrics;
pub mod monitor_thread;
pub mod reporter;
