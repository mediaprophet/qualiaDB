use super::*;
use super::super::{SceneAudioBusGraph, pcm::PcmRenderState, pcm_schedule::ClipScheduleBudget};

/// Admit every refusal-path reset/zero/pose cost before touching caller storage.
pub(super) fn admit(graph: &SceneAudioBusGraph, state: &ClipScheduleState<'_>,
    clips: usize, left: usize, right: usize, obstacles: usize,
    mut budget: ClipScheduleBudget) -> Result<ClipScheduleBudget, SoundError> {
    if left == 0 || left != right || left > budget.render.max_frames
        || graph.emitter_count > MAX_SCENE_AUDIO_EMITTERS
        || clips > budget.max_authored_clips
        || state.active_assignment_count() > budget.max_authored_clips
        || obstacles > budget.render.max_obstacles || budget.render.sample_rate != 48_000 {
        return Err(SoundError::Budget);
    }
    let add = |a: usize, b: usize| a.checked_add(b).ok_or(SoundError::Budget);
    let mul = |a: usize, b: usize| a.checked_mul(b).ok_or(SoundError::Budget);
    let reset = add(state.active_assignment_count(), mul(core::mem::size_of::<PcmRenderState>(), 2)?)?;
    let work = add(add(mul(left, 4)?, reset)?, add(mul(MAX_SCENE_AUDIO_EMITTERS, 12)?, mul(clips, 2)?)?)?;
    let bytes = add(core::mem::size_of::<SoundPose>(),
        mul(MAX_SCENE_AUDIO_EMITTERS, core::mem::size_of::<[f32; 3]>())?)?;
    budget.max_work = budget.max_work.checked_sub(u64::try_from(work).map_err(|_| SoundError::Budget)?)
        .ok_or(SoundError::Budget)?;
    budget.max_transaction_bytes = budget.max_transaction_bytes.checked_sub(bytes).ok_or(SoundError::Budget)?;
    Ok(budget)
}
