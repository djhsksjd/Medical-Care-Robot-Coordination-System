//! Core library entry for the project-blaze style OS scheduler demo.
//! Acts as the kernel export table, exposing all subsystem modules and the HTTP API.

#![allow(non_snake_case)]

pub mod api;
pub mod coordinator;
pub mod mm;
pub mod monitor;
pub mod prelude;
pub mod scheduler;
pub mod sync;
pub mod types;
pub mod util;
pub mod worker;
