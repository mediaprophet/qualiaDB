//! Phase 3 — place / space / time binding for artefacts *(STELLAR §E step 3)*.
//!
//! An artefact is an **identifier** (`subject`); facts *about* it are companion NQuins that share
//! that subject (see [`crate::render::assets::mesh_to_nquins`], where a mesh's `subject` is the
//! `q_hash` of its asset IRI and its bbox/centroid/type are quins keyed by it). Phase 3 adds the
//! **spatio-temporal** facts — *where* (a world/geo point + a jurisdiction frame) and *when* (a
//! valid-time interval) — and demonstrates that the **same** artefact is queryable by the
//! **inherited modality stack two ways**:
//!
//! * a **spatio-temporal** query — RCC-8 place containment + Allen interval relation
//!   ([`crate::modalities::spatio_temporal`]); and
//! * a **deontic** query — a rights norm bound to the artefact's identity
//!   ([`crate::modalities::logic::deontic`]).
//!
//! One entity, one substrate, many modalities — this module is only a thin renderer-side *binding*
//! (it packs the artefact's place/time into NQuin fields and shapes a footprint polygon); all the
//! actual logic is delegated to the existing modalities. That is the Phase-3 rail: *spatio-temporal
//! logic uses the inherited modality stack, not a bespoke engine.*
//!
//! The artefact's situatedness is one NQuin:
//!
//! | field | carries |
//! |-------|---------|
//! | `subject`  | the artefact id (shared with its mesh facts and any norm about it) |
//! | `predicate`| [`P_SITUATED_AT`] semantic stamp |
//! | `object`   | the artefact's `(x, y)` location, via [`pack_point`] (shared encoding with RCC-8) |
//! | `context`  | the jurisdiction / frame id |
//! | `metadata` | the valid-time interval `[from, to]`, via [`pack_interval`] |

use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
};
use crate::modalities::spatio_temporal::{
    evaluate_rcc8_points, evaluate_temporal, pack_point, unpack_point, Rcc8Relation, TemporalOp,
};
use crate::{q_hash, NQuin};

/// Predicate stamp for an artefact's situatedness fact (place + valid-time).
pub const P_SITUATED_AT: u64 = q_hash("urn:qualia:place:situatedAt");

/// Property-path for a render/display norm (the action a deontic rule governs).
pub const P_RENDER_DISPLAY: u64 = q_hash("urn:qualia:render:display");

/// Max render norms evaluated in one [`render_permitted`] pass (stack-bounded, zero-heap).
pub const MAX_RENDER_NORMS: usize = 32;

// ── valid-time interval packing (one u64; whole seconds, i32 range) ──────────────────────────────

/// Pack a valid-time interval `[start, end]` (whole seconds) into one `u64`: `start` in the high 32
/// bits, `end` in the low 32 bits. Values are truncated to `i32` (≈ ±68 years around the epoch —
/// honest demo range; a wider encoding is a file-format-v2 concern, STELLAR §C).
#[inline]
pub fn pack_interval(start: i64, end: i64) -> u64 {
    let s = start as i32 as u32 as u64;
    let e = end as i32 as u32 as u64;
    (s << 32) | e
}

/// Inverse of [`pack_interval`] (sign-extends each 32-bit half back to `i64`).
#[inline]
pub fn unpack_interval(packed: u64) -> (i64, i64) {
    let s = (packed >> 32) as u32 as i32 as i64;
    let e = (packed & 0xFFFF_FFFF) as u32 as i32 as i64;
    (s, e)
}

// ── situatedness fact ────────────────────────────────────────────────────────────────────────────

