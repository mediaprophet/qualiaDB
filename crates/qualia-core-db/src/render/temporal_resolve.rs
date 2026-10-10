//! Bounded temporal-resolve policy and CPU reference for the renderer frame graph.
//!
//! This module deliberately owns the data-independent part of temporal output: capability
//! admission, history reset/publication state, and a small reference resolve used by focused
//! tests and image-oracle work. The current GPU mesh path does not yet produce motion vectors or
//! a reactive mask, so the frame graph only enables this path when a caller explicitly declares
//! those inputs available. That keeps the existing output path unchanged on every current
//! renderer backend.

use super::output::pbr_neutral_v1_srgb_with_controls;

/// Configuration supplied by the renderer owner for one temporal-output schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalResolveConfig {
    pub enabled: bool,
    pub motion_vectors_available: bool,
    pub reactive_mask_available: bool,
    pub history_valid: bool,
    pub reset_history: bool,
}

impl TemporalResolveConfig {
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            motion_vectors_available: false,
            reactive_mask_available: false,
            history_valid: false,
            reset_history: false,
        }
    }

    /// Whether the graph may safely schedule temporal accumulation.
    ///
    /// Linear depth is renderer-owned by the existing frame graph. Motion and reactive inputs
    /// are required because silently treating either as zero would turn disocclusions and
    /// transparencies into persistent history ghosts.
    pub const fn can_schedule(self) -> bool {
        self.enabled && self.motion_vectors_available && self.reactive_mask_available
    }

    /// Whether this frame may read the previous colour history target.
    pub const fn reads_history(self) -> bool {
        self.can_schedule() && self.history_valid && !self.reset_history
    }
}

impl Default for TemporalResolveConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Renderer-owned history lifecycle state. It contains no GPU handles and is safe to keep beside
/// a scheduler while the backend owns the actual history textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalHistoryState {
    history_valid: bool,
    frame_index: u64,
}

impl TemporalHistoryState {
    pub const fn new() -> Self {
        Self {
            history_valid: false,
            frame_index: 0,
        }
    }

    pub const fn history_valid(self) -> bool {
        self.history_valid
    }

    pub const fn frame_index(self) -> u64 {
        self.frame_index
    }

    /// Begin a frame and consume an explicit history reset request.
    pub fn begin_frame(&mut self, reset_history: bool) -> TemporalFramePlan {
        if reset_history {
            self.history_valid = false;
        }
        TemporalFramePlan {
            frame_index: self.frame_index,
            reads_history: self.history_valid,
            reset_history,
        }
    }

    /// Publish the resolved colour after the frame's temporal pass completed.
    pub fn publish_history(&mut self) {
        self.history_valid = true;
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Drop history after resize, camera cut, device recovery, or a seek.
    pub fn invalidate(&mut self) {
        self.history_valid = false;
    }
}

impl Default for TemporalHistoryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-frame lifecycle decision passed to a graph configuration or backend adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalFramePlan {
    pub frame_index: u64,
    pub reads_history: bool,
    pub reset_history: bool,
}

/// Inputs for the allocation-free reference resolve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalResolveSample {
    pub current_scene_linear: [f32; 4],
    pub history_scene_linear: [f32; 4],
    pub current_linear_depth: f32,
    pub history_linear_depth: f32,
    pub motion_vector: [f32; 2],
    pub reactive: f32,
    pub history_valid: bool,
    pub reset_history: bool,
}

const DISOCCLUSION_DEPTH_DELTA: f32 = 0.01;
const MAX_MOTION_FOR_HISTORY: f32 = 2.0;
const MAX_HISTORY_WEIGHT: f32 = 0.9;

/// Resolve one scene-linear sample with conservative rejection for invalid history, depth
/// disocclusion, motion, and reactive content. This is the CPU oracle for the eventual shader;
/// it does not allocate and never returns non-finite colour.
pub fn resolve_scene_linear_rgba(sample: TemporalResolveSample) -> Option<[f32; 4]> {
    if !finite_rgba(sample.current_scene_linear)
        || !finite_rgba(sample.history_scene_linear)
        || !sample.current_linear_depth.is_finite()
        || !sample.history_linear_depth.is_finite()
        || !sample.motion_vector[0].is_finite()
        || !sample.motion_vector[1].is_finite()
        || !sample.reactive.is_finite()
    {
        return None;
    }

    let current = sample.current_scene_linear;
    if sample.reset_history
        || !sample.history_valid
        || (sample.current_linear_depth - sample.history_linear_depth).abs()
            > DISOCCLUSION_DEPTH_DELTA
    {
        return Some(current);
    }

    let motion = (sample.motion_vector[0] * sample.motion_vector[0]
        + sample.motion_vector[1] * sample.motion_vector[1])
        .sqrt();
    let motion_confidence = 1.0 - (motion / MAX_MOTION_FOR_HISTORY).clamp(0.0, 1.0);
    let history_weight = ((1.0 - sample.reactive.clamp(0.0, 1.0))
        * motion_confidence
        * MAX_HISTORY_WEIGHT)
        .clamp(0.0, MAX_HISTORY_WEIGHT);

    Some(std::array::from_fn(|index| {
        current[index] * (1.0 - history_weight)
            + sample.history_scene_linear[index] * history_weight
    }))
}

