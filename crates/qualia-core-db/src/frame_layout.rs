//! FrameLayout — the canonical ABI for the ~6 computational-support bytes of the
//! 48-byte NQuin Frame.
//!
//! A Frame is `6 × u64 = 48 bytes`: roughly **42 bytes of semantics** (the
//! subject / property-path / object / context hashes and packed literals) plus
//! **~6 bytes of computational support** (an opcode, flags, a clock, and ECC
//! parity) — which is what makes a Frame both *data* and *executable code* in one
//! cell. Every modality MUST read and write those computational bytes through this
//! module, so the primitive stays universal: one opcode region, one defeater flag,
//! one truth-degree encoding, one clock, one parity — no two modalities may
//! silently disagree (enforced by `regions_do_not_collide`).
//!
//! ## `predicate` field (same-quin layout — collisions here are real bugs)
//! ```text
//! [63]    DEFEATER flag (q42:unless)
//! [8..62] property-path hash (the semantic ~42; deontic/epistemic pack here)
//! [0..7]  opcode byte (the deontic/epistemic modal opcode)
//! ```
//! Note: "proposition-mode" modalities (LTL, CTL, modal, abductive, causal, …)
//! use the WHOLE `predicate` as a plain `q_hash` (no opcode byte) — that is a
//! deliberate, documented mode, not a collision.
//!
//! ## `metadata` field (quin-type-dependent — one role per quin, mutually exclusive)
//! A given Frame uses exactly ONE of these readings depending on its modality:
//! - `expiry`        `[0..31]`  — deontic norm expiry (Unix-32)
//! - `truth_degree`  `[0..31]`  — IEEE-754 f32 belief/confidence/fuzzy degree
//! - `timestamp`     `[0..63]`  — metric-temporal event time / RCC-8 vertex sequence
//! These overlap by design (a deontic norm is never also a fuzzy belief); the ABI
//! gives each a typed accessor so the *encoding* is shared even though the *role*
//! is not.

use crate::NQuin;

// ── predicate: opcode | path | defeater ─────────────────────────────────────────
/// Deontic/epistemic opcode byte.
pub const OPCODE_MASK: u64 = 0xFF;
/// `q42:unless` defeater flag (predicate bit 63).
pub const DEFEATER_BIT: u64 = 1u64 << 63;
/// Property-path hash region (predicate bits [8..62]).
pub const PATH_MASK: u64 = 0x7FFF_FFFF_FFFF_FF00;

#[inline]
pub const fn opcode(predicate: u64) -> u8 {
    (predicate & OPCODE_MASK) as u8
}

/// The masked property-path region of a packed predicate (still shifted into [8..62]).
#[inline]
pub const fn path_bits(predicate: u64) -> u64 {
    predicate & PATH_MASK
}

#[inline]
pub const fn is_defeater(predicate: u64) -> bool {
    predicate & DEFEATER_BIT != 0
}

/// Pack a norm predicate: opcode in [0..7], `path_hash << 8` in [8..62], optional
/// defeater bit. Canonical — must equal `deontic::compile_norm_quin`'s packing
/// (asserted by `matches_deontic_packing`).
#[inline]
pub const fn pack_predicate(opcode: u8, path_hash: u64, defeater: bool) -> u64 {
    let d = if defeater { DEFEATER_BIT } else { 0 };
    d | ((path_hash << 8) & PATH_MASK) | (opcode as u64)
}

// ── metadata: typed, quin-type-dependent views (shared ENCODING) ────────────────
/// Low-32 mask shared by expiry / truth-degree / 32-bit timestamp.
pub const LOW32_MASK: u64 = 0xFFFF_FFFF;

/// Deontic norm expiry (Unix-32); 0 = never expires.
#[inline]
pub const fn expiry(metadata: u64) -> u32 {
    (metadata & LOW32_MASK) as u32
}

/// The CANONICAL truth/belief/confidence/fuzzy degree: an IEEE-754 f32 in the low
/// 32 bits. Replaces the divergent encodings (`metadata & 0xFFFF / 65535` in
/// core.rs vs raw f32 in fuzzy/probabilistic) with one shared reading.
#[inline]
pub fn truth_degree(metadata: u64) -> f32 {
    f32::from_bits((metadata & LOW32_MASK) as u32)
}

