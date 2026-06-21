//! FrameLayout — the single canonical registry for the ~6 computational-support
//! bytes of the 48-byte NQuin Frame.
//!
//! A Frame is `6 × u64 = 48 bytes`: roughly **42 bytes of semantics** (the
//! subject / property-path / object / context hashes and packed literals) plus
//! **~6 bytes of computational support** (opcode, flags, datatype tags, a clock,
//! ECC parity) — which is what makes a Frame both *data* and *executable code* in
//! one cell. Every modality reads and writes those computational bytes through
//! here, so the primitive stays universal and no two conventions silently collide
//! (enforced by the tests below).
//!
//! ## `predicate` — opcode | property-path | defeater (co-resident; MUST NOT overlap)
//! `[0..7]` deontic/epistemic opcode · `[8..62]` property-path hash · `[63]` defeater.
//! "Proposition-mode" modalities (LTL/CTL/modal/abductive/causal/…) use the WHOLE
//! predicate as a plain `q_hash` (no opcode) — a documented mode, not a collision.
//!
//! ## `object` — MSB pointer flag | inline datatype tag | value (co-resident)
//! `[63]` set ⇒ the value is a lexicon/embedded pointer. When `[63]` is clear,
//! `[60..62]` is an inline datatype tag and `[0..59]` is the value. Canonical tags
//! (from `resolver`): INTEGER 001 · DECIMAL 010 · BOOLEAN 011 · BLOB 100 ·
//! **FLOAT 101** (new — raw f32 bits; resolves the prior clash where f32 squatted
//! on the INTEGER tag).
//!
//! ## `metadata` — a ROLE-KEYED OVERLAY field (read this carefully)
//! Unlike `predicate`/`object`, the 32 high bits of `metadata` are NOT a flat set
//! of independently-addressable sub-fields. They are a union of overlays, each
//! valid only for a particular quin *role*, several of which share bit positions.
//! Two overlays sharing bits is safe **iff** their roles are mutually exclusive
//! (a quin is never both at once). The ONE invariant that always holds — and is
//! enforced below — is that the **low-32 payload is disjoint from every high
//! overlay**, so a degree/expiry/timestamp can never silently corrupt a type/flag.
//!
//! * `[0..31]` low payload (any quin): exactly ONE of expiry / f32 truth-degree /
//!   32-bit timestamp / packed (alpha,mu,sigma) — selected by role.
//! * `[32..59]` tensor-bake `t` clock — ONLY on Tensor10D ground-truth nodes
//!   (`tensor::bake_pipeline`).
//! * `[50..59]` per-modality flag bits — ONLY on that modality's quins (each flag
//!   is a single distinct bit; see the pairwise-distinct test).
//! * `[56..59]` ODRL sensitivity tier — ONLY on access-controlled quins.
//! * `[60..63]` quin-type nibble — general typing; `[61..62]` of it doubles as the
//!   permissive-routing lane (`cbor_compiler`, `daemon_swarm`). Typed vs routed are
//!   treated as exclusive roles. quin_type was deliberately NOT relocated lower:
//!   every lower slot lands inside the `[32..59]` bake clock, which would be a worse
//!   (cross-role) collision than the documented `[61..62]` overlap.

use crate::NQuin;

// ════════════════════════════════════════════════════════════════════════════════
//  predicate field
// ════════════════════════════════════════════════════════════════════════════════
pub const OPCODE_MASK: u64 = 0xFF;
pub const DEFEATER_BIT: u64 = 1u64 << 63;
pub const PATH_MASK: u64 = 0x7FFF_FFFF_FFFF_FF00;

#[inline]
pub const fn opcode(predicate: u64) -> u8 {
    (predicate & OPCODE_MASK) as u8
}
#[inline]
pub const fn path_bits(predicate: u64) -> u64 {
    predicate & PATH_MASK
}
#[inline]
pub const fn is_defeater(predicate: u64) -> bool {
    predicate & DEFEATER_BIT != 0
}
/// Canonical norm-predicate packing — MUST equal `deontic::compile_norm_quin`.
#[inline]
pub const fn pack_predicate(opcode: u8, path_hash: u64, defeater: bool) -> u64 {
    let d = if defeater { DEFEATER_BIT } else { 0 };
    d | ((path_hash << 8) & PATH_MASK) | (opcode as u64)
}

