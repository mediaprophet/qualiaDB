//! Bounded, non-mutating admission of one published bundle during recovery.
//!
//! No directory scan, head update, deletion or repair is performed. The host
//! must hold a read/policy lease and supply current rollback-resistant head
//! state. Equal snapshots alone cannot prove a head was never rolled back.
use super::{
    chain::{ChainAuthorityError, ChainHead},
    entry_size, read_bundle, validate_root, CustodyAuthority, CustodyError, SidecarReceipt,
    SIDECAR_BYTES,
};
use std::path::Path;

/// A host-verified membership assertion relative to the exact supplied head.
/// The implementation must validate authenticated ancestry or exclusion proof;
/// missing proof is `Unknown`, not evidence of an orphan or committed member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipAssertion {
    CommittedAncestor,
    Excluded,
    Unknown,
}

pub trait RecoveryAuthority: CustodyAuthority {
    /// Authorize the exact checkpoint, chain metadata lookup and recovery read.
    /// Maintain root protection, chain snapshot and current policy through return.
    fn permit_recovery(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        checkpoint: &[u8; 32],
        byte_budget: usize,
    ) -> bool;
    fn recovery_head(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError>;
    fn membership(
        &mut self,
        root: &Path,
        head: ChainHead,
        receipt: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDisposition {
    /// Checkpoint and sequence equal the authenticated current head.
    CommittedAtHead,
    /// Host has validated authenticated ancestry under this exact head.
    CommittedAncestor,
    /// Host has validated exclusion; this does not authorize deletion or repair.
    PublishedUncommitted,
    /// A valid signed bundle without sufficient membership evidence.
    Uncertain,
}

pub struct RecoveryReport {
    pub receipt: SidecarReceipt,
    pub payload_bytes: usize,
    pub head: ChainHead,
    pub disposition: RecoveryDisposition,
}

#[derive(Debug)]
pub enum RecoveryError {
    Custody(CustodyError),
    Authority(ChainAuthorityError),
    WrongChain,
    InvalidHead,
    InvalidMembership,
    HeadChanged,
}

/// Read one exact checkpoint with caller-owned bounded buffers. Errors and
/// unwind clear both buffers, including errors after a successful bundle read.
/// A report is admission under the host's current snapshot, not a reusable
/// permission, proof of directory durability, or independent rollback defence.
pub fn inspect_recovery_bundle(
    root: &Path,
    chain: &[u8; 32],
    checkpoint: &[u8; 32],
    byte_budget: usize,
    sidecar: &mut [u8; SIDECAR_BYTES],
    payload: &mut [u8],
    authority: &mut impl RecoveryAuthority,
) -> Result<RecoveryReport, RecoveryError> {
    sidecar.fill(0);
    payload.fill(0);
    let mut buffers = RecoveryBuffers {
        sidecar,
        payload,
        keep: false,
    };
    let result = inspect_inner(
        root,
        chain,
        checkpoint,
        byte_budget,
        buffers.sidecar,
        buffers.payload,
        authority,
    );
    if result.is_ok() {
        buffers.keep = true;
    }
    result
}

fn inspect_inner(
    root: &Path,
    chain: &[u8; 32],
    checkpoint: &[u8; 32],
    byte_budget: usize,
    sidecar: &mut [u8; SIDECAR_BYTES],
    payload: &mut [u8],
    authority: &mut impl RecoveryAuthority,
) -> Result<RecoveryReport, RecoveryError> {
    validate_root(root).map_err(RecoveryError::Custody)?;
    entry_size(0, byte_budget).map_err(RecoveryError::Custody)?;
    if chain.iter().all(|v| *v == 0) || checkpoint.iter().all(|v| *v == 0) {
        return Err(RecoveryError::InvalidHead);
    }
    if !authority.permit_recovery(root, chain, checkpoint, byte_budget)
        || !authority.permit_read(root, checkpoint, byte_budget)
    {
        return Err(RecoveryError::Custody(CustodyError::PermissionDenied));
    }
    let head = authority
        .recovery_head(root, chain)
        .map_err(RecoveryError::Authority)?;
    validate_head(head)?;
    let read = read_bundle(root, checkpoint, byte_budget, sidecar, payload, authority)
        .map_err(RecoveryError::Custody)?;
    if read.receipt.chain_id() != chain {
        return Err(RecoveryError::WrongChain);
    }
    let assertion = if head
        == (ChainHead::Present {
            sequence: read.receipt.sequence(),
            checkpoint: *checkpoint,
        }) {
        MembershipAssertion::Unknown
    } else {
        authority
            .membership(root, head, &read.receipt)
            .map_err(RecoveryError::Authority)?
    };
    let disposition = classify(head, &read.receipt, assertion)?;
    let final_head = authority
        .recovery_head(root, chain)
        .map_err(RecoveryError::Authority)?;
    if final_head != head {
        return Err(RecoveryError::HeadChanged);
    }
    read.receipt
        .verify_with_authority(
            sidecar,
            &payload[..read.payload_bytes],
            read.payload_bytes,
            authority,
        )
        .map_err(|e| RecoveryError::Custody(CustodyError::Receipt(e)))?;
    if !authority.permit_recovery(root, chain, checkpoint, byte_budget)
        || !authority.permit_read(root, checkpoint, byte_budget)
    {
        return Err(RecoveryError::Custody(CustodyError::PermissionDenied));
    }
    Ok(RecoveryReport {
        receipt: read.receipt,
        payload_bytes: read.payload_bytes,
        head,
        disposition,
    })
}

fn validate_head(head: ChainHead) -> Result<(), RecoveryError> {
    if matches!(head, ChainHead::Present { checkpoint, .. } if checkpoint.iter().all(|v| *v == 0)) {
        return Err(RecoveryError::InvalidHead);
    }
    Ok(())
}

fn classify(
    head: ChainHead,
    receipt: &SidecarReceipt,
    assertion: MembershipAssertion,
) -> Result<RecoveryDisposition, RecoveryError> {
    validate_head(head)?;
    if head
        == (ChainHead::Present {
            sequence: receipt.sequence(),
            checkpoint: receipt.checkpoint(),
        })
    {
        return Ok(RecoveryDisposition::CommittedAtHead);
    }
    match assertion {
        MembershipAssertion::CommittedAncestor => match head {
            ChainHead::Present { sequence, .. } if receipt.sequence() < sequence => {
                Ok(RecoveryDisposition::CommittedAncestor)
            }
            _ => Err(RecoveryError::InvalidMembership),
        },
        MembershipAssertion::Excluded => Ok(RecoveryDisposition::PublishedUncommitted),
        MembershipAssertion::Unknown => Ok(RecoveryDisposition::Uncertain),
    }
}

struct RecoveryBuffers<'a> {
    sidecar: &'a mut [u8; SIDECAR_BYTES],
    payload: &'a mut [u8],
    keep: bool,
}
impl Drop for RecoveryBuffers<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.sidecar.fill(0);
            self.payload.fill(0);
        }
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
