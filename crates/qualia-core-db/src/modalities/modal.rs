use crate::NQuin;

/// General modal logic over a Kripke frame — the shared modal substrate (□/◇) that
/// the *specific* modalities (deontic, epistemic) specialise. Accessibility is
/// `(world →accesses→ world')` (predicate == `accesses`); a world satisfies a
/// proposition when `(world, holds, prop)` is present. Zero-heap (linear scans).
///
/// With S4 (reflexive+transitive) or S5 (equivalence) accessibility supplied as
/// edges, this yields the corresponding modal system; the evaluator itself is
/// frame-agnostic (it reads whatever accessibility edges are asserted).

/// **◇φ (possible)** — SOME world accessible from `world` satisfies `prop`.
pub fn possible(graph: &[NQuin], world: u64, prop: u64, accesses: u64, holds: u64) -> bool {
    for q in graph {
        if q.subject == world && q.predicate == accesses {
            let w2 = q.object;
            if graph
                .iter()
                .any(|r| r.subject == w2 && r.predicate == holds && r.object == prop)
            {
                return true;
            }
        }
    }
    false
}

/// **□φ (necessary)** — ALL worlds accessible from `world` satisfy `prop`.
/// Vacuously true when no worlds are accessible (standard modal semantics).
pub fn necessary(graph: &[NQuin], world: u64, prop: u64, accesses: u64, holds: u64) -> bool {
    for q in graph {
        if q.subject == world && q.predicate == accesses {
            let w2 = q.object;
            if !graph
                .iter()
                .any(|r| r.subject == w2 && r.predicate == holds && r.object == prop)
            {
                return false; // an accessible world fails the proposition
            }
        }
    }
    true
}

// ─── Kripke frame properties → normal modal axiom systems (K, T, D, B, S4, S5) ──────
//
// Each characteristic axiom of a normal modal logic corresponds to a structural property
// of the accessibility relation `R`. The frame's domain `worlds` is supplied explicitly
// (zero-heap; no allocation, just nested slice scans).

/// Is `(from → to)` an accessibility edge?
#[inline]
fn edge(graph: &[NQuin], from: u64, to: u64, accesses: u64) -> bool {
    graph
        .iter()
        .any(|q| q.subject == from && q.predicate == accesses && q.object == to)
}

/// **Reflexive** (axiom T: □φ→φ): every world accesses itself.
pub fn is_reflexive(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    worlds.iter().all(|&w| edge(graph, w, w, accesses))
}

/// **Serial** (axiom D: □φ→◇φ): every world accesses at least one world.
pub fn is_serial(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    worlds
        .iter()
        .all(|&w| worlds.iter().any(|&v| edge(graph, w, v, accesses)))
}

/// **Symmetric** (axiom B: φ→□◇φ): `wRv ⇒ vRw`.
pub fn is_symmetric(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    for &w in worlds {
        for &v in worlds {
            if edge(graph, w, v, accesses) && !edge(graph, v, w, accesses) {
                return false;
            }
        }
    }
    true
}

/// **Transitive** (axiom 4: □φ→□□φ): `wRv ∧ vRu ⇒ wRu`.
pub fn is_transitive(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    for &w in worlds {
        for &v in worlds {
            if !edge(graph, w, v, accesses) {
                continue;
            }
            for &u in worlds {
                if edge(graph, v, u, accesses) && !edge(graph, w, u, accesses) {
                    return false;
                }
            }
        }
    }
    true
}

/// **Euclidean** (axiom 5: ◇φ→□◇φ): `wRv ∧ wRu ⇒ vRu`.
pub fn is_euclidean(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    for &w in worlds {
        for &v in worlds {
            if !edge(graph, w, v, accesses) {
                continue;
            }
            for &u in worlds {
                if edge(graph, w, u, accesses) && !edge(graph, v, u, accesses) {
                    return false;
                }
            }
        }
    }
    true
}

/// The normal modal axiom systems this engine recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalSystem {
    /// The base system — every frame validates K.
    K,
    /// Reflexive frames.
    T,
    /// Serial frames.
    D,
    /// Symmetric frames.
    B,
    /// Reflexive + transitive frames.
    S4,
    /// Equivalence frames (reflexive + symmetric + transitive).
    S5,
}

/// Does the frame validate the characteristic axiom(s) of `system` over `worlds`?
pub fn validates(system: ModalSystem, graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    match system {
        ModalSystem::K => true,
        ModalSystem::T => is_reflexive(graph, accesses, worlds),
        ModalSystem::D => is_serial(graph, accesses, worlds),
        ModalSystem::B => is_symmetric(graph, accesses, worlds),
        ModalSystem::S4 => {
            is_reflexive(graph, accesses, worlds) && is_transitive(graph, accesses, worlds)
        }
        ModalSystem::S5 => {
            is_reflexive(graph, accesses, worlds)
                && is_symmetric(graph, accesses, worlds)
                && is_transitive(graph, accesses, worlds)
        }
    }
}

