//! Worker lifecycle management utilities.
//!
//! This module provides small building blocks for starting/stopping worker
//! threads in a controlled way. Higher-level orchestration (pool/coordinator)
//! can use these without coupling to a specific worker implementation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Clone)]
pub struct LifecycleFlags {
    shutdown: Arc<AtomicBool>,
    paused: Arc<PauseController>,
}

impl LifecycleFlags {
    pub fn new(shutdown: Arc<AtomicBool>, paused: Arc<PauseController>) -> Self {
        Self { shutdown, paused }
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    pub fn pause_requested(&self) -> bool {
        self.paused.is_paused()
    }

    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn request_pause(&self) {
        self.paused.pause();
    }

    pub fn clear_pause(&self) {
        self.paused.resume();
    }
}

#[derive(Debug)]
pub struct PauseController {
    paused: Mutex<bool>,
    cv: Condvar,
}

impl PauseController {
    pub fn new() -> Self {
        Self {
            paused: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn pause(&self) {
        let mut guard = self.paused.lock().unwrap_or_else(|e| e.into_inner());
        *guard = true;
    }

    pub fn resume(&self) {
        let mut guard = self.paused.lock().unwrap_or_else(|e| e.into_inner());
        *guard = false;
        self.cv.notify_all();
    }

    /// Block the current thread while paused. Returns once resumed.
    /// The caller should still check shutdown flags around this call.
    pub fn wait_while_paused(&self) {
        let mut guard = self.paused.lock().unwrap_or_else(|e| e.into_inner());
        while *guard {
            guard = self.cv.wait(guard).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Default for PauseController {
    fn default() -> Self {
        Self::new()
    }
}
