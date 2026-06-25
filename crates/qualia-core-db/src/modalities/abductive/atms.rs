//! Assumption-based Truth Maintenance System (de Kleer's ATMS).
//!
//! Beliefs are tracked in terms of the **assumptions** that support them. An *environment* is a
//! set of assumptions (one bit each, ≤64 assumptions → a `u64` bitset). A node's *label* is the
//! set of **minimal** environments under which it holds (no environment in a label is a subset of
//! another — minimality is what makes an ATMS efficient). A *nogood* is an inconsistent
//! environment; every superset of a nogood is also inconsistent. A node is believed in a context
//! iff the context is consistent and contains one of the node's supporting environments.
//!
//! Zero-heap: environments are `u64` bitsets; labels live in caller-supplied slices.

/// A set of assumptions — one bit per assumption (≤64). The empty environment `0` is the
/// "holds unconditionally" (premise) environment.
pub type Environment = u64;

/// Is `sub` a subset of `sup`? (every assumption in `sub` is in `sup`)
#[inline]
pub fn env_subset(sub: Environment, sup: Environment) -> bool {
    sub & sup == sub
}

/// Is `env` inconsistent given `nogoods`? True iff `env` is a superset of any nogood (a nogood's
/// assumptions are all present, so the contradiction fires).
#[inline]
pub fn is_nogood(env: Environment, nogoods: &[Environment]) -> bool {
    nogoods.iter().any(|&ng| env_subset(ng, env))
}

/// Add `env` to a label held in `label[..n]`, **maintaining minimality**: if an existing
/// environment already subsumes `env` (existing ⊆ env), `env` is redundant and is dropped; any
/// existing environments that `env` subsumes (env ⊆ existing) are removed in favour of the more
/// general `env`. Returns the new label length. Zero-heap (in-place compaction of `label`).
pub fn label_add(label: &mut [Environment], n: usize, env: Environment) -> usize {
    // Redundant if a more-general (smaller) environment is already present.
    for &e in label.iter().take(n) {
        if env_subset(e, env) {
            return n;
        }
    }
    // Drop existing environments that `env` is more general than, compacting in place.
    let mut w = 0usize;
    for i in 0..n {
        if !env_subset(env, label[i]) {
            label[w] = label[i];
            w += 1;
        }
    }
    if w < label.len() {
        label[w] = env;
        w += 1;
    }
    w
}

/// Does some environment in `label` hold under `context`? (ignoring consistency — see
/// [`holds_in`]). True iff any label environment is a subset of `context`.
#[inline]
pub fn label_holds(label: &[Environment], context: Environment) -> bool {
    label.iter().any(|&e| env_subset(e, context))
}

/// Is a node with this `label` **believed** in `context`? The context must be consistent (not a
/// superset of any nogood) AND contain one of the node's supporting environments.
pub fn holds_in(label: &[Environment], context: Environment, nogoods: &[Environment]) -> bool {
    !is_nogood(context, nogoods) && label_holds(label, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assumption bits.
    const A: Environment = 1 << 0;
    const B: Environment = 1 << 1;
    const C: Environment = 1 << 2;

    #[test]
    fn label_maintains_minimal_environments() {
        let mut label = [0u64; 8];
        let mut n = 0;
        n = label_add(&mut label, n, A | B); // {A,B}
        assert_eq!(n, 1);
        // Adding the more-general {A} removes {A,B}.
        n = label_add(&mut label, n, A);
        assert_eq!(n, 1);
        assert_eq!(label[0], A, "{{A}} subsumes {{A,B}}");
        // Adding the more-specific {A,C} is redundant (A already supports) → dropped.
        n = label_add(&mut label, n, A | C);
        assert_eq!(n, 1);
        assert_eq!(label[0], A);
        // An independent environment {B} coexists.
        n = label_add(&mut label, n, B);
        assert_eq!(n, 2);
        assert!(label[..n].contains(&A) && label[..n].contains(&B));
    }

    #[test]
    fn nogoods_are_superset_closed() {
        let nogoods = [A | B]; // {A,B} is contradictory
        assert!(is_nogood(A | B, &nogoods));
        assert!(is_nogood(A | B | C, &nogoods), "any superset of a nogood is a nogood");
        assert!(!is_nogood(A | C, &nogoods));
        assert!(!is_nogood(A, &nogoods));
    }

    #[test]
    fn belief_requires_a_consistent_supporting_context() {
        // Node supported by {A} or {B}.
        let mut label = [0u64; 4];
        let mut n = 0;
        n = label_add(&mut label, n, A);
        n = label_add(&mut label, n, B);
        let nogoods = [A | C]; // assuming A and C together is contradictory

        // Context {A}: consistent, contains supporting env {A} → believed.
        assert!(holds_in(&label[..n], A, &nogoods));
        // Context {A,C}: contains support {A} but is a nogood → NOT believed (contradiction).
        assert!(!holds_in(&label[..n], A | C, &nogoods));
        // Context {B,C}: consistent, contains support {B} → believed.
        assert!(holds_in(&label[..n], B | C, &nogoods));
        // Context {C}: consistent but contains no supporting environment → not believed.
        assert!(!holds_in(&label[..n], C, &nogoods));
    }
}
