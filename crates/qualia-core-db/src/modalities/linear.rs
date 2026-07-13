use crate::NQuin;

// Marks a Quin as consumed by setting metadata bit 59 (CONSUMED_BIT).
// Canonical bit position lives in the FrameLayout ABI (single source of truth).
pub use crate::frame_layout::CONSUMED_BIT;

pub fn consume_quin(q: &mut NQuin) {
    q.metadata |= CONSUMED_BIT;
}

pub fn is_consumed(q: &NQuin) -> bool {
    (q.metadata & CONSUMED_BIT) != 0
}

// ─── Girard's linear-logic connectives ──────────────────────────────────────────────
//
// Linear logic treats propositions as RESOURCES: each is used exactly once unless explicitly
// marked reusable with the `!` exponential. The connectives split into multiplicatives
// (⊗ tensor, ⅋ par), additives (⊕ plus, & with), their units, and the exponentials (! ?).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connective {
    /// A positive literal.
    Atom,
    /// A literal's linear negation `a⊥`.
    AtomDual,
    /// `A ⊗ B` — multiplicative conjunction ("both, together").
    Tensor,
    /// `A ⅋ B` — multiplicative disjunction.
    Par,
    /// `A ⊕ B` — additive disjunction (internal choice).
    Plus,
    /// `A & B` — additive conjunction (external choice).
    With,
    /// `1` — unit of `⊗`.
    One,
    /// `⊥` — unit of `⅋`.
    Bottom,
    /// `0` — unit of `⊕`.
    Zero,
    /// `⊤` — unit of `&`.
    Top,
    /// `!A` — exponential "of course" (reusable: weakening + contraction apply).
    OfCourse,
    /// `?A` — exponential "why not" (dual of `!`).
    WhyNot,
}

impl Connective {
    /// Linear negation `(·)⊥` — the involutive De Morgan dual (`A⊥⊥ = A`).
    pub fn dual(self) -> Connective {
        use Connective::*;
        match self {
            Atom => AtomDual,
            AtomDual => Atom,
            Tensor => Par,
            Par => Tensor,
            Plus => With,
            With => Plus,
            One => Bottom,
            Bottom => One,
            Zero => Top,
            Top => Zero,
            OfCourse => WhyNot,
            WhyNot => OfCourse,
        }
    }

    /// Multiplicative connectives/units (`⊗ ⅋ 1 ⊥`).
    pub fn is_multiplicative(self) -> bool {
        use Connective::*;
        matches!(self, Tensor | Par | One | Bottom)
    }

    /// Additive connectives/units (`⊕ & 0 ⊤`).
    pub fn is_additive(self) -> bool {
        use Connective::*;
        matches!(self, Plus | With | Zero | Top)
    }

    /// Exponential connectives (`! ?`).
    pub fn is_exponential(self) -> bool {
        matches!(self, Connective::OfCourse | Connective::WhyNot)
    }

    /// A resource under `!` ("of course") is **reusable** — structural weakening and contraction
    /// are licensed. Everything else is linear (consume-once).
    pub fn is_reusable(self) -> bool {
        matches!(self, Connective::OfCourse)
    }
}

/// Whether a resource quin `q` may be consumed to satisfy a demand: a reusable (`!`-marked)
/// resource always can; a linear resource only if not already consumed.
pub fn can_consume(q: &NQuin, reusable: bool) -> bool {
    reusable || !is_consumed(q)
}

/// `A ⊗ B` consumption: a tensor demand needs **both** operands available *together*. Consumes
/// each linear operand (leaves reusable ones); returns `false` without mutating if either is
/// already exhausted.
pub fn tensor_consume(a: &mut NQuin, a_reusable: bool, b: &mut NQuin, b_reusable: bool) -> bool {
    if !can_consume(a, a_reusable) || !can_consume(b, b_reusable) {
        return false;
    }
    if !a_reusable {
        consume_quin(a);
    }
    if !b_reusable {
        consume_quin(b);
    }
    true
}

// ─── Structural-rule discipline (weakening & contraction strictly controlled) ───────
//
// Linear logic's defining feature: the structural rules WEAKENING (discard a resource) and
// CONTRACTION (duplicate a resource) are NOT freely available — they are licensed only on
// reusable `!`-marked formulas. Exchange (reorder) is always fine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralRule {
    /// Discard a formula from the sequent.
    Weakening,
    /// Duplicate a formula in the sequent.
    Contraction,
    /// Reorder formulas.
    Exchange,
}

/// Is applying `rule` to a formula with the given reusability licensed? Exchange always; Weakening
/// and Contraction **only** on a reusable (`!`-marked) formula — applying either to a linear
/// resource is an illegal proof step (resources must be used exactly once).
pub fn structural_rule_licensed(rule: StructuralRule, reusable: bool) -> bool {
    match rule {
        StructuralRule::Exchange => true,
        StructuralRule::Weakening | StructuralRule::Contraction => reusable,
    }
}

