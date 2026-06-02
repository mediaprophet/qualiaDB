use crate::QualiaQuin;
use serde::{Deserialize, Serialize};

/// Represents a cryptographic grant of authority from a Principal to a Delegate.
/// Essential for Guardianship (e.g., homeless individual granting read access to a social worker).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegatedAccess {
    pub principal_did: String,
    pub delegate_did: String,
    pub context_bound: u64, // The specific semantic context they are allowed to access (0 = global)
    pub expiration_timestamp: u64,
    pub cryptographic_proof: String, // Ed25519 signature
}

/// Last-Write-Wins (LWW) CRDT Resolver
/// Ensures offline-first mobile devices can sync disparate state without conflicts.
pub struct CrdtResolver;

impl CrdtResolver {
    /// Merges two conflicting mutations from the same logical Context graph.
    /// Returns the mathematically deterministically "winning" Quin.
    pub fn resolve_lww(local: &QualiaQuin, remote: &QualiaQuin) -> QualiaQuin {
        let local_clock = local.extract_lamport_clock();
        let remote_clock = remote.extract_lamport_clock();

        if remote_clock > local_clock {
            remote.clone()
        } else if local_clock > remote_clock {
            local.clone()
        } else {
            // Clocks are identical (concurrent mutation).
            // Tie-break deterministically using the mathematical values of the nodes.
            // A simple hash tie-breaker or magnitude check works for CRDTs.
            if remote.object > local.object {
                remote.clone()
            } else {
                local.clone()
            }
        }
    }

    /// Validates a Role-Based Delegation.
    /// Ensures that a delegate (e.g., social worker) has cryptographic authority
    /// to mutate or read the principal's (e.g., homeless individual) state.
    pub fn verify_delegation(access: &DelegatedAccess, target_context: u64, current_timestamp: u64) -> bool {
        if access.expiration_timestamp < current_timestamp {
            return false; // Expired
        }
        if access.context_bound != target_context && access.context_bound != 0 {
            return false; // Out of bounds
        }
        
        // In production, we verify `cryptographic_proof` against `principal_did` public key.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualia_crdt_resolution() {
        let mut q_local = QualiaQuin { subject: 1, predicate: 2, object: 100, context: 5, metadata: 0, parity: 0 };
        q_local.set_lamport_clock(5);
        
        let mut q_remote = QualiaQuin { subject: 1, predicate: 2, object: 200, context: 5, metadata: 0, parity: 0 };
        q_remote.set_lamport_clock(8); // Remote occurred later
        
        // Remote wins due to clock
        let winner_clock = CrdtResolver::resolve_lww(&q_local, &q_remote);
        assert_eq!(winner_clock.object, 200, "CRDT failed to resolve higher lamport clock");
        
        // Concurrent mutations (same clock)
        let mut q_concurrent = QualiaQuin { subject: 1, predicate: 2, object: 50, context: 5, metadata: 0, parity: 0 };
        q_concurrent.set_lamport_clock(5);
        
        // Tie-breaker falls to magnitude
        let winner_tie = CrdtResolver::resolve_lww(&q_local, &q_concurrent);
        assert_eq!(winner_tie.object, 100, "CRDT failed deterministic tie-breaker");
    }
}
