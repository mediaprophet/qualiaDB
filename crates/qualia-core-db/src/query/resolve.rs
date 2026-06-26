//! Unified graph resolver — the consumer wiring for the hybrid-modality stack (#22).
//!
//! One entry point that composes the pieces built this session: it resolves an
//! identifier's modal KIND (via the open identifier-kind fabric, `modal_kind`) and its
//! outgoing relations, over either a maintained [`QuinIndex`] (O(1) point lookups) or a
//! raw quin slice (O(n) scan, zero index build — for ad-hoc resolution against the live
//! daemon graph snapshot). Lexical VALUES are recovered separately through the lexicon
//! (with the collision backstop). This is what daemon / MCP / query callers route
//! through, so identifier resolution has a single, consistent path.

use crate::indexing::QuinIndex;
use crate::modal_kind::{resolve_kind, HAS_MODALITY_KIND};
use crate::NQuin;

/// A resolved view of an identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    pub identifier: u64,
    /// The modal identifier-kind, if asserted (`None` = a plain dictionary reference).
    pub kind: Option<u64>,
    /// Number of outgoing relations (its out-degree in the graph).
    pub out_degree: usize,
}

/// Resolve over a maintained index — O(1) point lookups; preferred when an index exists
/// (the per-cell cached path, task #22 step 3).
pub fn resolve_in_index(index: &QuinIndex, identifier: u64) -> Resolved {
    Resolved {
        identifier,
        kind: resolve_kind(index, identifier),
        out_degree: index.iter_by_subject(identifier).count(),
    }
}

/// Resolve over a raw quin slice — O(n) scan, no index build, no allocation. For ad-hoc
/// single-identifier resolution against e.g. the daemon graph snapshot.
pub fn resolve_in_slice(quins: &[NQuin], identifier: u64) -> Resolved {
    let mut kind = None;
    let mut out_degree = 0usize;
    for q in quins {
        if q.subject == identifier {
            out_degree += 1;
            if q.predicate == HAS_MODALITY_KIND {
                kind = Some(q.object);
            }
        }
    }
    Resolved {
        identifier,
        kind,
        out_degree,
    }
}

/// The object of a specific relation over a slice (zero-alloc scan).
pub fn related_in_slice(quins: &[NQuin], identifier: u64, predicate: u64) -> Option<u64> {
    quins
        .iter()
        .find(|q| q.subject == identifier && q.predicate == predicate)
        .map(|q| q.object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal_kind::{tag_kind, KIND_CONTENT_HASH, KIND_WEBIZEN};
    use crate::q_hash;

    fn rel(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn resolves_kind_and_out_degree_over_slice() {
        let id = q_hash("https://example.org/thing");
        let p1 = q_hash("p1");
        let p2 = q_hash("p2");
        let quins = [
            tag_kind(id, KIND_WEBIZEN),
            rel(id, p1, 10),
            rel(id, p2, 20),
            rel(q_hash("other"), p1, 99),
        ];
        let r = resolve_in_slice(&quins, id);
        assert_eq!(r.kind, Some(KIND_WEBIZEN));
        assert_eq!(r.out_degree, 3); // the kind quin + p1 + p2 (not the "other" subject)
        assert_eq!(related_in_slice(&quins, id, p1), Some(10));
    }

    #[test]
    fn index_and_slice_resolution_agree() {
        let id: u64 = 0xF000_0000_0000_00AB; // a full-width identifier
        let quins = [tag_kind(id, KIND_CONTENT_HASH), rel(id, q_hash("x"), 7)];
        let idx = QuinIndex::from_slice(&quins);
        assert_eq!(resolve_in_index(&idx, id), resolve_in_slice(&quins, id));
        assert_eq!(resolve_in_index(&idx, id).kind, Some(KIND_CONTENT_HASH));
    }

    #[test]
    fn unkinded_identifier_has_none_kind() {
        let id = q_hash("plain");
        let quins = [rel(id, q_hash("p"), 1)];
        assert_eq!(resolve_in_slice(&quins, id).kind, None);
        assert_eq!(resolve_in_slice(&quins, id).out_degree, 1);
    }
}
