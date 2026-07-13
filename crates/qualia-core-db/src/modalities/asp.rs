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
    for rule in rules.iter().take(3) {
        // 2^3 = 8
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
        let mut r = AspRule {
            head,
            pos: [0; ASP_MAX_BODY],
            pos_len: 0,
            neg: [0; ASP_MAX_BODY],
            neg_len: 0,
        };
        for &a in pos.iter().take(ASP_MAX_BODY) {
            r.pos[r.pos_len] = a;
            r.pos_len += 1;
        }
        for &a in neg.iter().take(ASP_MAX_BODY) {
            r.neg[r.neg_len] = a;
            r.neg_len += 1;
        }
        r
    }
    pub fn fact(head: u64) -> Self {
        Self::new(head, &[], &[])
    }
    pub fn constraint(pos: &[u64], neg: &[u64]) -> Self {
        Self::new(0, pos, neg)
    }
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
                if cand & (1u64 << ni) != 0 {
                    return true;
                }
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
                if r.head == 0 || removed_by_reduct(r, cand) {
                    continue;
                }
                if body_pos_in(r, m) {
                    if let Some(hi) = idx(r.head) {
                        if m & (1u64 << hi) == 0 {
                            m |= 1u64 << hi;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // Stability: the least model of the reduct must equal the candidate …
        if m != cand {
            continue;
        }
        // … and no integrity constraint may be violated by it.
        let mut ok = true;
        for r in rules {
            if r.head != 0 || removed_by_reduct(r, cand) {
                continue;
            }
            if body_pos_in(r, m) {
                ok = false;
                break;
            }
        }
        if ok {
            if found >= out.len() {
                break;
            }
            out[found] = m;
            found += 1;
        }
    }
    found
}

// ─── Index helper ───────────────────────────────────────────────────────────────────

/// Index of atom `a` in `atoms` (bit position), if present.
#[inline]
pub fn atom_index(atoms: &[u64], a: u64) -> Option<usize> {
    atoms.iter().take(ASP_MAX_ATOMS).position(|&x| x == a)
}

#[inline]
fn body_holds(atoms: &[u64], model: u64, pos: &[u64], neg: &[u64]) -> bool {
    for &p in pos {
        match atom_index(atoms, p) {
            Some(i) if model & (1u64 << i) != 0 => {}
            _ => return false,
        }
    }
    for &nn in neg {
        if let Some(i) = atom_index(atoms, nn) {
            if model & (1u64 << i) != 0 {
                return false;
            }
        }
    }
    true
}

// ─── Grounding: zero-heap instantiation of a non-ground rule template ───────────────

/// Ground a rule TEMPLATE by substituting variable `var` with each element of `domain`, writing
/// the ground instances into `out`. Returns the count. Apply repeatedly (over the partially-ground
/// output) for multiple variables. Zero-heap — bounded by `out.len()` (the "millions of
/// constraints" ceiling is the caller's buffer, not a heap allocation here).
pub fn ground_rule(template: &AspRule, var: u64, domain: &[u64], out: &mut [AspRule]) -> usize {
    let subst = |a: u64, d: u64| if a == var { d } else { a };
    let mut n = 0usize;
    for &d in domain {
        if n >= out.len() {
            break;
        }
        let mut g = *template;
        g.head = subst(g.head, d);
        for i in 0..g.pos_len {
            g.pos[i] = subst(g.pos[i], d);
        }
        for i in 0..g.neg_len {
            g.neg[i] = subst(g.neg[i], d);
        }
        out[n] = g;
        n += 1;
    }
    n
}

// ─── Weak constraints & optimization (the "best" stable model) ──────────────────────

/// A weak constraint `:~ pos.., not neg.. [weight]` — incurs `weight` when its body holds in a
/// model. Optimal answer sets MINIMISE total incurred weight.
#[derive(Clone, Copy)]
pub struct WeakConstraint {
    pub pos: [u64; ASP_MAX_BODY],
    pub pos_len: usize,
    pub neg: [u64; ASP_MAX_BODY],
    pub neg_len: usize,
    pub weight: i64,
}

impl WeakConstraint {
    pub fn new(pos: &[u64], neg: &[u64], weight: i64) -> Self {
        let mut w = WeakConstraint {
            pos: [0; ASP_MAX_BODY],
            pos_len: 0,
            neg: [0; ASP_MAX_BODY],
            neg_len: 0,
            weight,
        };
        for &a in pos.iter().take(ASP_MAX_BODY) {
            w.pos[w.pos_len] = a;
            w.pos_len += 1;
        }
        for &a in neg.iter().take(ASP_MAX_BODY) {
            w.neg[w.neg_len] = a;
            w.neg_len += 1;
        }
        w
    }
}

/// Total penalty of `model` under `weak`: the sum of weights of the weak constraints whose body
/// holds in the model.
pub fn model_penalty(atoms: &[u64], model: u64, weak: &[WeakConstraint]) -> i64 {
    let mut total = 0i64;
    for w in weak {
        if body_holds(atoms, model, &w.pos[..w.pos_len], &w.neg[..w.neg_len]) {
            total += w.weight;
        }
    }
    total
}

/// The **optimal** answer set: the stable model minimising total weak-constraint penalty. Returns
/// `(model_bitmask, penalty)`, or `None` if the program has no stable model. `buf` is scratch for
/// the enumerated answer sets.
pub fn optimal_answer_set(
    atoms: &[u64],
    rules: &[AspRule],
    weak: &[WeakConstraint],
    buf: &mut [u64],
) -> Option<(u64, i64)> {
    let k = compute_answer_sets(atoms, rules, buf);
    if k == 0 {
        return None;
    }
    let mut best = (buf[0], model_penalty(atoms, buf[0], weak));
    for &m in &buf[1..k] {
        let p = model_penalty(atoms, m, weak);
        if p < best.1 {
            best = (m, p);
        }
    }
    Some(best)
}

// ─── Cautious / brave reasoning ─────────────────────────────────────────────────────

/// **Cautious** (skeptical) consequences: the atoms in EVERY answer set (bit-AND of all models).
/// `0` if there are no models.
pub fn cautious_consequences(models: &[u64]) -> u64 {
    match models.split_first() {
        Some((&first, rest)) => rest.iter().fold(first, |acc, &m| acc & m),
        None => 0,
    }
}

/// **Brave** (credulous) consequences: the atoms in SOME answer set (bit-OR of all models).
pub fn brave_consequences(models: &[u64]) -> u64 {
    models.iter().fold(0u64, |acc, &m| acc | m)
}

// ─── Paraconsistent routing: no-stable-model handling ───────────────────────────────

/// Outcome of an answer-set computation — distinguishes "no model" (an over-constrained /
/// inconsistent program) from genuine results, so the caller can route the former to
/// paraconsistent reasoning instead of treating absence-of-model as plain falsity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspOutcome {
    /// `n` stable models were written to the output buffer.
    Stable(usize),
    /// No stable model exists — the program is inconsistent; route to `paraconsistent`.
    NoStableModel,
}