// ════════════════════════════════════════════════════════════════════════════════
//  object field — inline literal datatype tags (canonical in `resolver`)
// ════════════════════════════════════════════════════════════════════════════════
pub use crate::resolver::{
    INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL, INLINE_TAG_INTEGER, INLINE_TAG_MASK,
    INLINE_VALUE_MASK, MSB_FLAG,
};
/// Blob/byte-offset pointer tag (canonical in `dicom`).
pub const INLINE_TAG_BLOB: u64 = 0b100u64 << 60;
/// Raw IEEE-754 f32 bits tag (NEW — was incorrectly sharing INTEGER's `0b001`).
pub const INLINE_TAG_FLOAT: u64 = 0b101u64 << 60;

/// The 3-bit inline datatype tag of an object value (only meaningful when MSB clear).
#[inline]
pub const fn object_tag(object: u64) -> u64 {
    object & INLINE_TAG_MASK
}
/// Pack an f32 into an object field with the canonical FLOAT tag.
#[inline]
pub fn pack_float_object(f: f32) -> u64 {
    INLINE_TAG_FLOAT | (f.to_bits() as u64 & INLINE_VALUE_MASK)
}
/// Recover an f32 from a FLOAT-tagged object field.
#[inline]
pub fn unpack_float_object(object: u64) -> f32 {
    f32::from_bits((object & INLINE_VALUE_MASK) as u32)
}

// ════════════════════════════════════════════════════════════════════════════════
//  metadata field
// ════════════════════════════════════════════════════════════════════════════════
/// Low-32 payload mask: expiry | f32 truth-degree | 32-bit timestamp (typed).
pub const LOW32_MASK: u64 = 0xFFFF_FFFF;

#[inline]
pub const fn expiry(metadata: u64) -> u32 {
    (metadata & LOW32_MASK) as u32
}
/// CANONICAL truth/belief/confidence/fuzzy degree (IEEE-754 f32 in the low 32 bits).
#[inline]
pub fn truth_degree(metadata: u64) -> f32 {
    f32::from_bits((metadata & LOW32_MASK) as u32)
}
#[inline]
pub fn with_truth_degree(d: f32) -> u64 {
    let d = if d.is_finite() { d } else { 0.0 };
    d.to_bits() as u64
}
#[inline]
pub const fn timestamp(metadata: u64) -> u64 {
    metadata
}

// ── high-overlay metadata fields (role-keyed; see module header) ──
/// quin-type nibble `[60..63]` (general typing). Its low two bits `[61..62]` are
/// reused as the permissive-routing lane on routed quins — typed vs routed are
/// exclusive roles. NOT relocated lower: every lower slot collides with the
/// `[32..59]` tensor-bake clock (a worse, cross-role collision).
pub const QUIN_TYPE_SHIFT: u32 = 60;
pub const QUIN_TYPE_MASK: u64 = 0xFu64 << QUIN_TYPE_SHIFT;
/// ODRL sensitivity tier `[56..59]` (only on access-controlled quins).
pub const SENSITIVITY_SHIFT: u32 = 56;
pub const SENSITIVITY_MASK: u64 = 0xFu64 << SENSITIVITY_SHIFT;
/// Permissive-routing lane `[61..62]` (only on routed quins; shares bits with the
/// quin-type nibble by role-exclusivity).
pub const ROUTING_LANE_SHIFT: u32 = 61;
pub const ROUTING_LANE_MASK: u64 = 0b11u64 << ROUTING_LANE_SHIFT;
/// Tensor-bake `t` clock (only on Tensor10D ground-truth nodes). Matches
/// `bake_pipeline`'s `(metadata >> 32) & 0x1FFF_FFFF`, i.e. bits `[32..60]` — it
/// even grazes quin_type's bit 60, an existing tensor-only quirk. Documentary
/// only; never asserted disjoint (its role excludes the others).
pub const BAKE_CLOCK_MASK: u64 = 0x1FFF_FFFFu64 << 32;

