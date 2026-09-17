//! Digital signal processing — DAW-style synthesis, effects, MIDI, transport, meters.
//!
//! This module provides the traditional DAW functionality that the spectral-first
//! audio engine does not cover:
//!
//! - `oscillator`: Sine, square, sawtooth, triangle waveform synthesis.
//! - `envelope`: ADSR amplitude envelopes.
//! - `filter`: Biquad filters (LP, HP, BP, notch) via RBJ cookbook formulas.
//! - `lfo`: Low-frequency oscillator for modulation.
//! - `effects`: Delay, reverb, compressor, EQ.
//! - `midi_transport`: MIDI note utilities, quantization, transpose, transport state machine.
//! - `meters`: Waveform, phase correlation, and LUFS loudness meters.

pub mod effects;
pub mod envelope;
pub mod filter;
pub mod lfo;
pub mod meters;
pub mod midi_transport;
pub mod oscillator;

pub use effects::{Compressor, Delay, Equalizer, Reverb};
pub use envelope::{AdsrEnvelope, EnvStage};
pub use filter::{BiquadFilter, FilterType};
pub use lfo::Lfo;
pub use meters::{LoudnessMeter, PhaseMeter, WaveformMeter};
pub use midi_transport::{
    freq_to_midi_note, midi_note_to_freq, midi_to_note_name, note_name_to_midi, quantize,
    transpose, Transport, TransportState,
};
pub use oscillator::{Oscillator, Waveform};