/// Build the artefact's situatedness NQuin: the **same `subject`** as its mesh facts, carrying its
/// world/geo location (`object`) and valid-time interval (`metadata`) in a jurisdiction frame
/// (`context`). Parity is the XOR fold used throughout the engine.
pub fn situate_artefact(
    artefact_id: u64,
    x: f64,
    y: f64,
    valid_from: i64,
    valid_to: i64,
    jurisdiction_frame: u64,
) -> NQuin {
    let object = pack_point(x, y);
    let metadata = pack_interval(valid_from, valid_to);
    let mut q = NQuin {
        subject: artefact_id,
        predicate: P_SITUATED_AT,
        object,
        context: jurisdiction_frame,
        metadata,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
    q
}

/// Recover the artefact's `(x, y)` location from its situatedness NQuin.
#[inline]
pub fn artefact_location(art: &NQuin) -> (f64, f64) {
    unpack_point(art.object)
}

/// Recover the artefact's valid-time interval `[from, to]` from its situatedness NQuin.
#[inline]
pub fn artefact_interval(art: &NQuin) -> (i64, i64) {
    unpack_interval(art.metadata)
}

// ── spatio-temporal query (over the artefact NQuin) ──────────────────────────────────────────────

/// **Spatio-temporal query.** The RCC-8 topological relation of the artefact's footprint (a square
/// of half-side `radius` centred on its location) to a `jurisdiction` polygon. Delegates to
/// [`evaluate_rcc8_points`] — no bespoke geometry.
pub fn place_relation(
    art: &NQuin,
    jurisdiction_id: u64,
    jurisdiction_poly: &[(f64, f64)],
    radius: f64,
) -> Rcc8Relation {
    let (cx, cy) = artefact_location(art);
    let footprint = [
        (cx - radius, cy - radius),
        (cx + radius, cy - radius),
        (cx + radius, cy + radius),
        (cx - radius, cy + radius),
    ];
    evaluate_rcc8_points(art.subject, &footprint, jurisdiction_id, jurisdiction_poly)
}

/// True iff the artefact's footprint is **within** the jurisdiction (a proper part — tangential or
/// not — or equal). A footprint that straddles the boundary (`PartiallyOverlapping`) is *not*
/// within: a rights-bounded view does not render an artefact that pokes outside permitted space.
pub fn situated_within(
    art: &NQuin,
    jurisdiction_id: u64,
    jurisdiction_poly: &[(f64, f64)],
    radius: f64,
) -> bool {
    matches!(
        place_relation(art, jurisdiction_id, jurisdiction_poly, radius),
        Rcc8Relation::NonTangentialProperPart
            | Rcc8Relation::TangentiallyProperPart
            | Rcc8Relation::Equal
    )
}

/// **Temporal query.** The Allen relation `op` between the artefact's valid-time and a window.
/// Delegates to [`evaluate_temporal`].
pub fn time_relation(art: &NQuin, op: TemporalOp, window_start: i64, window_end: i64) -> bool {
    let (s, e) = artefact_interval(art);
    evaluate_temporal(op, s, e, window_start, window_end)
}

/// True iff the artefact's valid-time falls wholly **during** the window (Allen `During`).
pub fn active_during(art: &NQuin, window_start: i64, window_end: i64) -> bool {
    time_relation(art, TemporalOp::During, window_start, window_end)
}

// ── deontic query (over the same artefact's identity) ────────────────────────────────────────────

/// Build a rights norm **bound to the artefact's identity**: `(party) OPCODE display(artefact)` in
/// a `frame`, optionally expiring at `expiry_unix32` (`0` = no expiry). The action target
/// (`object`) is the artefact's id, so this norm and the artefact's situatedness fact share the
/// artefact identity — the deontic and spatio-temporal queries are over the *same* artefact.
pub fn render_norm(
    party: u64,
    opcode: u8,
    artefact_id: u64,
    frame: u64,
    expiry_unix32: u32,
) -> NQuin {
    compile_norm_quin(
        party,
        opcode,
        P_RENDER_DISPLAY,
        artefact_id,
        frame,
        expiry_unix32,
        false,
    )
}

/// **Deontic query.** Evaluate the render `norms` and return whether displaying this artefact is
/// permitted: it is, unless some **Active `FORBID`** norm's action targets this artefact's id.
///
/// **Fails closed:** if the norm set exceeds [`MAX_RENDER_NORMS`] or cannot be evaluated, this
/// returns `false` (deny) — a governance default, never a silent permit.
pub fn render_permitted(art: &NQuin, norms: &[NQuin], now_unix: u32) -> bool {
    if norms.len() > MAX_RENDER_NORMS {
        return false; // fail closed: cannot evaluate the whole set in the bounded buffer
    }
    let mut out = [DeonticVerdict::default(); MAX_RENDER_NORMS];
    let n = match evaluate_deontic_contract(norms, now_unix, &mut out) {
        Ok(n) => n,
        Err(_) => return false, // fail closed
    };
    let forbidden = out[..n].iter().any(|v| {
        v.opcode == OP_FORBID && v.status == DeonticStatus::Active && v.norm.object == art.subject
    });
    !forbidden
}

// ── the integrative verdict (place + time + rights over ONE NQuin) ───────────────────────────────

/// The structured outcome of querying ONE artefact NQuin across both modalities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SituatedVerdict {
    /// RCC-8 relation of the artefact footprint to the jurisdiction.
    pub place: Rcc8Relation,
    /// Footprint is within the jurisdiction (spatio-temporal).
    pub within_place: bool,
    /// Valid-time is during the window (spatio-temporal).
    pub within_time: bool,
    /// No Active FORBID targets the artefact (deontic).
    pub deontic_permits: bool,
    /// Admit render iff situated in place AND time AND deontically permitted.
    pub admit: bool,
}

