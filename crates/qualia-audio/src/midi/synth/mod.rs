//! Synth — voice allocator + ADSR + oscillator voices → audio. Re-exports only (AU-MIDI-SYNTH).
//!
//! Polyphonic subtractive-lite synth: a fixed-capacity [`VoiceAllocator`] of
//! [`OscillatorVoice`]s (waveform × [`AdsrEnvelope`] × velocity), summed by
//! [`render_block`]. The render path is zero-alloc, lock-free, and FS-free — all voice
//! state is inline in a `[Voice; N]` array. Frequencies are supplied directly so a
//! tuning table can drive pitch; [`VoiceAllocator::note_on_12tet`] is the equal-temperament
//! default.

mod adsr;
mod oscillator_voice;
mod render_block;
mod voice;
mod voice_allocator;

pub use adsr::AdsrEnvelope;
pub use oscillator_voice::{OscillatorVoice, Waveform};
pub use render_block::render_block;
pub use voice::Voice;
pub use voice_allocator::{VoiceAllocator, MAX_VOICES};
