//! Command target and interaction context definitions.

use crate::error::ContractError;
use crate::id::{validate_finite_f64, DecimalU64};
use serde::{Deserialize, Serialize};

/// Detailed selection payload within a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum SelectionKind {
    None,
    Single(String),
    Multiple(Vec<String>),
    Range { start: String, end: String },
}

/// Normalized pointer hit event data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointerHit {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure: Option<f64>,
    pub surface_id: String,
}

impl PointerHit {
    pub fn new(x: f64, y: f64, surface_id: impl Into<String>) -> Result<Self, ContractError> {
        let x = validate_finite_f64(x, "x")?;
        let y = validate_finite_f64(y, "y")?;
        Ok(Self {
            x,
            y,
            pressure: None,
            surface_id: surface_id.into(),
        })
    }

    pub fn with_pressure(mut self, pressure: f64) -> Result<Self, ContractError> {
        let p = validate_finite_f64(pressure, "pressure")?;
        if !(0.0..=1.0).contains(&p) {
            return Err(ContractError::MalformedParams("pressure must be between 0.0 and 1.0".into()));
        }
        self.pressure = Some(p);
        Ok(self)
    }
}

/// Precise target entity for command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandTarget {
    pub session_id: String,
    pub document_id: String,
    pub document_revision: DecimalU64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_ids: Vec<String>,
    pub selection: SelectionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtarget: Option<String>,
}

/// Interaction context encompassing pointer, focus, and hierarchy state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionContext {
    pub context_id: String,
    pub principal_id: String,
    pub construct_id: String,
    pub manifold_id: String,
    pub container_id: String,
    pub target: CommandTarget,
    pub focus_origin: String,
    pub editing_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer_hit: Option<PointerHit>,
    pub navigation_depth: u32,
    pub capability_snapshot_revision: DecimalU64,
}
