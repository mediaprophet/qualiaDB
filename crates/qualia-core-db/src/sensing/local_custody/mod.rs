//! Cold, caller-budgeted local custody bundle publication and reading.
//!
//! The host must enforce locality, directory ownership, exclusive chain
//! coordination, current authorization and aggregate quotas. Atomic publication
//! is one non-replacing bundle link, not a transactional chain-head update. A successful
//! link does not prove directory durability or prevent rollback. Mounts, mapped
//! drives, reparse points and filesystem behavior require platform qualification.
//! Reads reject final-component links/reparse points on Unix and Windows; this
//! does not guard root ancestors or establish that a volume is local. Unix
//! nonblocking open avoids FIFO hangs but cannot prevent device-node effects.

pub mod chain;
pub mod local_head;
pub mod local_head_adapter;
pub mod local_recovery;
mod open_checked;
mod publish;
pub mod quota;
mod read;
pub mod recovery;

pub use publish::{publish_bundle, PublishedBundle};
pub use read::{read_bundle, ReadBundle};

use super::sidecar_attestation::{
    SidecarReceipt, SidecarReceiptAuthority, SidecarReceiptError, SIDECAR_BYTES,
    SIDECAR_RECEIPT_BYTES,
};
use std::path::Path;

const HEADER_BYTES: usize = 16;
const FIXED_BYTES: usize = HEADER_BYTES + SIDECAR_BYTES + SIDECAR_RECEIPT_BYTES;
const MAGIC: &[u8; 4] = b"QCB1";

/// Host checks are trust boundaries, not OS-enforced locality or locks. The
/// caller must hold an exclusive chain lease across the complete operation.
pub trait CustodyAuthority: SidecarReceiptAuthority {
    /// Admit the root and budget, enforce locality, directory ownership and
    /// restrictive traversal/read ACLs (including Windows file inheritance),
    /// and verify the exclusive chain lease and current Store policy.
    fn permit_publish(
        &mut self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        total_bytes: usize,
    ) -> bool;

    /// Admit reading this exact checkpoint under current policy before any I/O.
    fn permit_read(&mut self, root: &Path, checkpoint: &[u8; 32], max_bytes: usize) -> bool;
}

#[derive(Debug)]
pub enum CustodyError {
    InvalidRoot,
    InvalidBudget,
    TooLarge,
    PermissionDenied,
    InvalidEncoding,
    CheckpointMismatch,
    Receipt(SidecarReceiptError),
    Io(std::io::Error),
}

impl From<std::io::Error> for CustodyError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

fn entry_size(payload_len: usize, budget: usize) -> Result<usize, CustodyError> {
    if budget < FIXED_BYTES {
        return Err(CustodyError::InvalidBudget);
    }
    let total = FIXED_BYTES
        .checked_add(payload_len)
        .ok_or(CustodyError::TooLarge)?;
    if total > budget {
        return Err(CustodyError::TooLarge);
    }
    Ok(total)
}

fn validate_root(root: &Path) -> Result<(), CustodyError> {
    if !root.is_absolute() || root.as_os_str().len() > 4096 {
        return Err(CustodyError::InvalidRoot);
    }
    if root
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(CustodyError::InvalidRoot);
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(root.components().next(),
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(CustodyError::InvalidRoot);
        }
    }
    Ok(())
}

fn filename(checkpoint: &[u8; 32]) -> [u8; 68] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut name = [0; 68];
    for (i, value) in checkpoint.iter().enumerate() {
        name[2 * i] = HEX[(value >> 4) as usize];
        name[2 * i + 1] = HEX[(value & 15) as usize];
    }
    name[64..].copy_from_slice(b".qcb");
    name
}

#[cfg(test)]
mod tests;