#[inline]
pub const fn quin_type(metadata: u64) -> u8 {
    ((metadata >> QUIN_TYPE_SHIFT) & 0xF) as u8
}
#[inline]
pub const fn with_quin_type(metadata: u64, ty: u8) -> u64 {
    (metadata & !QUIN_TYPE_MASK) | (((ty as u64) & 0xF) << QUIN_TYPE_SHIFT)
}

// ── per-modality flag bits [50..59] (each set only on its OWN modality's quins) ──
// These are mutually exclusive by quin type (a dialectical-synthesis quin is never
// also an argumentation node), so they may share bit positions with the ODRL
// sensitivity tier [56..59] WITHOUT a real same-quin collision. They are pairwise
// distinct (enforced) and disjoint from routing/quin_type (enforced).
pub const STABILIZATION_BIT: u64 = 1u64 << 50;
pub const FEEDBACK_BIT: u64 = 1u64 << 51;
pub const CONTROL_BIT: u64 = 1u64 << 52;
pub const DEFENSE_BIT: u64 = 1u64 << 53;
pub const ATTACK_BIT: u64 = 1u64 << 54;
pub const ARGUMENT_BIT: u64 = 1u64 << 55;
pub const COUNTERFACTUAL_BIT: u64 = 1u64 << 56;
pub const DO_INTERVENTION_BIT: u64 = 1u64 << 57;
pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;
pub const CONSUMED_BIT: u64 = 1u64 << 59;