/// Encode a truth degree into `metadata` (clamped to a finite value).
#[inline]
pub fn with_truth_degree(d: f32) -> u64 {
    let d = if d.is_finite() { d } else { 0.0 };
    d.to_bits() as u64
}

/// A 32-bit timestamp / sequence index (metric-temporal event time, RCC-8 vertex).
#[inline]
pub const fn timestamp(metadata: u64) -> u64 {
    metadata
}

// ── parity (ECC over the four semantic fields) ──────────────────────────────────
#[inline]
pub const fn parity(subject: u64, predicate: u64, object: u64, context: u64) -> u64 {
    subject ^ predicate ^ object ^ context
}

/// Recompute and stamp the ECC parity of a Frame.
#[inline]
pub fn sealed(mut q: NQuin) -> NQuin {
    q.parity = parity(q.subject, q.predicate, q.object, q.context);
    q
}

/// True if a Frame's parity matches its semantic fields (the integrity check
/// `collect_active_quins` uses).
#[inline]
pub fn parity_valid(q: &NQuin) -> bool {
    q.parity == parity(q.subject, q.predicate, q.object, q.context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_do_not_collide() {
        // The predicate sub-fields (which co-exist in the SAME quin) must partition
        // cleanly — this is the invariant whose violation causes silent rule misses.
        assert_eq!(OPCODE_MASK & PATH_MASK, 0, "opcode and path overlap");
        assert_eq!(OPCODE_MASK & DEFEATER_BIT, 0, "opcode and defeater overlap");
        assert_eq!(PATH_MASK & DEFEATER_BIT, 0, "path and defeater overlap");
        // Their union leaves only the reserved bit [7-of-each-byte]? Check coverage:
        // opcode[0..7] + path[8..62] + defeater[63] = all 64 bits except none missing
        // except bit positions are contiguous: 0xFF | 0x7FFF…FF00 | (1<<63) == all set.
        assert_eq!(OPCODE_MASK | PATH_MASK | DEFEATER_BIT, u64::MAX);
    }

    #[test]
    fn predicate_round_trips() {
        let path = crate::q_hash("q42:disclose");
        let p = pack_predicate(0x12, path, true);
        assert_eq!(opcode(p), 0x12);
        assert!(is_defeater(p));
        // A non-defeater norm clears bit 63.
        let p2 = pack_predicate(0x10, path, false);
        assert!(!is_defeater(p2));
        assert_eq!(opcode(p2), 0x10);
    }

    #[test]
    fn truth_degree_round_trips() {
        for d in [0.0f32, 0.25, 0.5, 0.8, 1.0] {
            assert!((truth_degree(with_truth_degree(d)) - d).abs() < 1e-6, "degree {d}");
        }
        // non-finite is coerced to 0.0
        assert_eq!(truth_degree(with_truth_degree(f32::NAN)), 0.0);
    }

    #[test]
    fn parity_seals_and_validates() {
        let q = sealed(NQuin { subject: 7, predicate: 8, object: 9, context: 10, metadata: 99, parity: 0 });
        assert!(parity_valid(&q));
        // metadata is NOT part of parity (it carries the computational support).
        let mut q2 = q;
        q2.metadata = 12345;
        assert!(parity_valid(&q2), "parity must ignore the metadata/computational field");
    }

    #[test]
    fn matches_deontic_packing() {
        // The FrameLayout predicate packing MUST equal deontic::compile_norm_quin's,
        // so the two cannot drift. (This is the no-divergence enforcement.)
        use crate::modalities::logic::deontic::{compile_norm_quin, OP_FORBID, OP_PERMIT};
        let path = crate::q_hash("q42:x");
        let norm = compile_norm_quin(1, OP_FORBID, path, 2, 3, 0, false);
        assert_eq!(norm.predicate, pack_predicate(OP_FORBID, path, false));
        assert_eq!(opcode(norm.predicate), OP_FORBID);

        let defeater = compile_norm_quin(1, OP_PERMIT, path, 2, 3, 0, true);
        assert_eq!(defeater.predicate, pack_predicate(OP_PERMIT, path, true));
        assert!(is_defeater(defeater.predicate));
    }
}
