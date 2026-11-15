//! Epistemic, temporal and observer-relative bifurcation kernel (§3.6.3 / §3.4).
//!
//! Provides the 64-byte cache-aligned `BifurcationContext64` descriptor and
//! zero-heap filtering primitives (`evaluate_quin_bifurcation`, `filter_quins_bifurcated_into`)
//! for fine-grained per-element visibility, LOD culling, and epistemic isolation.

use crate::NQuin;
use bytemuck::{Pod, Zeroable};

/// Epistemic truth status for an element or proposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
#[repr(u8)]
pub enum EpistemicKind {
    /// Empirically verified observational or sensor datum.
    #[default]
    EmpiricalFact = 0,
    /// Rigorous, non-fictional scientific inference or theoretical prediction.
    TheoreticalHypothesis = 1,
    /// Speculative counterfactual variant.
    Counterfactual = 2,
    /// Narrative, fictional, or pedagogical construct.
    FictionalConstruct = 3,
    /// Epistemically degraded claim under Socratic qualification gate.
    SocraticDegraded = 4,
}

// ── Sensitivity Class Constants ──────────────────────────────────────────
pub const SENSITIVITY_PUBLIC: u8 = 0;
pub const SENSITIVITY_RESTRICTED: u8 = 1;
pub const SENSITIVITY_CLASSIFIED: u8 = 2;
pub const SENSITIVITY_SANCTUARY: u8 = 3;

// ── Socio-Cognitive Wing Selectors ───────────────────────────────────────
pub const WING_PUBLIC_TWIN: u8 = 0;
pub const WING_PERSONAL_TWIN: u8 = 1;
pub const WING_CLINICAL_DIAGNOSTIC: u8 = 2;
pub const WING_ENGINEERING_AUDIT: u8 = 3;

// ── Bifurcation Context Flags ────────────────────────────────────────────
pub const FLAG_REQUIRE_PARITY: u16 = 0x0001;
pub const FLAG_STRICT_SUBJECT: u16 = 0x0002;
pub const FLAG_STRICT_EPOCH: u16 = 0x0004;
pub const FLAG_DEONTIC_FILTER: u16 = 0x0008;

/// 64-byte cache-aligned context for multi-observer epistemic bifurcation.
///
/// Guaranteed `Pod + Zeroable` layout with zero internal padding bytes.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct BifurcationContext64 {
    /// FNV-1a hash of observer DID.
    pub observer_did_hash: u64,
    /// FNV-1a hash of subject entity DID (0 = wildcard / accept any).
    pub subject_did_hash: u64,
    /// Cryptographic consent receipt token hash.
    pub consent_receipt_hash: u64,
    /// Bitmask of active `ManifoldDomain` indices (bits 0..=12).
    pub active_manifold_mask: u16,
    /// Maximum sensitivity class allowed (0=Public..3=Sanctuary).
    pub max_sensitivity_class: u8,
    /// Active representation wing selector (0=PublicTwin..3=EngineeringAudit).
    pub wing_selector: u8,
    /// Maximum visible quantum state index `q` (0 = ground truth only).
    pub allowed_q_max: u32,
    /// Lamport logical clock ceiling / epoch boundary.
    pub lamport_epoch: u64,
    /// Affordance permission bitmask.
    pub affordance_mask: u64,
    /// Finest permitted spatial LOD tier (0..=7).
    pub spatial_lod_cap: u8,
    /// Deontic routing lane filter (0 = passthrough / any).
    pub deontic_routing_lane: u8,
    /// Operational flags (`FLAG_REQUIRE_PARITY`, etc.).
    pub flags: u16,
    /// Castagnoli CRC-32C of bytes 0..52.
    pub context_crc32c: u32,
    /// Reserved for hardware alignment and future extensions.
    pub reserved: [u8; 8],
}

// Compile-time layout guarantees
const _: () = assert!(core::mem::size_of::<BifurcationContext64>() == 64);
const _: () = assert!(core::mem::align_of::<BifurcationContext64>() == 64);

impl Default for BifurcationContext64 {
    fn default() -> Self {
        let mut ctx = Self {
            observer_did_hash: 0,
            subject_did_hash: 0,
            consent_receipt_hash: 0,
            active_manifold_mask: 0x1FFF, // All 13 manifold domains enabled
            max_sensitivity_class: SENSITIVITY_PUBLIC,
            wing_selector: WING_PUBLIC_TWIN,
            allowed_q_max: u32::MAX,
            lamport_epoch: 0,
            affordance_mask: u64::MAX,
            spatial_lod_cap: 7,
            deontic_routing_lane: 0,
            flags: FLAG_REQUIRE_PARITY,
            context_crc32c: 0,
            reserved: [0u8; 8],
        };
        ctx.seal_crc32c();
        ctx
    }
}

impl BifurcationContext64 {
    /// Compute the CRC-32C checksum over the 52 header bytes preceding `context_crc32c`.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes = bytemuck::bytes_of(self);
        crate::container_10d::crc32c::crc32c(&bytes[..52])
    }

    /// Update `context_crc32c` in place.
    #[inline]
    pub fn seal_crc32c(&mut self) {
        self.context_crc32c = self.compute_crc32c();
    }

    /// Check if `context_crc32c` matches the computed checksum.
    #[inline]
    pub fn verify_crc32c(&self) -> bool {
        self.context_crc32c == self.compute_crc32c()
    }
}

