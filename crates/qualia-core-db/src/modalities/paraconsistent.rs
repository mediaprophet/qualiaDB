use crate::{NQuin, q_hash};

pub const OP_ISOLATE: u8 = 0x30;
pub const OP_CONTRADICTION_SCORE: u8 = 0x31;
pub const OP_PARACONSISTENT_MERGE: u8 = 0x32;

pub const ISOLATED_CONTEXT_PREFIX: u64 = q_hash("q42:isolated");

#[derive(Debug)]
pub enum ParaconsistentError {
    BufferOverflow,
}

pub enum ContradictionStatus {
    Consistent,
    Isolated { severity: u8, isolation_context: u64 },
}

/// Routes Quins into consistent and isolated (contradictory) sub-contexts
pub fn route_paraconsistent(
    quins: &[NQuin],
    out_consistent: &mut [NQuin],
    out_isolated: &mut [NQuin],
) -> Result<(usize, usize), ParaconsistentError> {
    let mut consistent_count = 0;
    let mut isolated_count = 0;

    for q in quins {
        // If it's already isolated, pass it through to consistent to avoid recursive isolation
        // (Wait, the requirements say: "Already-isolated Quin passes through without re-isolation")
        if q.context == ISOLATED_CONTEXT_PREFIX {
            if consistent_count >= out_consistent.len() {
                return Err(ParaconsistentError::BufferOverflow);
            }
            out_consistent[consistent_count] = *q;
            consistent_count += 1;
            continue;
        }

        let mut is_contradiction = false;
        
        // Contradiction rule: same subject + predicate, different object
        for i in 0..consistent_count {
            let prev = &out_consistent[i];
            if prev.context == q.context && prev.subject == q.subject && prev.predicate == q.predicate && prev.object != q.object {
                is_contradiction = true;
                break;
            }
        }

        if is_contradiction {
            if isolated_count >= out_isolated.len() {
                return Err(ParaconsistentError::BufferOverflow);
            }
            let mut isolated_q = *q;
            isolated_q.context = ISOLATED_CONTEXT_PREFIX ^ q.context;
            isolated_q.parity = isolated_q.subject ^ isolated_q.predicate ^ isolated_q.object ^ isolated_q.context;
            out_isolated[isolated_count] = isolated_q;
            isolated_count += 1;
        } else {
            if consistent_count >= out_consistent.len() {
                return Err(ParaconsistentError::BufferOverflow);
            }
            out_consistent[consistent_count] = *q;
            consistent_count += 1;
        }
    }

    Ok((consistent_count, isolated_count))
}

// ─── Belnap's four-valued logic (FOUR) ──────────────────────────────────────────────
//
// The basis of inconsistency-tolerant reasoning: a proposition is tracked by two
// INDEPENDENT evidence bits — "told true" and "told false" — yielding four values:
//   Neither (no info) · True · False · Both (a contained contradiction — no explosion).
// Conjunction/disjunction are the meet/join in the truth order
//   False ≤t Neither ≤t True  and  False ≤t Both ≤t True,
// computed componentwise on the evidence bits (the standard Belnap tables).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Belnap {
    /// No information either way.
    Neither,
    /// Told true only.
    True,
    /// Told false only.
    False,
    /// Told both true and false — a contradiction, contained rather than exploding.
    Both,
}

impl Belnap {
    /// Assign a Belnap value from two independent evidence flags.
    #[inline]
    pub fn from_evidence(told_true: bool, told_false: bool) -> Belnap {
        match (told_true, told_false) {
            (false, false) => Belnap::Neither,
            (true, false) => Belnap::True,
            (false, true) => Belnap::False,
            (true, true) => Belnap::Both,
        }
    }

    /// Decompose into `(told_true, told_false)`.
    #[inline]
    pub fn evidence(self) -> (bool, bool) {
        match self {
            Belnap::Neither => (false, false),
            Belnap::True => (true, false),
            Belnap::False => (false, true),
            Belnap::Both => (true, true),
        }
    }

    /// Is this value a contained contradiction?
    #[inline]
    pub fn is_contradiction(self) -> bool {
        matches!(self, Belnap::Both)
    }

