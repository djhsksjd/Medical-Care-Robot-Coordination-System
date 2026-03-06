//! Logging facility.
// Version 1 provides a very small wrapper over `println!`.

use std::time::SystemTime;

pub fn log_info(message: impl AsRef<str>) {
    let ts = SystemTime::now();
    println!("[INFO] [{ts:?}] {}", message.as_ref());
}
