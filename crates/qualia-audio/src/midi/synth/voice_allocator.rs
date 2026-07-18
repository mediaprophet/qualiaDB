//! Fixed-capacity polyphonic voice pool — zero-alloc, no locks, no FS.
//!
//! [`VoiceAllocator`] owns a `[Voice; N]` array on the stack/inline. `note_on` assigns
//! a free (inactive) voice, or **steals the oldest** voice when the pool is full. Voice
//! age is a monotonic counter, so the lowest age is the oldest allocation. Frequency is
//! passed in directly (a tuning table can drive it); `note_on_12tet` provides the default
//! 12-TET mapping.

use super::adsr::AdsrEnvelope;
use super::oscillator_voice::Waveform;
use super::voice::Voice;

/// Default maximum simultaneous voices for a full-size pool.
pub const MAX_VOICES: usize = 32;

/// Fixed pool of `N` voices. All state is inline; no allocation on any path.
#[derive(Clone, Copy, Debug)]
pub struct VoiceAllocator<const N: usize> {
    voices: [Voice; N],
    /// Monotonic allocation counter (assigned as each voice's `age`).
    next_age: u64,
}

impl<const N: usize> VoiceAllocator<N> {
    /// Build a pool of `N` idle voices sharing a waveform + envelope template.
    pub fn new(sample_rate: f32, waveform: Waveform, env: AdsrEnvelope) -> Self {
        let template = Voice::new(sample_rate, waveform, env);
        Self {
            voices: [template; N],
            next_age: 0,
        }
    }

    /// Equal-temperament frequency (Hz) for a MIDI note (A4 = 69 = 440 Hz).
    pub fn freq_12tet(note: u8) -> f32 {
        440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
    }

    /// Start a note at an explicit frequency (Hz). Assigns a free voice, or steals the
    /// oldest voice when the pool is full. The active voice count never exceeds `N`.
    pub fn note_on(&mut self, note: u8, velocity: u8, freq_hz: f32) {
        if N == 0 {
            return;
        }
        let age = self.next_age;
        self.next_age = self.next_age.wrapping_add(1);

        // Prefer a free (inactive) voice.
        let mut target: Option<usize> = None;
        for (i, v) in self.voices.iter().enumerate() {
            if !v.is_active() {
                target = Some(i);
                break;
            }
        }
        let idx = target.unwrap_or_else(|| self.oldest_index());
        self.voices[idx].note_on(note, velocity, freq_hz, age);
    }

    /// Start a note using the default 12-TET frequency for `note`.
    pub fn note_on_12tet(&mut self, note: u8, velocity: u8) {
        self.note_on(note, velocity, Self::freq_12tet(note));
    }

    /// Release all held voices matching `note`.
    pub fn note_off(&mut self, note: u8) {
        for v in self.voices.iter_mut() {
            if v.is_held() && v.note() == note {
                v.note_off();
            }
        }
    }

    /// Number of voices currently producing signal.
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// Pool capacity (`N`).
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Immutable view of the voice array (for rendering / inspection).
    #[inline]
    pub fn voices(&self) -> &[Voice] {
        &self.voices
    }

    /// Mutable view of the voice array (used by the render path).
    #[inline]
    pub fn voices_mut(&mut self) -> &mut [Voice] {
        &mut self.voices
    }

    /// Index of the oldest voice (smallest age). `N >= 1` guaranteed by callers.
    fn oldest_index(&self) -> usize {
        let mut best = 0usize;
        let mut best_age = u64::MAX;
        for (i, v) in self.voices.iter().enumerate() {
            if v.age() < best_age {
                best_age = v.age();
                best = i;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn env() -> AdsrEnvelope {
        AdsrEnvelope::new(SR, 0.001, 0.0, 1.0, 0.100)
    }

    #[test]
    fn twelve_tet_reference_pitches() {
        assert!((VoiceAllocator::<4>::freq_12tet(69) - 440.0).abs() < 1e-3);
        assert!((VoiceAllocator::<4>::freq_12tet(57) - 220.0).abs() < 1e-2);
        assert!((VoiceAllocator::<4>::freq_12tet(60) - 261.6256).abs() < 1e-2);
    }

    #[test]
    fn full_pool_then_one_more_steals_oldest() {
        let mut alloc: VoiceAllocator<4> = VoiceAllocator::new(SR, Waveform::Sine, env());
        // Fill all four voices with distinct notes; note 60 is oldest (age 0).
        for &n in &[60u8, 62, 64, 65] {
            alloc.note_on(n, 100, VoiceAllocator::<4>::freq_12tet(n));
        }
        assert_eq!(alloc.active_count(), 4);

        // One more note must steal the oldest (60), not exceed capacity.
        alloc.note_on(67, 100, VoiceAllocator::<4>::freq_12tet(67));
        assert!(alloc.active_count() <= 4, "count must stay <= N");
        assert_eq!(alloc.active_count(), 4);

        let notes: [u8; 4] = {
            let mut arr = [0u8; 4];
            for (i, v) in alloc.voices().iter().enumerate() {
                arr[i] = v.note();
            }
            arr
        };
        assert!(!notes.contains(&60), "oldest note 60 should have been stolen: {notes:?}");
        assert!(notes.contains(&67), "new note 67 should be present: {notes:?}");
        // The three younger notes survive.
        for n in [62u8, 64, 65] {
            assert!(notes.contains(&n), "younger note {n} should survive: {notes:?}");
        }
    }

    #[test]
    fn note_off_releases_matching_voice() {
        let mut alloc: VoiceAllocator<4> = VoiceAllocator::new(SR, Waveform::Sine, env());
        alloc.note_on(60, 100, 261.63);
        assert!(alloc.voices()[0].is_held());
        alloc.note_off(60);
        assert!(!alloc.voices()[0].is_held());
        // Still active during release.
        assert!(alloc.voices()[0].is_active());
    }

    #[test]
    fn zero_capacity_is_safe() {
        let mut alloc: VoiceAllocator<0> = VoiceAllocator::new(SR, Waveform::Sine, env());
        alloc.note_on(60, 100, 261.63);
        assert_eq!(alloc.active_count(), 0);
    }
}
