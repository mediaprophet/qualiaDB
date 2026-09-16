//! Borrowed bounded input contract.

use super::budget::ConditioningBudget;
use super::requirement::RequirementRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditioningError {
    UnsupportedVersion,
    InvalidReference,
    DuplicateRequirement,
    ConflictingRequirements,
    DisclosureDenied,
    RequiredCapabilityMissing,
    ContextBudgetExceeded,
    OutputBufferFull,
    IncompatibleArtifact,
    DeadlineExceeded,
    ValidationFailed,
    CodecError,
}

#[derive(Debug, Clone, Copy)]
pub struct OutputContractRef<'a> {
    pub schema_hint: Option<&'a str>,
    pub min_citations: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ConditioningSpec<'a> {
    pub schema_version: u16,
    pub profile_id: &'a str,
    pub objective: &'a str,
    pub requirements: &'a [RequirementRef<'a>],
    pub domain_refs: &'a [u64],
    pub output_contract: OutputContractRef<'a>,
    pub budget: ConditioningBudget,
}
