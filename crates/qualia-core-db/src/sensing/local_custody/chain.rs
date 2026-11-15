//! Chain-aware publication using host-supplied authenticated head state.
//!
//! This module cannot itself provide a durable head, lease, rollback defence,
//! or atomic compare-and-swap. The host must supply and qualify those services.
//! A bundle may be published without its head being committed; such an orphan
//! requires host-controlled recovery and must not be treated as a chain member.
use std::path::Path;

use super::{
    entry_size, publish_bundle, validate_root, CustodyAuthority, CustodyError, PublishedBundle,
    SidecarReceipt, SIDECAR_BYTES,
};

/// Current chain state asserted by a host-protected, anti-rollback registry.
/// `Empty` must mean authenticated absence, not an unavailable lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainHead {
    Empty,
    Present { sequence: u64, checkpoint: [u8; 32] },
}

/// Result reported by the host's atomic head update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadCommit {
    Committed,
    /// Another head won or the expected head no longer matches.
    Conflict,
    /// The host cannot say whether its update committed. Reconcile before use.
    Indeterminate,
}

/// A failure to obtain current authenticated chain state is not an empty chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainAuthorityError {
    Unavailable,
    NotAuthorized,
    InvalidState,
}

/// Host boundary for an exact root and chain. Implementations must provide an
/// authenticated, complete, anti-rollback head and maintain the exclusive
/// chain lease through publication and commit. `compare_and_commit` must be
/// atomic with respect to competing writers and check current Store policy;
/// the host must hold a policy lease through the complete head update so
/// expiry and revocation checks do not race a blocking commit;
/// on uncertain completion it must return `Indeterminate`, never `Committed`.
/// Durability and trusted freshness are host obligations, not checked here.
pub trait ChainAuthority: CustodyAuthority {
    fn authenticated_head(
        &mut self,
        root: &Path,
        chain_id: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError>;

    fn compare_and_commit(
        &mut self,
        root: &Path,
        chain_id: &[u8; 32],
        expected: ChainHead,
        next: ChainHead,
        receipt: &SidecarReceipt,
    ) -> HeadCommit;
}

#[derive(Debug)]
pub enum ChainPublicationError {
    Authority(ChainAuthorityError),
    InvalidHead,
    WrongSequence,
    WrongPredecessor,
    SequenceExhausted,
    Publication(CustodyError),
    /// The bundle was linked but the head did not commit or its result is
    /// uncertain. Host recovery must inspect authenticated durable state.
    PublishedUncommitted {
        bundle: PublishedBundle,
        outcome: HeadCommit,
    },
}

/// Result is based on the host's commit assertion, not local proof of durable
/// head state or directory synchronization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainPublication {
    pub bundle: PublishedBundle,
    pub head: ChainHead,
}

fn expected_transition(
    head: ChainHead,
    receipt: &SidecarReceipt,
) -> Result<ChainHead, ChainPublicationError> {
    if receipt.chain_id().iter().all(|byte| *byte == 0) {
        return Err(ChainPublicationError::InvalidHead);
    }
    match head {
        ChainHead::Empty => {
            if receipt.sequence() != 0 {
                return Err(ChainPublicationError::WrongSequence);
            }
            if receipt.previous().iter().any(|byte| *byte != 0) {
                return Err(ChainPublicationError::WrongPredecessor);
            }
        }
        ChainHead::Present {
            sequence,
            checkpoint,
        } => {
            if checkpoint.iter().all(|byte| *byte == 0) {
                return Err(ChainPublicationError::InvalidHead);
            }
            let next = sequence
                .checked_add(1)
                .ok_or(ChainPublicationError::SequenceExhausted)?;
            if receipt.sequence() != next {
                return Err(ChainPublicationError::WrongSequence);
            }
            if receipt.previous() != &checkpoint {
                return Err(ChainPublicationError::WrongPredecessor);
            }
        }
    }
    let checkpoint = receipt.checkpoint();
    if checkpoint.iter().all(|byte| *byte == 0) {
        return Err(ChainPublicationError::InvalidHead);
    }
    Ok(ChainHead::Present {
        sequence: receipt.sequence(),
        checkpoint,
    })
}

/// Verify and publish a bundle against an authenticated head, then ask the host
/// for atomic compare-and-commit. Store policy, locality and receipt authority
/// are also checked by `publish_bundle`. A publish error, panic, process crash,
/// or indeterminate head response can leave artifacts or an uncertain outcome;
/// callers must reconcile using the host's authenticated recovery protocol.
#[allow(clippy::too_many_arguments)]
pub fn publish_chain_bundle(
    root: &Path,
    sidecar: &[u8; SIDECAR_BYTES],
    payload: &[u8],
    receipt: &SidecarReceipt,
    byte_budget: usize,
    authority: &mut impl ChainAuthority,
) -> Result<ChainPublication, ChainPublicationError> {
    validate_root(root).map_err(ChainPublicationError::Publication)?;
    let total =
        entry_size(payload.len(), byte_budget).map_err(ChainPublicationError::Publication)?;
    receipt
        .verify_with_authority(sidecar, payload, payload.len(), authority)
        .map_err(|error| ChainPublicationError::Publication(CustodyError::Receipt(error)))?;
    if !authority.permit_publish(root, receipt, sidecar, payload, total) {
        return Err(ChainPublicationError::Publication(
            CustodyError::PermissionDenied,
        ));
    }
    let expected = authority
        .authenticated_head(root, receipt.chain_id())
        .map_err(ChainPublicationError::Authority)?;
    let next = expected_transition(expected, receipt)?;
    let bundle = publish_bundle(root, sidecar, payload, receipt, byte_budget, authority)
        .map_err(ChainPublicationError::Publication)?;
    let outcome = authority.compare_and_commit(root, receipt.chain_id(), expected, next, receipt);
    match outcome {
        HeadCommit::Committed => Ok(ChainPublication { bundle, head: next }),
        HeadCommit::Conflict | HeadCommit::Indeterminate => {
            Err(ChainPublicationError::PublishedUncommitted { bundle, outcome })
        }
    }
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
