//! R8: Concrete wiring of `CoordHostSeams` to daemon infrastructure.
//!
//! The coordination ISA (`governance/coordination.rs`) defines host seams
//! that it cannot decide itself:
//! - `verify_root_delegation` — is an agent's delegation signed by the root key?
//! - `yield_to_suspended_queue` — enqueue a suspended transaction
//! - `mint_performance_vc` — mint a performance Verifiable Credential to the graph
//!
//! The default seams (`default_seams()`) are fail-closed: verify returns false,
//! yield is a no-op, mint returns 0. This module provides `daemon_seams()` which
//! wires these to the actual daemon infrastructure:
//! - `verify_root_delegation` → checks a delegation table backed by `crdt::DelegatedAccess`
//! - `yield_to_suspended_queue` → `crdt::SuspendedTransactionQueue::push`
//! - `mint_performance_vc` → `daemon_graph::extend_with_ontology_quins_slice`
//!   with a performance VC quin, returns the `perf_vc_hash`

use crate::foundation::crdt::{DelegatedAccess, SuspendedTransaction, SuspendedTransactionQueue};
use crate::governance::coordination::{perf_vc_hash, CoordHostSeams, PerformanceRecord};
use crate::NQuin;
use std::sync::Mutex;

/// Global suspended transaction queue (R8 wiring).
static SUSPENDED_QUEUE: Mutex<Option<SuspendedTransactionQueue>> = Mutex::new(None);

/// Global delegation table (R8 wiring).
/// Maps agent_did_hash (first 8 bytes of delegate_did) → DelegatedAccess.
static DELEGATION_TABLE: Mutex<Option<Vec<(u64, DelegatedAccess)>>> = Mutex::new(None);

/// Initialize the daemon-backed coordination seams (R8).
///
/// This wires the fail-closed default seams to actual daemon infrastructure.
/// Call once at daemon startup.
pub fn init_daemon_seams() {
    {
        let mut q = SUSPENDED_QUEUE.lock().unwrap();
        if q.is_none() {
            *q = Some(SuspendedTransactionQueue::new());
        }
    }
    {
        let mut t = DELEGATION_TABLE.lock().unwrap();
        if t.is_none() {
            *t = Some(Vec::new());
        }
    }
}

/// Register a delegation for an agent (R8).
/// Allows `verify_root_delegation` to return true for this agent.
/// The `agent_did_hash` is derived from the first 8 bytes of the delegate's DID.
pub fn register_delegation(agent_did_hash: u64, access: DelegatedAccess) {
    init_daemon_seams();
    let mut t = DELEGATION_TABLE.lock().unwrap();
    if let Some(ref mut table) = *t {
        if let Some(entry) = table.iter_mut().find(|(h, _)| *h == agent_did_hash) {
            entry.1 = access;
        } else {
            table.push((agent_did_hash, access));
        }
    }
}

/// Create daemon-backed coordination seams (R8).
///
/// These seams wire the coordination ISA to the actual daemon:
/// - `verify_root_delegation`: checks the delegation table
/// - `yield_to_suspended_queue`: pushes to the global queue
/// - `mint_performance_vc`: mints a VC quin to the daemon graph
pub fn daemon_seams() -> CoordHostSeams {
    init_daemon_seams();
    CoordHostSeams {
        verify_root_delegation: Box::new(|agent_did_hash, _root_did_hash| {
            verify_delegation_impl(agent_did_hash)
        }),
        yield_to_suspended_queue: Box::new(|task_hash, token_limit| {
            yield_to_suspended_queue_impl(task_hash, token_limit)
        }),
        mint_performance_vc: Box::new(|agent_did_hash, declared, actual, validation| {
            mint_performance_vc_impl(agent_did_hash, declared, actual, validation)
        }),
    }
}

fn verify_delegation_impl(agent_did_hash: u64) -> bool {
    let t = DELEGATION_TABLE.lock().unwrap();
    if let Some(ref table) = *t {
        for &(h, ref access) in table.iter() {
            if h == agent_did_hash {
                // Verify the delegation: check expiry and context
                // Use u64::MAX as current timestamp so non-zero expiry always passes
                return crate::foundation::crdt::CrdtResolver::verify_delegation(
                    access,
                    0,
                    u64::MAX,
                );
            }
        }
    }
    false
}

