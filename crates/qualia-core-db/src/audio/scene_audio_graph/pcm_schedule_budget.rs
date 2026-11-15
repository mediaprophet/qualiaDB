//! Checked logical storage/work admission for the PCM scheduling transaction.
use super::super::{
    pcm::{PcmRenderState, PcmVoiceBlock},
    MAX_SCENE_AUDIO_EMITTERS,
};
use super::{ClipAssignment, ClipScheduleBudget, ClipScheduleError, ClipSpanReceipt, Resident};
fn add(a: usize, b: usize) -> Result<usize, ClipScheduleError> {
    a.checked_add(b).ok_or(ClipScheduleError::Budget)
}
fn mul(a: usize, b: usize) -> Result<usize, ClipScheduleError> {
    a.checked_mul(b).ok_or(ClipScheduleError::Budget)
}
pub(super) fn admit(
    budget: &ClipScheduleBudget,
    catalog_count: usize,
    previous_count: usize,
    resident_count: usize,
    frames: usize,
    mono_len: usize,
    selections: usize,
) -> Result<(), ClipScheduleError> {
    // Charge retained authoritative/transaction DSP, exact catalog storage,
    // touched scratch prefixes and bounded per-call voice/residency arrays.
    // Extra unused caller capacity and allocator overhead are not measured.
    let bytes = add(
        mul(core::mem::size_of::<PcmRenderState>(), 2)?,
        add(
            mul(
                catalog_count.max(previous_count),
                core::mem::size_of::<Option<ClipAssignment>>(),
            )?,
            add(
                mul(
                    MAX_SCENE_AUDIO_EMITTERS,
                    add(
                        core::mem::size_of::<PcmVoiceBlock<'_>>(),
                        core::mem::size_of::<Option<Resident>>(),
                    )?,
                )?,
                add(
                    mul(mono_len, 4)?,
                    add(
                        mul(frames, 16)?,
                        add(
                            mul(selections, core::mem::size_of::<usize>())?,
                            mul(
                                budget.max_event_spans,
                                core::mem::size_of::<ClipSpanReceipt>(),
                            )?,
                        )?,
                    )?,
                )?,
            )?,
        )?,
    )?;
    if bytes > budget.max_transaction_bytes {
        return Err(ClipScheduleError::Budget);
    }
    let charge = add(
        mul(
            catalog_count,
            add(
                4,
                mul(budget.max_event_spans, add(budget.max_selected_clips, 4)?)?,
            )?,
        )?,
        add(
            mul(
                frames,
                add(budget.max_selected_clips, mul(resident_count, 2)?)?,
            )?,
            add(
                mul(
                    mul(budget.max_event_spans, budget.max_selected_clips)?,
                    budget.max_selected_clips,
                )?,
                add(
                    mul(MAX_SCENE_AUDIO_EMITTERS, budget.max_event_spans)?,
                    add(
                        mul(frames, 4)?,
                        add(
                            previous_count,
                            mul(core::mem::size_of::<PcmRenderState>(), 4)?,
                        )?,
                    )?,
                )?,
            )?,
        )?,
    )?;
    if u64::try_from(charge).map_err(|_| ClipScheduleError::Budget)? > budget.max_work {
        return Err(ClipScheduleError::Budget);
    }
    Ok(())
}
