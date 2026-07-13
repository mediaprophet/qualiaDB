//! Temporal scrub and frame-diff API for reconstructing the world state
//! at a given historical coordinate, supporting Phase 3 replay-to-state
//! and continuous spawn/decay validation.

use crate::git_bridge::DagStore;
use crate::NQuin;

/// Replay mutations up to `as_of_ms` and return the materialized `NQuin` state.
///
/// Uses `resolver` to map from a DagNode's `quins_merkle` to the actual `NQuin` block.
pub fn replay_to_state<'a, F>(dag: &'a DagStore, as_of_ms: u64, resolver: F) -> Vec<NQuin>
where
    F: Fn([u8; 32]) -> Option<Vec<NQuin>>,
{
    let mut state = Vec::new();

    // Iterate nodes in topological order (insertion order) up to as_of_ms
    for (node, _) in dag.nodes().iter().filter(|(n, _)| n.timestamp <= as_of_ms) {
        if let Some(mutations) = resolver(node.quins_merkle) {
            for quin in mutations {
                state.push(quin);
            }
        }
    }

    state
}

/// Compute the frame-diff hints between two timestamps `t0` and `t1`.
///
/// Returns `(added_quins, removed_quins)` to tell the renderer exactly what changed.
pub fn frame_diff_hints<'a, F>(
    dag: &'a DagStore,
    t0: u64,
    t1: u64,
    resolver: F,
) -> (Vec<NQuin>, Vec<NQuin>)
where
    F: Fn([u8; 32]) -> Option<Vec<NQuin>>,
{
    let mut added = Vec::new();
    let mut removed = Vec::new();

    // To properly diff, we normally just take the mutations between t0 and t1.
    // If the DAG nodes between t0 and t1 contain additions/removals, we extract them.
    // Assuming monotonic additions for now since Q42 mutations are append-only.
    // Invalidations are tombstoned logically rather than deleted physically.
    let min_t = t0.min(t1);
    let max_t = t0.max(t1);

    for (node, _) in dag.nodes().iter() {
        if node.timestamp > min_t && node.timestamp <= max_t {
            if let Some(mutations) = resolver(node.quins_merkle) {
                for quin in mutations {
                    if t1 >= t0 {
                        added.push(quin); // Rolling forward
                    } else {
                        removed.push(quin); // Rolling backward
                    }
                }
            }
        }
    }

    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_to_state() {
        let mut dag = DagStore::new();
        let q1 = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let q2 = NQuin {
            subject: 4,
            predicate: 5,
            object: 6,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        dag.commit_node([0u8; 32], &[q1.clone()], 123, 1000, "commit1");
        dag.commit_node([0u8; 32], &[q2.clone()], 123, 2000, "commit2");

        let resolver = |merkle: [u8; 32]| -> Option<Vec<NQuin>> {
            if merkle == crate::git_bridge::quins_merkle(&[q1.clone()]) {
                Some(vec![q1.clone()])
            } else if merkle == crate::git_bridge::quins_merkle(&[q2.clone()]) {
                Some(vec![q2.clone()])
            } else {
                None
            }
        };

        let state_at_1500 = replay_to_state(&dag, 1500, &resolver);
        assert_eq!(state_at_1500.len(), 1);
        assert_eq!(state_at_1500[0].subject, 1);

        let state_at_2500 = replay_to_state(&dag, 2500, &resolver);
        assert_eq!(state_at_2500.len(), 2);
    }

    #[test]
    fn test_frame_diff_hints() {
        let mut dag = DagStore::new();
        let q1 = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let q2 = NQuin {
            subject: 4,
            predicate: 5,
            object: 6,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        dag.commit_node([0u8; 32], &[q1.clone()], 123, 1000, "commit1");
        dag.commit_node([0u8; 32], &[q2.clone()], 123, 2000, "commit2");

        let resolver = |merkle: [u8; 32]| -> Option<Vec<NQuin>> {
            if merkle == crate::git_bridge::quins_merkle(&[q1.clone()]) {
                Some(vec![q1.clone()])
            } else if merkle == crate::git_bridge::quins_merkle(&[q2.clone()]) {
                Some(vec![q2.clone()])
            } else {
                None
            }
        };

        // Forward scrub
        let (added, removed) = frame_diff_hints(&dag, 1500, 2500, &resolver);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].subject, 4);
        assert_eq!(removed.len(), 0);

        // Backward scrub
        let (added, removed) = frame_diff_hints(&dag, 2500, 1500, &resolver);
        assert_eq!(added.len(), 0);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].subject, 4);
    }
}
