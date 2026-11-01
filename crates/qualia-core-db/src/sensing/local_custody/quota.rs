//! Conservative caller-owned aggregate admission for one protected root.
//!
//! Charges survive every publication error and unwind. There is no refund or
//! directory scan; recovery may construct a new ledger only from a freshly
//! authenticated usage snapshot. This is cooperative in-process accounting,
//! not a durable cross-process quota or a filesystem allocation limit.
use super::{
    chain::{publish_chain_bundle, ChainAuthority, ChainPublication, ChainPublicationError},
    entry_size, publish_bundle, validate_root, CustodyAuthority, CustodyError, PublishedBundle,
    SidecarReceipt, SIDECAR_BYTES,
};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuotaLimits {
    pub encoded_bytes: u64,
    pub entries: u64,
}

/// Host snapshot must include committed, orphan and uncertain bundle charges.
/// The opaque snapshot ID must bind the root and current usage under the host's
/// trusted freshness and lease policy. Counts exclude OS/runtime allocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuotaSnapshot {
    pub id: [u8; 32],
    pub encoded_bytes: u64,
    pub entries: u64,
}

pub trait QuotaSnapshotAuthority {
    fn admit_usage(&self, root: &Path, limits: QuotaLimits, snapshot: QuotaSnapshot) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuotaError {
    InvalidScope,
    InvalidLimits,
    SnapshotDenied,
    ScopeMismatch,
    AlreadyCharged,
    SlotsExhausted,
    BytesExceeded,
    EntriesExceeded,
    Overflow,
}

/// N bounds additional publication attempts tracked in this ledger. The host
/// must hold its aggregate-quota lease for the ledger's complete lifetime.
pub struct CustodyQuota<'root, const N: usize> {
    root: &'root Path,
    limits: QuotaLimits,
    usage: QuotaSnapshot,
    charged: [Option<[u8; 32]>; N],
}

impl<'root, const N: usize> CustodyQuota<'root, N> {
    pub fn from_authenticated_usage(
        root: &'root Path,
        limits: QuotaLimits,
        snapshot: QuotaSnapshot,
        authority: &impl QuotaSnapshotAuthority,
    ) -> Result<Self, QuotaError> {
        validate_root(root).map_err(|_| QuotaError::InvalidScope)?;
        if N == 0
            || limits.encoded_bytes == 0
            || limits.entries == 0
            || snapshot.encoded_bytes > limits.encoded_bytes
            || snapshot.entries > limits.entries
            || snapshot.id.iter().all(|byte| *byte == 0)
        {
            return Err(QuotaError::InvalidLimits);
        }
        if !authority.admit_usage(root, limits, snapshot) {
            return Err(QuotaError::SnapshotDenied);
        }
        Ok(Self {
            root,
            limits,
            usage: snapshot,
            charged: [None; N],
        })
    }

    /// Current charged usage, including attempts that may not have published.
    /// This does not mint a fresh authenticated snapshot identifier.
    pub const fn charged_usage(&self) -> (u64, u64) {
        (self.usage.encoded_bytes, self.usage.entries)
    }

    fn charge(
        &mut self,
        root: &Path,
        checkpoint: [u8; 32],
        bytes: usize,
    ) -> Result<(), QuotaError> {
        if root != self.root {
            return Err(QuotaError::ScopeMismatch);
        }
        if self.charged.contains(&Some(checkpoint)) {
            return Err(QuotaError::AlreadyCharged);
        }
        let index = self
            .charged
            .iter()
            .position(Option::is_none)
            .ok_or(QuotaError::SlotsExhausted)?;
        let bytes = u64::try_from(bytes).map_err(|_| QuotaError::Overflow)?;
        let total = self
            .usage
            .encoded_bytes
            .checked_add(bytes)
            .ok_or(QuotaError::Overflow)?;
        let entries = self
            .usage
            .entries
            .checked_add(1)
            .ok_or(QuotaError::Overflow)?;
        if total > self.limits.encoded_bytes {
            return Err(QuotaError::BytesExceeded);
        }
        if entries > self.limits.entries {
            return Err(QuotaError::EntriesExceeded);
        }
        self.usage.encoded_bytes = total;
        self.usage.entries = entries;
        self.charged[index] = Some(checkpoint);
        Ok(())
    }
}

