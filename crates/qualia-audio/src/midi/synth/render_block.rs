//! Block renderer — sums all active voices of a pool into a caller-provided buffer.
//!
//! Production hard rule: allocation-free, no locks, no FS. The output buffer is owned by
//! the caller; only const-sized voice state is touched here.

use super::voice_allocator::VoiceAllocator;
use crate::types::AudioError;

/// Render one audio block by summing every active voice into `out` (mono, one sample
/// per element). Each element is fully mixed before advancing to the next, so voice
/// phase/envelope stay sample-accurate. Returns `Ok(())`; reserved `AudioError` variants
/// allow future validation without changing the signature.
pub fn render_block<const N: usize>(
    allocator: &mut VoiceAllocator<N>,
    out: &mut [f32],
) -> Result<(), AudioError> {
    for sample in out.iter_mut() {
        let mut acc = 0.0f32;
        for v in allocator.voices_mut() {
            if v.is_active() {
                acc += v.render_sample();
            }
        }
        *sample = acc;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::adsr::AdsrEnvelope;
    use super::super::oscillator_voice::Waveform;
    use super::*;

    const SR: f32 = 48_000.0;

    fn env() -> AdsrEnvelope {
        AdsrEnvelope::new(SR, 0.001, 0.0, 1.0, 0.050)
    }

    #[test]
    fn sums_two_voice_chord_bounded_and_finite() {
        let mut alloc: VoiceAllocator<8> = VoiceAllocator::new(SR, Waveform::Sine, env());
        alloc.note_on(69, 100, 440.0); // A4
        alloc.note_on(64, 100, VoiceAllocator::<8>::freq_12tet(64)); // E4
        assert_eq!(alloc.active_count(), 2);

        let mut out = vec![0.0f32; 1024];
        render_block(&mut alloc, &mut out).expect("render");

        let mut max_abs = 0.0f32;
        let mut any_signal = false;
        for &x in &out {
            assert!(x.is_finite(), "sample must be finite: {x}");
            max_abs = max_abs.max(x.abs());
            if x.abs() > 1e-4 {
                any_signal = true;
            }
        }
        // Two voices, each gain <= 100/127 ≈ 0.787 → sum bounded well under 2.0.
        assert!(
            max_abs <= 2.0,
            "chord sum should stay bounded, got {max_abs}"
        );
        assert!(any_signal, "chord should produce non-zero signal");
    }

    #[test]
    fn empty_pool_renders_silence() {
        let mut alloc: VoiceAllocator<4> = VoiceAllocator::new(SR, Waveform::Sine, env());
        let mut out = vec![1.0f32; 64];
        render_block(&mut alloc, &mut out).expect("render");
        assert!(out.iter().all(|&x| x == 0.0), "no active voices → silence");
    }
}
