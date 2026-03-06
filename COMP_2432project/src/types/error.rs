//! Error types used across the system.
// Version 1 keeps a single error enum for simplicity.

use std::fmt;

/// Top-level error type for the system.
#[derive(Debug)]
pub enum Error {
    SchedulerEmpty,
    WorkerStopped,
    ZoneUnavailable,
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SchedulerEmpty => write!(f, "no tasks available in scheduler"),
            Error::WorkerStopped => write!(f, "worker is stopped"),
            Error::ZoneUnavailable => write!(f, "requested zone is unavailable"),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}
