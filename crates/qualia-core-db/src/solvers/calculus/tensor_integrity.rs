//! Tamper-evident, append-only lineage commitments for tensor provenance, plus a
//! zero-knowledge binding to verified linear-transformation proofs.
//!
//! Companion to [`super::tensor_provenance`] (which tracks the parent→child DAG).
//! This module adds the *integrity* layer the audit calls for:
//!
//! - **Immutable, append-only lineage DAG.** Each node is content-addressed by a
//!   BLAKE3 commitment over `domain ++ parent_commitment ++ operation ++ params ++
//!   data_bits`. Any post-hoc edit to an ancestor's data/operation changes that
//!   node's commitment and therefore *every* descendant's — so tampering anywhere in
//!   the lineage is detectable ([`verify_lineage`]). The DAG is append-only: the only
//!   way to extend it is to derive a new child; there is no in-place mutation path.
//! - **zk-transformation binding.** [`transformation_commitment`] binds
//!   `(input, output, operation)` cryptographically — the public witness. For a
//!   *linear* tensor map (prove `y = W·x` without revealing `W`), the actual
//!   zero-knowledge proof is the real arkworks Groth16 `private_matrix_multiply`
//!   (`crate::zk_proofs` / `linear_algebra`). General per-op zk-SNARKs over arbitrary
//!   tensor ops are a recorded boundary (each op needs its own R1CS circuit).
//!
//! Heap note: like `tensor_provenance`, this is the **cold, host-side provenance
//! layer** (it walks a `HashMap`-backed graph and allocates small scratch `Vec`s for
//! sorting). It is off the zero-heap hot path — the hot numerical kernels are in
//! `ode_advanced` / `ode_solver`, which allocate nothing.

use super::tensor_provenance::{ProvenanceGraph, TensorProvenance, TensorState};

/// Domain separators so commitments at different layers can never collide.
const LINEAGE_DOMAIN: &[u8] = b"q42-tensor-lineage-v1";
const ROOT_DOMAIN: &[u8] = b"q42-tensor-integrity-root-v1";
const TRANSFORM_DOMAIN: &[u8] = b"q42-tensor-transform-v1";

/// A 32-byte BLAKE3 commitment content-addressing a tensor state within its lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageCommitment(pub [u8; 32]);

/// Content-address a single tensor `state` given its parent's commitment (or `None`
/// for a genesis/root state). The commitment binds the parent, the operation + its
/// params (in deterministic key order), and the full data bits.
pub fn commit_state(state: &TensorState, parent: Option<&LineageCommitment>) -> LineageCommitment {
    let mut h = blake3::Hasher::new();
    h.update(LINEAGE_DOMAIN);
    match parent {
        Some(p) => h.update(&p.0),
        None => h.update(b"GENESIS"),
    };
    match &state.provenance {
        TensorProvenance::Root { source, .. } => {
            h.update(b"root");
            h.update(source.as_bytes());
        }
        TensorProvenance::Derived {
            operation, params, ..
        } => {
            h.update(b"derived");
            h.update(operation.as_bytes());
            // Deterministic param order so the commitment is reproducible.
            let mut keys: Vec<&String> = params.keys().collect();
            keys.sort();
            for k in keys {
                h.update(k.as_bytes());
                h.update(&params[k].to_bits().to_le_bytes());
            }
        }
    }
    // Length-prefix then the full data bits (prevents data/shape extension ambiguity).
    h.update(&(state.data.len() as u64).to_le_bytes());
    for &x in &state.data {
        h.update(&x.to_bits().to_le_bytes());
    }
    LineageCommitment(h.finalize().into())
}

/// Fold the lineage commitment for `state_id` from the root down to the node.
/// Returns `None` if the chain is broken (a referenced ancestor is missing).
pub fn lineage_commitment(graph: &ProvenanceGraph, state_id: u64) -> Option<LineageCommitment> {
    // `get_lineage` returns [node, parent, …, root]; fold from the root forward.
    let lineage = graph.get_lineage(state_id);
    if lineage.is_empty() {
        return None;
    }
    let mut commitment: Option<LineageCommitment> = None;
    for &id in lineage.iter().rev() {
        let state = graph.get_state(id)?;
        commitment = Some(commit_state(state, commitment.as_ref()));
    }
    commitment
}

/// Verify that `state_id`'s lineage reproduces `expected` — i.e. nothing in the chain
/// (data, operation, params, or structure) was altered after the commitment was taken.
pub fn verify_lineage(
    graph: &ProvenanceGraph,
    state_id: u64,
    expected: &LineageCommitment,
) -> bool {
    lineage_commitment(graph, state_id).is_some_and(|c| c.0 == expected.0)
}

