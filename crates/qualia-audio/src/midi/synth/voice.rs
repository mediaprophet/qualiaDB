//! Voice state shared by the allocator — an [`OscillatorVoice`] plus the bookkeeping
//! (note number, allocation age, held flag) the [`VoiceAllocator`] needs for note
//! matching and oldest-voice stealing. `Copy`, inline, zero-alloc.
//!
//! [`VoiceAllocator`]: super::voice_allocator::VoiceAllocator

use super::adsr::AdsrEnvelope;
use super::oscillator_voice::{OscillatorVoice, Waveform};

/// A pool voice: the sound generator plus allocator bookkeeping.
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    osc: OscillatorVoice,
    /// MIDI note currently (or last) assigned to this voice.
    note: u8,
    /// Monotonic allocation sequence number — lower is older.
    age: u64,
    /// `true` between `note_on` and `note_off` (key still down).
    held: bool,
}

impl Voice {
    /// Build an idle voice from a waveform + envelope template.
    pub fn new(sample_rate: f32, waveform: Waveform, env: AdsrEnvelope) -> Self {
        Self {
            osc: OscillatorVoice::new(sample_rate, waveform, env),
            note: 0,
            age: 0,
            held: false,
        }
    }

    /// Assign a note to this voice. `age` is the allocator's monotonic counter.
    pub fn note_on(&mut self, note: u8, velocity: u8, freq_hz: f32, age: u64) {
        self.note = note;
        self.age = age;
        self.held = true;
        self.osc.note_on(freq_hz, velocity);
    }

    /// Release the note (envelope enters release; voice stays active until it decays).
    pub fn note_off(&mut self) {
        self.held = false;
        self.osc.note_off();
    }

    /// `true` while the voice still contributes signal.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.osc.is_active()
    }

    /// `true` while the key is held (post `note_on`, pre `note_off`).
    #[inline]
    pub fn is_held(&self) -> bool {
        self.held
    }

    /// MIDI note number currently assigned.
    #[inline]
    pub fn note(&self) -> u8 {
        self.note
    }

    /// Allocation age (lower is older).
    #[inline]
    pub fn age(&self) -> u64 {
        self.age
    }

    /// Render one sample from this voice's oscillator.
    #[inline]
    pub fn render_sample(&mut self) -> f32 {
        self.osc.render_sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn env() -> AdsrEnvelope {
        AdsrEnvelope::new(SR, 0.001, 0.0, 1.0, 0.010)
    }

    #[test]
    fn lifecycle_active_and_held_flags() {
        let mut v = Voice::new(SR, Waveform::Sine, env());
        assert!(!v.is_active());
        assert!(!v.is_held());

        v.note_on(60, 100, 261.63, 7);
        assert!(v.is_active());
        assert!(v.is_held());
        assert_eq!(v.note(), 60);
        assert_eq!(v.age(), 7);

        v.note_off();
        // Still active (in release) but no longer held.
        assert!(v.is_active());
        assert!(!v.is_held());
    }
}
