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
}