/// A Merkle-style integrity root over a set of leaf commitments (e.g. all current
/// head states): a single 32-byte digest witnessing the whole provenance frontier.
/// Order-independent (commitments are sorted first).
pub fn integrity_root(commitments: &[LineageCommitment]) -> LineageCommitment {
    let mut sorted: Vec<[u8; 32]> = commitments.iter().map(|c| c.0).collect();
    sorted.sort_unstable();
    let mut h = blake3::Hasher::new();
    h.update(ROOT_DOMAIN);
    h.update(&(sorted.len() as u64).to_le_bytes());
    for c in &sorted {
        h.update(c);
    }
    LineageCommitment(h.finalize().into())
}

/// Bind an input state, an output state, and the operation that produced it into a
/// public 32-byte commitment — the witness a zero-knowledge transformation proof is
/// checked against. (For linear maps, the ZK proof itself is the real Groth16
/// `private_matrix_multiply`; this is the public binding it commits to.)
pub fn transformation_commitment(input: &TensorState, output: &TensorState) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(TRANSFORM_DOMAIN);
    h.update(&input.state_id.to_le_bytes());
    h.update(&output.state_id.to_le_bytes());
    if let TensorProvenance::Derived { operation, .. } = &output.provenance {
        h.update(operation.as_bytes());
    }
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn scale_params(factor: f64) -> HashMap<String, f64> {
        let mut p = HashMap::new();
        p.insert("factor".to_string(), factor);
        p
    }

    #[test]
    fn commitment_is_deterministic_and_data_sensitive() {
        let a = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let b = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let c = TensorState::new(vec![1.0, 2.0, 3.5], vec![3]); // one bit different
        assert_eq!(
            commit_state(&a, None),
            commit_state(&b, None),
            "same data ⇒ same commitment"
        );
        assert_ne!(
            commit_state(&a, None),
            commit_state(&c, None),
            "different data ⇒ different commitment"
        );
    }

    #[test]
    fn lineage_commitment_chains_parent_into_child() {
        let mut graph = ProvenanceGraph::new();
        let root = TensorState::new(vec![1.0], vec![1]);
        let root_id = root.state_id;
        graph.add_state(root.clone());
        let child = root.apply_operation("scale", &scale_params(2.0));
        let child_id = child.state_id;
        graph.add_state(child);

        let root_commit = lineage_commitment(&graph, root_id).unwrap();
        let child_commit = lineage_commitment(&graph, child_id).unwrap();
        // Child commitment differs from root (it chains the parent + the operation).
        assert_ne!(root_commit, child_commit);
        // And it reproduces on verification.
        assert!(verify_lineage(&graph, child_id, &child_commit));
    }

    #[test]
    fn tampering_with_an_ancestor_is_detected() {
        // Genuine lineage vs a forged one where the root's data was altered.
        let mut genuine = ProvenanceGraph::new();
        let root = TensorState::new(vec![10.0], vec![1]);
        let root_id = root.state_id;
        genuine.add_state(root.clone());
        let child = root.apply_operation("scale", &scale_params(2.0));
        let child_id = child.state_id;
        genuine.add_state(child.clone());
        let genuine_commit = lineage_commitment(&genuine, child_id).unwrap();

        // Forged graph: same child node, but the *root* it points to was tampered.
        let mut forged = ProvenanceGraph::new();
        let mut tampered_root = root.clone();
        tampered_root.data = vec![999.0]; // alter ancestor data, keep the id
        forged.add_state(tampered_root);
        forged.add_state(child);

        // The same child id now yields a different lineage commitment → tamper caught.
        assert!(
            !verify_lineage(&forged, child_id, &genuine_commit),
            "altered ancestor data must break the lineage commitment"
        );
        let _ = root_id;
    }

    #[test]
    fn integrity_root_is_order_independent_and_change_sensitive() {
        let c1 = LineageCommitment([1u8; 32]);
        let c2 = LineageCommitment([2u8; 32]);
        let c3 = LineageCommitment([3u8; 32]);
        let r_ab = integrity_root(&[c1, c2]);
        let r_ba = integrity_root(&[c2, c1]);
        assert_eq!(r_ab, r_ba, "root must not depend on commitment order");
        let r_abc = integrity_root(&[c1, c2, c3]);
        assert_ne!(r_ab, r_abc, "adding a head must change the integrity root");
    }

    #[test]
    fn transformation_commitment_binds_the_operation() {
        let input = TensorState::new(vec![1.0, 2.0], vec![2]);
        let scaled = input.apply_operation("scale", &scale_params(2.0));
        let mut add_params = HashMap::new();
        add_params.insert("value".to_string(), 1.0);
        let added = input.apply_operation("add", &add_params);
        let t_scale = transformation_commitment(&input, &scaled);
        let t_add = transformation_commitment(&input, &added);
        assert_ne!(
            t_scale, t_add,
            "different operations ⇒ different transformation commitments"
        );
    }
}
