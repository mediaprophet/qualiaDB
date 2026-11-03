use super::PolicyError;
use crate::modalities::logic::WebizenOpcode;
use serde::{Deserialize, Serialize};

/// Versioned pure-constraint subset of the real Webizen ISA. Unsupported names
/// fail deserialization. Lifecycle/emit instructions are not silently ignored;
/// real temporal policy belongs to the graph guard, not the legacy pseudo-LTL.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum ConstraintInstruction {
    MatchSubject { value: u64 },
    MatchPredicate { value: u64 },
    MatchObject { value: u64 },
    MetadataMask { mask: u32 },
    Bind { vector: u8, register: u8 },
    MatchRegister { vector: u8, register: u8 },
    HaltIfFalse,
    LessThan { vector: u8, value: f32 },
    GreaterThan { vector: u8, value: f32 },
    LessOrEqual { vector: u8, value: f32 },
    GreaterOrEqual { vector: u8, value: f32 },
    Confidence { minimum: f32 },
}
impl ConstraintInstruction {
    pub(crate) fn lower(self) -> Result<WebizenOpcode, PolicyError> {
        use ConstraintInstruction::*;
        let vector = |v| {
            if v <= 3 {
                Ok(v)
            } else {
                Err(PolicyError::InvalidProgram)
            }
        };
        let register = |r| {
            if r < 16 {
                Ok(r as usize)
            } else {
                Err(PolicyError::InvalidProgram)
            }
        };
        let finite = |v: f32| {
            if v.is_finite() {
                Ok(v)
            } else {
                Err(PolicyError::InvalidProgram)
            }
        };
        Ok(match self {
            MatchSubject { value } => WebizenOpcode::MatchSubject(value),
            MatchPredicate { value } => WebizenOpcode::MatchPredicate(value),
            MatchObject { value } => WebizenOpcode::MatchObject(value),
            MetadataMask { mask } => WebizenOpcode::EvalMetadataMask(mask),
            Bind {
                vector: v,
                register: r,
            } => WebizenOpcode::BindRegister {
                vector_id: vector(v)?,
                register_index: register(r)?,
            },
            MatchRegister {
                vector: v,
                register: r,
            } => WebizenOpcode::MatchRegister {
                vector_id: vector(v)?,
                register_index: register(r)?,
            },
            HaltIfFalse => WebizenOpcode::HaltIfFalse,
            LessThan { vector: v, value } => WebizenOpcode::LessThan {
                vector_id: vector(v)?,
                value: finite(value)?,
            },
            GreaterThan { vector: v, value } => WebizenOpcode::GreaterThan {
                vector_id: vector(v)?,
                value: finite(value)?,
            },
            LessOrEqual { vector: v, value } => WebizenOpcode::LessOrEqual {
                vector_id: vector(v)?,
                value: finite(value)?,
            },
            GreaterOrEqual { vector: v, value } => WebizenOpcode::GreaterOrEqual {
                vector_id: vector(v)?,
                value: finite(value)?,
            },
            Confidence { minimum } => {
                if !(0.0..=1.0).contains(&minimum) {
                    return Err(PolicyError::InvalidProgram);
                }
                WebizenOpcode::YieldConfidence(minimum)
            }
        })
    }
    pub(crate) fn canonical(self) -> [u8; 12] {
        use ConstraintInstruction::*;
        let (tag, a, b, value) = match self {
            MatchSubject { value } => (0, 0, 0, value),
            MatchPredicate { value } => (1, 0, 0, value),
            MatchObject { value } => (2, 0, 0, value),
            MetadataMask { mask } => (3, 0, 0, mask as u64),
            Bind { vector, register } => (4, vector, register, 0),
            MatchRegister { vector, register } => (5, vector, register, 0),
            HaltIfFalse => (6, 0, 0, 0),
            LessThan { vector, value } => (7, vector, 0, value.to_bits() as u64),
            GreaterThan { vector, value } => (8, vector, 0, value.to_bits() as u64),
            LessOrEqual { vector, value } => (9, vector, 0, value.to_bits() as u64),
            GreaterOrEqual { vector, value } => (10, vector, 0, value.to_bits() as u64),
            Confidence { minimum } => (11, 0, 0, minimum.to_bits() as u64),
        };
        let mut bytes = [0; 12];
        bytes[..3].copy_from_slice(&[tag, a, b]);
        bytes[4..].copy_from_slice(&value.to_le_bytes());
        bytes
    }
}
