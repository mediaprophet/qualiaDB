//! Swarm G_audio — reference tone synthesis, consent revoke, simple stem separation.
//!
//! Licensed TTS / neural separation remain COMPLETE-WITH-GATE (HA6/HA7).

use crate::hash::q_hash;
use crate::types::AudioError;

#[derive(Debug, Clone, Copy)]
pub struct VoiceConsent {
    pub voice_asset_hash: u64,
    pub allow_synthesis: bool,
    pub allow_clone_training: bool,
    /// Lamport-ish revoke epoch; synthesis denied if `now_epoch < revoked_before`.
    pub revoked_before_epoch: u32,
}

impl VoiceConsent {
    pub fn synthesis_only(voice_id: &str) -> Self {
        Self {
            voice_asset_hash: q_hash(voice_id),
            allow_synthesis: true,
            allow_clone_training: false,
            revoked_before_epoch: 0,
        }
    }

    pub fn denied(voice_id: &str) -> Self {
        Self {
            voice_asset_hash: q_hash(voice_id),
            allow_synthesis: false,
            allow_clone_training: false,
            revoked_before_epoch: 0,
        }
    }

    /// Explicit revoke: future calls with `now_epoch >= revoked_before` still need
    /// `allow_synthesis`; revoke sets allow_synthesis=false and stamps epoch.
    pub fn revoke(&mut self, epoch: u32) {
        self.allow_synthesis = false;
        self.allow_clone_training = false;
        self.revoked_before_epoch = epoch;
    }

    pub fn is_allowed_at(&self, now_epoch: u32) -> bool {
        self.allow_synthesis && (self.revoked_before_epoch == 0 || now_epoch < self.revoked_before_epoch)
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
/// **Fails closed** if consent denies synthesis or is revoked.
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

/// Stem provenance for separation output (not evidence of source identity).
#[derive(Debug, Clone, Copy)]
pub struct StemReceipt {
    pub model_hash: u64,
    pub source_media_hash: u64,
    pub stem_class: u64,
    pub is_reference_separator: bool,
}

/// Very coarse spectral-band split: low band → "body", high residual → "detail".
/// Reference separator only — not a licensed demucs-class model.
pub fn separate_two_stems_reference(
    mono: &[f32],
    source_media_hash: u64,
    body: &mut [f32],
    detail: &mut [f32],
) -> Result<(StemReceipt, StemReceipt), AudioError> {
    let n = mono.len().min(body.len()).min(detail.len());
    if n == 0 {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let mut z = 0.0f32;
    let lp = 0.15f32;
    for i in 0..n {
        let s = mono[i];
        z += lp * (s - z);
        body[i] = z;
        detail[i] = s - z;
    }
    for i in n..body.len() {
        body[i] = 0.0;
    }
    for i in n..detail.len() {
        detail[i] = 0.0;
    }
    let model = q_hash("qualia-audio-ref-sep-v1");
    Ok((
        StemReceipt {
            model_hash: model,
            source_media_hash,
            stem_class: q_hash("https://ns.webizen.org/q42/audio/stem/body"),
            is_reference_separator: true,
        },
        StemReceipt {
            model_hash: model,
            source_media_hash,
            stem_class: q_hash("https://ns.webizen.org/q42/audio/stem/detail"),
            is_reference_separator: true,
        },
    ))
}

#[cfg(test)]
mod sep_tests {
    use super::*;

    #[test]
    fn revoke_blocks_synth() {
        let mut c = VoiceConsent::synthesis_only("v1");
        c.revoke(100);
        assert!(!c.allow_synthesis);
        let mut o = [0.0f32; 32];
        assert!(matches!(
            synthesize_reference_tone(c, 440.0, 16000, 32, 1, &mut o),
            Err(AudioError::PermissionDenied)
        ));
    }

    #[test]
    fn separation_splits_energy() {
        let mut mono = [0.0f32; 256];
        for i in 0..256 {
            mono[i] = (i as f32 * 0.1).sin() + (i as f32 * 0.7).sin() * 0.3;
        }
        let mut body = [0.0f32; 256];
        let mut detail = [0.0f32; 256];
        let (a, b) = separate_two_stems_reference(&mono, 42, &mut body, &mut detail).unwrap();
        assert!(a.is_reference_separator && b.is_reference_separator);
        let e_b: f32 = body.iter().map(|x| x * x).sum();
        let e_d: f32 = detail.iter().map(|x| x * x).sum();
        assert!(e_b > 0.0 && e_d > 0.0);
    }
}
