//! Zero-Infrastructure Acoustic & BLE Mesh Implementation
//!
//! This module provides zero-infrastructure acoustic and BLE mesh networking for distributed
//! scientific computing in crisis scenarios. Designed for delay-tolerant networking and
//! emergency response operations.

use crate::q_hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

mod acoustic;
mod ble;
mod data_store;
mod manager;
mod metrics;
mod routing;
mod types;

pub use acoustic::*;
pub use ble::*;
pub use data_store::*;
pub use manager::*;
pub use metrics::*;
pub use routing::*;
pub use types::*;

#[cfg(test)]
mod tests;