    /// Belnap negation: swap the true/false evidence (Both and Neither are fixed points).
    #[inline]
    pub fn negate(self) -> Belnap {
        let (t, f) = self.evidence();
        Belnap::from_evidence(f, t)
    }

    /// Conjunction (∧) — truth-order meet: told_true ∧, told_false ∨.
    #[inline]
    pub fn and(self, other: Belnap) -> Belnap {
        let (at, af) = self.evidence();
        let (bt, bf) = other.evidence();
        Belnap::from_evidence(at && bt, af || bf)
    }

    /// Disjunction (∨) — truth-order join: told_true ∨, told_false ∧.
    #[inline]
    pub fn or(self, other: Belnap) -> Belnap {
        let (at, af) = self.evidence();
        let (bt, bf) = other.evidence();
        Belnap::from_evidence(at || bt, af && bf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belnap_four_valued_tables() {
        // Negation: True↔False; Both and Neither are fixed.
        assert_eq!(Belnap::True.negate(), Belnap::False);
        assert_eq!(Belnap::False.negate(), Belnap::True);
        assert_eq!(Belnap::Both.negate(), Belnap::Both);
        assert_eq!(Belnap::Neither.negate(), Belnap::Neither);

        // Conjunction (meet): True∧False = False; Both∧True = Both; Neither∧True = Neither.
        assert_eq!(Belnap::True.and(Belnap::False), Belnap::False);
        assert_eq!(Belnap::Both.and(Belnap::True), Belnap::Both);
        assert_eq!(Belnap::Neither.and(Belnap::True), Belnap::Neither);
        assert_eq!(Belnap::Both.and(Belnap::False), Belnap::False);

        // Disjunction (join): True∨False = True; Both∨False = Both; Neither∨False = Neither.
        assert_eq!(Belnap::True.or(Belnap::False), Belnap::True);
        assert_eq!(Belnap::Both.or(Belnap::False), Belnap::Both);
        assert_eq!(Belnap::Neither.or(Belnap::False), Belnap::Neither);

        // A contradiction is contained, not exploded: Both ∧ ¬Both stays in FOUR.
        assert!(Belnap::Both.is_contradiction());
        assert_eq!(Belnap::Both.and(Belnap::Both.negate()), Belnap::Both);
        assert!(!Belnap::True.is_contradiction());
    }

    #[test]
    fn test_paraconsistent_routing() {
        let mut out_c = [NQuin::default(); 10];
        let mut out_i = [NQuin::default(); 10];

        // 1. No contradictions -> all in out_consistent
        let q1 = NQuin { subject: 1, predicate: 2, object: 3, context: 42, ..Default::default() };
        let q2 = NQuin { subject: 1, predicate: 3, object: 3, context: 42, ..Default::default() };
        let (c, i) = route_paraconsistent(&[q1, q2], &mut out_c, &mut out_i).unwrap();
        assert_eq!(c, 2);
        assert_eq!(i, 0);

        // 2. Two Quins, same sub+pred, diff obj -> second isolated
        let q3 = NQuin { subject: 1, predicate: 2, object: 99, context: 42, ..Default::default() };
        let (c, i) = route_paraconsistent(&[q1, q3], &mut out_c, &mut out_i).unwrap();
        assert_eq!(c, 1);
        assert_eq!(i, 1);
        assert_eq!(out_i[0].context, ISOLATED_CONTEXT_PREFIX ^ 42);

        // 3. Three Quins: 1 normal, 2 contradicts 1, 3 normal
        let q4 = NQuin { subject: 10, predicate: 20, object: 30, context: 42, ..Default::default() };
        let (c, i) = route_paraconsistent(&[q1, q3, q4], &mut out_c, &mut out_i).unwrap();
        assert_eq!(c, 2);
        assert_eq!(i, 1);

        // 4. Already isolated Quin
        let mut q_iso = q3;
        q_iso.context = ISOLATED_CONTEXT_PREFIX; // Simplify for test
        let (c, i) = route_paraconsistent(&[q_iso], &mut out_c, &mut out_i).unwrap();
        assert_eq!(c, 1);
        assert_eq!(i, 0);
    }
}
