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
pub mod capability_registry;
pub mod io;
pub mod fx;
pub mod midi;
pub mod models;
pub mod aqa;
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
pub mod capture;
pub mod aed_weights;
pub mod speech;
pub mod sonify;
pub mod session_history;
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
    chroma12_from_mel, detect_onsets, estimate_f0_hz, estimate_tempo_from_onsets,
    propose_structure_segments, track_pitch, MusicAssumptions, OnsetEvent, PitchEstimate,
    StructureSegment, TempoEstimate,
};
pub use production::{
    apply_comp_sample, apply_eq_sample, DelayLine, ProcessPlan, TrackState, MAX_BLOCK,
    MAX_DELAY_SAMPLES, MAX_TRACKS,
};
pub use generation::{
    separate_two_stems_reference, synthesize_reference_tone, StemReceipt, SynthReceipt,
    VoiceConsent,
};
pub use cross_modal::{
    events_overlapping_window, frames_to_media_ms, propose_temporal_correlations,
    AvCorrelationProposal, SharedMediaClock, TimeIntervalMs,
};
pub use pipeline::{
    run_ears_demo, run_ears_on_wav_file, run_ears_weighted, section18_smoke, sonify_demo_to_wav,
    speech_phone_demo, EarsDemoResult,
};
pub use capture::{CapturePurpose, CaptureSession};
pub use aed_weights::{AedWeightBundle, WeightedAedModel};
pub use speech::{
    decode_for_language, greedy_phone_decode, SpeechEncoderWeights, StreamingSpeechDecoder, PHONES,
};
pub use sonify::{class_to_hz, sonify_events_mono};
pub use session_history::{
    AutomationLane, AutomationPoint, OpKind, SessionHistory, SessionOp, MAX_AUTO_POINTS,
    MAX_HISTORY,
};
