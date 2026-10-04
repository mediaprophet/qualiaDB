//! Transport-neutral cooperative domain.
//!
//! Numeric and merge discipline: money in signed integer minor units, effort in
//! integer minutes, records merge by stable id before aggregation, and corrections append
//! superseding records rather than mutating signed history.

#![recursion_limit = "256"]

pub mod record;
pub mod finance;
pub mod projects;

pub mod agency_delegation;
pub mod agency_domain;
pub mod authority_type;
pub mod provenance;
pub mod qapp_package;
pub mod taxonomy;
pub mod trigger;
pub mod work_item;

pub use record::{DurationBridge, InstantBridge, NQuin, RecordEnvelope, SensitivityClass, EpistemicStatus, EvidenceType};
pub use finance::*;
pub use projects::*;