/// Compute answer sets; if NONE exist, return [`AspOutcome::NoStableModel`] so the caller routes
/// the (inconsistent) program to `modalities::paraconsistent::route_paraconsistent` rather than
/// silently concluding falsity. This is the tight integration with paraconsistent routing.
pub fn answer_sets_or_paraconsistent(
    atoms: &[u64],
    rules: &[AspRule],
    out: &mut [u64],
) -> AspOutcome {
    let k = compute_answer_sets(atoms, rules, out);
    if k == 0 {
        AspOutcome::NoStableModel
    } else {
        AspOutcome::Stable(k)
    }
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
        assert!(
            out[..k].contains(&bp) && out[..k].contains(&bq),
            "the two answer sets are {{p}} and {{q}}"
        );

        // Add `:- q` (forbid q) → only {p} survives.
        let prog2 = [
            AspRule::new(p, &[], &[q]),
            AspRule::new(q, &[], &[p]),
            AspRule::constraint(&[q], &[]),
        ];
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

    #[test]
    fn grounder_instantiates_a_template_over_a_domain() {
        // Template:  node(X).   with X a variable, domain {a,b,c} → three ground facts.
        let var = crate::q_hash("var:X");
        let node = |x: u64| x; // identity: the head IS the (variable) atom node(X)≡X here
        let template = AspRule::fact(var);
        let (a, b, c) = (node(11), node(22), node(33));
        let mut out = [AspRule::fact(0); 8];
        let n = ground_rule(&template, var, &[a, b, c], &mut out);
        assert_eq!(n, 3);
        assert_eq!(out[0].head, a);
        assert_eq!(out[1].head, b);
        assert_eq!(out[2].head, c);
    }

    #[test]
    fn weak_constraints_select_the_optimal_model() {
        let (p, q) = (101u64, 202u64);
        let atoms = [p, q];
        // Even loop → {p} and {q}. Weak constraint `:~ q [1]` penalises q.
        let prog = [AspRule::new(p, &[], &[q]), AspRule::new(q, &[], &[p])];
        let weak = [WeakConstraint::new(&[q], &[], 1)];
        let mut buf = [0u64; 8];
        let (best, penalty) = optimal_answer_set(&atoms, &prog, &weak, &mut buf).unwrap();
        assert_eq!(best, 1u64 << 0, "optimal model is {{p}} (no penalty)");
        assert_eq!(penalty, 0);
        // {q} would have incurred penalty 1.
        assert_eq!(model_penalty(&atoms, 1u64 << 1, &weak), 1);
    }

    #[test]
    fn cautious_and_brave_consequences() {
        let (p, q) = (101u64, 202u64);
        let atoms = [p, q];
        let prog = [AspRule::new(p, &[], &[q]), AspRule::new(q, &[], &[p])];
        let mut buf = [0u64; 8];
        let k = compute_answer_sets(&atoms, &prog, &mut buf);
        assert_eq!(k, 2);
        // Cautious: in BOTH {p} and {q} → neither p nor q → 0. Brave: in SOME → both bits set.
        assert_eq!(cautious_consequences(&buf[..k]), 0);
        assert_eq!(brave_consequences(&buf[..k]), (1u64 << 0) | (1u64 << 1));
    }

    #[test]
    fn no_stable_model_routes_to_paraconsistent() {
        let p = 101u64;
        let atoms = [p];
        // `p :- not p` has NO stable model — an inconsistent program.
        let prog = [AspRule::new(p, &[], &[p])];
        let mut out = [0u64; 8];
        assert_eq!(
            answer_sets_or_paraconsistent(&atoms, &prog, &mut out),
            AspOutcome::NoStableModel
        );
        // A consistent program reports its model count.
        let prog2 = [AspRule::fact(p)];
        assert_eq!(
            answer_sets_or_paraconsistent(&atoms, &prog2, &mut out),
            AspOutcome::Stable(1)
        );
    }
}
