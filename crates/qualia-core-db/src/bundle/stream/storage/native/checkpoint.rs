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
    hook(WriteStep::PayloadWrite)?;
    gates(control, authority, identity, clock, policy)?;
    session.write_exact_at(RangeFile::Payload, layout.leaf_offset(leaf)?, view.bytes())?;
    hook(WriteStep::PayloadWritten)?;
    gates(control, authority, identity, clock, policy)?;
    hook(WriteStep::ProofWrite)?;
    gates(control, authority, identity, clock, policy)?;
    session.write_exact_at(RangeFile::Proof, layout.proof_offset(leaf)?, proof_slot)?;
    hook(WriteStep::ProofWritten)?;
    gates(control, authority, identity, clock, policy)?;
    hook(WriteStep::PayloadSync)?;
    gates(control, authority, identity, clock, policy)?;
    session.sync_file(RangeFile::Payload)?;
    hook(WriteStep::PayloadSynced)?;
    gates(control, authority, identity, clock, policy)?;
    hook(WriteStep::ProofSync)?;
    gates(control, authority, identity, clock, policy)?;
    session.sync_file(RangeFile::Proof)?;
    hook(WriteStep::ProofSynced)?;
    gates(control, authority, identity, clock, policy)?;
    page.record_integrity(view)?;
    encode_checkpoint(page, next, header)?;
    let base = layout.checkpoint_offset(page_number, (next & 1) as u8)?;
    checkpoint(
        session,
        base,
        page.payload(),
        header,
        control,
        authority,
        identity,
        clock,
        policy,
        hook,
    )?;
    hook(WriteStep::FinalCheck)?;
    gates(control, authority, identity, clock, policy)?;
    Ok(LeafBarrierObservation {
        identity: *identity,
        leaf_index: leaf,
        sequence: next,
        backend_attempt,
    })
}

#[allow(clippy::too_many_arguments)]
fn checkpoint(
    session: &mut RangeSession<'_, '_>,
    base: u64,
    payload: &[u8],
    header: &[u8],
    control: &mut StorageStageControl<'_, '_>,
    authority: &mut LocalEffectControl<'_, '_>,
    identity: &StorageIdentity,
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
    hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
) -> Result<(), E> {
    hook(WriteStep::BitmapWrite)?;
    gates(control, authority, identity, clock, policy)?;
    session.write_exact_at(
        RangeFile::Checkpoint,
        base.checked_add(CHECKPOINT_BYTES as u64)
            .ok_or(E::Overflow)?,
        payload,
    )?;
    hook(WriteStep::BitmapWritten)?;
    gates(control, authority, identity, clock, policy)?;
    hook(WriteStep::HeaderWrite)?;
    gates(control, authority, identity, clock, policy)?;
    session.write_exact_at(RangeFile::Checkpoint, base, header)?;
    hook(WriteStep::HeaderWritten)?;
    gates(control, authority, identity, clock, policy)?;
    hook(WriteStep::CheckpointSync)?;
    gates(control, authority, identity, clock, policy)?;
    session.sync_file(RangeFile::Checkpoint)?;
    hook(WriteStep::CheckpointSynced)?;
    gates(control, authority, identity, clock, policy)?;
    Ok(())
}
