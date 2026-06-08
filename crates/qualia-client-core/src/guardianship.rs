//! Bilateral guardianship — global suspended-transaction queue for M:N co-signature.

use std::sync::{Mutex, OnceLock};

use qualia_core_db::crdt::SuspendedTransactionQueue;
use qualia_core_db::wal::WriteAheadLog;
use qualia_core_db::{q_hash, QualiaQuin};
use serde::{Deserialize, Serialize};

const MAX_RATIFIED: usize = 32;

static SUSPENDED_QUEUE: OnceLock<Mutex<SuspendedTransactionQueue>> = OnceLock::new();
static RATIFIED_IDS: OnceLock<Mutex<[Option<u64>; MAX_RATIFIED]>> = OnceLock::new();

pub fn suspended_queue() -> &'static Mutex<SuspendedTransactionQueue> {
    SUSPENDED_QUEUE.get_or_init(|| Mutex::new(SuspendedTransactionQueue::new()))
}

fn ratified_ids() -> &'static Mutex<[Option<u64>; MAX_RATIFIED]> {
    RATIFIED_IDS.get_or_init(|| Mutex::new([None; MAX_RATIFIED]))
}

fn mark_ratified(agreement_id: u64) {
    let mut slots = ratified_ids().lock().expect("ratified_ids");
    if slots.iter().any(|s| *s == Some(agreement_id)) {
        return;
    }
    for slot in slots.iter_mut() {
        if slot.is_none() {
            *slot = Some(agreement_id);
            return;
        }
    }
}

/// View of a suspended guardianship transaction for UI trays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendedTxView {
    pub agreement_id: u64,
    pub threshold: u8,
    pub collected_signatures: u8,
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianTokenOutcome {
    Ratified,
    Pending,
    Denied,
    NotFound,
}

pub fn list_pending_affirmations() -> Vec<SuspendedTxView> {
    let guard = suspended_queue().lock().expect("suspended_queue");
    let mut out = Vec::new();
    for slot in guard.queue.iter() {
        if let Some(tx) = slot {
            let q = tx.suspended_quin;
            out.push(SuspendedTxView {
                agreement_id: tx.agreement_id,
                threshold: tx.threshold,
                collected_signatures: tx.collected_signatures,
                subject: q.subject,
                predicate: q.predicate,
                object: q.object,
                context: q.context,
                metadata: q.metadata,
                label: format!(
                    "Guardianship Proposal — agreement 0x{:016x}",
                    tx.agreement_id
                ),
            });
        }
    }
    out
}

pub fn pending_affirmation_count() -> usize {
    list_pending_affirmations().len()
}

pub fn is_agreement_ratified(agreement_id: u64) -> bool {
    ratified_ids()
        .lock()
        .expect("ratified_ids")
        .iter()
        .any(|s| *s == Some(agreement_id))
}

/// Apply a guardian consent token (`q42:issuesConsentToken`) for the given agreement.
pub fn apply_guardian_token(agreement_id: u64, token_fields: [u64; 6]) -> GuardianTokenOutcome {
    let token = QualiaQuin {
        subject: token_fields[0],
        predicate: token_fields[1],
        object: token_fields[2],
        context: token_fields[3],
        metadata: token_fields[4],
        parity: token_fields[5],
    };
    if token.context != agreement_id {
        return GuardianTokenOutcome::NotFound;
    }

    let mut guard = suspended_queue().lock().expect("suspended_queue");
    if let Some(tx) = guard.apply_consensus_token(&token) {
        mark_ratified(agreement_id);
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(mut wal) = WriteAheadLog::open(".qualia_graph_mutations.wal") {
                let mut quin = tx.suspended_quin;
                let _ = wal.append_mutation_volatile(&mut quin);
            }
        }
        GuardianTokenOutcome::Ratified
    } else if guard.queue.iter().any(|s| {
        s.as_ref()
            .map(|tx| tx.agreement_id == agreement_id)
            .unwrap_or(false)
    }) {
        GuardianTokenOutcome::Pending
    } else {
        GuardianTokenOutcome::NotFound
    }
}

/// Build a consent token for the local principal co-signing an agreement.
pub fn build_consent_token(agreement_id: u64, principal_hash: u64) -> [u64; 6] {
    let q = QualiaQuin {
        subject: principal_hash,
        predicate: q_hash("q42:issuesConsentToken"),
        object: agreement_id,
        context: agreement_id,
        metadata: 0,
        parity: 0,
    };
    [
        q.subject, q.predicate, q.object, q.context, q.metadata, q.parity,
    ]
}

/// Remove a suspended transaction without committing to the WAL.
pub fn deny_guardian_affirmation(agreement_id: u64) -> bool {
    let mut guard = suspended_queue().lock().expect("suspended_queue");
    for slot in guard.queue.iter_mut() {
        if let Some(tx) = slot {
            if tx.agreement_id == agreement_id {
                *slot = None;
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::crdt::SuspendedTransaction;

    #[test]
    fn list_and_deny_pending() {
        let queue = suspended_queue();
        let mut guard = queue.lock().unwrap();
        *guard = SuspendedTransactionQueue::new();
        drop(guard);

        let tx = SuspendedTransaction {
            agreement_id: 0xABCD,
            threshold: 2,
            collected_signatures: 1,
            registers: [None; 16],
            bytecode_buffer: [None; 64],
            yielded_op: None,
            suspended_quin: QualiaQuin::default(),
        };
        suspended_queue().lock().unwrap().push(tx).unwrap();

        assert_eq!(pending_affirmation_count(), 1);
        assert!(deny_guardian_affirmation(0xABCD));
        assert_eq!(pending_affirmation_count(), 0);
    }
}