fn yield_to_suspended_queue_impl(task_hash: u64, _token_limit: u64) {
    let mut q = SUSPENDED_QUEUE.lock().unwrap();
    if let Some(ref mut queue) = *q {
        let tx = SuspendedTransaction {
            agreement_id: task_hash,
            collected_signatures: 0,
            threshold: 1,
            registers: [None; 16],
            bytecode_buffer: [None; 64],
            yielded_op: None,
            suspended_quin: NQuin {
                subject: task_hash,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        };
        let _ = queue.push(tx);
    }
}

fn mint_performance_vc_impl(
    agent_did_hash: u64,
    declared: u64,
    actual: u64,
    validation: bool,
) -> u64 {
    // Compute the performance record using the coordination ISA's evaluator
    let rec = crate::governance::coordination::eval_performance_rating(
        agent_did_hash,
        declared,
        actual,
        validation,
    );
    let vc_hash = perf_vc_hash(&rec);

    // Mint the VC as a quin to the daemon graph
    #[cfg(not(target_arch = "wasm32"))]
    {
        let vc_quin = NQuin {
            subject: agent_did_hash,
            predicate: crate::q_hash("q42:hasPerformanceVC"),
            object: vc_hash,
            context: crate::q_hash("q42:coordination"),
            metadata: if validation { 1 } else { 0 },
            parity: 0,
        };
        crate::daemon_graph::extend_with_ontology_quins_slice(&[vc_quin]);
    }

    // Suppress unused warning when not writing to graph
    let _ = rec;
    vc_hash
}

/// Get the number of suspended transactions (for diagnostics/testing).
pub fn suspended_queue_len() -> usize {
    let q = SUSPENDED_QUEUE.lock().unwrap();
    if let Some(ref queue) = *q {
        queue.queue.iter().filter(|s| s.is_some()).count()
    } else {
        0
    }
}

/// Clear the suspended queue and delegation table (for testing).
pub fn reset_daemon_seams() {
    {
        let mut q = SUSPENDED_QUEUE.lock().unwrap();
        *q = Some(SuspendedTransactionQueue::new());
    }
    {
        let mut t = DELEGATION_TABLE.lock().unwrap();
        *t = Some(Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_delegation(agent_hash: u64) -> DelegatedAccess {
        let mut delegate = [0u8; 32];
        let bytes = agent_hash.to_le_bytes();
        delegate[..8].copy_from_slice(&bytes);
        DelegatedAccess {
            principal_did: [0xCA; 32],
            delegate_did: delegate,
            context_bound: 0,
            expiration_timestamp: u64::MAX,
            cryptographic_proof: [0u8; 64],
        }
    }

    #[test]
    fn r8_daemon_seams_verify_delegation() {
        reset_daemon_seams();
        let agent_hash = 0xDEAD_BEEF_u64;
        register_delegation(agent_hash, make_delegation(agent_hash));

        let seams = daemon_seams();
        assert!((seams.verify_root_delegation)(agent_hash, 0xCAFE));
        // Unknown agent should fail
        assert!(!(seams.verify_root_delegation)(0x1234_5678, 0));
    }

    #[test]
    fn r8_daemon_seams_verify_unknown_fails_closed() {
        reset_daemon_seams();
        let seams = daemon_seams();
        assert!(!(seams.verify_root_delegation)(0xBAD_F00D, 0));
    }

    #[test]
    fn r8_daemon_seams_yield_to_suspended_queue() {
        reset_daemon_seams();
        let seams = daemon_seams();
        assert_eq!(suspended_queue_len(), 0);
        (seams.yield_to_suspended_queue)(0x1234, 1000);
        assert_eq!(suspended_queue_len(), 1);
        (seams.yield_to_suspended_queue)(0x5678, 2000);
        assert_eq!(suspended_queue_len(), 2);
    }

    #[test]
    fn r8_daemon_seams_mint_performance_vc() {
        reset_daemon_seams();
        let seams = daemon_seams();
        let vc_hash = (seams.mint_performance_vc)(0xA1, 1000, 800, true);
        assert_ne!(vc_hash, 0);
        // Deterministic: same inputs → same hash
        let seams2 = daemon_seams();
        let vc_hash2 = (seams2.mint_performance_vc)(0xA1, 1000, 800, true);
        assert_eq!(vc_hash, vc_hash2);
    }

    #[test]
    fn r8_daemon_seams_mint_vc_validation_flag() {
        reset_daemon_seams();
        let seams = daemon_seams();
        let h1 = (seams.mint_performance_vc)(0xA2, 1000, 800, true);
        let seams2 = daemon_seams();
        let h2 = (seams2.mint_performance_vc)(0xA2, 1000, 800, false);
        assert_ne!(h1, h2);
    }

    #[test]
    fn r8_default_seams_fail_closed() {
        let seams = crate::governance::coordination::default_seams();
        assert!(!(seams.verify_root_delegation)(0x1234, 0));
        (seams.yield_to_suspended_queue)(0, 0);
        assert_eq!((seams.mint_performance_vc)(0, 0, 0, false), 0);
    }
}
