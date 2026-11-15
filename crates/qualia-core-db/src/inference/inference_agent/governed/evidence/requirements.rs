use super::{BoundProposal, Payload, ScoreMeaning};
use crate::inference::inference_agent::governed::{ConstraintInstruction, PolicyError, PolicyList};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "requirement", content = "meaning", deny_unknown_fields)]
pub enum ScoreRequirement {
    Optional,
    Required(ScoreMeaning),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRequirements {
    pub allowed_sources: u16,
    pub allowed_artifact_origins: u8,
    pub max_payload_bytes: u64,
    pub allow_empty_artifact: bool,
    pub score: ScoreRequirement,
    /// Explicit separate projection: this program sees only a PRESENT actual
    /// numerical observation. No absent score can reach these operands.
    pub score_program: PolicyList<ConstraintInstruction, 64>,
}
impl ObservationRequirements {
    pub(super) fn validate(&self) -> Result<(), PolicyError> {
        if self.allowed_sources == 0
            || self.allowed_sources & !0x7f != 0
            || self.max_payload_bytes == 0
            || (!self.score_program.as_slice().is_empty()
                && matches!(self.score, ScoreRequirement::Optional))
        {
            return Err(PolicyError::InvalidPolicy);
        }
        let text = self.allowed_sources & 0x30 != 0;
        let semantic = self.allowed_sources & 0x40 != 0;
        let origins = self.allowed_artifact_origins;
        if origins & !0x07 != 0
            || (text != (origins & 0x03 != 0))
            || (semantic != (origins & 0x04 != 0))
        {
            return Err(PolicyError::InvalidPolicy);
        }
        let mut comparison = false;
        for instruction in self.score_program.as_slice() {
            use ConstraintInstruction::*;
            match *instruction {
                LessThan { vector: 2, .. }
                | GreaterThan { vector: 2, .. }
                | LessOrEqual { vector: 2, .. }
                | GreaterOrEqual { vector: 2, .. } => comparison = true,
                HaltIfFalse => (),
                _ => return Err(PolicyError::InvalidProgram),
            }
            instruction.lower()?;
        }
        if !self.score_program.as_slice().is_empty() && !comparison {
            return Err(PolicyError::InvalidProgram);
        }
        Ok(())
    }
    pub(super) fn check(&self, p: &BoundProposal<'_>) -> Result<(), PolicyError> {
        self.validate()?;
        if self.allowed_sources & p.proposal.source.bit() == 0 {
            return Err(PolicyError::Scope);
        }
        if let Some(origin) = p.proposal.artifact_origin {
            if self.allowed_artifact_origins & origin.bit() == 0 {
                return Err(PolicyError::Scope);
            }
        }
        if p.bytes().len() as u64 > self.max_payload_bytes {
            return Err(PolicyError::Budget);
        }
        if matches!(p.proposal.payload, Payload::Artifact { .. })
            && p.bytes().is_empty()
            && !self.allow_empty_artifact
        {
            return Err(PolicyError::InvalidCandidate);
        }
        if let ScoreRequirement::Required(meaning) = self.score {
            if !p.proposal.numerical.is_some_and(|s| s.meaning == meaning) {
                return Err(PolicyError::InvalidCandidate);
            }
        }
        Ok(())
    }
}
