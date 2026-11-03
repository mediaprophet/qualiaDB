//! Source/pose leases over the existing PCM scheduler. No clock, player, codec,
//! device queue or new DSP is owned here. Host admission remains authoritative.
mod render;
mod budget;
pub mod cycle;
#[cfg(test)]
mod tests;
use super::pcm_schedule::{ClipScheduleError, ClipScheduleReport, ClipScheduleState};
use super::{AudioListenerPose, MAX_SCENE_AUDIO_EMITTERS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundBinding {
    pub model_sha256: [u8; 32],
    pub source_sha256: [u8; 32],
    pub graph_sha256: [u8; 32],
    pub source_generation: u64,
    pub graph_generation: u64,
    pub transport_epoch: u64,
}
impl SoundBinding {
    pub fn validate(self) -> Result<(), SoundError> {
        if self.model_sha256 == [0;32] || self.source_sha256 == [0;32]
            || self.graph_sha256 == [0;32] || self.source_generation == 0
            || self.graph_generation == 0 || self.transport_epoch == 0 {
            return Err(SoundError::Binding);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EmitterPose {
    pub node_id: u32,
    pub origin_quin_hash: u64,
    pub position_m: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct SoundPose {
    pub binding: SoundBinding,
    pub pose_sequence: u64,
    /// Existing editorial clock at which this source pose was actually sampled.
    pub pose_tick: u64,
    pub listener: AudioListenerPose,
    pub emitters: [EmitterPose; MAX_SCENE_AUDIO_EMITTERS],
    pub emitter_count: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundError {
    Binding, Stale, Denied, Pose, Budget, Clock, UnsupportedRate, Paused,
    Schedule(ClipScheduleError),
}
impl core::fmt::Display for SoundError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f,"source-bound sound refused: {self:?}")
    }
}
impl std::error::Error for SoundError {}

/// Only a lease and initial-catalogue marker; scheduling/DSP history remains
/// in the existing ClipScheduleState, and the host retains the only transport.
#[derive(Default)]
pub struct SoundScopeState {
    current: Option<SoundBinding>,
    last: Option<SoundBinding>,
    started: bool,
    next_output_frame: Option<u64>,
    last_pose: Option<(u64, u64)>,
}
impl SoundScopeState {
    pub fn binding(&self) -> Option<SoundBinding> { self.current }
    /// The host must stop/discard its old device job before changing this lease.
    /// This method does not certify device flushing or authority by itself.
    pub fn bind(&mut self, binding: SoundBinding, state: &mut ClipScheduleState<'_>)
        -> Result<(), SoundError> {
        binding.validate()?;
        if self.current == Some(binding) { return Ok(()); }
        if let Some(old) = self.last {
            if binding.source_generation == old.source_generation
                && (binding.graph_generation < old.graph_generation || binding.transport_epoch < old.transport_epoch) {
                return Err(SoundError::Stale);
            }
            if binding.source_generation == old.source_generation
                && (binding.model_sha256 != old.model_sha256 || binding.source_sha256 != old.source_sha256
                    || binding.graph_generation == old.graph_generation && binding.graph_sha256 != old.graph_sha256) {
                return Err(SoundError::Binding);
            }
            let newer = binding.source_generation > old.source_generation
                || binding.source_generation == old.source_generation
                    && (binding.graph_generation > old.graph_generation
                        || binding.graph_generation == old.graph_generation
                            && binding.transport_epoch > old.transport_epoch);
            if !newer { return Err(SoundError::Stale); }
        }
        state.reset_active();
        self.current = Some(binding);
        self.last = Some(binding);
        self.started = false;
        self.next_output_frame = None;
        self.last_pose = None;
        Ok(())
    }
    pub fn retire(&mut self, state: &mut ClipScheduleState<'_>) {
        state.reset_active();
        self.invalidate();
    }
    /// Constant-work lease invalidation when caller storage/reset work was not
    /// admitted. A later bind resets scheduler history before publication.
    pub fn invalidate(&mut self) {
        self.current = None;
        self.started = false;
        self.next_output_frame = None;
        self.last_pose = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundBlockMode {
    Playing,
    /// Only after all declared PCM clip intervals ended. Holds the caller's
    /// final source pose while the SAME scheduler history drains naturally.
    NaturalTail,
    Paused,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundBlockReceipt {
    pub binding: SoundBinding,
    pub pose_sequence: u64,
    pub pose_tick: u64,
    pub first_output_frame: u64,
    pub first_source_frame: u64,
    pub output_frames: usize,
    pub mode: SoundBlockMode,
    pub schedule: ClipScheduleReport,
}
pub use render::render_guarded;
