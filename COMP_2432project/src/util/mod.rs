//! Utility helpers shared across the project.
//! Provides common utility modules such as:
//! - logger: unified logging interface
//! - timer: simple timing wrapper based on `Instant`
//! - id_generator: globally unique ID generator
//! - rand: random number utilities (extensible as needed)

pub mod id_generator;
pub mod logger;
pub mod rand;
pub mod timer;
