//! Hohfeldian jural relations (Phase 2, DEONTIC_LOGIC_PLAN §6).
//!
//! Wesley Newcomb Hohfeld decomposed the ambiguous word "right" into eight strict
//! fundamental positions arranged as **correlatives** and **opposites**. This is the
//! computational bridge for legal ontologies: a vague "right" becomes a precise position
//! held by one agent *toward another*, with a **necessary correlative** the counterparty
//! must hold. A Claim with no correlative Duty-bearer is therefore a *legible structural
//! gap*, not silence ([`find_unmet_correlatives`]).
//!
//! First-order (rules of conduct):  Claim↔Duty,    Privilege↔No-Right
//! Second-order (rules of control): Power↔Liability, Immunity↔Disability
//! Opposites:  Claim/No-Right · Duty/Privilege · Power/Disability · Immunity/Liability
//!
//! ## NQuin encoding
//! A jural relation "`holder` holds `position` over `content` toward `counterparty` in
//! `frame`" packs as: `subject = holder`, `object = counterparty`, `context = frame`,
//! `predicate = (content << 8) | position` (low byte = position opcode, bits [8..62] =
//! content path — same convention as `deontic.rs`). Zero-heap throughout.

use crate::agent::A_NATURAL_PERSON;
use crate::NQuin;

// ─── The eight positions (opcode block 0x30–0x37; distinct from deontic 0x10–0x1F
//     and epistemic 0x20–0x26) ──────────────────────────────────────────────────
pub const JURAL_CLAIM: u8 = 0x30;
pub const JURAL_DUTY: u8 = 0x31;
pub const JURAL_PRIVILEGE: u8 = 0x32;
pub const JURAL_NO_RIGHT: u8 = 0x33;
pub const JURAL_POWER: u8 = 0x34;
pub const JURAL_LIABILITY: u8 = 0x35;
pub const JURAL_IMMUNITY: u8 = 0x36;
pub const JURAL_DISABILITY: u8 = 0x37;

/// Content-path mask: bits [8..62] of the predicate (opcode byte + bit 63 excluded).
const CONTENT_MASK: u64 = 0x7FFF_FFFF_FFFF_FF00;

/// The **correlative** position the counterparty necessarily holds. If A holds `pos`
/// toward B over content φ, then B holds `correlative(pos)` toward A over φ.
pub const fn correlative(pos: u8) -> u8 {
    match pos {
        JURAL_CLAIM => JURAL_DUTY,
        JURAL_DUTY => JURAL_CLAIM,
        JURAL_PRIVILEGE => JURAL_NO_RIGHT,
        JURAL_NO_RIGHT => JURAL_PRIVILEGE,
        JURAL_POWER => JURAL_LIABILITY,
        JURAL_LIABILITY => JURAL_POWER,
        JURAL_IMMUNITY => JURAL_DISABILITY,
        JURAL_DISABILITY => JURAL_IMMUNITY,
        other => other,
    }
}

/// The **jural opposite** (the position whose presence negates this one for the holder).
pub const fn jural_opposite(pos: u8) -> u8 {
    match pos {
        JURAL_CLAIM => JURAL_NO_RIGHT,
        JURAL_NO_RIGHT => JURAL_CLAIM,
        JURAL_DUTY => JURAL_PRIVILEGE,
        JURAL_PRIVILEGE => JURAL_DUTY,
        JURAL_POWER => JURAL_DISABILITY,
        JURAL_DISABILITY => JURAL_POWER,
        JURAL_IMMUNITY => JURAL_LIABILITY,
        JURAL_LIABILITY => JURAL_IMMUNITY,
        other => other,
    }
}

/// Is `pos` one of the eight jural positions?
#[inline]
pub const fn is_jural_position(pos: u8) -> bool {
    matches!(
        pos,
        JURAL_CLAIM | JURAL_DUTY | JURAL_PRIVILEGE | JURAL_NO_RIGHT
            | JURAL_POWER | JURAL_LIABILITY | JURAL_IMMUNITY | JURAL_DISABILITY
    )
}

