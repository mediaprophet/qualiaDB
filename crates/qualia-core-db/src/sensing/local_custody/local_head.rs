//! Signed local chain heads under a held, cooperative OS file lock.
//!
//! The host must protect the root and lock file from replacement, establish
//! that storage is local, and authenticate freshness/absence with independent
//! anti-rollback state. Filesystem locks coordinate only cooperating writers.
//! This module does not establish durable chain membership on its own.
use super::super::sidecar_attestation::{SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES};
use super::{
    chain::{ChainHead, HeadCommit},
    open_checked::open_regular,
    validate_root,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const SIZE: usize = 176;
const MAGIC: &[u8; 4] = b"QCH1";
const DOMAIN: &[u8] = b"qualia:sensing:local-chain-head:v1\0";

/// External policy establishes local root/ACL and rollback-resistant head state.
/// The host must hold a policy lease across final checks and file replacement;
/// callback results alone cannot make external revocation atomic with I/O.
/// `Empty` means authenticated genesis absence, not an unavailable lookup.
pub trait LocalHeadPolicy: SidecarReceiptAuthority {
    fn permit_open(&mut self, root: &Path, chain: &[u8; 32]) -> bool;
    fn admit_current(&mut self, root: &Path, chain: &[u8; 32], head: ChainHead) -> bool;
    /// Check current Store policy and confirm the exact referenced bundle is
    /// published and admissible. This module does not perform that lookup.
    fn permit_commit(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        expected: ChainHead,
        next: ChainHead,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool;
}

#[derive(Debug)]
pub enum LocalHeadError {
    InvalidRoot,
    InvalidChain,
    InvalidBudget,
    PermissionDenied,
    InvalidEncoding,
    InvalidSigner,
    InvalidSignature,
    Receipt,
    Conflict,
    Io(std::io::Error),
}
impl From<std::io::Error> for LocalHeadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Holds an exclusive advisory lock. Root protection and lock-file identity
/// must be enforced by the host for the entire lifetime of the lease.
pub struct LocalHeadLease<'a> {
    root: PathBuf,
    chain: [u8; 32],
    head_path: PathBuf,
    _lock: File,
    signer: &'a SigningKey,
}

impl<'a> LocalHeadLease<'a> {
    /// Exact lexical scope of this lease; the host protects root identity for
    /// the whole lease and must not assume this checks filesystem aliases.
    pub fn matches_scope(&self, root: &Path, chain: &[u8; 32]) -> bool {
        self.root == root && &self.chain == chain
    }
    /// `byte_budget` bounds the single temporary head record, not aggregate
    /// directory usage or runtime/OS allocation. No implicit directories are
    /// created; the caller supplies an already protected root.
    pub fn acquire(
        root: &Path,
        chain: [u8; 32],
        signer: &'a SigningKey,
        byte_budget: usize,
        policy: &mut impl LocalHeadPolicy,
    ) -> Result<Self, LocalHeadError> {
        validate_root(root).map_err(|_| LocalHeadError::InvalidRoot)?;
        if chain.iter().all(|b| *b == 0) {
            return Err(LocalHeadError::InvalidChain);
        }
        if byte_budget < SIZE {
            return Err(LocalHeadError::InvalidBudget);
        }
        if !policy.permit_open(root, &chain) {
            return Err(LocalHeadError::PermissionDenied);
        }
        let mut name = [0u8; 69];
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for (i, byte) in chain.iter().enumerate() {
            name[i * 2] = HEX[(byte >> 4) as usize];
            name[i * 2 + 1] = HEX[(byte & 15) as usize];
        }
        name[64..].copy_from_slice(b".lock");
        let lock_path =
            root.join(std::str::from_utf8(&name).map_err(|_| LocalHeadError::InvalidEncoding)?);
        name[64..].copy_from_slice(b".head");
        let head_path =
            root.join(std::str::from_utf8(&name).map_err(|_| LocalHeadError::InvalidEncoding)?);
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000);
        }
        #[cfg(not(any(unix, windows)))]
        {
            return Err(LocalHeadError::InvalidRoot);
        }
        let lock = options.open(lock_path)?;
        let metadata = lock.metadata()?;
        if !metadata.is_file() {
            return Err(LocalHeadError::InvalidEncoding);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(LocalHeadError::InvalidEncoding);
            }
        }
        lock.try_lock().map_err(|_| LocalHeadError::Conflict)?;
        Ok(Self {
            root: root.to_owned(),
            chain,
            head_path,
            _lock: lock,
            signer,
        })
    }

    pub fn current(&self, policy: &mut impl LocalHeadPolicy) -> Result<ChainHead, LocalHeadError> {
        if !policy.permit_open(&self.root, &self.chain) {
            return Err(LocalHeadError::PermissionDenied);
        }
        let head = match open_regular(&self.head_path) {
            Ok(mut file) => {
                let mut bytes = [0u8; SIZE];
                file.read_exact(&mut bytes)?;
                let mut trailing = [0u8; 1];
                if file.read(&mut trailing)? != 0 {
                    return Err(LocalHeadError::InvalidEncoding);
                }
                self.decode(&bytes)?
            }
            Err(super::CustodyError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                ChainHead::Empty
            }
            Err(_) => return Err(LocalHeadError::InvalidEncoding),
        };
        if !policy.admit_current(&self.root, &self.chain, head) {
            return Err(LocalHeadError::PermissionDenied);
        }
        Ok(head)
    }

    /// Compare, sign and atomically replace a local head while holding the
    /// lease. A post-replacement failure yields `Indeterminate`, never success.
    /// The caller must have published the corresponding bundle and must use
    /// external rollback/freshness checks during recovery.
    #[allow(clippy::too_many_arguments)]
    pub fn compare_and_commit(
        &mut self,
        expected: ChainHead,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        payload_budget: usize,
        policy: &mut impl LocalHeadPolicy,
    ) -> Result<HeadCommit, LocalHeadError> {
        if receipt.chain_id() != &self.chain {
            return Err(LocalHeadError::InvalidChain);
        }
        receipt
            .verify_with_authority(sidecar, payload, payload_budget, policy)
            .map_err(|_| LocalHeadError::Receipt)?;
        if self.current(policy)? != expected {
            return Ok(HeadCommit::Conflict);
        }
        match expected {
            ChainHead::Empty
                if receipt.sequence() == 0 && receipt.previous().iter().all(|b| *b == 0) => {}
            ChainHead::Present {
                sequence,
                checkpoint,
            } if sequence.checked_add(1) == Some(receipt.sequence())
                && receipt.previous() == &checkpoint => {}
            _ => return Ok(HeadCommit::Conflict),
        }
        let next = ChainHead::Present {
            sequence: receipt.sequence(),
            checkpoint: receipt.checkpoint(),
        };
        if !policy.permit_commit(
            &self.root,
            &self.chain,
            expected,
            next,
            receipt,
            sidecar,
            payload,
        ) {
            return Err(LocalHeadError::PermissionDenied);
        }
        let bytes = self.encode(next)?;
        let mut temp = tempfile::Builder::new()
            .prefix("qualia-head-")
            .tempfile_in(&self.root)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        // Check current policy and head again before the replacement.
        if self.current(policy)? != expected {
            return Ok(HeadCommit::Conflict);
        }
        if !policy.permit_commit(
            &self.root,
            &self.chain,
            expected,
            next,
            receipt,
            sidecar,
            payload,
        ) {
            return Err(LocalHeadError::PermissionDenied);
        }
        // Recheck signer, grant and chain authority after staging, before rename.
        // The host must hold its policy lease through the actual replacement.
        receipt
            .verify_with_authority(sidecar, payload, payload_budget, policy)
            .map_err(|_| LocalHeadError::Receipt)?;
        if temp.persist(&self.head_path).is_err() {
            return Ok(HeadCommit::Indeterminate);
        }
        // The platform/filesystem must additionally be qualified by the host.
        #[cfg(unix)]
        {
            return Ok(
                if File::open(&self.root).and_then(|f| f.sync_all()).is_ok() {
                    HeadCommit::Committed
                } else {
                    HeadCommit::Indeterminate
                },
            );
        }
        #[cfg(not(unix))]
        {
            Ok(HeadCommit::Indeterminate)
        }
    }

    fn encode(&self, head: ChainHead) -> Result<[u8; SIZE], LocalHeadError> {
        let ChainHead::Present {
            sequence,
            checkpoint,
        } = head
        else {
            return Err(LocalHeadError::InvalidEncoding);
        };
        if checkpoint.iter().all(|b| *b == 0) {
            return Err(LocalHeadError::InvalidEncoding);
        }
        let mut bytes = [0u8; SIZE];
        bytes[..4].copy_from_slice(MAGIC);
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&(SIZE as u16).to_le_bytes());
        bytes[8..40].copy_from_slice(&self.chain);
        bytes[40..48].copy_from_slice(&sequence.to_le_bytes());
        bytes[48..80].copy_from_slice(&checkpoint);
        bytes[80..112].copy_from_slice(&self.signer.verifying_key().to_bytes());
        let mut message = [0u8; DOMAIN.len() + 112];
        message[..DOMAIN.len()].copy_from_slice(DOMAIN);
        message[DOMAIN.len()..DOMAIN.len() + 112].copy_from_slice(&bytes[..112]);
        bytes[112..].copy_from_slice(&self.signer.sign(&message[..DOMAIN.len() + 112]).to_bytes());
        Ok(bytes)
    }

    fn decode(&self, bytes: &[u8; SIZE]) -> Result<ChainHead, LocalHeadError> {
        if &bytes[..4] != MAGIC
            || bytes[4..6] != 1u16.to_le_bytes()
            || bytes[6..8] != (SIZE as u16).to_le_bytes()
            || bytes[8..40] != self.chain
            || bytes[80..112] != self.signer.verifying_key().to_bytes()
        {
            return Err(LocalHeadError::InvalidEncoding);
        }
        let key = VerifyingKey::from_bytes(&self.signer.verifying_key().to_bytes())
            .map_err(|_| LocalHeadError::InvalidSigner)?;
        let signature = Signature::from_bytes(
            bytes[112..176]
                .try_into()
                .map_err(|_| LocalHeadError::InvalidEncoding)?,
        );
        let mut message = [0u8; DOMAIN.len() + 112];
        message[..DOMAIN.len()].copy_from_slice(DOMAIN);
        message[DOMAIN.len()..DOMAIN.len() + 112].copy_from_slice(&bytes[..112]);
        key.verify_strict(&message[..DOMAIN.len() + 112], &signature)
            .map_err(|_| LocalHeadError::InvalidSignature)?;
        let sequence = u64::from_le_bytes(
            bytes[40..48]
                .try_into()
                .map_err(|_| LocalHeadError::InvalidEncoding)?,
        );
        let checkpoint: [u8; 32] = bytes[48..80]
            .try_into()
            .map_err(|_| LocalHeadError::InvalidEncoding)?;
        if checkpoint.iter().all(|b| *b == 0) {
            return Err(LocalHeadError::InvalidEncoding);
        }
        Ok(ChainHead::Present {
            sequence,
            checkpoint,
        })
    }
}