// ─── Multi-agent epistemic modality (K_i) ───────────────────────────────────────────
//
// Each agent `i` carries its OWN accessibility relation, supplied as a distinct `accesses_i`
// predicate (e.g. `q_hash("agent:alice:accesses")`). "Agent i knows φ at w" (K_i φ) is then just
// □ over i's relation — so the frame-agnostic `necessary` specialises per agent at no extra cost.

/// `K_i φ` — agent `i` (via its `accesses_i` relation) **knows** `prop` at `world`: every
/// world i-accessible from `world` satisfies it.
#[inline]
pub fn knows(graph: &[NQuin], accesses_i: u64, world: u64, prop: u64, holds: u64) -> bool {
    necessary(graph, world, prop, accesses_i, holds)
}

/// "Everybody knows" `prop` at `world`: K_i φ holds for **every** agent in `agent_accesses`.
pub fn everyone_knows(
    graph: &[NQuin],
    agent_accesses: &[u64],
    world: u64,
    prop: u64,
    holds: u64,
) -> bool {
    agent_accesses
        .iter()
        .all(|&acc| knows(graph, acc, world, prop, holds))
}

// ─── AGM belief revision ────────────────────────────────────────────────────────────
//
// A finite belief base over signed literals, with the three AGM operations. Revision uses the
// Levi identity (`K*φ = (K−¬φ)+φ`), so a revised set is always consistent in `φ`: you can never
// believe an atom both ways. Zero-heap (caller-supplied `out`, sized ≥ the result).

/// A signed belief: `atom` held positively (`positive == true`) or negatively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Belief {
    pub atom: u64,
    pub positive: bool,
}

impl Belief {
    /// The contrary belief (same atom, flipped polarity).
    #[inline]
    pub fn negate(self) -> Belief {
        Belief {
            atom: self.atom,
            positive: !self.positive,
        }
    }
}

/// A belief base is **consistent** iff no atom is held both positively and negatively.
pub fn is_consistent(set: &[Belief]) -> bool {
    for (i, a) in set.iter().enumerate() {
        for b in &set[i + 1..] {
            if a.atom == b.atom && a.positive != b.positive {
                return false;
            }
        }
    }
    true
}

/// **AGM expansion** `K + φ`: add `belief` if absent. Writes the result into `out`, returns its
/// length (no deductive closure beyond the explicit literals — a finite base model).
pub fn expand(set: &[Belief], belief: Belief, out: &mut [Belief]) -> usize {
    let mut n = 0usize;
    for &x in set {
        if n < out.len() {
            out[n] = x;
            n += 1;
        }
    }
    if !set.contains(&belief) && n < out.len() {
        out[n] = belief;
        n += 1;
    }
    n
}

/// **AGM contraction** `K − φ`: remove `belief` if present (vacuous if absent). Writes the
/// result into `out`, returns its length.
pub fn contract(set: &[Belief], belief: Belief, out: &mut [Belief]) -> usize {
    let mut n = 0usize;
    for &x in set {
        if x == belief {
            continue;
        }
        if n < out.len() {
            out[n] = x;
            n += 1;
        }
    }
    n
}