/// Validate a whole sequence of structural-rule applications `(rule, reusable)`; every step must
/// be licensed for the derivation to be well-formed.
pub fn structural_derivation_valid(steps: &[(StructuralRule, bool)]) -> bool {
    steps
        .iter()
        .all(|&(rule, reusable)| structural_rule_licensed(rule, reusable))
}

// ─── Proof-net validation (Danos-Regnier correctness, multiplicative fragment) ──────
//
// A multiplicative proof structure is a genuine PROOF NET iff every Danos-Regnier *switching*
// (each `⅋` node keeps exactly one of its two premise edges) yields a tree — acyclic AND
// connected. We enumerate the 2^(#par) switchings and check each with union-find. Bounded and
// zero-heap (fixed-size stack arrays).

/// Max nodes / par-links for the bounded DR check.
pub const MAX_PN_NODES: usize = 32;
/// Max `⅋` links (2^MAX_PN_PARS switchings enumerated).
pub const MAX_PN_PARS: usize = 12;

#[inline]
fn pn_find(parent: &mut [usize; MAX_PN_NODES], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]]; // path halving
        x = parent[x];
    }
    x
}

/// Union `a`,`b`; returns `false` if they were already connected (i.e. this edge closes a cycle).
#[inline]
fn pn_union(parent: &mut [usize; MAX_PN_NODES], a: usize, b: usize) -> bool {
    let ra = pn_find(parent, a);
    let rb = pn_find(parent, b);
    if ra == rb {
        return false;
    }
    parent[ra] = rb;
    true
}

/// Danos-Regnier check: is the proof structure a **proof net**? `n_nodes` formula occurrences,
/// the always-present `fixed_edges` (axiom / cut / `⊗` links), and the `par_switches` (each a
/// pair of candidate edges for one `⅋`, one chosen per switching). Returns true iff EVERY
/// switching graph is a tree. Bounded by [`MAX_PN_NODES`] / [`MAX_PN_PARS`].
pub fn is_proof_net(
    n_nodes: usize,
    fixed_edges: &[(usize, usize)],
    par_switches: &[((usize, usize), (usize, usize))],
) -> bool {
    if n_nodes == 0 || n_nodes > MAX_PN_NODES || par_switches.len() > MAX_PN_PARS {
        return false;
    }
    let k = par_switches.len();
    for mask in 0u32..(1u32 << k) {
        let mut parent = [0usize; MAX_PN_NODES];
        for (i, p) in parent.iter_mut().enumerate().take(n_nodes) {
            *p = i;
        }
        let mut acyclic = true;
        let mut edge_count = 0usize;

        for &(a, b) in fixed_edges {
            if a >= n_nodes || b >= n_nodes {
                return false;
            }
            if !pn_union(&mut parent, a, b) {
                acyclic = false;
            }
            edge_count += 1;
        }
        for (j, &(ea, eb)) in par_switches.iter().enumerate() {
            let (a, b) = if (mask >> j) & 1 == 0 { ea } else { eb };
            if a >= n_nodes || b >= n_nodes {
                return false;
            }
            if !pn_union(&mut parent, a, b) {
                acyclic = false;
            }
            edge_count += 1;
        }

        // Tree ⟺ acyclic ∧ connected ⟺ acyclic ∧ edge_count == n-1 ∧ single component.
        if !acyclic || edge_count != n_nodes - 1 {
            return false;
        }
        let root = pn_find(&mut parent, 0);
        for i in 1..n_nodes {
            if pn_find(&mut parent, i) != root {
                return false; // disconnected switching → not a proof net
            }
        }
    }
    true
}

// ─── Zero-knowledge–gated resource exhaustion ───────────────────────────────────────

