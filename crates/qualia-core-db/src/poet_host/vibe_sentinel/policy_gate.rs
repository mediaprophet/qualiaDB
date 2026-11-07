//! Real QualiaDB Webizen Sentinel policy gate (`WebizenSentinelGate`, `VS03-09`)
//! evaluating deontic contract `NQuin`s (`OP_OBLIGATE`, `OP_PERMIT`, `OP_FORBID`, `DEFEATER_BIT`).

use crate::modalities::logic::deontic::{
    evaluate_deontic_contract, DeonticStatus, DeonticVerdict, DEFEATER_BIT, OP_FORBID, OP_OBLIGATE,
    OP_PERMIT,
};
use crate::NQuin;
use vibescript_abi::{
    AdmissionRequest, AdmissionTicket, EffectSet, ExecutionMode, ExecutionReceiptHeader, NQuin48,
    ResourceEnvelope, SentinelPolicyGate, VibeOpCode, VibeTrap,
};

/// Stack capacity for deontic verdicts per `admit_call` evaluation (`64 × 64B = 4 KiB`).
pub const MAX_SENTINEL_VERDICTS: usize = 64;

/// Converts a `vibescript_abi::NQuin48` into `crate::NQuin`.
#[inline]
pub const fn quin48_to_nquin(q: &NQuin48) -> NQuin {
    NQuin {
        subject: q.0[0],
        predicate: q.0[1],
        object: q.0[2],
        context: q.0[3],
        metadata: q.0[4],
        parity: q.0[5],
    }
}

/// Converts a `crate::NQuin` into `vibescript_abi::NQuin48`.
#[inline]
pub const fn nquin_to_quin48(q: &NQuin) -> NQuin48 {
    NQuin48([
        q.subject,
        q.predicate,
        q.object,
        q.context,
        q.metadata,
        q.parity,
    ])
}

