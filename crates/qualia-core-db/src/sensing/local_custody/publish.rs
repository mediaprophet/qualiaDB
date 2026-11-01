use super::{
    entry_size, filename, validate_root, CustodyAuthority, CustodyError, SidecarReceipt,
    HEADER_BYTES, MAGIC, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// Atomic bundle publication was attempted and the target link was created.
/// Directory synchronization and chain-head publication are not established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublishedBundle {
    pub checkpoint: [u8; 32],
    pub bytes: usize,
}

/// Publish a single receipt-addressed bundle without replacing an existing
/// destination. Other links or writers may still modify the underlying file.
/// Host must hold an
/// exclusive lease and enforce local-root/aggregate-budget policy throughout.
/// A same-filesystem hard-link create is used; no replacement is attempted.
/// Temporary cleanup is best effort and crash cleanup is external.
pub fn publish_bundle(
    root: &Path,
    sidecar: &[u8; SIDECAR_BYTES],
    payload: &[u8],
    receipt: &SidecarReceipt,
    byte_budget: usize,
    authority: &mut impl CustodyAuthority,
) -> Result<PublishedBundle, CustodyError> {
    validate_root(root)?;
    let total = entry_size(payload.len(), byte_budget)?;
    let payload_len = u64::try_from(payload.len()).map_err(|_| CustodyError::TooLarge)?;
    receipt
        .verify_with_authority(sidecar, payload, payload.len(), authority)
        .map_err(CustodyError::Receipt)?;
    if !authority.permit_publish(root, receipt, sidecar, payload, total) {
        return Err(CustodyError::PermissionDenied);
    }
    let temp = tempfile::Builder::new()
        .prefix("qualia-sensing-")
        .tempdir_in(root)?;
    let temporary_file = temp.path().join("bundle");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_file)?;
    let mut header = [0u8; HEADER_BYTES];
    header[..4].copy_from_slice(MAGIC);
    header[4..6].copy_from_slice(&1u16.to_le_bytes());
    header[6..8].copy_from_slice(&(HEADER_BYTES as u16).to_le_bytes());
    header[8..16].copy_from_slice(&payload_len.to_le_bytes());
    let mut receipt_bytes = [0u8; SIDECAR_RECEIPT_BYTES];
    receipt
        .encode(&mut receipt_bytes)
        .map_err(CustodyError::Receipt)?;
    file.write_all(&header)?;
    file.write_all(sidecar)?;
    file.write_all(&receipt_bytes)?;
    file.write_all(payload)?;
    file.sync_all()?;
    drop(file);
    // Recheck current policy and lease immediately before publication.
    receipt
        .verify_with_authority(sidecar, payload, payload.len(), authority)
        .map_err(CustodyError::Receipt)?;
    if !authority.permit_publish(root, receipt, sidecar, payload, total) {
        return Err(CustodyError::PermissionDenied);
    }
    let checkpoint = receipt.checkpoint();
    let name = filename(&checkpoint);
    let target = root.join(std::str::from_utf8(&name).map_err(|_| CustodyError::InvalidEncoding)?);
    fs::hard_link(&temporary_file, &target)?;
    Ok(PublishedBundle {
        checkpoint,
        bytes: total,
    })
}
