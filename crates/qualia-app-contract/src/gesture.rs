//! Interactive gesture lifecycle and streaming pointer sample DTOs.

use crate::error::ContractError;
use crate::id::DecimalU64;
use crate::target::{CommandTarget, PointerHit};
use serde::{Deserialize, Serialize};

/// Active gesture session maintaining streaming pointer samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureSession {
    pub gesture_id: String,
    pub session_id: String,
    pub tool_id: String,
    pub start_revision: DecimalU64,
    pub initial_point: PointerHit,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<PointerHit>,
    pub is_cancelled: bool,
}

impl GestureSession {
    pub fn new(
        gesture_id: impl Into<String>,
        session_id: impl Into<String>,
        tool_id: impl Into<String>,
        start_revision: DecimalU64,
        initial_point: PointerHit,
    ) -> Self {
        Self {
            gesture_id: gesture_id.into(),
            session_id: session_id.into(),
            tool_id: tool_id.into(),
            start_revision,
            initial_point,
            samples: Vec::new(),
            is_cancelled: false,
        }
    }

    pub fn push_sample(&mut self, sample: PointerHit) -> Result<(), ContractError> {
        if self.is_cancelled {
            return Err(ContractError::MalformedParams("cannot push sample to cancelled gesture".into()));
        }
        self.samples.push(sample);
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.is_cancelled = true;
    }
}

/// Gesture incremental update packet containing batched pointer samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureUpdate {
    pub gesture_id: String,
    pub samples: Vec<PointerHit>,
}

/// Gesture commit packet finalizing the interactive action into an authoritative revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureCommit {
    pub gesture_id: String,
    pub target: CommandTarget,
    pub final_point: PointerHit,
}

/// Gesture cancellation packet restoring the starting document revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GestureCancel {
    pub gesture_id: String,
    pub reason: String,
}
