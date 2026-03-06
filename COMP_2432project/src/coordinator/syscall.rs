//! System call style interface layer.
// Version 1 exposes only a very small API.

use crate::coordinator::lifecycle::Coordinator;

/// Run the built-in demo scenario through the coordinator.
pub fn run_demo(coordinator: Coordinator) {
    coordinator.run_demo();
}
