//! Zero-heap hot dispatch (`dispatch_prepared_call`) enforcing the 6-stage lifecycle
//! (`Admit -> Execute -> Validate -> Commit/Abort`) across all 4 `ExecutionMode`s.

use super::registry::ExtensionRegistry;
use vibescript_abi::{
    q_hash, AdmissionRequest, AdmissionTicket, BorrowedSliceView, CallerOutputSlice,
    CompletionStatus, EffectSet, ExecutionMode, ExecutionReceiptHeader, NQuin48,
    PreparedCallHandle, ResourceEnvelope, SentinelPolicyGate, VibeOpCode, VibeTrap,
};

/// Context supplied by the caller for a single prepared extension invocation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CallExecutionContext {
    pub actor_did_hash: u64,
    pub context_world_hash: u64,
    pub lamport_timestamp: u64,
    pub mode: ExecutionMode,
}

/// Complete outcome of a prepared extension call, including its sealed receipt and audit `NQuin48`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub ticks_consumed: u64,
    pub output_elems_written: usize,
    pub output_bytes_written: usize,
    pub receipt: ExecutionReceiptHeader,
    pub audit_nquins: [NQuin48; 2],
}

/// Executes a `PreparedCallHandle` in zero heap allocations (`Tier-1 HotZeroHeap`).
///
/// Enforces:
/// 1. `RegistryGeneration` freshness (`StaleRegistryGeneration` on mismatch).
/// 2. `contract_digest` integrity (`ContractDigestMismatch`).
/// 3. Parameter arity, type, and element count bounds (`ArityMismatch`, `TypeMismatch`, `SliceLengthMismatch`).
/// 4. Pre-dispatch output buffer capacity (`OutputBufferTooSmall`) before any admission or side effects.
/// 5. Mode-specific Sentinel admission (`GovernedApplication`, `StandaloneDeveloper`, `SimulatedGovernanceTest`, `IntegratedGovernanceTest`).
/// 6. Post-execution validation and fail-closed receipt persistence (`ReceiptPersistenceFailed`).
pub fn dispatch_prepared_call<'g>(
    registry: &ExtensionRegistry<'_>,
    handle: &PreparedCallHandle,
    ctx: &CallExecutionContext,
    mut gate: Option<&mut (dyn SentinelPolicyGate + 'g)>,
    inputs: &[BorrowedSliceView<'_>],
    output: &mut CallerOutputSlice<'_>,
) -> Result<DispatchOutcome, VibeTrap> {
    // 1. Verify registry generation freshness in O(1)
    if handle.registry_generation != registry.generation() {
        return Err(VibeTrap::StaleRegistryGeneration);
    }

    // 2. Resolve provider and function descriptor in O(1)
    let (provider, func) = registry.resolve_opcode(handle.opcode)?;
    if func.contract.contract_digest != handle.contract_digest {
        return Err(VibeTrap::ContractDigestMismatch);
    }

    // 3. Validate input parameter arity, types, and element count bounds
    if inputs.len() != func.params.len() {
        return Err(VibeTrap::ArityMismatch);
    }
    let mut input_fold = handle.func_iri_hash;
    let mut i = 0;
    while i < inputs.len() {
        let spec = &func.params[i];
        let view = &inputs[i];
        if view.kind() != spec.kind {
            return Err(VibeTrap::TypeMismatch);
        }
        let count = view.elem_count();
        if count < spec.min_elems as usize || count > spec.max_elems as usize {
            return Err(VibeTrap::SliceLengthMismatch);
        }
        input_fold ^= q_hash(view.as_bytes());
        i += 1;
    }

    // 4. Pre-validate caller output slice type and capacity BEFORE admission or execution
    if output.kind() != func.output_kind {
        return Err(VibeTrap::TypeMismatch);
    }
    output.ensure_elem_capacity(func.required_output_elems as usize)?;

    // 5. Stage 3: Admission
    let ticket = match ctx.mode {
        ExecutionMode::StandaloneDeveloper => {
            // Standalone developer mode permits PureCompute without a Sentinel gate.
            if !func.effects.is_pure() {
                return Err(VibeTrap::EffectNotPermittedInMode);
            }
            AdmissionTicket {
                ticket_id: ctx.lamport_timestamp ^ handle.func_iri_hash,
                policy_generation: 0,
                actor_did_hash: ctx.actor_did_hash,
                context_world_hash: ctx.context_world_hash,
                granted_effects: EffectSet::PURE_COMPUTE,
                approved_envelope: func.envelope,
                expiry_lamport: u64::MAX,
                simulated: false,
            }
        }
        ExecutionMode::GovernedApplication
        | ExecutionMode::SimulatedGovernanceTest
        | ExecutionMode::IntegratedGovernanceTest => {
            let policy_gate = gate.as_deref_mut().ok_or(VibeTrap::SentinelUnavailable)?;
            if !policy_gate.is_controller_alive() {
                return Err(VibeTrap::SentinelUnavailable);
            }
            let req = AdmissionRequest {
                handle: *handle,
                actor_did_hash: ctx.actor_did_hash,
                context_world_hash: ctx.context_world_hash,
                lamport_timestamp: ctx.lamport_timestamp,
                mode: ctx.mode,
                requested_effects: func.effects,
                declared_envelope: func.envelope,
                input_fingerprint: input_fold,
            };
            policy_gate.admit_call(&req)?
        }
    };

    // 6. Stage 4: Execute
    let ticks_consumed = provider.call(func.stable_id, inputs, output)?;
    if ticks_consumed > ticket.approved_envelope.max_ticks {
        return Err(VibeTrap::TickBudgetExhausted);
    }
    let output_bytes_written = output.written_bytes();
    if output_bytes_written > ticket.approved_envelope.max_output_bytes as usize {
        return Err(VibeTrap::ResourceEnvelopeExceeded);
    }

    // 7. Stage 5 & 6: Validate and Commit Receipt
    let output_hash = q_hash(output.written_byte_slice());
    let io_fold = input_fold ^ output_hash.rotate_left(17);

    let (status, simulated) = match ctx.mode {
        ExecutionMode::StandaloneDeveloper => (CompletionStatus::StandaloneCompleted, false),
        ExecutionMode::SimulatedGovernanceTest => (CompletionStatus::SimulatedCommitted, true),
        ExecutionMode::GovernedApplication | ExecutionMode::IntegratedGovernanceTest => {
            (CompletionStatus::Committed, ticket.simulated)
        }
    };

    let receipt = ExecutionReceiptHeader {
        ticket_id: ticket.ticket_id,
        actor_did_hash: ctx.actor_did_hash,
        provider_iri_hash: handle.provider_iri_hash,
        func_iri_hash: handle.func_iri_hash,
        stable_id: func.stable_id,
        mode: ctx.mode,
        status,
        simulated,
        reserved: [0; 5],
        policy_generation: ticket.policy_generation,
        registry_generation: handle.registry_generation,
        ticks_consumed: ticks_consumed as u32,
        output_bytes_written: output_bytes_written as u32,
        trap_code: 0,
        failure_stage: 0,
        input_output_fold_hash: io_fold,
        lamport_timestamp: ctx.lamport_timestamp,
    };

    let audit_nquins = if let Some(policy_gate) = gate {
        policy_gate.validate_post_execution(
            &ticket,
            handle.opcode,
            ticks_consumed,
            output_bytes_written,
            output_hash,
        )?;
        policy_gate.commit_receipt(&receipt)?
    } else {
        receipt.to_audit_nquins()
    };

    Ok(DispatchOutcome {
        ticks_consumed,
        output_elems_written: output.written_elems(),
        output_bytes_written,
        receipt,
        audit_nquins,
    })
}

/// Deterministic in-memory policy gate for `ExecutionMode::SimulatedGovernanceTest`.
pub struct SimulatedPolicyGate {
    pub controller_alive: bool,
    pub policy_generation: u64,
    pub revoke_during_execution: bool,
    pub fail_receipt_commit: bool,
    pub allowed_actor_hash: u64,
    pub explicit_deny_func_hash: u64,
    pub max_expiry_lamport: u64,
    pub allowed_effects: EffectSet,
    pub budget: ResourceEnvelope,
    pub committed_receipts_count: usize,
}

impl SimulatedPolicyGate {
    /// Creates a permissive test policy gate for `allowed_actor_hash`.
    pub const fn new_for_actor(allowed_actor_hash: u64) -> Self {
        Self {
            controller_alive: true,
            policy_generation: 1,
            revoke_during_execution: false,
            fail_receipt_commit: false,
            allowed_actor_hash,
            explicit_deny_func_hash: 0,
            max_expiry_lamport: 1_000_000,
            allowed_effects: EffectSet(0xFFFF_FFFF),
            budget: ResourceEnvelope {
                max_ticks: 100_000,
                max_scratch_bytes: 0,
                max_vram_bytes: 0,
                max_output_bytes: 4096,
                max_spawned_tasks: 0,
                flags: 0,
            },
            committed_receipts_count: 0,
        }
    }
}

impl SentinelPolicyGate for SimulatedPolicyGate {
    fn is_controller_alive(&self) -> bool {
        self.controller_alive
    }

    fn current_policy_generation(&self) -> u64 {
        self.policy_generation
    }

    fn admit_call(&mut self, request: &AdmissionRequest) -> Result<AdmissionTicket, VibeTrap> {
        if !self.controller_alive {
            return Err(VibeTrap::SentinelUnavailable);
        }
        if request.handle.func_iri_hash == self.explicit_deny_func_hash {
            return Err(VibeTrap::ExplicitDenyRule);
        }
        if self.allowed_actor_hash != 0 && request.actor_did_hash != self.allowed_actor_hash {
            return Err(VibeTrap::DelegationScopeMismatch);
        }
        if request.lamport_timestamp > self.max_expiry_lamport {
            return Err(VibeTrap::AuthorityExpired);
        }
        if !request.requested_effects.is_subset_of(self.allowed_effects) {
            return Err(VibeTrap::PolicyDenied);
        }
        if !request.declared_envelope.fits_within(&self.budget) {
            return Err(VibeTrap::ResourceEnvelopeExceeded);
        }
        Ok(AdmissionTicket {
            ticket_id: request.lamport_timestamp ^ request.input_fingerprint ^ 0xA5A5_5A5A,
            policy_generation: self.policy_generation,
            actor_did_hash: request.actor_did_hash,
            context_world_hash: request.context_world_hash,
            granted_effects: request.requested_effects,
            approved_envelope: self.budget,
            expiry_lamport: self.max_expiry_lamport,
            simulated: request.mode == ExecutionMode::SimulatedGovernanceTest,
        })
    }

    fn validate_post_execution(
        &mut self,
        ticket: &AdmissionTicket,
        _opcode: VibeOpCode,
        ticks_consumed: u64,
        output_bytes_written: usize,
        _output_digest: u64,
    ) -> Result<(), VibeTrap> {
        if self.revoke_during_execution || ticket.policy_generation != self.policy_generation {
            return Err(VibeTrap::MidExecutionPolicyRevoked);
        }
        if ticks_consumed > ticket.approved_envelope.max_ticks {
            return Err(VibeTrap::TickBudgetExhausted);
        }
        if output_bytes_written > ticket.approved_envelope.max_output_bytes as usize {
            return Err(VibeTrap::ResourceEnvelopeExceeded);
        }
        Ok(())
    }

    fn commit_receipt(
        &mut self,
        receipt: &ExecutionReceiptHeader,
    ) -> Result<[NQuin48; 2], VibeTrap> {
        if self.fail_receipt_commit {
            return Err(VibeTrap::ReceiptPersistenceFailed);
        }
        self.committed_receipts_count += 1;
        Ok(receipt.to_audit_nquins())
    }
}
