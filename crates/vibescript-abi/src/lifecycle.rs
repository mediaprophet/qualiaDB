//! Six-stage governed execution lifecycle (`Register -> Prepare -> Admit -> Execute -> Validate -> Commit/Abort`),
//! deterministic trap taxonomy (`VibeTrap`), and `SentinelPolicyGate`.

use crate::effects::{EffectSet, ExecutionMode, ResourceEnvelope};
use crate::identity::{PreparedCallHandle, RegistryGeneration, StableFuncId, VibeOpCode};
use crate::memory::NQuin48;

/// Stage at which an extension lifecycle failure occurred.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureStage {
    Register = 1,
    Prepare = 2,
    Admit = 3,
    Execute = 4,
    Validate = 5,
    CommitReceipt = 6,
}

/// Deterministic, zero-allocation trap code returned across the extension boundary.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VibeTrap {
    // Registration stage (100..199)
    RegistryCapacityExceeded = 100,
    InvalidCanonicalIri = 101,
    HashMismatch = 102,
    DuplicateNamespaceIri = 103,
    DuplicateShortAlias = 104,
    DuplicateFunctionIri = 105,
    DuplicateStableFuncId = 106,
    HashCollisionDetected = 107,
    IncompatibleVersionAxes = 108,
    OntologyBudgetExceeded = 109,
    InvalidOntologySpan = 110,
    InvalidQuinParity = 111,

    // Preparation stage (200..299)
    NamespaceNotFound = 200,
    FunctionNotFound = 201,
    VersionRequirementUnsatisfied = 202,
    TargetProfileUnsupported = 203,
    StaleRegistryGeneration = 204,
    ContractDigestMismatch = 205,

    // Admission stage (300..399)
    SentinelUnavailable = 300,
    PolicyDenied = 301,
    ExplicitDenyRule = 302,
    AuthorityExpired = 303,
    DelegationScopeMismatch = 304,
    RevokedPolicyGeneration = 305,
    EffectNotPermittedInMode = 306,
    ResourceEnvelopeExceeded = 307,
    PreconditionFailed = 308,

    // Execution & memory stage (400..499)
    ArityMismatch = 400,
    TypeMismatch = 401,
    SliceBoundsOverflow = 402,
    SliceLengthMismatch = 403,
    OutputBufferTooSmall = 404,
    InvalidResourceHandleOwner = 405,
    StaleResourceHandleGeneration = 406,
    TickBudgetExhausted = 407,
    NumericalDomainError = 408,

    // Validation & Commit/Receipt stage (500..599)
    PostconditionFailed = 500,
    MidExecutionPolicyRevoked = 501,
    ReceiptPersistenceFailed = 502,
    StagingAbortRequired = 503,
}

impl VibeTrap {
    /// Returns the lifecycle stage associated with this trap.
    #[inline]
    pub const fn stage(self) -> FailureStage {
        let code = self as u16;
        if code < 200 {
            FailureStage::Register
        } else if code < 300 {
            FailureStage::Prepare
        } else if code < 400 {
            FailureStage::Admit
        } else if code < 500 {
            FailureStage::Execute
        } else if code < 502 {
            FailureStage::Validate
        } else {
            FailureStage::CommitReceipt
        }
    }
}

/// Terminal completion status recorded in an `ExecutionReceiptHeader`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionStatus {
    /// Governed call admitted, validated, receipt persisted, and state committed.
    Committed = 1,
    /// Simulated governance test call completed (receipt flagged `simulated = true`).
    SimulatedCommitted = 2,
    /// Standalone developer call completed (no governed authority claimed).
    StandaloneCompleted = 3,
    /// Rejected during `Admit` by the Sentinel policy gate.
    AbortedPolicyDenied = 4,
    /// Rejected during `Validate` (postcondition or mid-call policy revocation); staged state rolled back.
    AbortedValidationFailed = 5,
    /// Rejected during `Commit` because receipt persistence failed; staged state rolled back.
    AbortedReceiptFailed = 6,
    /// Trapped during `Execute`; staged state rolled back.
    TrappedExecution = 7,
}

