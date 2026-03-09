//! System call style interface layer.
// Version 1 exposes only a very small API.

use crate::coordinator::lifecycle::Coordinator;
use crate::monitor::heartbeat::HeartbeatRegistry;
use crate::monitor::metrics::MetricsRegistry;

/// Run the built-in demo scenario through the coordinator.
pub fn run_demo(
    coordinator: &mut Coordinator,
    heartbeats: &HeartbeatRegistry,
    metrics: &MetricsRegistry,
) {
    coordinator.run_demo(heartbeats, metrics);
}
