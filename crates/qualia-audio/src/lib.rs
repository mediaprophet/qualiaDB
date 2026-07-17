//! Qualia Audio — native auditory intelligence.
//!
//! Architecture: `docs/plans/native-auditory-language-and-music-intelligence.md`  
//! Swarm: `docs/plans/native-auditory-swarm-delivery.md`
//!
//! # Rules
//! - Hot path caller-buffered; no Python.
//! - Model outputs are epistemic proposals.
//! - Frames authoritative; PCM never in NQuins.
//! - Oral languages need no orthography.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod types;
pub mod hash;
pub mod convert;
pub mod resample;
pub mod wav;
pub mod features;
pub mod semantic;
pub mod events;
pub mod media_store;
pub mod language;
pub mod music;
pub mod production;
pub mod generation;
pub mod cross_modal;
pub mod pipeline;

pub use types::*;
pub use hash::{media_digest, q_hash, MediaDigest};
pub use convert::{mono_f32_to_i16_le, to_mono_f32};
pub use resample::resample_linear_mono;
pub use wav::{decode_wav, encode_wav_i16_mono, DecodedWav};
pub use features::{
    forward_cqt_mono, frame_energy, frame_zcr, log_mel_from_mono, magnitude_stft_chunk,
    StreamingStft,
};
pub use semantic::{
    compile_auditory_quins, human_correct_quin, human_reject_quin, AudioQuin, CTX_AUDIO,
    P_AUDITORY_OBSERVATION, P_PROPOSES_SOUND_CLASS,
};
pub use events::{
    ReferenceEventModel, CLASS_NOISE, CLASS_SILENCE, CLASS_SPEECH_LIKE, CLASS_TONAL,
};
pub use media_store::{
    guard_duration_bytes, synth_silence, synth_tone_f32, AudioMediaRecord, AudioMediaStore,
    RetentionClass,
};
pub use language::{AccessClass, AnnotationTier, LanguageResourceBundle};
pub use music::{
    chroma12_from_mel, detect_onsets, estimate_f0_hz, MusicAssumptions, OnsetEvent, PitchEstimate,
};
pub use production::{ProcessPlan, TrackState, MAX_BLOCK, MAX_TRACKS};
pub use generation::{synthesize_reference_tone, SynthReceipt, VoiceConsent};
pub use cross_modal::{
    frames_to_media_ms, propose_temporal_correlations, AvCorrelationProposal, TimeIntervalMs,
};
pub use pipeline::{run_ears_demo, run_ears_on_wav_file, section18_smoke, EarsDemoResult};