/// Zero-heap evaluation of an `NQuin` against a `BifurcationContext64`.
///
/// Returns `true` if the quin satisfies parity, sensitivity, subject, and epoch constraints.
#[inline]
pub fn evaluate_quin_bifurcation(ctx: &BifurcationContext64, quin: &NQuin) -> bool {
    // 1. ECC parity verification
    if (ctx.flags & FLAG_REQUIRE_PARITY) != 0 && !quin.verify_ecc_parity() {
        return false;
    }

    // 2. Sensitivity ceiling check
    let quin_sens = quin.get_sensitivity_byte();
    if quin_sens > ctx.max_sensitivity_class {
        return false;
    }

    // 3. Subject DID filtering
    if (ctx.flags & FLAG_STRICT_SUBJECT) != 0 && ctx.subject_did_hash != 0 {
        let quin_subj = quin.subject & 0x7FFF_FFFF_FFFF_FFFF;
        let expected_subj = ctx.subject_did_hash & 0x7FFF_FFFF_FFFF_FFFF;
        if quin_subj != expected_subj {
            return false;
        }
    }

    // 4. Lamport epoch boundary check
    if (ctx.flags & FLAG_STRICT_EPOCH) != 0 && ctx.lamport_epoch != 0 {
        let quin_clock = quin.extract_lamport_clock() as u64;
        if quin_clock > ctx.lamport_epoch {
            return false;
        }
    }

    // 5. Deontic routing lane filter
    if (ctx.flags & FLAG_DEONTIC_FILTER) != 0 && ctx.deontic_routing_lane != 0 {
        let quin_lane = ((quin.metadata >> 61) & 0b11) as u8;
        if quin_lane != ctx.deontic_routing_lane {
            return false;
        }
    }

    true
}

/// Filter a slice of `NQuin`s into a caller-supplied fixed buffer without allocating.
///
/// Returns the number of quins written into `out`.
pub fn filter_quins_bifurcated_into(
    ctx: &BifurcationContext64,
    quins: &[NQuin],
    out: &mut [NQuin],
) -> usize {
    let mut written = 0;
    for q in quins {
        if evaluate_quin_bifurcation(ctx, q) {
            if written < out.len() {
                out[written] = *q;
                written += 1;
            } else {
                break;
            }
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_quin(subject: u64, pred: u64, obj: u64, ctx: u64, meta: u64) -> NQuin {
        let parity = NQuin::calculate_parity(subject, pred, obj, ctx, meta);
        NQuin {
            subject,
            predicate: pred,
            object: obj,
            context: ctx,
            metadata: meta,
            parity,
        }
    }

    #[test]
    fn context_size_and_alignment_are_64() {
        assert_eq!(core::mem::size_of::<BifurcationContext64>(), 64);
        assert_eq!(core::mem::align_of::<BifurcationContext64>(), 64);
    }

    #[test]
    fn crc32c_sealing_and_verification() {
        let mut ctx = BifurcationContext64::default();
        assert!(ctx.verify_crc32c());

        ctx.allowed_q_max = 42;
        assert!(!ctx.verify_crc32c());
        ctx.seal_crc32c();
        assert!(ctx.verify_crc32c());
    }

    #[test]
    fn parity_rejection() {
        let ctx = BifurcationContext64::default();
        let mut q = make_valid_quin(1, 2, 3, 0, 0);
        assert!(evaluate_quin_bifurcation(&ctx, &q));

        q.parity ^= 1; // Corrupt parity
        assert!(!evaluate_quin_bifurcation(&ctx, &q));
    }

    #[test]
    fn sensitivity_ceiling_filter() {
        let mut ctx = BifurcationContext64::default();
        ctx.max_sensitivity_class = SENSITIVITY_PUBLIC;

        let q_pub = make_valid_quin(1, 2, 3, (SENSITIVITY_PUBLIC as u64) << 56, 0);
        let q_restr = make_valid_quin(1, 2, 3, (SENSITIVITY_RESTRICTED as u64) << 56, 0);

        assert!(evaluate_quin_bifurcation(&ctx, &q_pub));
        assert!(!evaluate_quin_bifurcation(&ctx, &q_restr));

        ctx.max_sensitivity_class = SENSITIVITY_RESTRICTED;
        assert!(evaluate_quin_bifurcation(&ctx, &q_restr));
    }

    #[test]
    fn filter_quins_zero_heap_into_buffer() {
        let ctx = BifurcationContext64::default();
        let q1 = make_valid_quin(1, 2, 3, 0, 0);
        let q2 = make_valid_quin(4, 5, 6, 0, 0);
        let mut q3_corrupted = make_valid_quin(7, 8, 9, 0, 0);
        q3_corrupted.parity ^= 0xFF;

        let input = [q1, q2, q3_corrupted];
        let mut out = [NQuin::default(); 4];

        let n = filter_quins_bifurcated_into(&ctx, &input, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0].subject, 1);
        assert_eq!(out[1].subject, 4);
    }
}
