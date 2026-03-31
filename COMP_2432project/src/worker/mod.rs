//! Worker module representing user-space style workers.
//! Conceptually analogous to user-space processes/threads, composed of RobotWorker + WorkerPool:
//! - robot: execution unit bound to a single Robot
//! - pool: scheduling loop managing multiple RobotWorkers
//! - state / lifecycle: worker state and lifecycle abstractions

pub mod lifecycle;
pub mod pool;
pub mod robot;
pub mod state;
