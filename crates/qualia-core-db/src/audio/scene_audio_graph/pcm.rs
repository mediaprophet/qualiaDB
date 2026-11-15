//! Caller-buffered PCM rendering through the existing scene acoustic buses.
//! Scheduling, decoding, resampling, authorization and device output belong to
//! the host. The active graph capacity is not a limit on authored source count.
use super::{
    binaural_from_position, evaluate_acoustic_occlusion_zero_heap, length3, sub3,
    AcousticObstacle, AcousticObstacleShape, AcousticOcclusionResult, BinauralGains,
    DirectBinauralBus, EarlyReflectionsBus, LateReverbFreeverbBus, SceneAudioBusGraph,
    MAX_SCENE_AUDIO_EMITTERS, EARLY_REFLECTION_TAPS,
};

/// An application-qualified clip/revision mapped onto one current graph node.
/// These keys identify the caller's assignment; they do not grant authority or
/// assert universal identity equivalence. Change generation after seek/rebind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmSourceBinding {
    pub source_key: u64,
    pub generation: u64,
    pub node_id: u32,
    pub origin_quin_hash: u64,
}

pub struct PcmVoiceBlock<'a> {
    pub slot: usize,
    pub binding: PcmSourceBinding,
    /// Mono PCM already converted/resampled to the output clock. Exactly one
    /// sample per output frame; no additional Doppler resampling occurs here.
    pub samples: &'a [f32],
    pub gain: f32,
}

#[derive(Clone, Copy)]
pub struct PcmRenderBudget {
    pub max_frames: usize,
    pub max_active_voices: usize,
    pub max_obstacles: usize,
    /// Current bus calibration is 48 kHz. Other rates are refused explicitly.
    pub sample_rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmRenderError { Budget, Layout, Binding, NumericInput, UnsupportedRate, DspFailure }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcmRenderReport {
    pub active_voices: usize,
    pub frames: usize,
    pub peak_before_limit: f32,
    pub limited_samples: usize,
    /// Conservative assignment/revocation handling resets the shared room
    /// history, potentially truncating other active sources' existing tails.
    pub histories_reset: bool,
}

/// Private caller-owned persistent PCM state, reusing the existing bus types.
/// Retain this across blocks. Synthesis has separate histories in the graph;
/// it neither contaminates nor is reset by this PCM rendering path.
#[derive(Clone)]
pub struct PcmRenderState {
    direct: DirectBinauralBus,
    early: EarlyReflectionsBus,
    late: LateReverbFreeverbBus,
    bindings: [Option<PcmSourceBinding>; MAX_SCENE_AUDIO_EMITTERS],
    initialized: bool,
}
impl Default for PcmRenderState {
    fn default() -> Self {
        Self { direct: DirectBinauralBus::default(), early: EarlyReflectionsBus::default(),
            late: LateReverbFreeverbBus::new(0.65, 0.35, 0.22),
            bindings: [None; MAX_SCENE_AUDIO_EMITTERS], initialized: false }
    }
}
impl PcmRenderState {
    /// Explicitly clear all PCM histories on transport discontinuity or revoked
    /// access. The host must cancel queued/device output separately.
    pub fn reset(&mut self) {
        self.direct.reset(); self.early.reset(); self.late.reset();
        self.bindings.fill(None); self.initialized = false;
    }
}

// Conservative numeric domain for local scene coordinates, not a global
// geographic extent. The host should choose an appropriate local origin.
fn point_ok(p: [f32; 3]) -> bool { p.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.0) }
fn size_ok(v: f32) -> bool { (0.0..=1_000_000.0).contains(&v) }
fn obstacle_ok(o: &AcousticObstacle) -> bool {
    if !size_ok(o.thickness_m) || !size_ok(o.smooth_blend_radius) { return false; }
    match o.shape {
        AcousticObstacleShape::AnalyticalSphere {center, radius} => point_ok(center) && size_ok(radius),
        AcousticObstacleShape::AnalyticalCapsule {a,b,radius} => point_ok(a) && point_ok(b) && size_ok(radius),
        AcousticObstacleShape::AnalyticalRoundedBox {center,half_extents,round_radius} =>
            point_ok(center) && half_extents.iter().all(|&v| size_ok(v)) && size_ok(round_radius),
        AcousticObstacleShape::Aabb {min,max} => point_ok(min) && point_ok(max) && (0..3).all(|i| min[i] <= max[i]),
        AcousticObstacleShape::Triangle {v0,v1,v2} => point_ok(v0) && point_ok(v1) && point_ok(v2),
    }
}
fn occlusion_ok(o: &AcousticOcclusionResult) -> bool {
    (0.0..=1.0).contains(&o.direct_gain) && (0.0..=1.0).contains(&o.enclosure_density)
        && o.early_reflection_delays_s.iter().all(|v| v.is_finite() && *v >= 0.0)
        && o.early_reflection_gains.iter().all(|v| (0.0..=1.0).contains(v))
        && [o.lowpass_biquad.b0,o.lowpass_biquad.b1,o.lowpass_biquad.b2,
            o.lowpass_biquad.a1,o.lowpass_biquad.a2].iter().all(|v| v.is_finite())
}