/// First-order positions are rules of *conduct*; second-order are rules of *control*.
#[inline]
pub const fn is_first_order(pos: u8) -> bool {
    matches!(pos, JURAL_CLAIM | JURAL_DUTY | JURAL_PRIVILEGE | JURAL_NO_RIGHT)
}

/// Readable name for a jural position.
pub const fn position_name(pos: u8) -> Option<&'static str> {
    Some(match pos {
        JURAL_CLAIM => "Claim",
        JURAL_DUTY => "Duty",
        JURAL_PRIVILEGE => "Privilege",
        JURAL_NO_RIGHT => "No-Right",
        JURAL_POWER => "Power",
        JURAL_LIABILITY => "Liability",
        JURAL_IMMUNITY => "Immunity",
        JURAL_DISABILITY => "Disability",
        _ => return None,
    })
}

#[inline]
pub const fn jural_position(predicate: u64) -> u8 {
    (predicate & 0xFF) as u8
}

#[inline]
pub const fn jural_content(predicate: u64) -> u64 {
    predicate & CONTENT_MASK
}

/// Build a jural-relation Quin: `holder` holds `position` over `content` toward
/// `counterparty` within `frame`.
pub fn compile_jural_quin(
    holder: u64,
    position: u8,
    content_path: u64,
    counterparty: u64,
    frame: u64,
) -> NQuin {
    let predicate = ((content_path << 8) & CONTENT_MASK) | (position as u64);
    let parity = holder ^ predicate ^ counterparty ^ frame;
    NQuin {
        subject: holder,
        predicate,
        object: counterparty,
        context: frame,
        metadata: 0,
        parity,
    }
}

/// The relation the counterparty NECESSARILY holds: swap holder/counterparty and map the
/// position to its correlative, keeping the same content and frame.
pub fn correlative_quin(rel: &NQuin) -> NQuin {
    let pos = jural_position(rel.predicate);
    let predicate = jural_content(rel.predicate) | (correlative(pos) as u64);
    let holder = rel.object; // counterparty becomes holder of the correlative
    let counterparty = rel.subject;
    let frame = rel.context;
    let parity = holder ^ predicate ^ counterparty ^ frame;
    NQuin {
        subject: holder,
        predicate,
        object: counterparty,
        context: frame,
        metadata: 0,
        parity,
    }
}

/// Does `graph` already contain the necessary correlative of `rel`?
pub fn jural_correlativity_holds(rel: &NQuin, graph: &[NQuin]) -> bool {
    let expected = correlative_quin(rel);
    graph.iter().any(|q| {
        q.subject == expected.subject
            && q.object == expected.object
            && q.predicate == expected.predicate
            && q.context == expected.context
    })
}

/// "Make the absence legible." For every jural relation whose correlative is NOT present
/// in `rels`, emit the *expected* (missing) correlative into `out` — e.g. a Claim to a
/// resource with no funded Duty-bearer surfaces the duty that ought to exist. Returns the
/// count written. Zero-heap (caller-supplied `out`).
pub fn find_unmet_correlatives(rels: &[NQuin], out: &mut [NQuin]) -> usize {
    let mut n = 0usize;
    for rel in rels {
        if !is_jural_position(jural_position(rel.predicate)) {
            continue;
        }
        if !jural_correlativity_holds(rel, rels) {
            if n >= out.len() {
                break;
            }
            out[n] = correlative_quin(rel);
            n += 1;
        }
    }
    n
}

/// Personhood **category-error** guard (composes `dl::check_subsumption_quin`): a benefit
/// position (Claim / Privilege / Immunity) over a content that is exclusive to natural
/// persons (e.g. `values:inherentDignity`) is a category error when the holder is **not**
/// subsumed by `values:NaturalPerson` — i.e. a corporate/legal/artificial person asserting
/// a human-only right. Returns `true` when the error fires.
///
/// * `holder_class` — the holder's declared `rdf:type` class hash (see `agent::agent_type`).
/// * `content_is_np_exclusive` — whether the claimed content is natural-person-only.
/// * `position` — the jural position asserted.
/// * `tbox` — `rdfs:subClassOf` Quins for the subsumption check.
pub fn personhood_category_error(
    holder_class: u64,
    content_is_np_exclusive: bool,
    position: u8,
    tbox: &[NQuin],
) -> bool {
    if !content_is_np_exclusive {
        return false;
    }
    // Only benefit-holding positions over a human-only content can be a category error.
    if !matches!(position, JURAL_CLAIM | JURAL_PRIVILEGE | JURAL_IMMUNITY) {
        return false;
    }
    // Error iff the holder is NOT a NaturalPerson (nor a subclass of one).
    !crate::modalities::dl::check_subsumption_quin(holder_class, A_NATURAL_PERSON, tbox)
}

