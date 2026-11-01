//! Transactional event-span PCM scheduling over existing scene acoustic buses.
//! Host owners supply a complete immutable authorized clip view and exact
//! 48 kHz frame clock. This is render state, not a persisted timeline.
use super::pcm::{
    PcmRenderBudget, PcmRenderError, PcmRenderState, PcmSourceBinding, PcmVoiceBlock,
};
use super::{AcousticObstacle, SceneAudioBusGraph, MAX_SCENE_AUDIO_EMITTERS};

pub struct PcmClip<'a> {
    pub clip_key: u64,
    pub binding: PcmSourceBinding,
    pub emitter_slot: usize,
    pub start_frame: u64,
    pub end_frame: u64,
    pub source_start_frame: u64,
    pub loop_frames: Option<u64>,
    pub gain: f32,
    pub priority: i32,
    /// Decoded/resampled mono, 48 kHz; change source generation on replacement.
    pub samples: &'a [f32],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipAssignment {
    key: u64,
    binding: PcmSourceBinding,
    slot: usize,
    start: u64,
    end: u64,
    offset: u64,
    repeat: Option<u64>,
}
impl PcmClip<'_> {
    fn assignment(&self) -> ClipAssignment {
        ClipAssignment {
            key: self.clip_key,
            binding: self.binding,
            slot: self.emitter_slot,
            start: self.start_frame,
            end: self.end_frame,
            offset: self.source_start_frame,
            repeat: self.loop_frames,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Resident {
    node: u32,
    origin: u64,
    permitted: bool,
}
pub struct ClipScheduleBudget {
    pub max_authored_clips: usize,
    pub max_selected_clips: usize,
    pub max_event_spans: usize,
    /// Scheduler visits/state-copy charge; acoustic DSP is bounded separately
    /// through render frame/voice/obstacle limits, not by this number.
    pub max_work: u64,
    pub max_transaction_bytes: usize,
    pub render: PcmRenderBudget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipScheduleError {
    Budget,
    Clip,
    Binding,
    Scratch,
    TimeOverflow,
    EpochExhausted,
    NumericSample,
    Render(PcmRenderError),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClipSpanReceipt {
    pub start_frame: u64,
    pub end_frame: u64,
    pub selection_offset: usize,
    pub selected_clips: usize,
    pub eligible_clips: usize,
    pub virtualized_clips: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipScheduleReport {
    pub event_spans: usize,
    pub eligible_clip_frames: u64,
    pub selected_clip_frames: u64,
    pub virtualized_clip_frames: u64,
    pub max_selected_clips: usize,
    pub limited_input_samples: usize,
    pub limited_output_samples: usize,
    pub peak_before_limit: f32,
    pub timeline_discontinuity: bool,
    pub source_rebound: bool,
    pub histories_reset: bool,
}
/// Keep across blocks; private caller assignment memory retains exact catalog
/// records, avoiding identity-hash comparisons. Supply the complete clip view,
/// including future/past clips, rather than a changing resident-window list.
pub struct ClipScheduleState<'a> {
    assignments: &'a mut [Option<ClipAssignment>],
    count: usize,
    next_frame: Option<u64>,
    transport: u64,
    epoch: u64,
    residents: [Option<Resident>; MAX_SCENE_AUDIO_EMITTERS],
    pcm: PcmRenderState,
}
impl<'a> ClipScheduleState<'a> {
    pub fn new(assignments: &'a mut [Option<ClipAssignment>]) -> Self {
        assignments.fill(None);
        Self {
            assignments,
            count: 0,
            next_frame: None,
            transport: 0,
            epoch: 0,
            residents: [None; MAX_SCENE_AUDIO_EMITTERS],
            pcm: PcmRenderState::default(),
        }
    }
    pub fn reset(&mut self) {
        self.assignments.fill(None);
        self.count = 0;
        self.next_frame = None;
        self.pcm.reset();
        self.residents.fill(None);
    }
    pub fn active_assignment_count(&self) -> usize { self.count }
    /// Reset only records used by the admitted catalogue, not unused caller capacity.
    pub fn reset_active(&mut self) {
        self.assignments[..self.count].fill(None);
        self.count = 0;
        self.next_frame = None;
        self.pcm.reset();
        self.residents.fill(None);
    }
}
/// Disposable transaction scratch; failed blocks never publish partial output
/// or advance authoritative state. Retain to avoid reconstruction each block.
pub struct ClipScheduleWorkspace<'a> {
    mono: &'a mut [f32],
    left: &'a mut [f32],
    right: &'a mut [f32],
    selected: &'a mut [usize],
    spans: &'a mut [ClipSpanReceipt],
    valid_spans: usize,
    pcm: PcmRenderState,
}
impl<'a> ClipScheduleWorkspace<'a> {
    pub fn new(
        mono: &'a mut [f32],
        left: &'a mut [f32],
        right: &'a mut [f32],
        selected: &'a mut [usize],
        spans: &'a mut [ClipSpanReceipt],
    ) -> Self {
        Self {
            mono,
            left,
            right,
            selected,
            spans,
            valid_spans: 0,
            pcm: PcmRenderState::default(),
        }
    }
    pub fn spans(&self) -> &[ClipSpanReceipt] {
        &self.spans[..self.valid_spans]
    }
    /// Resolve only ranges named by successful span receipts.
    pub fn selections(&self) -> &[usize] {
        self.selected
    }
}
fn mul(a: usize, b: usize) -> Result<usize, ClipScheduleError> {
    a.checked_mul(b).ok_or(ClipScheduleError::Budget)
}

impl SceneAudioBusGraph {
    /// Render exact event subspans privately, then commit a complete block.
    /// End is exclusive; simultaneous end/start events take effect together.
    /// Every permitted resident emitter receives PCM or silence, preserving
    /// direct/room tails at natural clip events. Rebind, permission/residency
    /// changes and transport discontinuity clear all histories. The host must
    /// additionally cancel queued/device output on revoked access.
    /// Resident count must fit the caller's active budget (at most 64); dynamic
    /// emitter residency remains host work, independently of authored clip count.
    pub fn render_scheduled_pcm_quantum_zero_heap(
        &self,
        state: &mut ClipScheduleState<'_>,
        work: &mut ClipScheduleWorkspace<'_>,
        clips: &[PcmClip<'_>],
        block_start: u64,
        transport_generation: u64,
        obstacles: &[AcousticObstacle],
        budget: ClipScheduleBudget,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) -> Result<ClipScheduleReport, ClipScheduleError> {
        work.valid_spans = 0;
        let frames = out_l.len();
        if frames == 0
            || frames != out_r.len()
            || frames > budget.render.max_frames
            || budget.max_event_spans == 0
            || self.emitter_count > MAX_SCENE_AUDIO_EMITTERS
            || self.emitter_count > budget.render.max_active_voices
            || clips.len() > budget.max_authored_clips
        {
            return Err(ClipScheduleError::Budget);
        }
        let block_end = block_start
            .checked_add(u64::try_from(frames).map_err(|_| ClipScheduleError::TimeOverflow)?)
            .ok_or(ClipScheduleError::TimeOverflow)?;
        let selections = mul(budget.max_event_spans, budget.max_selected_clips)?;
        let mono_len = mul(self.emitter_count, frames)?;
        if work.left.len() < frames
            || work.right.len() < frames
            || work.mono.len() < mono_len
            || work.selected.len() < selections
            || work.spans.len() < budget.max_event_spans
            || state.assignments.len() < clips.len()
        {
            return Err(ClipScheduleError::Scratch);
        }
        budget_admission::admit(
            &budget,
            clips.len(),
            state.count,
            self.emitter_count,
            frames,
            mono_len,
            selections,
        )?;
        for (i, c) in clips.iter().enumerate() {
            let len = u64::try_from(c.samples.len()).map_err(|_| ClipScheduleError::Clip)?;
            if c.clip_key == 0
                || i > 0 && clips[i - 1].clip_key >= c.clip_key
                || c.start_frame >= c.end_frame
                || !(0.0..=1.0).contains(&c.gain)
                || c.source_start_frame >= len
            {
                return Err(ClipScheduleError::Clip);
            }
            let length = c.loop_frames.unwrap_or(c.end_frame - c.start_frame);
            if length == 0
                || c.source_start_frame
                    .checked_add(length)
                    .is_none_or(|end| end > len)
            {
                return Err(ClipScheduleError::Clip);
            }
            if c.emitter_slot >= self.emitter_count || c.binding.source_key == 0 {
                return Err(ClipScheduleError::Binding);
            }
            let n = &self.emitters[c.emitter_slot];
            if c.binding.node_id != n.node_id || c.binding.origin_quin_hash != n.origin_quin_hash {
                return Err(ClipScheduleError::Binding);
            }
        }
        let mut residents = [None; MAX_SCENE_AUDIO_EMITTERS];
        for (i, slot) in residents.iter_mut().enumerate().take(self.emitter_count) {
            let n = &self.emitters[i];
            *slot = Some(Resident {
                node: n.node_id,
                origin: n.origin_quin_hash,
                permitted: n.deontic_permit < 2,
            });
        }
        let rebound = state.count != clips.len()
            || clips
                .iter()
                .enumerate()
                .any(|(i, c)| state.assignments[i] != Some(c.assignment()))
            || residents != state.residents;
        let discontinuity = state.next_frame.is_some_and(|n| n != block_start)
            || state.next_frame.is_some() && state.transport != transport_generation;
        let epoch = if rebound || discontinuity || state.next_frame.is_none() {
            state
                .epoch
                .checked_add(1)
                .ok_or(ClipScheduleError::EpochExhausted)?
        } else {
            state.epoch
        };
        work.pcm.clone_from(&state.pcm);
        if rebound || discontinuity || state.next_frame.is_none() {
            work.pcm.reset();
        }
        let mut report = ClipScheduleReport {
            event_spans: 0,
            eligible_clip_frames: 0,
            selected_clip_frames: 0,
            virtualized_clip_frames: 0,
            max_selected_clips: 0,
            limited_input_samples: 0,
            limited_output_samples: 0,
            peak_before_limit: 0.0,
            timeline_discontinuity: discontinuity,
            source_rebound: rebound,
            histories_reset: false,
        };
        let mut start = block_start;
        while start < block_end {
            let span = report.event_spans;
            if span >= budget.max_event_spans {
                return Err(ClipScheduleError::Budget);
            }
            let end = clips
                .iter()
                .flat_map(|c| [c.start_frame, c.end_frame])
                .filter(|&t| t > start && t < block_end)
                .min()
                .unwrap_or(block_end);
            let n = usize::try_from(end - start).map_err(|_| ClipScheduleError::TimeOverflow)?;
            let offset = usize::try_from(start - block_start)
                .map_err(|_| ClipScheduleError::TimeOverflow)?;
            let chosen = &mut work.selected
                [span * budget.max_selected_clips..(span + 1) * budget.max_selected_clips];
            let eligible = |c: &PcmClip<'_>| {
                c.start_frame <= start
                    && start < c.end_frame
                    && self.emitters[c.emitter_slot].deontic_permit < 2
            };
            let eligible_count = clips.iter().filter(|c| eligible(c)).count();
            let mut count = 0;
            let mut previous: Option<usize> = None;
            while count < budget.max_selected_clips {
                let mut best: Option<usize> = None;
                for (i, c) in clips.iter().enumerate() {
                    if !eligible(c)
                        || previous.is_some_and(|p| {
                            c.priority > clips[p].priority
                                || c.priority == clips[p].priority
                                    && c.clip_key <= clips[p].clip_key
                        })
                    {
                        continue;
                    }
                    if best.is_none_or(|p| {
                        c.priority > clips[p].priority
                            || c.priority == clips[p].priority && c.clip_key < clips[p].clip_key
                    }) {
                        best = Some(i);
                    }
                }
                let Some(i) = best else {
                    break;
                };
                chosen[count] = i;
                count += 1;
                previous = Some(i);
            }
            for i in 1..count {
                let v = chosen[i];
                let mut j = i;
                while j > 0 && chosen[j - 1] > v {
                    chosen[j] = chosen[j - 1];
                    j -= 1;
                }
                chosen[j] = v;
            }
            work.mono[..self.emitter_count * n].fill(0.0);
            for &i in &chosen[..count] {
                let c = &clips[i];
                for frame in 0..n {
                    let elapsed = start + frame as u64 - c.start_frame;
                    let relative = c.loop_frames.map_or(elapsed, |length| elapsed % length);
                    let source = usize::try_from(c.source_start_frame + relative)
                        .map_err(|_| ClipScheduleError::Clip)?;
                    let sample = c.samples[source];
                    if !(-1.0..=1.0).contains(&sample) {
                        return Err(ClipScheduleError::NumericSample);
                    }
                    work.mono[c.emitter_slot * n + frame] += sample * c.gain;
                }
            }
            for v in &mut work.mono[..self.emitter_count * n] {
                if !v.is_finite() {
                    return Err(ClipScheduleError::NumericSample);
                }
                if v.abs() > 1.0 {
                    report.limited_input_samples += 1;
                    *v = v.clamp(-1.0, 1.0);
                }
            }
            let voices: [PcmVoiceBlock<'_>; MAX_SCENE_AUDIO_EMITTERS] =
                std::array::from_fn(|slot| {
                    let node = self.emitters[slot];
                    PcmVoiceBlock {
                        slot,
                        binding: PcmSourceBinding {
                            source_key: slot as u64 + 1,
                            generation: epoch,
                            node_id: node.node_id,
                            origin_quin_hash: node.origin_quin_hash,
                        },
                        samples: if slot < self.emitter_count {
                            &work.mono[slot * n..(slot + 1) * n]
                        } else {
                            &[]
                        },
                        gain: 1.0,
                    }
                });
            let rendered = self
                .render_pcm_quantum_zero_heap(
                    &mut work.pcm,
                    &voices[..self.emitter_count],
                    obstacles,
                    budget.render,
                    &mut work.left[offset..offset + n],
                    &mut work.right[offset..offset + n],
                )
                .map_err(ClipScheduleError::Render)?;
            work.spans[span] = ClipSpanReceipt {
                start_frame: start,
                end_frame: end,
                selection_offset: span * budget.max_selected_clips,
                selected_clips: count,
                eligible_clips: eligible_count,
                virtualized_clips: eligible_count - count,
            };
            let duration = end - start;
            report.eligible_clip_frames = report
                .eligible_clip_frames
                .checked_add(
                    duration
                        .checked_mul(eligible_count as u64)
                        .ok_or(ClipScheduleError::Budget)?,
                )
                .ok_or(ClipScheduleError::Budget)?;
            report.selected_clip_frames = report
                .selected_clip_frames
                .checked_add(
                    duration
                        .checked_mul(count as u64)
                        .ok_or(ClipScheduleError::Budget)?,
                )
                .ok_or(ClipScheduleError::Budget)?;
            report.virtualized_clip_frames =
                report.eligible_clip_frames - report.selected_clip_frames;
            report.max_selected_clips = report.max_selected_clips.max(count);
            report.limited_output_samples += rendered.limited_samples;
            report.peak_before_limit = report.peak_before_limit.max(rendered.peak_before_limit);
            report.histories_reset |= rendered.histories_reset;
            report.event_spans += 1;
            start = end;
        }
        // Only infallible bounded copies remain: no parsing/allocation/device call.
        state.pcm.clone_from(&work.pcm);
        state.assignments[..state.count.max(clips.len())].fill(None);
        for (i, c) in clips.iter().enumerate() {
            state.assignments[i] = Some(c.assignment());
        }
        state.count = clips.len();
        state.next_frame = Some(block_end);
        state.transport = transport_generation;
        state.epoch = epoch;
        state.residents = residents;
        out_l.copy_from_slice(&work.left[..frames]);
        out_r.copy_from_slice(&work.right[..frames]);
        work.valid_spans = report.event_spans;
        Ok(report)
    }
}
#[path = "pcm_schedule_budget.rs"]
mod budget_admission;
#[cfg(test)]
#[path = "pcm_schedule_tests.rs"]
mod tests;
