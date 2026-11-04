use super::CpuAdmissionError;
use crate::inference::inference_agent::governed::ConstraintInstruction;

/// Every requested feature is explicit. These refusals apply only to this new
/// entrypoint, not the capabilities of the existing native/browser backends.
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestedCapabilities {
    pub sampling: bool,
    pub grammar: bool,
    pub speculative: bool,
    pub topology: bool,
    pub external_tokenizer: bool,
    pub helper_model: bool,
    pub lora: bool,
    pub implicit_chat_template: bool,
}
impl RequestedCapabilities {
    pub(super) fn validate(self) -> Result<(), CpuAdmissionError> {
        if self.sampling
            || self.grammar
            || self.speculative
            || self.topology
            || self.external_tokenizer
            || self.helper_model
            || self.lora
            || self.implicit_chat_template
        {
            return Err(CpuAdmissionError::UnsupportedCapability);
        }
        Ok(())
    }
}

/// Real token-event fields are subject, predicate, integer token ID and context.
/// Metadata contains no semantic confidence. Numeric comparisons belong to the
/// separate signed score program, which receives the actual finite raw logit.
pub(in crate::inference::inference_agent) fn check_projection_program(
    program: &[ConstraintInstruction],
) -> Result<(), CpuAdmissionError> {
    use ConstraintInstruction::*;
    for instruction in program {
        match *instruction {
            MatchSubject { .. } | MatchPredicate { .. } | MatchObject { .. } | HaltIfFalse => (),
            Bind {
                vector: 0..=3,
                register: 0..=15,
            }
            | MatchRegister {
                vector: 0..=3,
                register: 0..=15,
            } => (),
            _ => return Err(CpuAdmissionError::UnsupportedObservation),
        }
    }
    Ok(())
}