/// Phase-3 acceptance in one call: given a **single artefact NQuin**, query it by the
/// spatio-temporal modality (place + time) **and** the deontic modality (render norms bound to its
/// identity), and combine. Render is admitted only when the artefact is situated within the
/// jurisdiction, active during the window, **and** not under an Active prohibition.
#[allow(clippy::too_many_arguments)]
pub fn situated_render_verdict(
    art: &NQuin,
    jurisdiction_id: u64,
    jurisdiction_poly: &[(f64, f64)],
    footprint_radius: f64,
    window_start: i64,
    window_end: i64,
    norms: &[NQuin],
    now_unix: u32,
) -> SituatedVerdict {
    let place = place_relation(art, jurisdiction_id, jurisdiction_poly, footprint_radius);
    let within_place = matches!(
        place,
        Rcc8Relation::NonTangentialProperPart
            | Rcc8Relation::TangentiallyProperPart
            | Rcc8Relation::Equal
    );
    let within_time = active_during(art, window_start, window_end);
    let deontic_permits = render_permitted(art, norms, now_unix);
    SituatedVerdict {
        place,
        within_place,
        within_time,
        deontic_permits,
        admit: within_place && within_time && deontic_permits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::OP_PERMIT;

    // A square jurisdiction [0,10]^2 and an artefact id, reused across tests.
    const JURIS_ID: u64 = 0xAB;
    fn jurisdiction() -> [(f64, f64); 4] {
        [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    }
    fn art_id() -> u64 {
        q_hash("urn:qualia:geometry:demo-cube")
    }

    #[test]
    fn interval_and_location_round_trip() {
        let (s, e) = unpack_interval(pack_interval(-1000, 2000));
        assert_eq!((s, e), (-1000, 2000));

        let art = situate_artefact(art_id(), 5.0, 5.0, 100, 200, JURIS_ID);
        let (x, y) = artefact_location(&art);
        assert!((x - 5.0).abs() < 1e-5 && (y - 5.0).abs() < 1e-5);
        assert_eq!(artefact_interval(&art), (100, 200));
        // parity is the canonical XOR fold
        assert_eq!(
            art.parity,
            art.subject ^ art.predicate ^ art.object ^ art.context ^ art.metadata
        );
    }

    #[test]
    fn spatio_temporal_query_over_artefact() {
        let poly = jurisdiction();
        // Inside the jurisdiction, footprint strictly interior → NTPP / within.
        let inside = situate_artefact(art_id(), 5.0, 5.0, 100, 200, JURIS_ID);
        assert_eq!(
            place_relation(&inside, JURIS_ID, &poly, 0.5),
            Rcc8Relation::NonTangentialProperPart
        );
        assert!(situated_within(&inside, JURIS_ID, &poly, 0.5));
        assert!(active_during(&inside, 0, 1000)); // [100,200] During [0,1000]

        // Far outside → Disconnected / not within.
        let outside = situate_artefact(art_id(), 50.0, 50.0, 100, 200, JURIS_ID);
        assert_eq!(
            place_relation(&outside, JURIS_ID, &poly, 0.5),
            Rcc8Relation::Disconnected
        );
        assert!(!situated_within(&outside, JURIS_ID, &poly, 0.5));

        // Straddling the boundary → PartiallyOverlapping / not within.
        let straddle = situate_artefact(art_id(), 9.8, 5.0, 100, 200, JURIS_ID);
        assert_eq!(
            place_relation(&straddle, JURIS_ID, &poly, 0.5),
            Rcc8Relation::PartiallyOverlapping
        );
        assert!(!situated_within(&straddle, JURIS_ID, &poly, 0.5));

        // Outside the window → not during.
        assert!(!active_during(&inside, 300, 1000)); // [100,200] is Before [300,1000]
    }

    #[test]
    fn deontic_query_over_same_artefact() {
        let art = situate_artefact(art_id(), 5.0, 5.0, 100, 200, JURIS_ID);
        let viewer = q_hash("did:example:viewer");
        let frame = q_hash("urn:qualia:frame:civic");

        // No norms → permitted (a liberty).
        assert!(render_permitted(&art, &[], 150));

        // An Active FORBID targeting THIS artefact → denied.
        let forbid = render_norm(viewer, OP_FORBID, art.subject, frame, 0);
        assert!(!render_permitted(&art, &[forbid], 150));

        // A PERMIT (no forbid) → permitted.
        let permit = render_norm(viewer, OP_PERMIT, art.subject, frame, 0);
        assert!(render_permitted(&art, &[permit], 150));

        // An EXPIRED forbid (expiry in the past) is not Active → permitted again.
        let expired = render_norm(viewer, OP_FORBID, art.subject, frame, 100);
        assert!(render_permitted(&art, &[expired], 150));

        // A forbid targeting a DIFFERENT artefact does not bind this one.
        let other = render_norm(viewer, OP_FORBID, q_hash("urn:qualia:other"), frame, 0);
        assert!(render_permitted(&art, &[other], 150));
    }

    /// PHASE-3 ACCEPTANCE: one artefact NQuin, queried by the spatio-temporal modality (place +
    /// time) AND the deontic modality (a render norm bound to its identity) — over the *same* NQuin.
    #[test]
    fn same_nquin_two_modalities() {
        let poly = jurisdiction();
        let viewer = q_hash("did:example:viewer");
        let frame = q_hash("urn:qualia:frame:civic");

        // ONE artefact NQuin, situated at (5,5) during [100,200] in the civic frame.
        let art = situate_artefact(art_id(), 5.0, 5.0, 100, 200, JURIS_ID);

        // 1) Situated in place + time, no prohibition → admitted.
        let v = situated_render_verdict(&art, JURIS_ID, &poly, 0.5, 0, 1000, &[], 150);
        assert!(v.within_place && v.within_time && v.deontic_permits);
        assert!(v.admit);

        // 2) Same place/time, but an Active deontic FORBID over the SAME artefact → refused,
        //    even though it is spatio-temporally fine. (Deontic governs the render.)
        let forbid = render_norm(viewer, OP_FORBID, art.subject, frame, 0);
        let v2 = situated_render_verdict(&art, JURIS_ID, &poly, 0.5, 0, 1000, &[forbid], 150);
        assert!(v2.within_place && v2.within_time);
        assert!(!v2.deontic_permits);
        assert!(!v2.admit);

        // 3) Deontically permitted, but OUTSIDE the jurisdiction → refused on the spatio-temporal
        //    leg. (Place governs too — a rights-bounded view won't render out of permitted space.)
        let outside = situate_artefact(art_id(), 50.0, 50.0, 100, 200, JURIS_ID);
        let v3 = situated_render_verdict(&outside, JURIS_ID, &poly, 0.5, 0, 1000, &[], 150);
        assert!(!v3.within_place);
        assert!(v3.deontic_permits);
        assert!(!v3.admit);

        // 4) In place + permitted, but OUTSIDE the time window → refused on the temporal leg.
        let v4 = situated_render_verdict(&art, JURIS_ID, &poly, 0.5, 300, 1000, &[], 150);
        assert!(v4.within_place && v4.deontic_permits);
        assert!(!v4.within_time);
        assert!(!v4.admit);
    }
}
