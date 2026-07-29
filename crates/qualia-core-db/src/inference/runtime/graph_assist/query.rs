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