/// ZK-gated consumption: a linear resource may be exhausted **only** if a zero-knowledge proof of
/// entitlement verifies (the witness stays private). Composes the verification boolean — produced
/// by `zk_proofs` / `legal_compose::zk_eligibility` — with the linear consume-once discipline.
/// Consumes (and returns `true`) iff the proof holds AND the resource is available. The webizen-VM
/// opcode dispatch that *invokes* this gate is a separate, out-of-(this-crate-scope) wiring step.
pub fn zk_gated_consume(q: &mut NQuin, reusable: bool, proof_verified: bool) -> bool {
    if !proof_verified || !can_consume(q, reusable) {
        return false;
    }
    if !reusable {
        consume_quin(q);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_quin() {
        let mut q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert!(!is_consumed(&q));
        consume_quin(&mut q);
        assert!(is_consumed(&q));
    }

    #[test]
    fn linear_negation_is_involutive_and_dualises_connectives() {
        use Connective::*;
        for c in [
            Atom, AtomDual, Tensor, Par, Plus, With, One, Bottom, Zero, Top, OfCourse, WhyNot,
        ] {
            assert_eq!(c.dual().dual(), c, "(A⊥)⊥ = A for {:?}", c);
            assert_ne!(c.dual(), c, "no connective is its own dual: {:?}", c);
        }
        // The characteristic De Morgan dualities.
        assert_eq!(Tensor.dual(), Par);
        assert_eq!(Plus.dual(), With);
        assert_eq!(One.dual(), Bottom);
        assert_eq!(Zero.dual(), Top);
        assert_eq!(OfCourse.dual(), WhyNot);
    }

    #[test]
    fn connective_classification_and_reuse() {
        use Connective::*;
        assert!(Tensor.is_multiplicative() && !Tensor.is_additive());
        assert!(With.is_additive() && !With.is_multiplicative());
        assert!(OfCourse.is_exponential() && WhyNot.is_exponential());
        // Only `!A` is reusable; linear atoms are consume-once.
        assert!(OfCourse.is_reusable());
        assert!(!Atom.is_reusable());
    }

    #[test]
    fn tensor_consumes_both_and_respects_reuse() {
        let mk = || NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        // Two linear resources: tensor consumes both; a second demand fails.
        let mut a = mk();
        let mut b = mk();
        assert!(tensor_consume(&mut a, false, &mut b, false));
        assert!(is_consumed(&a) && is_consumed(&b));
        assert!(
            !tensor_consume(&mut a, false, &mut b, false),
            "linear resources are exhausted"
        );

        // A reusable (!-marked) resource is never exhausted.
        let mut r = mk();
        let mut s = mk();
        assert!(tensor_consume(&mut r, true, &mut s, false));
        assert!(!is_consumed(&r), "reusable resource is not consumed");
        assert!(is_consumed(&s));
        assert!(
            tensor_consume(&mut r, true, &mut s, true),
            "reusable can satisfy again"
        );
    }

    #[test]
    fn structural_rules_are_controlled() {
        // Weakening / contraction only on reusable (!) formulas; never on linear ones.
        assert!(!structural_rule_licensed(StructuralRule::Weakening, false));
        assert!(!structural_rule_licensed(
            StructuralRule::Contraction,
            false
        ));
        assert!(structural_rule_licensed(StructuralRule::Weakening, true));
        assert!(structural_rule_licensed(StructuralRule::Contraction, true));
        // Exchange is always fine.
        assert!(structural_rule_licensed(StructuralRule::Exchange, false));
        // A whole derivation: contraction on a reusable + exchange on a linear → valid.
        assert!(structural_derivation_valid(&[
            (StructuralRule::Contraction, true),
            (StructuralRule::Exchange, false),
        ]));
        // …but contraction on a linear resource invalidates it.
        assert!(!structural_derivation_valid(&[(
            StructuralRule::Contraction,
            false
        )]));
    }

    #[test]
    fn danos_regnier_distinguishes_nets_from_non_nets() {
        // A single edge over 2 nodes, no pars → tree → net.
        assert!(is_proof_net(2, &[(0, 1)], &[]));
        // Disconnected: 3 nodes, 1 edge → not connected → not a net.
        assert!(!is_proof_net(3, &[(0, 1)], &[]));
        // Cyclic: 3 nodes, edges forming a triangle → cycle → not a net.
        assert!(!is_proof_net(3, &[(0, 1), (1, 2), (0, 2)], &[]));

        // A par link: 3 nodes, fixed edge (0,1), the ⅋ switches between (0,2) and (1,2).
        // BOTH switchings give a 3-node/2-edge connected acyclic tree → net.
        assert!(is_proof_net(3, &[(0, 1)], &[((0, 2), (1, 2))]));

        // A par whose both candidate edges duplicate the fixed edge → every switching makes a
        // cycle on {0,1} and leaves node 2 isolated → not a net.
        assert!(!is_proof_net(3, &[(0, 1)], &[((0, 1), (0, 1))]));

        // Bounds are enforced.
        assert!(!is_proof_net(0, &[], &[]));
        assert!(
            !is_proof_net(2, &[(0, 5)], &[]),
            "edge to out-of-range node rejected"
        );
    }

    #[test]
    fn zk_gate_controls_resource_exhaustion() {
        let mk = || NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        // No proof → no consumption, resource untouched.
        let mut q = mk();
        assert!(!zk_gated_consume(&mut q, false, false));
        assert!(!is_consumed(&q));
        // Valid proof → linear resource consumed once, then exhausted.
        assert!(zk_gated_consume(&mut q, false, true));
        assert!(is_consumed(&q));
        assert!(!zk_gated_consume(&mut q, false, true), "already exhausted");
        // Reusable resource with a valid proof is never exhausted.
        let mut r = mk();
        assert!(zk_gated_consume(&mut r, true, true));
        assert!(!is_consumed(&r));
    }
}
