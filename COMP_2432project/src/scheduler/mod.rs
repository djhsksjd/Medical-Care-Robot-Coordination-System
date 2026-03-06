//! Scheduler interfaces and strategy registration.
// TODO: Define common scheduler traits and expose concrete policies.

pub mod queue;
pub mod fifo;
pub mod priority;
pub mod round_robin;
pub mod stats;
