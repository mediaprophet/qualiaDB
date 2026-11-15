use super::write::{gates, WriteStep};
use super::{LeafBarrierObservation, StoreError as E, StoreLayout};
use crate::bundle::stream::{
    policy::LocalEffectControl,
    runtime::StorageStageControl,
    storage::{
        encode_checkpoint, CandidateBitmapPage, StorageIdentity, VerifiedLeafView, CHECKPOINT_BYTES,
    },
};
use crate::net::qdnf::types::Generation;
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

/// Recovery/resume path. Still called under a live stage callback; I/O itself
/// does not take StorageStageControl / LocalEffectControl.
#[allow(clippy::too_many_arguments)]
pub(super) fn store_verified(
    session: &mut RangeSession<'_, '_>,
    identity: &StorageIdentity,
    view: &VerifiedLeafView<'_>,
    proof_slot: &[u8],
    header: &mut [u8],
    page: &mut CandidateBitmapPage<'_>,
    control: &mut StorageStageControl<'_, '_>,
    authority: &mut LocalEffectControl<'_, '_>,
    layout: StoreLayout,
    next: u64,
    backend_attempt: [u8; 32],
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
    hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
) -> Result<LeafBarrierObservation, E> {
    let leaf = view.leaf_index();
    let page_number = page.page_number();
    page.record_integrity(view)?;
    encode_checkpoint(page, next, header)?;
    let base = layout.checkpoint_offset(page_number, (next & 1) as u8)?;
    write_verified_bytes(
        layout.leaf_offset(leaf)?,
        layout.proof_offset(leaf)?,
        base,
        view.bytes(),
        proof_slot,
        header,
        page.payload(),
        hook,
        &mut |op, bytes| session_io(session, op, bytes),
    )?;
    gates(control, authority, identity, clock, policy)?;
    Ok(LeafBarrierObservation {
        identity: *identity,
        leaf_index: leaf,
        sequence: next,
        backend_attempt,
    })
}
