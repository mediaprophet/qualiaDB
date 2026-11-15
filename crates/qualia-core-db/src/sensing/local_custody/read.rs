use super::{
    entry_size, filename, validate_root, CustodyAuthority, CustodyError, SidecarReceipt,
    HEADER_BYTES, MAGIC, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES,
};
use std::{io::Read, path::Path};

pub struct ReadBundle {
    pub receipt: SidecarReceipt,
    pub payload_bytes: usize,
}

/// Read and verify one complete bundle into caller-owned buffers. Output is
/// cleared on failure or unwind. Host read permission precedes I/O and is
/// rechecked after receipt verification. The budget limits encoded entry size;
/// an additional byte may be read to reject trailing data. This does not
/// establish chain completeness.
pub fn read_bundle(
    root: &Path,
    checkpoint: &[u8; 32],
    byte_budget: usize,
    sidecar: &mut [u8; SIDECAR_BYTES],
    payload: &mut [u8],
    authority: &mut impl CustodyAuthority,
) -> Result<ReadBundle, CustodyError> {
    sidecar.fill(0);
    payload.fill(0);
    let mut output = ClearOnDrop {
        sidecar,
        payload,
        keep: false,
    };
    let result = read_inner(
        root,
        checkpoint,
        byte_budget,
        output.sidecar,
        output.payload,
        authority,
    );
    if result.is_ok() {
        output.keep = true;
    }
    result
}

struct ClearOnDrop<'a> {
    sidecar: &'a mut [u8; SIDECAR_BYTES],
    payload: &'a mut [u8],
    keep: bool,
}

impl Drop for ClearOnDrop<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.sidecar.fill(0);
            self.payload.fill(0);
        }
    }
}

fn read_inner(
    root: &Path,
    checkpoint: &[u8; 32],
    byte_budget: usize,
    sidecar: &mut [u8; SIDECAR_BYTES],
    payload: &mut [u8],
    authority: &mut impl CustodyAuthority,
) -> Result<ReadBundle, CustodyError> {
    validate_root(root)?;
    entry_size(0, byte_budget)?;
    if !authority.permit_read(root, checkpoint, byte_budget) {
        return Err(CustodyError::PermissionDenied);
    }
    let name = filename(checkpoint);
    let path = root.join(std::str::from_utf8(&name).map_err(|_| CustodyError::InvalidEncoding)?);
    let mut file = super::open_checked::open_regular(&path)?;
    let mut header = [0u8; HEADER_BYTES];
    file.read_exact(&mut header)?;
    if &header[..4] != MAGIC
        || u16::from_le_bytes([header[4], header[5]]) != 1
        || u16::from_le_bytes([header[6], header[7]]) as usize != HEADER_BYTES
    {
        return Err(CustodyError::InvalidEncoding);
    }
    let len = usize::try_from(u64::from_le_bytes(
        header[8..16]
            .try_into()
            .map_err(|_| CustodyError::InvalidEncoding)?,
    ))
    .map_err(|_| CustodyError::TooLarge)?;
    entry_size(len, byte_budget)?;
    if len > payload.len() {
        return Err(CustodyError::TooLarge);
    }
    file.read_exact(sidecar)?;
    let mut encoded = [0u8; SIDECAR_RECEIPT_BYTES];
    file.read_exact(&mut encoded)?;
    let receipt = SidecarReceipt::decode(&encoded).map_err(CustodyError::Receipt)?;
    if receipt.checkpoint() != *checkpoint {
        return Err(CustodyError::CheckpointMismatch);
    }
    file.read_exact(&mut payload[..len])?;
    let mut trailing = [0u8; 1];
    if file.read(&mut trailing)? != 0 {
        return Err(CustodyError::InvalidEncoding);
    }
    receipt
        .verify_with_authority(sidecar, &payload[..len], len, authority)
        .map_err(CustodyError::Receipt)?;
    if !authority.permit_read(root, checkpoint, byte_budget) {
        return Err(CustodyError::PermissionDenied);
    }
    Ok(ReadBundle {
        receipt,
        payload_bytes: len,
    })
}
