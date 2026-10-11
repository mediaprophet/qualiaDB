//! # Qualia Application Contract
//!
//! Canonical portable command, context, session, and surface contracts for
//! Storytold applications, Poet, and Webizen OS.
//!
//! Aligned with POET Implementation Plan ST-E, ST-G, and ST-H.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

pub mod adapter;
pub mod catalog;
pub mod delegation;
pub mod descriptor;
pub mod effects;
pub mod error;
pub mod gesture;
pub mod id;
pub mod request;
pub mod target;

pub use adapter::AppAdapter;
pub use catalog::CommandCatalog;
pub use delegation::{
    DelegationLifecycle, DelegationMode, ResourceBudgets, TaskDelegation,
};
pub use descriptor::{Availability, CommandDescriptor, TargetKind};
pub use effects::{CommandEffect, EffectSet, InteractionKind, UndoPolicy};
pub use error::ContractError;
pub use gesture::{GestureCancel, GestureCommit, GestureSession, GestureUpdate};
pub use id::{validate_finite_f64, AppId, CommandId, DecimalU64, JS_MAX_SAFE_INTEGER};
pub use request::{
    AppSessionDescriptor, CommandReceipt, CommandRequest, CommandStatus, JobDescriptor, JobState,
    SurfaceDescriptor,
};
pub use target::{CommandTarget, InteractionContext, PointerHit, SelectionKind};

/// Serialize a contract data structure to deterministic, pretty or compact JSON.
pub fn to_json_string<T: serde::Serialize>(val: &T) -> Result<String, ContractError> {
    serde_json::to_string(val).map_err(|e| ContractError::Serialization(e.to_string()))
}

/// Deserialize a contract data structure from JSON string.
pub fn from_json_str<'a, T: serde::Deserialize<'a>>(s: &'a str) -> Result<T, ContractError> {
    serde_json::from_str(s).map_err(|e| ContractError::Serialization(e.to_string()))
}

/// Serialize a contract data structure to canonical CBOR bytes.
pub fn to_cbor_bytes<T: serde::Serialize>(val: &T) -> Result<Vec<u8>, ContractError> {
    let mut buf = Vec::new();
    ciborium::into_writer(val, &mut buf).map_err(|e| ContractError::Serialization(e.to_string()))?;
    Ok(buf)
}

/// Deserialize a contract data structure from CBOR bytes.
pub fn from_cbor_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, ContractError> {
    ciborium::from_reader(slice).map_err(|e| ContractError::Serialization(e.to_string()))
}
