//! Swarm G_audio — reference tone synthesis with explicit voice consent gate.

use crate::hash::q_hash;
use crate::types::AudioError;

#[derive(Debug, Clone, Copy)]
pub struct VoiceConsent {
    pub voice_asset_hash: u64,
    pub allow_synthesis: bool,
    pub allow_clone_training: bool,
}

impl VoiceConsent {
    pub fn synthesis_only(voice_id: &str) -> Self {
        Self {
            voice_asset_hash: q_hash(voice_id),
            allow_synthesis: true,
            allow_clone_training: false,
        }
    }

    pub fn denied(voice_id: &str) -> Self {
        Self {
            voice_asset_hash: q_hash(voice_id),
            allow_synthesis: false,
            allow_clone_training: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SynthReceipt {
    pub model_hash: u64,
    pub voice_hash: u64,
    pub seed: u64,
    pub sample_rate: u32,
    pub frames: u32,
    pub is_reference_synth: bool,
}

const MODEL: &str = "qualia-audio-ref-tone-synth-v1";

/// Synthesize a simple multi-partial tone into mono f32.
/// **Fails closed** if `consent.allow_synthesis` is false.
pub fn synthesize_reference_tone(
    consent: VoiceConsent,
    freq_hz: f32,
    sample_rate: u32,
    frames: usize,
    seed: u64,
    out: &mut [f32],
) -> Result<SynthReceipt, AudioError> {
    if !consent.allow_synthesis {
        return Err(AudioError::PermissionDenied);
    }
    if out.len() < frames || sample_rate == 0 {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let phase0 = (seed as f32 / u64::MAX as f32) * core::f32::consts::TAU;
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let mut s = 0.0f32;
        s += 0.5 * (core::f32::consts::TAU * freq_hz * t + phase0).sin();
        s += 0.2 * (core::f32::consts::TAU * freq_hz * 2.0 * t + phase0).sin();
        out[i] = s * 0.4;
    }
    Ok(SynthReceipt {
        model_hash: q_hash(MODEL),
        voice_hash: consent.voice_asset_hash,
        seed,
        sample_rate,
        frames: frames as u32,
        is_reference_synth: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_without_consent() {
        let mut o = [0.0f32; 64];
        let r = synthesize_reference_tone(
            VoiceConsent::denied("v1"),
            440.0,
            16000,
            64,
            1,
            &mut o,
        );
        assert!(matches!(r, Err(AudioError::PermissionDenied)));
    }

    #[test]
    fn allow_synth() {
        let mut o = [0.0f32; 256];
        let r = synthesize_reference_tone(
            VoiceConsent::synthesis_only("v1"),
            440.0,
            16000,
            256,
            2,
            &mut o,
        )
        .unwrap();
        assert!(r.is_reference_synth);
        assert!(!r.allow_clone());
    }
}

impl SynthReceipt {
    /// Receipt never elevates clone rights (always false).
    pub fn allow_clone(&self) -> bool {
        false
    }
}
