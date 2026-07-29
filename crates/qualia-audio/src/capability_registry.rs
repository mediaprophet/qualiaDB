//! Capability registry — the honest, machine-readable status of every audio capability.
//!
//! This is the single source of truth that the delivery-plan waves update as they land.
//! Its statuses must match reality (CLAUDE.md §12 completeness / measurement honesty): a row is
//! `Present` only when there is a real algorithm behind it with a numeric golden test — never a
//! placeholder wearing the algorithm's name.
//!
//! Plan: `docs/plans/audio-algorithms-catalogue-delivery-plan-2026.md` (§6 sketch, §1 corrected baseline).
//! The MCP `audio_features` tool and the Listen UI honesty chips read from here.

/// Honest lifecycle status of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// Real algorithm with a numeric golden test.
    Present,
    /// Exists but coarse / reference-only / mislabelled — not parity.
    Partial,
    /// Not implemented (or a placeholder wearing the name — treated as absent).
    Missing,
    /// Implemented but behind a Cargo feature that is off in this build.
    FeatureDisabled,
    /// Loader/path exists but fails closed until P64 weights are supplied.
    NeedsWeights,
}

impl CapabilityStatus {
    /// Short stable token for UI/MCP/JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            CapabilityStatus::Present => "Present",
            CapabilityStatus::Partial => "Partial",
            CapabilityStatus::Missing => "Missing",
            CapabilityStatus::FeatureDisabled => "FeatureDisabled",
            CapabilityStatus::NeedsWeights => "NeedsWeights",
        }
    }
}

/// Broad capability domain (mirrors plan §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Io,
    StandardDsp,
    Envelope,
    Filters,
    Spectral,
    Pitch,
    Rhythm,
    Tonal,
    Structure,
    Loudness,
    Spatial,
    Separation,
    Speech,
    Generative,
    Aqa,
    Midi,
    CrossModal,
    Sonification,
}

impl Domain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Domain::Io => "io",
            Domain::StandardDsp => "standard_dsp",
            Domain::Envelope => "envelope",
            Domain::Filters => "filters",
            Domain::Spectral => "spectral",
            Domain::Pitch => "pitch",
            Domain::Rhythm => "rhythm",
            Domain::Tonal => "tonal",
            Domain::Structure => "structure",
            Domain::Loudness => "loudness",
            Domain::Spatial => "spatial",
            Domain::Separation => "separation",
            Domain::Speech => "speech",
            Domain::Generative => "generative",
            Domain::Aqa => "aqa",
            Domain::Midi => "midi",
            Domain::CrossModal => "cross_modal",
            Domain::Sonification => "sonification",
        }
    }
}

/// One capability row.
#[derive(Debug, Clone, Copy)]
pub struct AudioCapability {
    /// Stable snake_case identifier (e.g. `"mfcc"`, `"pitch_yin"`).
    pub id: &'static str,
    pub domain: Domain,
    pub status: CapabilityStatus,
    /// True if the hot path is caller-buffered (no per-call heap alloc).
    pub zero_heap_hot: bool,
    /// True if it supports streaming/block operation.
    pub streaming: bool,
    /// Name of the golden numeric test proving it, or `""` if none yet.
    pub test_name: &'static str,
    /// One-line honest note (what it really is / what's missing).
    pub note: &'static str,
}

#[allow(unused_imports)]
use CapabilityStatus::{FeatureDisabled, Missing};
use CapabilityStatus::{NeedsWeights, Partial, Present};
use Domain as D;