/// Admission request submitted to `SentinelPolicyGate::admit_call`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub handle: PreparedCallHandle,
    pub actor_did_hash: u64,
    pub context_world_hash: u64,
    pub lamport_timestamp: u64,
    pub mode: ExecutionMode,
    pub requested_effects: EffectSet,
    pub declared_envelope: ResourceEnvelope,
    pub input_fingerprint: u64,
}

/// Scoped admission ticket returned by `SentinelPolicyGate::admit_call`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionTicket {
    pub ticket_id: u64,
    pub policy_generation: u64,
    pub actor_did_hash: u64,
    pub context_world_hash: u64,
    pub granted_effects: EffectSet,
    pub approved_envelope: ResourceEnvelope,
    pub expiry_lamport: u64,
    pub simulated: bool,
}

/// Compact, zero-heap cryptographic/audit receipt header for a VibeScript extension call.
///
/// Seals cleanly into two parity-valid `NQuin48` records via `to_audit_nquins()`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionReceiptHeader {
    pub ticket_id: u64,
    pub actor_did_hash: u64,
    pub provider_iri_hash: u64,
    pub func_iri_hash: u64,
    pub stable_id: StableFuncId,
    pub mode: ExecutionMode,
    pub status: CompletionStatus,
    pub simulated: bool,
    pub reserved: [u8; 5],
    pub policy_generation: u64,
    pub registry_generation: RegistryGeneration,
    pub ticks_consumed: u32,
    pub output_bytes_written: u32,
    pub trap_code: u16,
    pub failure_stage: u16,
    pub input_output_fold_hash: u64,
    pub lamport_timestamp: u64,
}

impl ExecutionReceiptHeader {
    /// Encodes this receipt into two parity-valid `NQuin48` records for zero-heap persistence.
    #[inline]
    pub const fn to_audit_nquins(&self) -> [NQuin48; 2] {
        let status_pack = ((self.mode as u64) << 56)
            | ((self.status as u64) << 48)
            | ((self.simulated as u64) << 40)
            | ((self.trap_code as u64) << 16)
            | (self.failure_stage as u64);
        let q0 = NQuin48::from_parts(
            self.actor_did_hash,
            self.func_iri_hash,
            self.provider_iri_hash,
            self.ticket_id,
            status_pack,
        );
        let metrics_pack = ((self.registry_generation.0 as u64) << 32) | (self.ticks_consumed as u64);
        let q1 = NQuin48::from_parts(
            self.ticket_id,
            self.policy_generation,
            self.input_output_fold_hash,
            self.lamport_timestamp,
            metrics_pack ^ ((self.output_bytes_written as u64) << 16),
        );
        [q0, q1]
    }
}

/// Host-neutral policy & receipt gate implemented by QualiaDB's Webizen Sentinel
/// (and by deterministic simulated test harnesses).
pub trait SentinelPolicyGate {
    /// Returns `true` if the underlying Sentinel controller is alive and reachable.
    fn is_controller_alive(&self) -> bool;

    /// Returns the current active policy generation counter.
    fn current_policy_generation(&self) -> u64;

    /// Stage 3 (`Admit`): Evaluates deontic norms, delegation scope, expiry, and resource bounds.
    fn admit_call(&mut self, request: &AdmissionRequest) -> Result<AdmissionTicket, VibeTrap>;

    /// Stage 5 (`Validate`): Verifies postconditions, mid-execution policy generation validity,
    /// and actual resource usage before state publication.
    fn validate_post_execution(
        &mut self,
        ticket: &AdmissionTicket,
        opcode: VibeOpCode,
        ticks_consumed: u64,
        output_bytes_written: usize,
        output_digest: u64,
    ) -> Result<(), VibeTrap>;

    /// Stage 6 (`Commit` / `Abort`): Persists the execution receipt.
    ///
    /// **Fail-closed rule**: If `commit_receipt` returns `Err(VibeTrap::ReceiptPersistenceFailed)`,
    /// the caller MUST abort all staged state mutations and withhold publication.
    fn commit_receipt(&mut self, receipt: &ExecutionReceiptHeader) -> Result<[NQuin48; 2], VibeTrap>;
}