/// Apply the existing versioned output transform exactly once to a resolved scene-linear sample.
pub fn resolve_to_srgb_capture(
    sample: TemporalResolveSample,
    exposure_ev: f32,
    white_balance_gains: [f32; 3],
) -> Option<[f32; 4]> {
    let resolved = resolve_scene_linear_rgba(sample)?;
    let rgb = pbr_neutral_v1_srgb_with_controls(
        [resolved[0], resolved[1], resolved[2]],
        exposure_ev,
        white_balance_gains,
    )?;
    Some([rgb[0], rgb[1], rgb[2], resolved[3].clamp(0.0, 1.0)])
}

fn finite_rgba(color: [f32; 4]) -> bool {
    color.iter().all(|channel| channel.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TemporalResolveSample {
        TemporalResolveSample {
            current_scene_linear: [0.4, 0.2, 0.1, 0.75],
            history_scene_linear: [0.2, 0.1, 0.05, 0.5],
            current_linear_depth: 0.5,
            history_linear_depth: 0.5,
            motion_vector: [0.0, 0.0],
            reactive: 0.0,
            history_valid: true,
            reset_history: false,
        }
    }

    #[test]
    fn invalid_or_reset_history_uses_current_sample() {
        let mut reset = sample();
        reset.reset_history = true;
        assert_eq!(resolve_scene_linear_rgba(reset), Some(reset.current_scene_linear));

        let mut invalid = sample();
        invalid.history_valid = false;
        assert_eq!(resolve_scene_linear_rgba(invalid), Some(invalid.current_scene_linear));

        let mut disoccluded = sample();
        disoccluded.history_linear_depth = 0.7;
        assert_eq!(resolve_scene_linear_rgba(disoccluded), Some(disoccluded.current_scene_linear));
    }

    #[test]
    fn reactive_and_motion_inputs_reduce_history_weight() {
        let stable = resolve_scene_linear_rgba(sample()).unwrap();
        let mut reactive = sample();
        reactive.reactive = 1.0;
        assert_eq!(resolve_scene_linear_rgba(reactive), Some(reactive.current_scene_linear));

        let mut moving = sample();
        moving.motion_vector = [MAX_MOTION_FOR_HISTORY, 0.0];
        assert_eq!(resolve_scene_linear_rgba(moving), Some(moving.current_scene_linear));
        assert!(stable[0] < sample().current_scene_linear[0]);
    }

    #[test]
    fn history_state_requires_publication_after_reset() {
        let mut state = TemporalHistoryState::new();
        assert!(!state.begin_frame(false).reads_history);
        state.publish_history();
        assert!(state.begin_frame(false).reads_history);
        let reset = state.begin_frame(true);
        assert!(reset.reset_history);
        assert!(!reset.reads_history);
        state.publish_history();
        assert_eq!(state.frame_index(), 2);
        assert!(state.history_valid());
    }

    #[test]
    fn final_transform_is_applied_after_scene_linear_resolve() {
        let sample = sample();
        let resolved = resolve_scene_linear_rgba(sample).unwrap();
        let output = resolve_to_srgb_capture(sample, 0.0, [1.0; 3]).unwrap();
        let expected = pbr_neutral_v1_srgb_with_controls(
            [resolved[0], resolved[1], resolved[2]],
            0.0,
            [1.0; 3],
        )
        .unwrap();
        assert_eq!(&output[..3], &expected);
        assert_eq!(output[3], resolved[3]);
    }

    #[test]
    fn malformed_samples_fail_closed() {
        let mut malformed = sample();
        malformed.current_scene_linear[0] = f32::NAN;
        assert!(resolve_scene_linear_rgba(malformed).is_none());
        malformed = sample();
        malformed.reactive = f32::INFINITY;
        assert!(resolve_to_srgb_capture(malformed, 0.0, [1.0; 3]).is_none());
    }
}
