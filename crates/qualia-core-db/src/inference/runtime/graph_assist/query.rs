use crate::indexing::QuinIndex;
use crate::NQuin;

use super::identity::{derive_prefix_identity, PrefixIdentity};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphQuery {
    /// Zero is a wildcard.
    pub context: u64,
    /// Zero is a wildcard.
    pub subject: u64,
    /// Zero is a wildcard.
    pub predicate: u64,
    /// Zero is a wildcard.
    pub object: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphAssistPolicy {
    pub max_facts: u32,
    pub sensitivity_ceiling: u8,
    pub require_valid_parity: bool,
    pub _pad: [u8; 2],
}

impl Default for GraphAssistPolicy {
    fn default() -> Self {
        Self {
            max_facts: 64,
            sensitivity_ceiling: 0,
            require_valid_parity: true,
            _pad: [0; 2],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphQueryError {
    InvalidPolicy,
    OutputBufferFull,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphSelectionReceipt {
    pub scanned: u32,
    pub matched: u32,
    pub written: u32,
    pub rejected_sensitivity: u32,
    pub rejected_parity: u32,
    pub prefix_identity: PrefixIdentity,
}

#[inline]
fn matches(query: &GraphQuery, quin: &NQuin) -> bool {
    (query.context == 0 || query.context == quin.context)
        && (query.subject == 0 || query.subject == quin.subject)
        && (query.predicate == 0 || query.predicate == quin.predicate)
        && (query.object == 0 || query.object == quin.object)
}

/// Select graph facts directly from the flat Quin store into caller-owned storage.
///
/// This is the native inference query interface: hashed fields avoid SPARQL parsing and strings in
/// the request path, while callers that need full SPARQL can resolve a graph slice before calling.
pub fn query_graph_into(
    graph: &[NQuin],
    query: &GraphQuery,
    policy: GraphAssistPolicy,
    model_instance: u64,
    tokenizer_revision: u64,
    graph_revision: u64,
    out: &mut [NQuin],
) -> Result<GraphSelectionReceipt, GraphQueryError> {
    if policy.max_facts == 0 {
        return Err(GraphQueryError::InvalidPolicy);
    }
    let capacity = out.len().min(policy.max_facts as usize);
    let mut receipt = GraphSelectionReceipt::default();
    for quin in graph {
        receipt.scanned = receipt.scanned.saturating_add(1);
        if !matches(query, quin) {
            continue;
        }
        let sensitivity = (quin.context >> 56) as u8;
        if sensitivity > policy.sensitivity_ceiling {
            receipt.rejected_sensitivity = receipt.rejected_sensitivity.saturating_add(1);
            continue;
        }
        if policy.require_valid_parity
            && quin.parity != (quin.subject ^ quin.predicate ^ quin.object ^ quin.context)
        {
            receipt.rejected_parity = receipt.rejected_parity.saturating_add(1);
            continue;
        }
        receipt.matched = receipt.matched.saturating_add(1);
        if receipt.written as usize >= capacity {
            return Err(GraphQueryError::OutputBufferFull);
        }
        out[receipt.written as usize] = *quin;
        receipt.written += 1;
    }
    receipt.prefix_identity = derive_prefix_identity(
        model_instance,
        tokenizer_revision,
        query.context,
        graph_revision,
        &out[..receipt.written as usize],
    );
    Ok(receipt)
}

/// Select graph facts using a pre-built [`QuinIndex`] to scan only the narrowest non-wildcard
/// posting list in ascending graph order ($O(k)$ candidate inspection instead of $O(N)$ full scan).
///
/// Produces bit-identical `out` facts, `matched`, `written`, `rejected_sensitivity`,
/// `rejected_parity`, and `prefix_identity` to [`query_graph_into`] without heap allocation.
pub fn query_graph_indexed_into(
    graph: &[NQuin],
    index: &QuinIndex,
    query: &GraphQuery,
    policy: GraphAssistPolicy,
    model_instance: u64,
    tokenizer_revision: u64,
    graph_revision: u64,
    out: &mut [NQuin],
) -> Result<GraphSelectionReceipt, GraphQueryError> {
    if policy.max_facts == 0 {
        return Err(GraphQueryError::InvalidPolicy);
    }
    // Fall back to linear scan when all query fields are wildcards or index length differs.
    if (query.context == 0 && query.subject == 0 && query.predicate == 0 && query.object == 0)
        || index.len() != graph.len()
    {
        return query_graph_into(
            graph,
            query,
            policy,
            model_instance,
            tokenizer_revision,
            graph_revision,
            out,
        );
    }

    let mut slices: [&[usize]; 4] = [&[]; 4];
    let mut slice_count = 0usize;
    if query.subject != 0 {
        slices[slice_count] = index.rows_by_subject(query.subject);
        slice_count += 1;
    }
    if query.predicate != 0 {
        slices[slice_count] = index.rows_by_predicate(query.predicate);
        slice_count += 1;
    }
    if query.object != 0 {
        slices[slice_count] = index.rows_by_object(query.object);
        slice_count += 1;
    }
    if query.context != 0 {
        slices[slice_count] = index.rows_by_context(query.context);
        slice_count += 1;
    }

    let mut best_idx = 0usize;
    for i in 1..slice_count {
        if slices[i].len() < slices[best_idx].len() {
            best_idx = i;
        }
    }
    let candidates = slices[best_idx];

    let capacity = out.len().min(policy.max_facts as usize);
    let mut receipt = GraphSelectionReceipt::default();
    for &row_idx in candidates {
        let Some(quin) = graph.get(row_idx) else {
            continue;
        };
        receipt.scanned = receipt.scanned.saturating_add(1);
        if !matches(query, quin) {
            continue;
        }
        let sensitivity = (quin.context >> 56) as u8;
        if sensitivity > policy.sensitivity_ceiling {
            receipt.rejected_sensitivity = receipt.rejected_sensitivity.saturating_add(1);
            continue;
        }
        if policy.require_valid_parity
            && quin.parity != (quin.subject ^ quin.predicate ^ quin.object ^ quin.context)
        {
            receipt.rejected_parity = receipt.rejected_parity.saturating_add(1);
            continue;
        }
        receipt.matched = receipt.matched.saturating_add(1);
        if receipt.written as usize >= capacity {
            return Err(GraphQueryError::OutputBufferFull);
        }
        out[receipt.written as usize] = *quin;
        receipt.written += 1;
    }
    receipt.prefix_identity = derive_prefix_identity(
        model_instance,
        tokenizer_revision,
        query.context,
        graph_revision,
        &out[..receipt.written as usize],
    );
    Ok(receipt)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn make_quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity: subject ^ predicate ^ object ^ context,
        }
    }

    #[test]
    fn indexed_query_is_bit_exact_with_linear_scan_and_reduces_scanned_rows() {
        let mut bad_parity = make_quin(1, 10, 999, 7);
        bad_parity.parity ^= 1;
        let restricted_ctx = (1u64 << 56) | 7;
        let restricted = make_quin(1, 10, 888, restricted_ctx);

        let graph = [
            make_quin(1, 10, 100, 7),
            make_quin(2, 20, 200, 7),
            bad_parity,
            make_quin(1, 10, 300, 7),
            restricted,
            make_quin(3, 10, 400, 8),
            make_quin(4, 30, 500, 7),
            make_quin(1, 10, 600, 7),
        ];
        let index = QuinIndex::from_slice(&graph);

        let queries = [
            // Narrowest on subject=1 (4 rows in subject=1 vs 5 rows in predicate=10 vs 5 rows in context=7)
            GraphQuery {
                context: 7,
                subject: 1,
                predicate: 10,
                object: 0,
            },
            // Single predicate filter
            GraphQuery {
                context: 0,
                subject: 0,
                predicate: 10,
                object: 0,
            },
            // Narrowest on object=300 (1 row)
            GraphQuery {
                context: 7,
                subject: 0,
                predicate: 10,
                object: 300,
            },
            // Restricted context query
            GraphQuery {
                context: restricted_ctx,
                subject: 1,
                predicate: 10,
                object: 0,
            },
            // Non-existent key (0 candidate rows)
            GraphQuery {
                context: 7,
                subject: 9999,
                predicate: 10,
                object: 0,
            },
            // Wildcard all fields
            GraphQuery {
                context: 0,
                subject: 0,
                predicate: 0,
                object: 0,
            },
        ];

        for query in &queries {
            let mut out_linear = [NQuin::default(); 8];
            let mut out_indexed = [NQuin::default(); 8];
            let r_linear = query_graph_into(
                &graph,
                query,
                GraphAssistPolicy::default(),
                41,
                42,
                43,
                &mut out_linear,
            )
            .unwrap();
            let r_indexed = query_graph_indexed_into(
                &graph,
                &index,
                query,
                GraphAssistPolicy::default(),
                41,
                42,
                43,
                &mut out_indexed,
            )
            .unwrap();

            assert_eq!(r_indexed.matched, r_linear.matched);
            assert_eq!(r_indexed.written, r_linear.written);
            assert_eq!(r_indexed.rejected_sensitivity, r_linear.rejected_sensitivity);
            assert_eq!(r_indexed.rejected_parity, r_linear.rejected_parity);
            assert_eq!(r_indexed.prefix_identity, r_linear.prefix_identity);
            assert_eq!(out_indexed, out_linear);
            assert!(r_indexed.scanned <= r_linear.scanned);
        }

        // Verify narrowest candidate selection on (context=7, subject=0, predicate=10, object=300):
        // object=300 has exactly 1 row in the index vs 8 total graph rows.
        let mut out_narrow = [NQuin::default(); 4];
        let r_narrow = query_graph_indexed_into(
            &graph,
            &index,
            &queries[2],
            GraphAssistPolicy::default(),
            41,
            42,
            43,
            &mut out_narrow,
        )
        .unwrap();
        assert_eq!(r_narrow.scanned, 1);
        assert_eq!(r_narrow.written, 1);
    }

    #[test]
    fn indexed_query_matches_error_contracts() {
        let graph = [make_quin(1, 10, 100, 7), make_quin(1, 10, 200, 7)];
        let index = QuinIndex::from_slice(&graph);
        let query = GraphQuery {
            context: 7,
            subject: 1,
            predicate: 10,
            object: 0,
        };

        let mut out = [NQuin::default(); 1];
        let bad_policy = GraphAssistPolicy {
            max_facts: 0,
            ..GraphAssistPolicy::default()
        };
        assert_eq!(
            query_graph_indexed_into(&graph, &index, &query, bad_policy, 1, 1, 1, &mut out),
            Err(GraphQueryError::InvalidPolicy)
        );
        assert_eq!(
            query_graph_indexed_into(
                &graph,
                &index,
                &query,
                GraphAssistPolicy::default(),
                1,
                1,
                1,
                &mut out
            ),
            Err(GraphQueryError::OutputBufferFull)
        );
    }
}
