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
            if graph.iter().any(|r| r.subject == w2 && r.predicate == holds && r.object == prop) {
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
            if !graph.iter().any(|r| r.subject == w2 && r.predicate == holds && r.object == prop) {
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
    graph.iter().any(|q| q.subject == from && q.predicate == accesses && q.object == to)
}

/// **Reflexive** (axiom T: □φ→φ): every world accesses itself.
pub fn is_reflexive(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    worlds.iter().all(|&w| edge(graph, w, w, accesses))
}

/// **Serial** (axiom D: □φ→◇φ): every world accesses at least one world.
pub fn is_serial(graph: &[NQuin], accesses: u64, worlds: &[u64]) -> bool {
    worlds.iter().all(|&w| worlds.iter().any(|&v| edge(graph, w, v, accesses)))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(from: u64, to: u64) -> NQuin {
        let mut q = NQuin { subject: from, predicate: crate::q_hash("modal:accesses"), object: to, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }
    fn label(world: u64, prop: u64) -> NQuin {
        let mut q = NQuin { subject: world, predicate: crate::q_hash("modal:holds"), object: prop, context: 0, metadata: 0, parity: 0 };
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
        assert!(possible(&g, 0, p, accesses, holds), "◇p: w1 (accessible) satisfies p");
        assert!(!necessary(&g, 0, p, accesses, holds), "□p fails: w2 (accessible) does not satisfy p");
        // Now p holds at both accessible worlds.
        let g2 = [acc(0, 1), acc(0, 2), label(1, p), label(2, p)];
        assert!(necessary(&g2, 0, p, accesses, holds), "□p: all accessible worlds satisfy p");
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
            acc(0, 0), acc(1, 1), acc(2, 2),
            acc(0, 1), acc(1, 2), acc(0, 2),
        ];
        assert!(is_reflexive(&s4, accesses, &worlds));
        assert!(is_transitive(&s4, accesses, &worlds));
        assert!(!is_symmetric(&s4, accesses, &worlds));
        assert!(validates(ModalSystem::S4, &s4, accesses, &worlds));
        assert!(!validates(ModalSystem::S5, &s4, accesses, &worlds));
        assert!(validates(ModalSystem::T, &s4, accesses, &worlds));
        assert!(validates(ModalSystem::D, &s4, accesses, &worlds)); // reflexive ⇒ serial
    }
}
