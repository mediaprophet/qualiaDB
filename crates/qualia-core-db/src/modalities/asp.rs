use crate::NQuin;

pub const MAX_STABLE_MODELS: usize = 8;

/// Returns number of stable models found (max MAX_STABLE_MODELS = 8)
/// Worlds are encoded as context-hash variants: world_i_context = base_context ^ (i as u64)
pub fn enumerate_stable_models(
    base: &NQuin,
    rules: &[NQuin],
    out_worlds: &mut [u64; MAX_STABLE_MODELS],
) -> usize {
    if rules.is_empty() {
        out_worlds[0] = base.context;
        return 1;
    }

    let mut num_worlds = 1;
    out_worlds[0] = base.context;

    // For each rule, we bifurcate the context simulating applying vs not applying the rule,
    // up to the maximum number of supported stable models.
    for rule in rules.iter().take(3) { // 2^3 = 8
        let current_worlds = num_worlds;
        for w in 0..current_worlds {
            if num_worlds < MAX_STABLE_MODELS {
                // Bifurcate by XORing the rule's hash components into the context
                out_worlds[num_worlds] = out_worlds[w] ^ rule.subject ^ rule.object;
                num_worlds += 1;
            }
        }
    }
    
    num_worlds
}

// ─── True stable-model (answer-set) semantics — Gelfond-Lifschitz ───────────────
//
// The function above is a legacy context-bifurcation heuristic kept for its callers.
// `compute_answer_sets` is the REAL thing: stable models of a normal logic program
// (`head :- p1..pk, not n1..nm`, plus integrity constraints `:- body`) under the
// Gelfond-Lifschitz reduct. Bounded + zero-heap: atoms are indexed into a u64 bitmask,
// candidate models are brute-forced over 2^|atoms|, each reduced + least-fixpoint'd +
// checked for stability. Correct (not heuristic): an under-determined norm
// ("permitted :- not forbidden; forbidden :- not permitted") yields its TWO consistent
// answer sets; a constraint prunes them.

pub const ASP_MAX_ATOMS: usize = 12; // 2^12 = 4096 candidate interpretations
pub const ASP_MAX_BODY: usize = 6;

/// A normal ASP rule `head :- pos.., not neg..`. `head == 0` encodes an integrity
/// constraint `:- pos.., not neg..` (admits no atom; prunes models satisfying the body).
#[derive(Clone, Copy)]
pub struct AspRule {
    pub head: u64,
    pub pos: [u64; ASP_MAX_BODY],
    pub pos_len: usize,
    pub neg: [u64; ASP_MAX_BODY],
    pub neg_len: usize,
}

impl AspRule {
    pub fn new(head: u64, pos: &[u64], neg: &[u64]) -> Self {
        let mut r = AspRule { head, pos: [0; ASP_MAX_BODY], pos_len: 0, neg: [0; ASP_MAX_BODY], neg_len: 0 };
        for &a in pos.iter().take(ASP_MAX_BODY) { r.pos[r.pos_len] = a; r.pos_len += 1; }
        for &a in neg.iter().take(ASP_MAX_BODY) { r.neg[r.neg_len] = a; r.neg_len += 1; }
        r
    }
    pub fn fact(head: u64) -> Self { Self::new(head, &[], &[]) }
    pub fn constraint(pos: &[u64], neg: &[u64]) -> Self { Self::new(0, pos, neg) }
}

/// Compute the stable models (answer sets) of `rules` over `atoms`. Each answer set is
/// written to `out` as a bitmask over atom indices (bit i ⇔ `atoms[i]` is in the set).
/// Returns the number found. Zero-heap; bounded to `ASP_MAX_ATOMS` atoms.
pub fn compute_answer_sets(atoms: &[u64], rules: &[AspRule], out: &mut [u64]) -> usize {
    let n = atoms.len().min(ASP_MAX_ATOMS);
    let idx = |a: u64| -> Option<usize> { atoms[..n].iter().position(|&x| x == a) };
    let removed_by_reduct = |r: &AspRule, cand: u64| -> bool {
        // GL reduct: a rule is removed iff some negative-body atom is IN the candidate.
        for &na in &r.neg[..r.neg_len] {
            if let Some(ni) = idx(na) {
                if cand & (1u64 << ni) != 0 { return true; }
            }
        }
        false
    };
    let body_pos_in = |r: &AspRule, m: u64| -> bool {
        for &pa in &r.pos[..r.pos_len] {
            match idx(pa) {
                Some(pi) if m & (1u64 << pi) != 0 => {}
                _ => return false,
            }
        }
        true
    };

    let mut found = 0usize;
    let total: u64 = 1u64 << n;
    for cand in 0..total {
        // Least model of the reduct (positive Horn) by fixpoint iteration.
        let mut m: u64 = 0;
        loop {
            let mut changed = false;
            for r in rules {
                if r.head == 0 || removed_by_reduct(r, cand) { continue; }
                if body_pos_in(r, m) {
                    if let Some(hi) = idx(r.head) {
                        if m & (1u64 << hi) == 0 { m |= 1u64 << hi; changed = true; }
                    }
                }
            }
            if !changed { break; }
        }
        // Stability: the least model of the reduct must equal the candidate …
        if m != cand { continue; }
        // … and no integrity constraint may be violated by it.
        let mut ok = true;
        for r in rules {
            if r.head != 0 || removed_by_reduct(r, cand) { continue; }
            if body_pos_in(r, m) { ok = false; break; }
        }
        if ok {
            if found >= out.len() { break; }
            out[found] = m;
            found += 1;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real stable-model semantics: an even loop has exactly TWO answer sets; a constraint prunes.
    #[test]
    fn answer_sets_even_loop_and_constraint() {
        let (p, q) = (101u64, 202u64);
        let atoms = [p, q];
        // p :- not q.   q :- not p.   → answer sets {p} and {q}.
        let prog = [AspRule::new(p, &[], &[q]), AspRule::new(q, &[], &[p])];
        let mut out = [0u64; 8];
        let k = compute_answer_sets(&atoms, &prog, &mut out);
        assert_eq!(k, 2, "even loop has exactly two stable models");
        let bp = 1u64 << 0; // p is atoms[0]
        let bq = 1u64 << 1; // q is atoms[1]
        assert!(out[..k].contains(&bp) && out[..k].contains(&bq), "the two answer sets are {{p}} and {{q}}");

        // Add `:- q` (forbid q) → only {p} survives.
        let prog2 = [AspRule::new(p, &[], &[q]), AspRule::new(q, &[], &[p]), AspRule::constraint(&[q], &[])];
        let mut out2 = [0u64; 8];
        let k2 = compute_answer_sets(&atoms, &prog2, &mut out2);
        assert_eq!(k2, 1, "the constraint prunes {{q}}");
        assert_eq!(out2[0], bp, "only {{p}} remains");
    }

    #[test]
    fn test_enumerate_stable_models() {
        let base = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 42,
            metadata: 0,
            parity: 0,
        };
        let mut out_worlds = [0; MAX_STABLE_MODELS];
        
        // Empty rules -> 1 world
        let count = enumerate_stable_models(&base, &[], &mut out_worlds);
        assert_eq!(count, 1);
        assert_eq!(out_worlds[0], 42);

        // One rule -> 2 worlds
        let rule = NQuin {
            subject: 10,
            predicate: 0,
            object: 20,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let count2 = enumerate_stable_models(&base, &[rule], &mut out_worlds);
        assert_eq!(count2, 2);
        assert_eq!(out_worlds[0], 42);
        assert_eq!(out_worlds[1], 42 ^ 10 ^ 20);
    }
}
