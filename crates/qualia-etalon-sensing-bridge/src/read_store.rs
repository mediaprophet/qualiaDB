//! Verified bundle read followed by canonical, explicitly untrusted decoding.
use crate::sidecar::{decode_sidecar, SidecarError, UntrustedSidecar};
use qualia_core_db::sensing::{
    local_custody::{read_bundle, CustodyAuthority, CustodyError},
    record::ResolvedFieldPayloadBinding,
    sidecar_attestation::{SidecarReceipt, SIDECAR_BYTES},
};
use std::path::Path;

/// Bind the decoded record to the exact admitted payload and its namespace.
/// Implementations enforce current read policy, signer authority, source and
/// artifact mappings, subject/guardian relations and container authenticity.
/// Historical permits in the sidecar are never current authorization.
pub trait StoredReadAuthority: CustodyAuthority {
    fn permit_decoded(
        &mut self,
        root: &Path,
        checkpoint: &[u8; 32],
        binding: &ResolvedFieldPayloadBinding,
        receipt: &SidecarReceipt,
        decoded: &UntrustedSidecar,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool;
}

#[derive(Debug)]
pub enum StoredReadError {
    Custody(CustodyError),
    Sidecar(SidecarError),
    NotAuthorized,
}

/// Receipt validity and host permission are checked for this read only.
/// The decoded metadata stays historical and untrusted; this does not prove
/// chain membership, scientific truth or continuing permission.
pub struct StoredObservation {
    pub receipt: SidecarReceipt,
    pub historical: UntrustedSidecar,
    pub payload_bytes: usize,
}

/// Read and structurally decode one bundle under a protected-root/policy lease.
/// Errors and unwind clear the whole supplied payload buffer. Success writes
/// the declared prefix; the underlying reader clears any unused tail.
pub fn read_stored_observation(
    root: &Path,
    checkpoint: &[u8; 32],
    bundle_byte_budget: usize,
    binding: ResolvedFieldPayloadBinding,
    payload: &mut [u8],
    authority: &mut impl StoredReadAuthority,
) -> Result<StoredObservation, StoredReadError> {
    payload.fill(0);
    let mut output = ClearPayload {
        bytes: payload,
        keep: false,
    };
    let mut sidecar = [0u8; SIDECAR_BYTES];
    let read = read_bundle(
        root,
        checkpoint,
        bundle_byte_budget,
        &mut sidecar,
        output.bytes,
        authority,
    )
    .map_err(StoredReadError::Custody)?;
    let decoded = decode_sidecar(&sidecar, binding).map_err(StoredReadError::Sidecar)?;
    if u64::try_from(read.payload_bytes).ok() != Some(binding.raw_payload().length())
        || !authority.permit_decoded(
            root,
            checkpoint,
            &binding,
            &read.receipt,
            &decoded,
            &sidecar,
            &output.bytes[..read.payload_bytes],
        )
    {
        return Err(StoredReadError::NotAuthorized);
    }
    read.receipt
        .verify_with_authority(
            &sidecar,
            &output.bytes[..read.payload_bytes],
            read.payload_bytes,
            authority,
        )
        .map_err(|e| StoredReadError::Custody(CustodyError::Receipt(e)))?;
    if !authority.permit_read(root, checkpoint, bundle_byte_budget) {
        return Err(StoredReadError::NotAuthorized);
    }
    output.keep = true;
    Ok(StoredObservation {
        receipt: read.receipt,
        historical: decoded,
        payload_bytes: read.payload_bytes,
    })
}

struct ClearPayload<'a> {
    bytes: &'a mut [u8],
    keep: bool,
}
impl Drop for ClearPayload<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.bytes.fill(0);
        }
    }
}