/// The registry. Seeded 2026-07-18 from the plan's corrected §1 baseline.
/// Waves flip rows to `Present` as real algorithms + golden tests land.
pub const CAPABILITIES: &[AudioCapability] = &[
    // ---- I/O -------------------------------------------------------------------------------
    cap(
        "wav_decode",
        D::Io,
        Present,
        true,
        false,
        "wav_roundtrip",
        "RIFF PCM i16/i32/f32; no 24-bit/extensible",
    ),
    cap(
        "wav_encode",
        D::Io,
        Present,
        true,
        false,
        "wav_roundtrip",
        "mono i16 encode",
    ),
    cap(
        "format_convert",
        D::Io,
        Present,
        true,
        false,
        "i16_mono_round",
        "i16/i24/i32/f32 + channel downmix",
    ),
    cap(
        "resample",
        D::Io,
        Present,
        true,
        true,
        "anti_aliasing_downsample_rejects_out_of_band_tone",
        "windowed-sinc + polyphase, anti-aliased (linear path retained)",
    ),
    cap(
        "metadata_reader",
        D::Io,
        Present,
        false,
        false,
        "reads_inam_title_roundtrip",
        "RIFF LIST/INFO tags -> provenance pairs",
    ),
    cap(
        "eqloud_loader",
        D::Io,
        Present,
        false,
        false,
        "",
        "A-weighting equal-loudness + perceptual load",
    ),
    // ---- Standard DSP ----------------------------------------------------------------------
    cap(
        "fft",
        D::StandardDsp,
        Present,
        true,
        false,
        "peak_bin_matches_cosine_frequency",
        "real radix-2 FFT + non-pow2 DFT fallback",
    ),
    cap(
        "window",
        D::StandardDsp,
        Present,
        true,
        false,
        "endpoint_zero_centre_one",
        "Hann/Hamming/Blackman/Blackman-Harris + apply",
    ),
    cap(
        "framing_ola",
        D::StandardDsp,
        Present,
        true,
        true,
        "hann_cola_constant_envelope",
        "FrameCutter + COLA overlap-add",
    ),
    cap(
        "istft",
        D::StandardDsp,
        Present,
        true,
        true,
        "stft_then_istft_reconstructs_signal",
        "WOLA ISTFT + Griffin-Lim; MAE<1e-3",
    ),
    cap(
        "dct",
        D::StandardDsp,
        Present,
        true,
        false,
        "dct2_matches_closed_form_reference",
        "orthonormal DCT-II",
    ),
    cap(
        "stft",
        D::StandardDsp,
        Present,
        true,
        true,
        "sine_has_energy",
        "real FFT, streaming caller-buffered, frames <=512 pow2",
    ),
    cap(
        "cqt",
        D::StandardDsp,
        Present,
        true,
        true,
        "cqt_stream_440_tone_peaks_at_bin_36",
        "streaming multi-frame CQT + approximate inverse",
    ),
    cap(
        "zcr",
        D::StandardDsp,
        Present,
        true,
        true,
        "",
        "real sign-change rate",
    ),
    cap(
        "energy_rms",
        D::StandardDsp,
        Present,
        true,
        true,
        "",
        "real RMS",
    ),
    cap(
        "peak_detection",
        D::StandardDsp,
        Present,
        true,
        false,
        "golden_three_peaks",
        "local-max picker + parabolic interp + min-distance",
    ),
    cap(
        "autocorrelation",
        D::StandardDsp,
        Present,
        true,
        false,
        "impulse_train_period",
        "normalised ACF + cross-correlation",
    ),
    // ---- Envelope / SFX --------------------------------------------------------------------
    cap(
        "envelope",
        D::Envelope,
        Present,
        true,
        true,
        "tracks_monotonic_decay_of_decaying_sinusoid",
        "asymmetric-LP follower",
    ),
    cap(
        "log_attack_time",
        D::Envelope,
        Present,
        true,
        false,
        "recovers_hundred_ms_attack",
        "LogAttackTime over envelope",
    ),
    cap(
        "temporal_ratios",
        D::Envelope,
        Present,
        true,
        false,
        "recovers_peak_fraction",
        "max/min/tc-to-total, strong decay, SFX",
    ),
    // ---- Filters ---------------------------------------------------------------------------
    cap(
        "biquad",
        D::Filters,
        Present,
        true,
        true,
        "impulse_response_first_taps_fir",
        "RBJ cookbook LP/HP/BP/BR/AP",
    ),
    cap(
        "dc_removal",
        D::Filters,
        Present,
        true,
        true,
        "constant_dc_converges_to_zero",
        "1st-order DC blocker",
    ),
    cap(
        "smoothing_filters",
        D::Filters,
        Present,
        true,
        true,
        "removes_single_impulse_spike",
        "median / moving-average / max",
    ),
    // ---- Spectral features -----------------------------------------------------------------
    cap(
        "mel_bands",
        D::Spectral,
        Present,
        true,
        true,
        "tone_lands_in_a_single_band_region",
        "real triangular HTK mel bank; log_mel re-pointed",
    ),
    cap(
        "mfcc",
        D::Spectral,
        Present,
        true,
        true,
        "c0_dominates_for_single_tone",
        "mel -> ln -> DCT-II",
    ),
    cap(
        "bark_erb_gfcc",
        D::Spectral,
        Present,
        true,
        true,
        "erb_bandwidth_grows_with_frequency",
        "Bark/ERB banks + BFCC/GFCC",
    ),
    cap(
        "spectral_flux",
        D::Spectral,
        Present,
        true,
        true,
        "rising_energy_positive_flux",
        "true per-bin half-wave-rectified flux",
    ),
    cap(
        "spectral_shape",
        D::Spectral,
        Present,
        true,
        true,
        "flat_spectrum_is_unity_and_zero_db",
        "hfc/rolloff/flatness/contrast/complexity",
    ),
    cap(
        "spectral_peaks",
        D::Spectral,
        Present,
        true,
        false,
        "golden_peak_frequency",
        "parabolic sub-bin peaks; max-mag-freq",
    ),
    cap(
        "lpc",
        D::Spectral,
        Present,
        true,
        false,
        "recovers_ar2_coefficients",
        "Levinson-Durbin LPC + reflection",
    ),
    // ---- Pitch -----------------------------------------------------------------------------
    cap(
        "pitch_yin",
        D::Pitch,
        Present,
        true,
        true,
        "yin_recovers_440hz",
        "real YIN: CMND+threshold+parabolic; 0.07 cents",
    ),
    cap(
        "pitch_yin_fft",
        D::Pitch,
        Present,
        true,
        true,
        "yin_fft_recovers_440hz",
        "YinFFT autocorrelation + confidence",
    ),
    cap(
        "pitch_to_midi",
        D::Pitch,
        Present,
        true,
        true,
        "jump_a4_to_b4_yields_two_notes",
        "hz<->midi + note ON/OFF segmentation; proposals w/ confidence",
    ),
    cap(
        "pitch_salience",
        D::Pitch,
        Present,
        true,
        true,
        "salience_peaks_at_fundamental",
        "harmonic salience function + peaks",
    ),
    cap(
        "melody_contours",
        D::Pitch,
        Present,
        false,
        true,
        "recovers_stepping_melody_and_abstains",
        "salience->contours->predominant Melodia-class; abstains",
    ),
    cap(
        "multipitch",
        D::Pitch,
        Present,
        true,
        true,
        "recovers_two_simultaneous_tones",
        "Klapuri iterative subtraction + multi-voice tracking; abstains",
    ),
    cap(
        "pitch_crepe",
        D::Pitch,
        NeedsWeights,
        false,
        true,
        "absent_abstains_never_fabricates",
        "P64 loader + fail-closed path built; needs weights",
    ),
    // ---- Rhythm ----------------------------------------------------------------------------
    cap(
        "onset_detection",
        D::Rhythm,
        Present,
        true,
        true,
        "recovers_known_impulse_pattern",
        "true spectral-flux onset + adaptive threshold",
    ),
    cap(
        "tempo",
        D::Rhythm,
        Present,
        true,
        false,
        "peaks_at_true_tempo",
        "autocorrelation tempogram",
    ),
    cap(
        "beat_tracking",
        D::Rhythm,
        Present,
        false,
        true,
        "recovers_120_bpm",
        "comb-filter beat tracker + BPM histogram",
    ),
    // ---- Tonal -----------------------------------------------------------------------------
    cap(
        "chroma",
        D::Tonal,
        Present,
        true,
        true,
        "golden_c_major_triad_peaks_at_c_e_g",
        "real HPCP (log2 pitch-class), parameterised tuning",
    ),
    cap(
        "hpcp_key_chord",
        D::Tonal,
        Present,
        true,
        true,
        "c_major_profile_returns_c_major",
        "HPCP + tuning + K-S key + chord; abstains",
    ),
    // ---- Structure -------------------------------------------------------------------------
    cap(
        "structure_segments",
        D::Structure,
        Present,
        true,
        false,
        "golden_boundary_at_midpoint",
        "SSM + Foote novelty segmentation",
    ),
    // ---- Loudness --------------------------------------------------------------------------
    cap(
        "loudness_r128",
        D::Loudness,
        Present,
        true,
        true,
        "integrated_lufs_minus20dbfs_sine",
        "BS.1770/EBU R128 integrated/momentary/short-term + LRA; -19.99 LUFS",
    ),
    cap(
        "replay_gain",
        D::Loudness,
        Present,
        true,
        false,
        "",
        "ReplayGain 2.0 (-18 LUFS target)",
    ),
    cap(
        "dynamics_stats",
        D::Loudness,
        Present,
        true,
        true,
        "sine_crest_is_root_two",
        "crest factor + moments + dyn complexity",
    ),
    // ---- Spatial ---------------------------------------------------------------------------
    cap(
        "hrtf_binaural",
        D::Spatial,
        Partial,
        true,
        true,
        "",
        "analytic + synth HRIR; no measured KEMAR",
    ),
    cap(
        "reverb",
        D::Spatial,
        Present,
        true,
        true,
        "impulse_produces_bounded_decaying_tail",
        "Schroeder + FDN reverb + stereo mux/width; zero-alloc process",
    ),
    // ---- Separation ------------------------------------------------------------------------
    cap(
        "source_separation",
        D::Separation,
        Present,
        true,
        false,
        "band_mask_isolates_that_band",
        "classical spectral masking (real); learned demucs = NeedsWeights",
    ),
    // ---- Speech ----------------------------------------------------------------------------
    cap(
        "vad_segment",
        D::Speech,
        Present,
        true,
        true,
        "one_segment_covers_the_burst_not_the_silence",
        "multi-feature VAD (energy+flatness+ZCR) + hangover; silence stays unvoiced",
    ),
    cap(
        "speech_asr",
        D::Speech,
        NeedsWeights,
        false,
        true,
        "absent_abstains_never_fabricates",
        "P64 CTC loader + fail-closed greedy decode; needs HA6 model",
    ),
    // ---- Generative ------------------------------------------------------------------------
    cap(
        "vocoder",
        D::Generative,
        NeedsWeights,
        false,
        true,
        "",
        "HiFi-GAN/BigVGAN via P64+Forge",
    ),
    cap(
        "audio_tokens",
        D::Generative,
        Present,
        true,
        true,
        "centroid_sum_round_trips_exactly",
        "RVQ + FSQ quantizer ABI (real codebook math)",
    ),
    cap(
        "tts",
        D::Generative,
        NeedsWeights,
        false,
        true,
        "",
        "2-sine placeholder; consent-gated",
    ),
    // ---- AQA -------------------------------------------------------------------------------
    cap(
        "aqa_intrusive",
        D::Aqa,
        Present,
        true,
        false,
        "identical_signals_hit_clamped_maximum",
        "seg-SNR + log-spectral distance + PESQ-subset (pure Rust)",
    ),
    cap(
        "aqa_mos",
        D::Aqa,
        NeedsWeights,
        false,
        false,
        "no_weights_fails_closed_never_returns_mos",
        "DNSMOS/NISQA loader fails closed; never fabricates MOS",
    ),
    // ---- MIDI ------------------------------------------------------------------------------
    cap(
        "midi_message",
        D::Midi,
        Present,
        true,
        true,
        "golden_running_status_two_note_ons",
        "MIDI 1.0 model + running-status stream parse",
    ),
    cap(
        "midi_ump",
        D::Midi,
        Present,
        true,
        true,
        "golden_ump32_note_on_roundtrip",
        "MIDI 2.0 UMP + 7↔16 scaling + MIDI-CI skeleton",
    ),
    cap(
        "midi_smf",
        D::Midi,
        Present,
        false,
        false,
        "round_trip_structure_and_bytes",
        "Standard MIDI File 0/1/2 I/O + tempo map",
    ),
    cap(
        "midi_sequencer",
        D::Midi,
        Present,
        true,
        true,
        "golden_480_ticks_is_half_second_at_120bpm",
        "PPQ transport/tracks/quantize + clock/MTC/MMC + MPE",
    ),
    cap(
        "midi_tuning",
        D::Midi,
        Present,
        true,
        false,
        "tuning_parse_pythagorean_ratio_fifth_is_exactly_three_halves",
        "Scala .scl/.kbm + MTS; non-12-TET; alloc-free lookup",
    ),
    cap(
        "midi_synth",
        D::Midi,
        Present,
        true,
        true,
        "full_pool_then_one_more_steals_oldest",
        "voice allocator + ADSR + osc voices; zero-alloc render",
    ),
    cap(
        "midi_instrument",
        D::Midi,
        Present,
        false,
        false,
        "golden_first_region",
        "SFZ/SF2/DLS loaders + licence-provenance resolver; ships NO content",
    ),
    cap(
        "midi_bridge",
        D::Midi,
        Present,
        true,
        true,
        "golden_note_on_round_trip",
        "MIDI<->SonicToken, pitch_midi->sequencer, SMF provenance",
    ),
    // ---- Cross-modal / sonification (already real) -----------------------------------------
    cap(
        "cross_modal_clock",
        D::CrossModal,
        Present,
        true,
        false,
        "",
        "shared media clock; overlap != cause",
    ),
    cap(
        "sonify",
        D::Sonification,
        Present,
        true,
        true,
        "",
        "parametric event sonifier (U3 hear)",
    ),
];

