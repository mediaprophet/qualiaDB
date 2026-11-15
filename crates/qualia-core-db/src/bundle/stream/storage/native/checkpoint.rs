use super::write::WriteStep;
use super::StoreError as E;
use crate::bundle::stream::storage::{
    encode_checkpoint, CandidateBitmapPage, VerifiedLeafView, CHECKPOINT_BYTES,
};
use crate::storage_driver::ranges::{RangeFile, RangeSession};

/// One OS-adjacent step. The wait site passes this to a session closure; it
/// does not carry StorageStageControl or LocalEffectControl.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeafIoOp {
    Write(RangeFile, u64),
    Sync(RangeFile),
}

/// Payload, proof, and checkpoint bytes only. No stage, grant, or gates.
#[allow(clippy::too_many_arguments)]
pub(super) fn write_verified_bytes(
    payload_offset: u64,
    proof_offset: u64,
    checkpoint_base: u64,
    payload: &[u8],
    proof_slot: &[u8],
    header: &[u8],
    bitmap: &[u8],
    hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
    io: &mut impl FnMut(LeafIoOp, &[u8]) -> Result<(), E>,
) -> Result<(), E> {
    let bitmap_off = checkpoint_base
        .checked_add(CHECKPOINT_BYTES as u64)
        .ok_or(E::Overflow)?;
    hook(WriteStep::PayloadWrite)?;
    io(LeafIoOp::Write(RangeFile::Payload, payload_offset), payload)?;
    hook(WriteStep::PayloadWritten)?;
    hook(WriteStep::ProofWrite)?;
    io(LeafIoOp::Write(RangeFile::Proof, proof_offset), proof_slot)?;
    hook(WriteStep::ProofWritten)?;
    hook(WriteStep::PayloadSync)?;
    io(LeafIoOp::Sync(RangeFile::Payload), &[])?;
    hook(WriteStep::PayloadSynced)?;
    hook(WriteStep::ProofSync)?;
    io(LeafIoOp::Sync(RangeFile::Proof), &[])?;
    hook(WriteStep::ProofSynced)?;
    hook(WriteStep::BitmapWrite)?;
    io(LeafIoOp::Write(RangeFile::Checkpoint, bitmap_off), bitmap)?;
    hook(WriteStep::BitmapWritten)?;
    hook(WriteStep::HeaderWrite)?;
    io(
        LeafIoOp::Write(RangeFile::Checkpoint, checkpoint_base),
        header,
    )?;
    hook(WriteStep::HeaderWritten)?;
    hook(WriteStep::CheckpointSync)?;
    io(LeafIoOp::Sync(RangeFile::Checkpoint), &[])?;
    hook(WriteStep::CheckpointSynced)?;
    hook(WriteStep::FinalCheck)?;
    Ok(())
}

pub(super) fn session_io(
    session: &mut RangeSession<'_, '_>,
    op: LeafIoOp,
    bytes: &[u8],
) -> Result<(), E> {
    match op {
        LeafIoOp::Write(file, offset) => {
            session.write_exact_at(file, offset, bytes).map_err(E::from)
        }
        LeafIoOp::Sync(file) => session.sync_file(file).map(|_| ()).map_err(E::from),
    }
}

/// Recovery/resume arming. `CandidateBitmapPage` is a stage loan, so this runs
/// inside a short callback. Destination I/O is `wait_leaf_write` after that
/// callback drops. Does not take StorageStageControl / LocalEffectControl.
pub(super) fn store_verified(
    view: &VerifiedLeafView<'_>,
    header: &mut [u8],
    page: &mut CandidateBitmapPage<'_>,
    next: u64,
) -> Result<(), E> {
    page.record_integrity(view)?;
    encode_checkpoint(page, next, header)?;
    Ok(())
}