impl SceneAudioBusGraph {
    /// Render authorized, caller-scheduled PCM voices through scene distance,
    /// occlusion, existing binaural, early-reflection and late-reverb buses.
    /// Validation errors leave outputs/state unchanged. Unexpected DSP failure
    /// clears the outputs and resets PCM state. Output is limited to ±1 and the
    /// report counts limited samples; this is not a mastering/normalization pass.
    ///
    /// The shared early bus uses gain-weighted geometry from active voices.
    /// This is an explicit room approximation, not per-source path tracing.
    /// Doppler, propagation time and authored clip scheduling remain host work.
    pub fn render_pcm_quantum_zero_heap(&self, state: &mut PcmRenderState,
        voices: &[PcmVoiceBlock<'_>], obstacles: &[AcousticObstacle], budget: PcmRenderBudget,
        out_l: &mut [f32], out_r: &mut [f32]) -> Result<PcmRenderReport, PcmRenderError> {
        if budget.sample_rate != 48_000 { return Err(PcmRenderError::UnsupportedRate); }
        if out_l.is_empty() || out_l.len() != out_r.len() { return Err(PcmRenderError::Layout); }
        if out_l.len() > budget.max_frames || voices.len() > budget.max_active_voices
            || budget.max_active_voices > MAX_SCENE_AUDIO_EMITTERS || self.emitter_count > MAX_SCENE_AUDIO_EMITTERS
            || obstacles.len() > budget.max_obstacles { return Err(PcmRenderError::Budget); }
        if !point_ok(self.listener.position) || !self.listener.yaw_rad.is_finite()
            || !(0.0..=4.0).contains(&self.listener.manifold_w) || obstacles.iter().any(|o| !obstacle_ok(o)) {
            return Err(PcmRenderError::NumericInput);
        }
        let mut bindings = [None; MAX_SCENE_AUDIO_EMITTERS];
        let mut seen = [false; MAX_SCENE_AUDIO_EMITTERS];
        let mut gains = [BinauralGains::default(); MAX_SCENE_AUDIO_EMITTERS];
        let mut attenuation = [0.0; MAX_SCENE_AUDIO_EMITTERS];
        let mut occlusions = [AcousticOcclusionResult::default(); MAX_SCENE_AUDIO_EMITTERS];
        let mut active = 0;
        let mut enclosure = 0.0;
        let mut room_weight = 0.0;
        let mut delays = [0.0; EARLY_REFLECTION_TAPS];
        let mut reflections = [0.0; EARLY_REFLECTION_TAPS];
        for voice in voices {
            if voice.slot >= self.emitter_count || voice.slot >= MAX_SCENE_AUDIO_EMITTERS || seen[voice.slot]
                || voice.binding.source_key == 0 { return Err(PcmRenderError::Binding); }
            seen[voice.slot] = true;
            let node = &self.emitters[voice.slot];
            if node.node_id != voice.binding.node_id || node.origin_quin_hash != voice.binding.origin_quin_hash {
                return Err(PcmRenderError::Binding);
            }
            if voice.samples.len() != out_l.len() { return Err(PcmRenderError::Layout); }
            if !(0.0..=1.0).contains(&voice.gain) || voice.samples.iter().any(|v| !(-1.0..=1.0).contains(v))
                || !point_ok(node.emitter.position) || !(0.0..=1.0).contains(&node.emitter.amplitude)
                || !(0.001..=1_000_000.0).contains(&node.emitter.coherence_radius) {
                return Err(PcmRenderError::NumericInput);
            }
            if node.deontic_permit >= 2 || node.emitter.amplitude == 0.0 { continue; }
            bindings[voice.slot] = Some(voice.binding);
            let rel = sub3(node.emitter.position, self.listener.position);
            let distance = length3(rel);
            if !node.is_ambient && distance > node.emitter.coherence_radius { continue; }
            evaluate_acoustic_occlusion_zero_heap(self.listener.position, node.emitter.position, obstacles, &mut occlusions[voice.slot]);
            if !occlusion_ok(&occlusions[voice.slot]) { return Err(PcmRenderError::NumericInput); }
            let o = &occlusions[voice.slot];
            let falloff = if node.is_ambient { 1.0 } else { (1.0-distance/node.emitter.coherence_radius).clamp(0.0,1.0)/(1.0+distance*0.45) };
            let weight = voice.gain * node.emitter.amplitude * falloff;
            attenuation[voice.slot] = weight * o.direct_gain;
            gains[voice.slot] = if node.is_ambient {
                BinauralGains {gain_l: 0.707, gain_r: 0.707, itd_seconds:0.0, azimuth_rad:0.0,elevation_rad:0.0,distance:distance.max(1.0)}
            } else { binaural_from_position(rel, self.listener.yaw_rad) };
            active += 1;
            enclosure += o.enclosure_density * weight; room_weight += weight;
            for i in 0..EARLY_REFLECTION_TAPS { delays[i] += o.early_reflection_delays_s[i]*weight; reflections[i] += o.early_reflection_gains[i]*weight; }
        }
        let reset = !state.initialized || bindings != state.bindings;
        if reset { state.reset(); state.bindings = bindings; state.initialized = true; }
        if room_weight > 0.0 { enclosure /= room_weight; for i in 0..EARLY_REFLECTION_TAPS { delays[i] /= room_weight; reflections[i] /= room_weight; } }
        state.late.configure_from_enclosure(enclosure, self.listener.manifold_w);
        let mut report = PcmRenderReport {active_voices:active,frames:out_l.len(),peak_before_limit:0.0,limited_samples:0,histories_reset:reset};
        for frame in 0..out_l.len() {
            let (mut left, mut right) = (0.0,0.0);
            for voice in voices {
                let slot = voice.slot;
                if bindings[slot].is_none() { continue; }
                let (l,r) = state.direct.process_emitter_sample(slot, voice.samples[frame]*attenuation[slot],
                    &occlusions[slot].lowpass_biquad,&gains[slot],48_000.0);
                left += l; right += r;
            }
            let (el,er) = state.early.tick(left,right,&delays,&reflections,48_000.0);
            let (rl,rr) = state.late.tick(left+el,right+er);
            let l = left+el+rl; let r = right+er+rr;
            if !l.is_finite() || !r.is_finite() {
                out_l.fill(0.0); out_r.fill(0.0); state.reset(); return Err(PcmRenderError::DspFailure);
            }
            report.peak_before_limit = report.peak_before_limit.max(l.abs()).max(r.abs());
            report.limited_samples += usize::from(l.abs()>1.0)+usize::from(r.abs()>1.0);
            out_l[frame] = l.clamp(-1.0,1.0); out_r[frame] = r.clamp(-1.0,1.0);
        }
        Ok(report)
    }
}

#[cfg(test)]
#[path = "pcm_tests.rs"]
mod tests;