/// **AGM revision** `K * φ` via the Levi identity: drop any belief about `belief.atom` (so the
/// contrary `¬φ` is contracted), then add `φ`. The result satisfies *success* (`φ ∈ K*φ`) and
/// *consistency* (never both `φ` and `¬φ`). Writes into `out`, returns its length.
pub fn revise(set: &[Belief], belief: Belief, out: &mut [Belief]) -> usize {
    let mut n = 0usize;
    for &x in set {
        if x.atom == belief.atom {
            continue; // contracts both the old φ and ¬φ on this atom
        }
        if n < out.len() {
            out[n] = x;
            n += 1;
        }
    }
    if n < out.len() {
        out[n] = belief;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(from: u64, to: u64) -> NQuin {
        let mut q = NQuin {
            subject: from,
            predicate: crate::q_hash("modal:accesses"),
            object: to,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }
    fn label(world: u64, prop: u64) -> NQuin {
        let mut q = NQuin {
            subject: world,
            predicate: crate::q_hash("modal:holds"),
            object: prop,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn box_and_diamond() {
        let accesses = crate::q_hash("modal:accesses");
        let holds = crate::q_hash("modal:holds");
        let p = 100u64;
        // w0 accesses w1, w2. p holds at w1 only.
        let g = [acc(0, 1), acc(0, 2), label(1, p)];
        assert!(
            possible(&g, 0, p, accesses, holds),
            "◇p: w1 (accessible) satisfies p"
        );
        assert!(
            !necessary(&g, 0, p, accesses, holds),
            "□p fails: w2 (accessible) does not satisfy p"
        );
        // Now p holds at both accessible worlds.
        let g2 = [acc(0, 1), acc(0, 2), label(1, p), label(2, p)];
        assert!(
            necessary(&g2, 0, p, accesses, holds),
            "□p: all accessible worlds satisfy p"
        );
    }

    #[test]
    fn frame_properties_and_axiom_systems() {
        let accesses = crate::q_hash("modal:accesses");
        let worlds = [0u64, 1u64, 2u64];

        // An equivalence frame (S5): reflexive + symmetric + transitive — here the total
        // relation over {0,1,2}.
        let mut s5 = Vec::new();
        for &w in &worlds {
            for &v in &worlds {
                s5.push(acc(w, v));
            }
        }
        assert!(is_reflexive(&s5, accesses, &worlds));
        assert!(is_symmetric(&s5, accesses, &worlds));
        assert!(is_transitive(&s5, accesses, &worlds));
        assert!(is_euclidean(&s5, accesses, &worlds));
        assert!(validates(ModalSystem::S5, &s5, accesses, &worlds));
        assert!(validates(ModalSystem::S4, &s5, accesses, &worlds));
        assert!(validates(ModalSystem::K, &s5, accesses, &worlds));

        // A bare chain 0→1→2 (irreflexive, asymmetric): validates neither T nor B; not transitive
        // (0→1, 1→2, but no 0→2). It is serial only if every world has a successor — world 2 has
        // none, so D fails too.
        let chain = [acc(0, 1), acc(1, 2)];
        assert!(!is_reflexive(&chain, accesses, &worlds));
        assert!(!is_symmetric(&chain, accesses, &worlds));
        assert!(!is_transitive(&chain, accesses, &worlds));
        assert!(!validates(ModalSystem::T, &chain, accesses, &worlds));
        assert!(!validates(ModalSystem::B, &chain, accesses, &worlds));
        assert!(!validates(ModalSystem::D, &chain, accesses, &worlds));
        assert!(validates(ModalSystem::K, &chain, accesses, &worlds)); // every frame validates K

        // Add 0→2 and reflexive loops → reflexive + transitive (S4) but NOT symmetric.
        let s4 = [
            acc(0, 0),
            acc(1, 1),
            acc(2, 2),
            acc(0, 1),
            acc(1, 2),
            acc(0, 2),
        ];
        assert!(is_reflexive(&s4, accesses, &worlds));
        assert!(is_transitive(&s4, accesses, &worlds));
        assert!(!is_symmetric(&s4, accesses, &worlds));
        assert!(validates(ModalSystem::S4, &s4, accesses, &worlds));
        assert!(!validates(ModalSystem::S5, &s4, accesses, &worlds));
        assert!(validates(ModalSystem::T, &s4, accesses, &worlds));
        assert!(validates(ModalSystem::D, &s4, accesses, &worlds)); // reflexive ⇒ serial
    }

    #[test]
    fn multi_agent_knowledge_is_per_relation() {
        let holds = crate::q_hash("modal:holds");
        let alice = crate::q_hash("agent:alice:accesses");
        let bob = crate::q_hash("agent:bob:accesses");
        let p = 100u64;
        let mk = |from: u64, to: u64, acc: u64| {
            let mut q = NQuin {
                subject: from,
                predicate: acc,
                object: to,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };
        // From w0: Alice accesses only w1 (where p holds); Bob accesses w1 and w2 (p fails at w2).
        let g = [mk(0, 1, alice), mk(0, 1, bob), mk(0, 2, bob), label(1, p)];
        assert!(
            knows(&g, alice, 0, p, holds),
            "Alice knows p (all her worlds satisfy it)"
        );
        assert!(
            !knows(&g, bob, 0, p, holds),
            "Bob does not know p (w2 fails)"
        );
        assert!(!everyone_knows(&g, &[alice, bob], 0, p, holds));
    }

    #[test]
    fn agm_revision_is_consistent_and_satisfies_success() {
        let p = 1u64;
        let bel_p = Belief {
            atom: p,
            positive: true,
        };
        let bel_not_p = Belief {
            atom: p,
            positive: false,
        };
        let q = Belief {
            atom: 2,
            positive: true,
        };
        let mut out = [Belief {
            atom: 0,
            positive: true,
        }; 8];

        // Start believing ¬p and q. Revise by p.
        let base = [bel_not_p, q];
        let n = revise(&base, bel_p, &mut out);
        let result = &out[..n];
        // Success: p ∈ K*p. Consistency: ¬p ∉ K*p. Minimal change: q retained.
        assert!(result.contains(&bel_p), "success postulate: φ ∈ K*φ");
        assert!(!result.contains(&bel_not_p), "consistency: ¬φ removed");
        assert!(
            result.contains(&q),
            "minimal change: unrelated beliefs kept"
        );
        assert!(is_consistent(result));

        // Expansion adds; idempotent if already present.
        let n2 = expand(&base, q, &mut out);
        assert_eq!(n2, 2, "q already present → no growth");
        let n3 = expand(&base, bel_p, &mut out);
        assert_eq!(
            n3, 3,
            "p added by expansion (may be inconsistent — that's expansion, not revision)"
        );
        assert!(
            !is_consistent(&out[..n3]),
            "expansion does NOT guarantee consistency"
        );

        // Contraction removes; vacuous when absent.
        let n4 = contract(&base, bel_not_p, &mut out);
        assert!(!out[..n4].contains(&bel_not_p));
        assert_eq!(
            contract(&base, bel_p, &mut out),
            2,
            "contracting an absent belief is vacuous"
        );
    }
}
