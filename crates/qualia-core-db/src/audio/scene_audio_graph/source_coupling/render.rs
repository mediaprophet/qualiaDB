use super::*;
use crate::audio::scene_audio_graph::{pcm_schedule::{ClipScheduleBudget,
    ClipScheduleWorkspace, PcmClip}, AcousticObstacle, SceneAudioBusGraph};

fn point_ok(p: [f32;3]) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)
}
fn valid_pose(graph: &SceneAudioBusGraph, pose: &SoundPose) -> Result<(), SoundError> {
    if pose.emitter_count != graph.emitter_count || pose.emitter_count > MAX_SCENE_AUDIO_EMITTERS
        || pose.pose_sequence == 0 || pose.pose_tick > i64::MAX as u64
        || !point_ok(pose.listener.position) || !pose.listener.yaw_rad.is_finite()
        || !(0.0..=4.0).contains(&pose.listener.manifold_w) {
        return Err(SoundError::Pose);
    }
    for (slot, p) in pose.emitters[..pose.emitter_count].iter().enumerate() {
        let node = &graph.emitters[slot];
        if p.node_id != node.node_id || p.origin_quin_hash != node.origin_quin_hash {
            return Err(SoundError::Binding);
        }
        if node.deontic_permit >= 2 { return Err(SoundError::Denied); }
        if !point_ok(p.position_m) { return Err(SoundError::Pose); }
    }
    Ok(())
}
fn sync_wavefield(graph: &mut SceneAudioBusGraph) {
    graph.wavefield.clear();
    // Preserve the existing bounded legacy synthesis projection (32 slots).
    // PCM reads the scene nodes directly, including its separate 64-slot profile.
    for node in &graph.emitters[..graph.emitter_count] { let _ = graph.wavefield.push(node.emitter); }
}

/// Render into private caller staging only. The host must recheck source,
/// authority, output lease and the complete receipt immediately before enqueue.
/// Pose is held constant within this block; no Doppler/resampling is invented.
/// On any error the host discards staging and flushes the owned output epoch.
/// Invalid/over-budget buffer shapes remain untouched, with the lease invalidated.
pub fn render_guarded(
    graph: &mut SceneAudioBusGraph,
    scope: &mut SoundScopeState,
    state: &mut ClipScheduleState<'_>,
    workspace: &mut ClipScheduleWorkspace<'_>,
    clips: &[PcmClip<'_>],
    pose: &SoundPose,
    block_start_frame: u64,
    first_output_frame: u64,
    mode: SoundBlockMode,
    obstacles: &[AcousticObstacle],
    budget: ClipScheduleBudget,
    staging_l: &mut [f32],
    staging_r: &mut [f32],
    live: &dyn Fn(SoundBinding) -> bool,
) -> Result<SoundBlockReceipt, SoundError> {
    let budget = match budget::admit(graph, state, clips.len(), staging_l.len(),
        staging_r.len(), obstacles.len(), budget) {
        Ok(budget) => budget,
        Err(error) => { scope.invalidate(); return Err(error); }
    };
    let preflight = (|| {
        if mode == SoundBlockMode::Paused { return Err(SoundError::Paused); }
        if scope.current != Some(pose.binding) || !live(pose.binding) { return Err(SoundError::Stale); }
        valid_pose(graph, pose)?;
        let frames = u64::try_from(staging_l.len()).map_err(|_| SoundError::Clock)?;
        let output_end = first_output_frame.checked_add(frames).ok_or(SoundError::Clock)?;
        block_start_frame.checked_add(frames).ok_or(SoundError::Clock)?;
        if scope.next_output_frame.is_some_and(|next| next != first_output_frame)
            || scope.last_pose.is_some_and(|(sequence, tick)| pose.pose_sequence < sequence || pose.pose_tick < tick) {
            return Err(SoundError::Stale);
        }
        if clips.iter().any(|c| c.binding.generation != pose.binding.source_generation) {
            return Err(SoundError::Binding);
        }
        if mode == SoundBlockMode::NaturalTail && clips.iter().any(|c| c.end_frame > block_start_frame) {
            return Err(SoundError::Clock);
        }
        Ok(output_end)
    })();
    let output_end = match preflight {
        Ok(end) => end,
        Err(error) => {
            scope.retire(state); staging_l.fill(0.); staging_r.fill(0.);
            return Err(error);
        }
    };
    let prior_listener = graph.listener;
    let prior_positions: [[f32;3]; MAX_SCENE_AUDIO_EMITTERS] =
        std::array::from_fn(|i| graph.emitters[i].emitter.position);
    graph.listener = pose.listener;
    for (node,p) in graph.emitters[..graph.emitter_count].iter_mut().zip(&pose.emitters) {
        node.emitter.position = p.position_m;
    }
    sync_wavefield(graph);
    let rendered = graph.render_scheduled_pcm_quantum_zero_heap(state,workspace,clips,
        block_start_frame,pose.binding.transport_epoch,obstacles,budget,staging_l,staging_r);
    let result = match rendered {
        Ok(report) if scope.started && (report.timeline_discontinuity || report.source_rebound) => Err(SoundError::Stale),
        Ok(report) if live(pose.binding) => Ok(report),
        Ok(_) => Err(SoundError::Stale),
        Err(e) => Err(SoundError::Schedule(e)),
    };
    match result {
        Ok(schedule) => {
            scope.started = true;
            scope.next_output_frame = Some(output_end);
            scope.last_pose = Some((pose.pose_sequence, pose.pose_tick));
            Ok(SoundBlockReceipt { binding:pose.binding,pose_sequence:pose.pose_sequence,
                pose_tick:pose.pose_tick,first_output_frame,first_source_frame:block_start_frame,
                output_frames:staging_l.len(),mode,schedule })
        }
        Err(e) => {
            graph.listener = prior_listener;
            for (node,p) in graph.emitters[..graph.emitter_count].iter_mut().zip(prior_positions) {
                node.emitter.position = p;
            }
            sync_wavefield(graph);
            // Discontinuity/cancel/DSP failures invalidate this unpublished job.
            // The existing device owner must also stop/discard its queued work.
            scope.retire(state); staging_l.fill(0.); staging_r.fill(0.);
            Err(e)
        }
    }
}