// ─── Multi-party jural chains (A's Power over B's Duty to C) ─────────────────────────

/// Second-order **control** positions (Power/Liability/Immunity/Disability) are the only ones
/// that can govern — alter, or be immune to alteration of — another agent's relations.
#[inline]
pub const fn is_second_order(pos: u8) -> bool {
    matches!(pos, JURAL_POWER | JURAL_LIABILITY | JURAL_IMMUNITY | JURAL_DISABILITY)
}

/// A multi-party chain link: a second-order control relation `upstream` (held by A toward B)
/// governs `downstream` (a relation held by B toward C). Valid iff `upstream` is a
/// second-order position AND the pivot matches — A's counterparty (`upstream.object`) is the
/// holder of the downstream relation (`downstream.subject`). Models "A has Power over B's
/// Duty to C".
pub fn jural_chain_links(upstream: &NQuin, downstream: &NQuin) -> bool {
    is_second_order(jural_position(upstream.predicate))
        && is_jural_position(jural_position(downstream.predicate))
        && upstream.object == downstream.subject
}

/// The pivot party B of a valid chain link (`upstream.object == downstream.subject`), else `None`.
pub fn jural_chain_pivot(upstream: &NQuin, downstream: &NQuin) -> Option<u64> {
    if jural_chain_links(upstream, downstream) {
        Some(upstream.object)
    } else {
        None
    }
}

/// Confirm an ordered chain A→B→C→… is fully connected: every adjacent pair links. A single
/// relation (or empty) is trivially valid. Zero-heap (slice windows, no allocation).
pub fn jural_chain_valid(rels: &[NQuin]) -> bool {
    rels.windows(2).all(|w| jural_chain_links(&w[0], &w[1]))
}

// ─── Rights-collision conflict resolution ───────────────────────────────────────────

/// The outcome of resolving a collision between two jural relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionResolution {
    /// `a` prevails — it is grounded in a non-derogable human right and `b` is not.
    FirstPrevails,
    /// `b` prevails — it is grounded in a non-derogable human right and `a` is not.
    SecondPrevails,
    /// A genuine proportionality conflict (both, or neither, non-derogable). Never
    /// auto-flattened — routed to human review per the Curation Directive.
    RequiresHumanReview,
    /// The inputs do not actually collide.
    NoCollision,
}

/// Two jural relations **collide** iff the same holder is assigned a position and its jural
/// *opposite* over the same content within the same frame (e.g. a Duty to φ and a Privilege
/// not to do φ) — a direct contradiction in that holder's normative position.
pub fn jural_collision(a: &NQuin, b: &NQuin) -> bool {
    a.subject == b.subject
        && a.context == b.context
        && is_jural_position(jural_position(a.predicate))
        && jural_content(a.predicate) == jural_content(b.predicate)
        && jural_opposite(jural_position(a.predicate)) == jural_position(b.predicate)
}