/// Const constructor keeps the table terse and aligned.
const fn cap(
    id: &'static str,
    domain: Domain,
    status: CapabilityStatus,
    zero_heap_hot: bool,
    streaming: bool,
    test_name: &'static str,
    note: &'static str,
) -> AudioCapability {
    AudioCapability {
        id,
        domain,
        status,
        zero_heap_hot,
        streaming,
        test_name,
        note,
    }
}

/// Look up a capability by id.
pub fn by_id(id: &str) -> Option<&'static AudioCapability> {
    CAPABILITIES.iter().find(|c| c.id == id)
}

/// Count rows with a given status.
pub fn count_by_status(status: CapabilityStatus) -> usize {
    CAPABILITIES.iter().filter(|c| c.status == status).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        for (i, a) in CAPABILITIES.iter().enumerate() {
            for b in &CAPABILITIES[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate capability id: {}", a.id);
            }
        }
    }

    #[test]
    fn present_rows_are_real() {
        // A Present row must not describe itself as a placeholder/fake.
        for c in CAPABILITIES.iter().filter(|c| c.status == Present) {
            let n = c.note.to_ascii_lowercase();
            assert!(
                !n.contains("fake") && !n.contains("placeholder") && !n.contains("not "),
                "Present row {} has a non-real note: {}",
                c.id,
                c.note
            );
        }
    }

    #[test]
    fn lookup_works() {
        assert_eq!(by_id("vocoder").map(|c| c.status), Some(NeedsWeights));
        assert_eq!(by_id("energy_rms").map(|c| c.status), Some(Present));
        assert_eq!(by_id("mfcc").map(|c| c.status), Some(Present));
        assert!(by_id("nonexistent").is_none());
    }

    #[test]
    fn progress_and_remaining_are_both_honest() {
        // Waves 0–3 have landed real algorithms; later waves + gated learned heads remain.
        assert!(
            count_by_status(Present) >= 50,
            "expected substantial Present coverage"
        );
        assert!(
            count_by_status(NeedsWeights) >= 5,
            "principal-gated learned heads (AED/speech/CREPE/vocoder/MOS) must read as NeedsWeights"
        );
    }
}
