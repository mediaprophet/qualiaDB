//! Fragment-level category attribution edges (not whole-corpus genre flags).
//!
//! Edges are pure descriptors; storage lives in host/graph layers.

use super::entity_id::EntityId;
use serde::{Deserialize, Serialize};

/// Typed attribution relation (extensible string in cold path; fixed set for product).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum AttributionRel {
    /// Narrative / fictional presentation.
    NarrativeFiction = 1,
    /// Formal STEM / technical evaluation.
    StemFormal = 2,
    /// Fragment illustrates a concept (STEM-via-story normal).
    Illustrates = 3,
    /// Grounded in commons / measured geo or attested fact.
    GroundsIn = 4,
    /// Depicts a social relation pattern.
    DepictsSocial = 5,
    /// Legal / statute citation.
    LegalCite = 6,
    /// Geographic fact.
    GeographicFact = 7,
}

/// One attribution edge: subject fragment - object entity/concept under a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributionEdge {
    pub subject: EntityId,
    pub rel: AttributionRel,
    pub object: EntityId,
    /// Lamport / unix attribution time (0 = unset).
    pub attributed_at: u64,
}

/// Bounded collect: write edges whose subject matches into `out`. Returns count.
pub fn edges_for_subject(
    edges: &[AttributionEdge],
    subject: EntityId,
    out: &mut [AttributionEdge],
) -> usize {
    let mut n = 0;
    for e in edges {
        if e.subject == subject {
            if n >= out.len() {
                break;
            }
            out[n] = *e;
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_in_story_two_edges_same_fragment() {
        let frag = EntityId::from_fragment("urn:script:1", "para:3");
        let concept = EntityId::from_uri("urn:concept:BayesRule");
        let edges = [
            AttributionEdge {
                subject: frag,
                rel: AttributionRel::NarrativeFiction,
                object: EntityId::from_uri("urn:genre:fiction"),
                attributed_at: 1,
            },
            AttributionEdge {
                subject: frag,
                rel: AttributionRel::Illustrates,
                object: concept,
                attributed_at: 1,
            },
        ];
        let mut out = [AttributionEdge {
            subject: EntityId::default(),
            rel: AttributionRel::NarrativeFiction,
            object: EntityId::default(),
            attributed_at: 0,
        }; 4];
        let n = edges_for_subject(&edges, frag, &mut out);
        assert_eq!(n, 2);
    }
}