#[derive(Debug)]
pub enum QuotaPublicationError<E> {
    Quota(QuotaError),
    Preflight(CustodyError),
    /// The charge is retained even if the attempt failed before publication.
    AttemptCharged {
        checkpoint: [u8; 32],
        bytes: usize,
        error: E,
    },
}

fn admit<const N: usize>(
    root: &Path,
    sidecar: &[u8; SIDECAR_BYTES],
    payload: &[u8],
    receipt: &SidecarReceipt,
    budget: usize,
    quota: &mut CustodyQuota<'_, N>,
    authority: &mut impl CustodyAuthority,
) -> Result<usize, QuotaPublicationError<CustodyError>> {
    validate_root(root).map_err(QuotaPublicationError::Preflight)?;
    let total = entry_size(payload.len(), budget).map_err(QuotaPublicationError::Preflight)?;
    receipt
        .verify_with_authority(sidecar, payload, payload.len(), authority)
        .map_err(|e| QuotaPublicationError::Preflight(CustodyError::Receipt(e)))?;
    if !authority.permit_publish(root, receipt, sidecar, payload, total) {
        return Err(QuotaPublicationError::Preflight(
            CustodyError::PermissionDenied,
        ));
    }
    quota
        .charge(root, receipt.checkpoint(), total)
        .map_err(QuotaPublicationError::Quota)?;
    Ok(total)
}

/// Admit aggregate encoded bundle bytes before any producer I/O. The encoded
/// byte cap excludes filesystem overhead and head/lock files; host disk quotas
/// must cover those. Authority still revalidates policy during publication.
#[allow(clippy::too_many_arguments)]
pub fn publish_bundle_with_quota<const N: usize>(
    root: &Path,
    sidecar: &[u8; SIDECAR_BYTES],
    payload: &[u8],
    receipt: &SidecarReceipt,
    budget: usize,
    quota: &mut CustodyQuota<'_, N>,
    authority: &mut impl CustodyAuthority,
) -> Result<PublishedBundle, QuotaPublicationError<CustodyError>> {
    let bytes = admit(root, sidecar, payload, receipt, budget, quota, authority)?;
    publish_bundle(root, sidecar, payload, receipt, budget, authority).map_err(|error| {
        QuotaPublicationError::AttemptCharged {
            checkpoint: receipt.checkpoint(),
            bytes,
            error,
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub fn publish_chain_bundle_with_quota<const N: usize>(
    root: &Path,
    sidecar: &[u8; SIDECAR_BYTES],
    payload: &[u8],
    receipt: &SidecarReceipt,
    budget: usize,
    quota: &mut CustodyQuota<'_, N>,
    authority: &mut impl ChainAuthority,
) -> Result<ChainPublication, QuotaPublicationError<ChainPublicationError>> {
    let bytes =
        admit(root, sidecar, payload, receipt, budget, quota, authority).map_err(|error| {
            match error {
                QuotaPublicationError::Quota(error) => QuotaPublicationError::Quota(error),
                QuotaPublicationError::Preflight(error) => QuotaPublicationError::Preflight(error),
                QuotaPublicationError::AttemptCharged { .. } => {
                    unreachable!("admission does not publish")
                }
            }
        })?;
    publish_chain_bundle(root, sidecar, payload, receipt, budget, authority).map_err(|error| {
        QuotaPublicationError::AttemptCharged {
            checkpoint: receipt.checkpoint(),
            bytes,
            error,
        }
    })
}

#[cfg(test)]
mod tests;