/// Helper to construct a parity-sealed deontic norm `NQuin` governing an extension call.
///
/// - `subject`: `actor_did_hash` (`0` = wildcard principal)
/// - `predicate`: `((func_or_provider_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64) | (DEFEATER_BIT if is_defeater)`
/// - `object`: `provider_iri_hash`
/// - `context`: `context_world_hash` (`0` = global world)
/// - `metadata`: `expiry_unix32 as u64` (`0` = no expiry)
#[inline]
pub const fn make_deontic_norm_quin(
    actor_did_hash: u64,
    opcode: u8,
    is_defeater: bool,
    func_or_provider_hash: u64,
    provider_iri_hash: u64,
    context_world_hash: u64,
    expiry_unix32: u32,
) -> NQuin {
    let mut predicate =
        ((func_or_provider_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater {
        predicate |= DEFEATER_BIT;
    }
    let metadata = expiry_unix32 as u64;
    let parity = actor_did_hash ^ predicate ^ provider_iri_hash ^ context_world_hash ^ metadata;
    NQuin {
        subject: actor_did_hash,
        predicate,
        object: provider_iri_hash,
        context: context_world_hash,
        metadata,
        parity,
    }
}

/// Extracts the 55-bit property-path hash (`bits 8..62`) from a deontic norm predicate.
#[inline]
pub const fn norm_property_hash55(predicate: u64) -> u64 {
    (predicate >> 8) & 0x007F_FFFF_FFFF_FFFF
}

/// Zero-heap Webizen Sentinel policy gate backed by a deontic `NQuin` slice and a caller-supplied
/// audit receipt sink (`&mut [NQuin]`).
pub struct WebizenSentinelGate<'a> {
    policy_quins: &'a [NQuin],
    receipt_sink: &'a mut [NQuin],
    receipt_write_cursor: usize,
    controller_alive: bool,
    policy_generation: u64,
    allowed_effects: EffectSet,
    max_envelope: ResourceEnvelope,
}

impl<'a> WebizenSentinelGate<'a> {
    /// Creates a new `WebizenSentinelGate` over `policy_quins` and `receipt_sink`.
    pub fn new(
        policy_quins: &'a [NQuin],
        receipt_sink: &'a mut [NQuin],
        policy_generation: u64,
        allowed_effects: EffectSet,
        max_envelope: ResourceEnvelope,
    ) -> Self {
        Self {
            policy_quins,
            receipt_sink,
            receipt_write_cursor: 0,
            controller_alive: true,
            policy_generation,
            allowed_effects,
            max_envelope,
        }
    }

    /// Simulates or signals loss of the Sentinel controller (`GovernedApplication` must fail closed).
    #[inline]
    pub fn set_controller_alive(&mut self, alive: bool) {
        self.controller_alive = alive;
    }

    /// Advances or revokes the active policy generation (invalidating in-flight tickets).
    #[inline]
    pub fn bump_policy_generation(&mut self) {
        self.policy_generation = self.policy_generation.saturating_add(1);
    }

    /// Number of audit `NQuin` records persisted into `receipt_sink`.
    #[inline]
    pub const fn persisted_receipt_quin_count(&self) -> usize {
        self.receipt_write_cursor
    }

    /// Returns the slice of persisted audit `NQuin` records.
    #[inline]
    pub fn persisted_receipt_quins(&self) -> &[NQuin] {
        &self.receipt_sink[..self.receipt_write_cursor]
    }

    /// Evaluates whether `(actor_did_hash, func_iri_hash, provider_iri_hash, context_world_hash)`
    /// is permitted at `now_unix32` according to `self.policy_quins`.
    pub fn evaluate_access(
        &self,
        actor_did_hash: u64,
        func_iri_hash: u64,
        provider_iri_hash: u64,
        context_world_hash: u64,
        now_unix32: u32,
    ) -> Result<(), VibeTrap> {
        if self.policy_quins.len() > MAX_SENTINEL_VERDICTS {
            return Err(VibeTrap::ResourceEnvelopeExceeded);
        }
        let mut adapted_norms = [NQuin::default(); MAX_SENTINEL_VERDICTS];
        let mut k = 0;
        while k < self.policy_quins.len() {
            let q = self.policy_quins[k];
            let fold4 = q.subject ^ q.predicate ^ q.object ^ q.context;
            let expected5 = fold4 ^ q.metadata;
            adapted_norms[k] = NQuin {
                parity: if q.parity == expected5 { fold4 } else { !fold4 },
                ..q
            };
            k += 1;
        }
        let mut verdicts = [DeonticVerdict::default(); MAX_SENTINEL_VERDICTS];
        let count = evaluate_deontic_contract(
            &adapted_norms[..self.policy_quins.len()],
            now_unix32,
            &mut verdicts,
        )
        .map_err(|_| VibeTrap::ResourceEnvelopeExceeded)?;

        let func_h55 = func_iri_hash & 0x007F_FFFF_FFFF_FFFF;
        let prov_h55 = provider_iri_hash & 0x007F_FFFF_FFFF_FFFF;

        let mut saw_permit = false;
        let mut saw_expired_match = false;
        let mut saw_wrong_world_match = false;

        let mut i = 0;
        while i < count {
            let v = &verdicts[i];
            let norm = &v.norm;

            // Subject match: exact actor DID or wildcard (0)
            let subject_matches = norm.subject == 0 || norm.subject == actor_did_hash;
            // Target match: function hash55 or provider hash55
            let prop_h55 = norm_property_hash55(norm.predicate);
            let target_matches =
                (prop_h55 == func_h55 || prop_h55 == prov_h55) && norm.object == provider_iri_hash;

            if subject_matches && target_matches {
                let world_matches = norm.context == 0 || norm.context == context_world_hash;
                if !world_matches {
                    if v.status == DeonticStatus::Active
                        && (v.opcode == OP_PERMIT || v.opcode == OP_OBLIGATE)
                    {
                        saw_wrong_world_match = true;
                    }
                } else {
                    match v.status {
                        DeonticStatus::Active => {
                            if v.opcode == OP_FORBID {
                                // Explicit active FORBID always wins over PERMIT
                                return Err(VibeTrap::ExplicitDenyRule);
                            }
                            if v.opcode == OP_PERMIT || v.opcode == OP_OBLIGATE {
                                saw_permit = true;
                            }
                        }
                        DeonticStatus::Expired => {
                            if v.opcode == OP_PERMIT || v.opcode == OP_OBLIGATE {
                                saw_expired_match = true;
                            }
                        }
                        DeonticStatus::Defeated => {
                            // If a FORBID was defeated by a DEFEATER_BIT (q42:unless) norm,
                            // treat the defeated prohibition as an active exception permit.
                            if v.opcode == OP_FORBID {
                                saw_permit = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            i += 1;
        }

        if saw_permit {
            Ok(())
        } else if saw_expired_match {
            Err(VibeTrap::AuthorityExpired)
        } else if saw_wrong_world_match {
            Err(VibeTrap::DelegationScopeMismatch)
        } else {
            Err(VibeTrap::PolicyDenied)
        }
    }
}

impl SentinelPolicyGate for WebizenSentinelGate<'_> {
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
        if !request.requested_effects.is_subset_of(self.allowed_effects) {
            return Err(VibeTrap::PolicyDenied);
        }
        if !request.declared_envelope.fits_within(&self.max_envelope) {
            return Err(VibeTrap::ResourceEnvelopeExceeded);
        }

        let now_unix32 = (request.lamport_timestamp & 0xFFFF_FFFF) as u32;
        self.evaluate_access(
            request.actor_did_hash,
            request.handle.func_iri_hash,
            request.handle.provider_iri_hash,
            request.context_world_hash,
            now_unix32,
        )?;

        Ok(AdmissionTicket {
            ticket_id: request.lamport_timestamp
                ^ request.input_fingerprint
                ^ self.policy_generation.rotate_left(13),
            policy_generation: self.policy_generation,
            actor_did_hash: request.actor_did_hash,
            context_world_hash: request.context_world_hash,
            granted_effects: request.requested_effects,
            approved_envelope: self.max_envelope,
            expiry_lamport: u64::MAX,
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
        if !self.controller_alive {
            return Err(VibeTrap::SentinelUnavailable);
        }
        if ticket.policy_generation != self.policy_generation {
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
        if !self.controller_alive {
            return Err(VibeTrap::ReceiptPersistenceFailed);
        }
        if self.receipt_write_cursor + 2 > self.receipt_sink.len() {
            return Err(VibeTrap::ReceiptPersistenceFailed);
        }
        let audit48 = receipt.to_audit_nquins();
        let q0 = quin48_to_nquin(&audit48[0]);
        let q1 = quin48_to_nquin(&audit48[1]);
        self.receipt_sink[self.receipt_write_cursor] = q0;
        self.receipt_sink[self.receipt_write_cursor + 1] = q1;
        self.receipt_write_cursor += 2;
        Ok(audit48)
    }
}
