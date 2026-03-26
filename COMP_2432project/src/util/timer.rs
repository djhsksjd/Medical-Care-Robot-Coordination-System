//! Timing utilities.
//!
//! This module intentionally stays small and std-only. It is used for lightweight
//! measurement in demos/tests/bench-like flows without pulling extra deps.

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Timer {
    start: Instant,
    last_lap: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        Self::start_new()
    }
}

impl Timer {
    pub fn start_new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_lap: now,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn lap(&mut self) -> Duration {
        let now = Instant::now();
        let d = now.duration_since(self.last_lap);
        self.last_lap = now;
        d
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        self.start = now;
        self.last_lap = now;
    }
}

pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Duration) {
    let start = Instant::now();
    let out = f();
    (out, start.elapsed())
}
