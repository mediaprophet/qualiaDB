//! MIDI 2.0 Universal MIDI Packet (UMP) + MIDI-CI (AU-MIDI-MSG).
//!
//! Re-exports only. UMP container/message-type/group fields live in [`packet`];
//! word building and MIDI 1.0→2.0 translation in [`encode`]; word parsing and
//! MIDI 2.0→1.0 translation in [`decode`]; the default resolution bit-scaling in
//! [`scale`]; the MIDI-CI Discovery skeleton in [`midi_ci`].
//!
//! - [`UmpPacket`] / [`Midi2ChannelVoice`] / [`packet_word_count`] — packet model.
//! - [`encode_midi1_channel_voice`] / [`encode_midi2_channel_voice`] — build words.
//! - [`translate_midi1_to_midi2`] / [`translate_midi2_to_midi1`] — protocol xlate.
//! - [`decode_midi1_channel_voice`] / [`decode_midi2_channel_voice`] / [`decode`] — parse.
//! - [`scale_up`] / [`scale_down`] — 7↔16 / 14↔32 resolution scaling.
//! - [`MidiCiDiscovery`] — MIDI-CI discovery message.

pub mod decode;
pub mod encode;
pub mod midi_ci;
pub mod packet;
pub mod scale;

pub use decode::{
    decode, decode_midi1_channel_voice, decode_midi2_channel_voice, translate_midi2_to_midi1,
    UmpDecoded,
};
pub use encode::{
    encode_midi1_channel_voice, encode_midi2_channel_voice, translate_midi1_to_midi2,
};
pub use midi_ci::MidiCiDiscovery;
pub use packet::{packet_word_count, Midi2ChannelVoice, UmpPacket};
pub use scale::{scale_down, scale_up};
