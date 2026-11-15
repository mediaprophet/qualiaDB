use crate::read_store::{
    read_stored_observation, StoredObservation, StoredReadAuthority, StoredReadError,
};
use qualia_core_db::sensing::container::ResolvedSensingMeasurement;
use std::path::Path;

#[derive(Debug)]
pub enum ContainerReadError {
    PayloadBudget,
    RecordMismatch,
    PayloadMismatch,
    Read(StoredReadError),
}

/// Read a signed bundle and require exact agreement with a selected canonical
/// container row and raw payload. Container authenticity, semantic namespaces
/// and current subject/guardian policy remain the StoredReadAuthority's job.
/// Errors/unwind clear all caller bytes; success does not prove chain membership.
pub fn read_container_observation(
    root: &Path,
    checkpoint: &[u8; 32],
    bundle_budget: usize,
    payload_budget: usize,
    selected: &ResolvedSensingMeasurement<'_>,
    payload: &mut [u8],
    authority: &mut impl StoredReadAuthority,
) -> Result<StoredObservation, ContainerReadError> {
    payload.fill(0);
    let mut output = ClearPayload {
        bytes: payload,
        keep: false,
    };
    if selected.payload().len() > payload_budget || selected.payload().len() > output.bytes.len() {
        return Err(ContainerReadError::PayloadBudget);
    }
    let read = read_stored_observation(
        root,
        checkpoint,
        bundle_budget,
        selected.binding(),
        &mut output.bytes[..selected.payload().len()],
        authority,
    )
    .map_err(ContainerReadError::Read)?;
    if *read.historical.record() != selected.record() {
        return Err(ContainerReadError::RecordMismatch);
    }
    if read.payload_bytes != selected.payload().len()
        || &output.bytes[..read.payload_bytes] != selected.payload()
    {
        return Err(ContainerReadError::PayloadMismatch);
    }
    output.keep = true;
    Ok(read)
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