/// Resolve a rights collision. `a_nonderogable` / `b_nonderogable` mark whether each relation
/// is grounded in a **non-derogable** human-rights instrument (the ingest non-derogable-set).
/// A non-derogable right defeats a derogable counterpart; two non-derogable (or two derogable)
/// positions in genuine conflict are **never auto-flattened** — they route to human review.
/// The engine proposes; the human disposes.
pub fn resolve_collision(
    a: &NQuin,
    b: &NQuin,
    a_nonderogable: bool,
    b_nonderogable: bool,
) -> CollisionResolution {
    if !jural_collision(a, b) {
        return CollisionResolution::NoCollision;
    }
    match (a_nonderogable, b_nonderogable) {
        (true, false) => CollisionResolution::FirstPrevails,
        (false, true) => CollisionResolution::SecondPrevails,
        _ => CollisionResolution::RequiresHumanReview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn correlatives_are_involutive_and_paired() {
        for &p in &[
            JURAL_CLAIM, JURAL_DUTY, JURAL_PRIVILEGE, JURAL_NO_RIGHT,
            JURAL_POWER, JURAL_LIABILITY, JURAL_IMMUNITY, JURAL_DISABILITY,
        ] {
            // correlative of correlative is the original (involution).
            assert_eq!(correlative(correlative(p)), p, "{:?}", position_name(p));
            assert_eq!(jural_opposite(jural_opposite(p)), p);
            // correlative and opposite are themselves distinct from the position.
            assert_ne!(correlative(p), p);
            assert_ne!(jural_opposite(p), p);
        }
        assert_eq!(correlative(JURAL_CLAIM), JURAL_DUTY);
        assert_eq!(correlative(JURAL_POWER), JURAL_LIABILITY);
        assert_eq!(correlative(JURAL_IMMUNITY), JURAL_DISABILITY);
        assert_eq!(jural_opposite(JURAL_CLAIM), JURAL_NO_RIGHT);
    }

    #[test]
    fn claim_implies_correlative_duty() {
        let (alice, bob, frame) = (q_hash("alice"), q_hash("bob"), q_hash("nda"));
        let content = q_hash("q42:repayLoan");
        // Alice holds a Claim toward Bob that he repay.
        let claim = compile_jural_quin(alice, JURAL_CLAIM, content, bob, frame);
        // The correlative is: Bob holds a Duty toward Alice to repay.
        let duty = correlative_quin(&claim);
        assert_eq!(jural_position(duty.predicate), JURAL_DUTY);
        assert_eq!(duty.subject, bob);
        assert_eq!(duty.object, alice);
        assert_eq!(jural_content(duty.predicate), jural_content(claim.predicate));
        assert_eq!(duty.context, frame);
        // Parity is a valid XOR fold.
        assert_eq!(duty.parity, duty.subject ^ duty.predicate ^ duty.object ^ duty.context);
    }

    #[test]
    fn correlativity_holds_when_duty_is_present() {
        let (alice, bob, frame) = (q_hash("alice"), q_hash("bob"), q_hash("nda"));
        let content = q_hash("q42:repayLoan");
        let claim = compile_jural_quin(alice, JURAL_CLAIM, content, bob, frame);
        let duty = correlative_quin(&claim);
        // Graph with the duty present → correlativity holds.
        assert!(jural_correlativity_holds(&claim, &[claim, duty]));
        // Graph without it → does not hold.
        assert!(!jural_correlativity_holds(&claim, &[claim]));
    }

    #[test]
    fn unmet_correlative_duty_is_made_legible() {
        let (alice, state, frame) = (q_hash("alice"), q_hash("state"), q_hash("icescr"));
        let content = q_hash("q42:adequateHousing");
        // Alice holds a Claim to housing toward the State — but no Duty is recorded.
        let claim = compile_jural_quin(alice, JURAL_CLAIM, content, state, frame);
        let mut out = [NQuin::default(); 4];
        let n = find_unmet_correlatives(&[claim], &mut out);
        assert_eq!(n, 1, "the missing duty must be surfaced");
        assert_eq!(jural_position(out[0].predicate), JURAL_DUTY);
        assert_eq!(out[0].subject, state, "the State is the would-be duty-bearer");
        assert_eq!(out[0].object, alice);

        // Once the duty exists, nothing is unmet.
        let duty = correlative_quin(&claim);
        assert_eq!(find_unmet_correlatives(&[claim, duty], &mut out), 0);
    }

    #[test]
    fn corporate_person_claiming_human_only_right_is_category_error() {
        // TBox: CorporatePerson ⊑ LegalPerson; NaturalPerson ⊑ Agent (disjoint branches).
        let np = A_NATURAL_PERSON;
        let legal = q_hash("https://ns.webcivics.net/values/LegalPerson");
        let corp = q_hash("https://ns.webcivics.net/values/CorporatePerson");
        let agent = q_hash("https://ns.webcivics.net/values/Agent");
        let sub = q_hash("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let e = |s: u64, o: u64| NQuin { subject: s, predicate: sub, object: o, context: 0, metadata: 0, parity: 0 };
        let tbox = [e(corp, legal), e(legal, agent), e(np, agent)];

        // A CorporatePerson asserting a Claim to a human-only right → category error.
        assert!(personhood_category_error(corp, true, JURAL_CLAIM, &tbox));
        // A NaturalPerson asserting the same → fine.
        assert!(!personhood_category_error(np, true, JURAL_CLAIM, &tbox));
        // A CorporatePerson over a NON-exclusive content → not an error.
        assert!(!personhood_category_error(corp, false, JURAL_CLAIM, &tbox));
        // A Duty (burden, not benefit) borne by a CorporatePerson → never a category error.
        assert!(!personhood_category_error(corp, true, JURAL_DUTY, &tbox));
    }

    #[test]
    fn multi_party_chain_a_power_over_b_duty_to_c() {
        let (a, b, c, frame) = (q_hash("A"), q_hash("B"), q_hash("C"), q_hash("frame"));
        let content = q_hash("q42:performService");
        // A holds a Power toward B; B holds a Duty toward C.
        let a_power = compile_jural_quin(a, JURAL_POWER, content, b, frame);
        let b_duty = compile_jural_quin(b, JURAL_DUTY, content, c, frame);
        assert!(jural_chain_links(&a_power, &b_duty), "A's power over B governs B's duty to C");
        assert_eq!(jural_chain_pivot(&a_power, &b_duty), Some(b), "the pivot is B");
        assert!(jural_chain_valid(&[a_power, b_duty]));

        // A first-order Claim cannot be the governing upstream (not a control position).
        let a_claim = compile_jural_quin(a, JURAL_CLAIM, content, b, frame);
        assert!(!jural_chain_links(&a_claim, &b_duty));
        // Broken pivot: B's duty is toward C, but the downstream is held by someone else.
        let x_duty = compile_jural_quin(q_hash("X"), JURAL_DUTY, content, c, frame);
        assert!(!jural_chain_links(&a_power, &x_duty));
        assert_eq!(jural_chain_pivot(&a_power, &x_duty), None);
    }

    #[test]
    fn colliding_rights_resolve_by_non_derogability_else_human_review() {
        let (holder, cp, frame) = (q_hash("holder"), q_hash("counter"), q_hash("frame"));
        let content = q_hash("q42:speak");
        // Holder has a Duty to φ AND (from another source) a Privilege not to do φ — opposites.
        let duty = compile_jural_quin(holder, JURAL_DUTY, content, cp, frame);
        let privilege = compile_jural_quin(holder, JURAL_PRIVILEGE, content, cp, frame);
        assert!(jural_collision(&duty, &privilege), "Duty vs Privilege over same content = collision");

        // The non-derogable right prevails over the derogable one.
        assert_eq!(resolve_collision(&duty, &privilege, true, false), CollisionResolution::FirstPrevails);
        assert_eq!(resolve_collision(&duty, &privilege, false, true), CollisionResolution::SecondPrevails);
        // Both (or neither) non-derogable → genuine proportionality conflict → human review, never flattened.
        assert_eq!(resolve_collision(&duty, &privilege, true, true), CollisionResolution::RequiresHumanReview);
        assert_eq!(resolve_collision(&duty, &privilege, false, false), CollisionResolution::RequiresHumanReview);

        // No collision when positions are not opposites (Duty vs Duty).
        let duty2 = compile_jural_quin(holder, JURAL_DUTY, content, cp, frame);
        assert!(!jural_collision(&duty, &duty2));
        assert_eq!(resolve_collision(&duty, &duty2, true, false), CollisionResolution::NoCollision);
    }
}
