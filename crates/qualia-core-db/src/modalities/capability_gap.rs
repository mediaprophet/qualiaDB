//! Gap analysis & capability logic (§24, legal_logic.md) — anti-deficit / RPL.
//!
//! Computes what is *present* versus what is *lacking* by set difference over capabilities —
//! the engine for Recognition of Prior Learning (RPL) and the "Peace-Infrastructure" deployment
//! strategy (deploy to the *computed gap*, not by assumption). Experiential/traditional
//! knowledge counts as held when an authoritative `skos:closeMatch` links it to the required
//! formal capability. Zero-heap (caller-supplied `out`).

/// Is `cap` held — directly, or via an experiential `equivalences` pair `(required, held)` that
/// recognises a held capability as equivalent to the required one?
fn holds(cap: u64, held: &[u64], equivalences: &[(u64, u64)]) -> bool {
    held.contains(&cap)
        || equivalences
            .iter()
            .any(|&(req, h)| req == cap && held.contains(&h))
}

/// The **computable gap**: the `required` capabilities that are NOT held (directly or by
/// recognised equivalence), written to `out`. Returns the count. `Gap = Req \ Holds`.
pub fn capability_gap(
    required: &[u64],
    held: &[u64],
    equivalences: &[(u64, u64)],
    out: &mut [u64],
) -> usize {
    let mut n = 0usize;
    for &cap in required {
        if !holds(cap, held, equivalences) {
            if n >= out.len() {
                break;
            }
            out[n] = cap;
            n += 1;
        }
    }
    n
}

/// Are all requirements met (the gap is empty)?
pub fn requirements_met(required: &[u64], held: &[u64], equivalences: &[(u64, u64)]) -> bool {
    required.iter().all(|&c| holds(c, held, equivalences))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn gap_is_the_set_difference() {
        let (welding, wiring, plumbing) = (q_hash("cap:welding"), q_hash("cap:wiring"), q_hash("cap:plumbing"));
        let required = [welding, wiring, plumbing];
        let held = [welding];
        let mut out = [0u64; 8];
        let n = capability_gap(&required, &held, &[], &mut out);
        assert_eq!(n, 2);
        assert!(out[..n].contains(&wiring) && out[..n].contains(&plumbing));
        assert!(!requirements_met(&required, &held, &[]));
    }

    #[test]
    fn experiential_equivalence_closes_the_gap() {
        let formal = q_hash("cap:formalDegree");
        let experiential = q_hash("cap:apprenticeship");
        let required = [formal];
        let held = [experiential];
        // Without recognition → a gap.
        assert_eq!(capability_gap(&required, &held, &[], &mut [0u64; 4]), 1);
        // With an authoritative closeMatch (formal ≈ experiential) → gap closed.
        let equiv = [(formal, experiential)];
        assert_eq!(capability_gap(&required, &held, &equiv, &mut [0u64; 4]), 0);
        assert!(requirements_met(&required, &held, &equiv));
    }
}