// ════════════════════════════════════════════════════════════════════════════════
//  parity (ECC over the four semantic fields)
// ════════════════════════════════════════════════════════════════════════════════
#[inline]
pub const fn parity(subject: u64, predicate: u64, object: u64, context: u64) -> u64 {
    subject ^ predicate ^ object ^ context
}
#[inline]
pub fn sealed(mut q: NQuin) -> NQuin {
    q.parity = parity(q.subject, q.predicate, q.object, q.context);
    q
}
#[inline]
pub fn parity_valid(q: &NQuin) -> bool {
    q.parity == parity(q.subject, q.predicate, q.object, q.context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_regions_do_not_collide() {
        assert_eq!(OPCODE_MASK & PATH_MASK, 0);
        assert_eq!(OPCODE_MASK & DEFEATER_BIT, 0);
        assert_eq!(PATH_MASK & DEFEATER_BIT, 0);
        assert_eq!(OPCODE_MASK | PATH_MASK | DEFEATER_BIT, u64::MAX);
    }

    #[test]
    fn object_datatype_tags_are_distinct() {
        let tags = [
            INLINE_TAG_INTEGER, INLINE_TAG_DECIMAL, INLINE_TAG_BOOLEAN,
            INLINE_TAG_BLOB, INLINE_TAG_FLOAT,
        ];
        for (i, &a) in tags.iter().enumerate() {
            assert_eq!(a & !INLINE_TAG_MASK, 0, "tag {i} outside the [60..62] tag region");
            for &b in &tags[i + 1..] {
                assert_ne!(a, b, "two object datatype tags collide");
            }
        }
        // value region is disjoint from the tag + MSB regions.
        assert_eq!(INLINE_VALUE_MASK & INLINE_TAG_MASK, 0);
        assert_eq!(INLINE_VALUE_MASK & MSB_FLAG, 0);
    }

    #[test]
    fn low32_payload_is_disjoint_from_every_high_overlay() {
        // The ONE always-true metadata invariant (see module header): the low-32
        // payload (degree/expiry/timestamp/packed-moments) never overlaps any high
        // overlay, so it can't silently corrupt a type / flag / sensitivity / clock.
        let high = QUIN_TYPE_MASK | ROUTING_LANE_MASK | SENSITIVITY_MASK | BAKE_CLOCK_MASK;
        assert_eq!(high & LOW32_MASK, 0, "a high overlay reaches into the low-32 payload");
        // Routing is a sub-lane of the quin-type nibble (documented role-exclusive overlap).
        assert_eq!(ROUTING_LANE_MASK & QUIN_TYPE_MASK, ROUTING_LANE_MASK,
            "routing lane must sit inside the quin-type nibble [60..63]");
    }

    #[test]
    fn modality_flag_bits_are_pairwise_distinct() {
        let flags = [
            COUNTERFACTUAL_BIT, DO_INTERVENTION_BIT, SYNTHESIZED_BIT, CONSUMED_BIT,
            STABILIZATION_BIT, FEEDBACK_BIT, CONTROL_BIT, DEFENSE_BIT, ATTACK_BIT, ARGUMENT_BIT,
        ];
        for (i, &a) in flags.iter().enumerate() {
            assert_eq!(a.count_ones(), 1, "flag {i} is not a single bit");
            for &b in &flags[i + 1..] {
                assert_ne!(a, b, "two modality flag bits collide");
            }
        }
        // Flags are disjoint from the general fields that co-exist on any quin and
        // from the typed payload. (They MAY share bits with the ODRL sensitivity
        // tier [56..59] — that is a quin-type-exclusive overlap, not a same-quin
        // collision, and is documented in the module header.)
        let all_flags = flags.iter().fold(0u64, |acc, &f| acc | f);
        assert_eq!(all_flags & ROUTING_LANE_MASK, 0, "a flag overlaps routing");
        assert_eq!(all_flags & QUIN_TYPE_MASK, 0, "a flag overlaps quin_type");
        assert_eq!(all_flags & LOW32_MASK, 0, "a flag overlaps the low-32 payload");
    }

    #[test]
    fn predicate_and_degree_round_trip() {
        let path = crate::q_hash("q42:disclose");
        let p = pack_predicate(0x12, path, true);
        assert_eq!(opcode(p), 0x12);
        assert!(is_defeater(p));
        for d in [0.0f32, 0.25, 0.8, 1.0] {
            assert!((truth_degree(with_truth_degree(d)) - d).abs() < 1e-6);
        }
        assert!((unpack_float_object(pack_float_object(3.5)) - 3.5).abs() < 1e-6);
        assert_eq!(object_tag(pack_float_object(3.5)), INLINE_TAG_FLOAT);
        // quin_type round-trips and leaves the low-32 payload untouched (the
        // always-true invariant). It deliberately overlays routing [61..62] — that
        // overlap is role-exclusive, not a bug, so we only check payload safety.
        let m = with_quin_type(12345, 0b1010);
        assert_eq!(quin_type(m), 0b1010);
        assert_eq!(m & LOW32_MASK, 12345, "quin_type must not disturb the low-32 payload");
    }

    #[test]
    fn matches_deontic_packing() {
        use crate::modalities::logic::deontic::{compile_norm_quin, OP_FORBID, OP_PERMIT};
        let path = crate::q_hash("q42:x");
        let norm = compile_norm_quin(1, OP_FORBID, path, 2, 3, 0, false);
        assert_eq!(norm.predicate, pack_predicate(OP_FORBID, path, false));
        let defeater = compile_norm_quin(1, OP_PERMIT, path, 2, 3, 0, true);
        assert!(is_defeater(defeater.predicate));
    }

    #[test]
    fn parity_ignores_computational_metadata() {
        let q = sealed(NQuin { subject: 7, predicate: 8, object: 9, context: 10, metadata: 0, parity: 0 });
        assert!(parity_valid(&q));
        let mut q2 = q;
        q2.metadata = QUIN_TYPE_MASK | ROUTING_LANE_MASK | 999;
        assert!(parity_valid(&q2), "parity must ignore the computational-support field");
    }
}
