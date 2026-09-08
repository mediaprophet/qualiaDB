//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_audio_toolbox(reg: &mut Registry) {
    let session_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:place_audio_session".into(),
                label: "+ Audio session".into(),
                icon: "audio".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "audio".into(),
                description: "Place an Audio session (transport + oscillator). Not a nested DAW."
                    .into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:place_media".into(),
                label: "+ Triad Formant Synthesizer".into(),
                icon: "media".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "audio".into(),
                description: "Place the live media/audio synthesis surface.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:mic_capture".into(),
                label: "Mic Capture (PCM Stream)".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "audio".into(),
                description: "Capture a bounded PCM stream.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:neural_latents".into(),
                label: "Neural Audio Latents (P64)".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "audio".into(),
                description: "Inspect P64 audio latent state.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let dsp_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_ep_temp".into(),
                label: "Epistemic τ".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.epistemic_temperature_from_q".into()),
                ontology_prefix: "audio".into(),
                description: "τ = clamp(q², 0, 4) via Audio.epistemic_temperature_from_q.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_ep_fm".into(),
                label: "Epistemic FM".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.epistemic_fm_index".into()),
                ontology_prefix: "audio".into(),
                description: "FM index from epistemic q and carrier μ.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_sigma_freq".into(),
                label: "σ dominant Hz".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.sigma_dominant_frequency".into()),
                ontology_prefix: "audio".into(),
                description: "Map 64 preview bins to a dominant frequency (Hz).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_parametric_sample".into(),
                label: "Parametric sample".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.parametric_sample".into()),
                ontology_prefix: "audio".into(),
                description: "Advance one sample from a parametric voice state.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_bin_freq_linear".into(),
                label: "Bin → Hz linear".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.bin_to_freq_linear".into()),
                ontology_prefix: "audio".into(),
                description: "STFT bin index to frequency (Hz).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_bin_freq_log".into(),
                label: "Bin → Hz log".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.bin_to_freq_log".into()),
                ontology_prefix: "audio".into(),
                description: "CQT-style log bin to frequency (Hz).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_midi_note".into(),
                label: "MIDI → Hz".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.midi_note".into()),
                ontology_prefix: "audio".into(),
                description: "Convert MIDI note number to frequency (to_freq).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_quantize".into(),
                label: "Quantize beat".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.quantize".into()),
                ontology_prefix: "audio".into(),
                description: "Quantize a beat position onto a grid.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:dsp_transpose".into(),
                label: "Transpose MIDI".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.transpose".into()),
                ontology_prefix: "audio".into(),
                description: "Transpose a MIDI note by semitones (0–127 clamp).".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let fx_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_oscillator".into(),
                label: "Oscillator".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.oscillator".into()),
                ontology_prefix: "audio".into(),
                description: "Render a waveform buffer (sine/square/sawtooth/triangle).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_envelope".into(),
                label: "Envelope (ADSR)".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.envelope".into()),
                ontology_prefix: "audio".into(),
                description: "Render an ADSR envelope buffer.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_filter".into(),
                label: "Biquad filter".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.filter".into()),
                ontology_prefix: "audio".into(),
                description: "Apply a biquad (lowpass/highpass/bandpass/notch).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_lfo".into(),
                label: "LFO".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.lfo".into()),
                ontology_prefix: "audio".into(),
                description: "Render an LFO modulation buffer.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_delay".into(),
                label: "Delay".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.delay".into()),
                ontology_prefix: "audio".into(),
                description: "Apply a delay effect (delay_samples, feedback, mix).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_reverb".into(),
                label: "Reverb".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.reverb".into()),
                ontology_prefix: "audio".into(),
                description: "Apply a reverb (room_size, damping, mix).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_compressor".into(),
                label: "Compressor".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.compressor".into()),
                ontology_prefix: "audio".into(),
                description: "Apply dynamic-range compression.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_eq".into(),
                label: "3-band EQ".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.eq".into()),
                ontology_prefix: "audio".into(),
                description: "Apply a 3-band EQ (low/mid/high gain).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_transport".into(),
                label: "Transport".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.transport".into()),
                ontology_prefix: "audio".into(),
                description: "Transport state (play/stop/pause/record/status).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_waveform_meter".into(),
                label: "Waveform meter".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.waveform_meter".into()),
                ontology_prefix: "audio".into(),
                description: "Peak/RMS and bucketed waveform display.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_phase_meter".into(),
                label: "Phase meter".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.phase_meter".into()),
                ontology_prefix: "audio".into(),
                description: "Stereo phase correlation (left/right; one list splits).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_loudness_meter".into(),
                label: "Loudness (LUFS)".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.loudness_meter".into()),
                ontology_prefix: "audio".into(),
                description: "Measure LUFS loudness of an input buffer.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "audio:fx_spectrum".into(),
                label: "Spectrum".into(),
                icon: "audio".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Audio.spectrum".into()),
                ontology_prefix: "audio".into(),
                description: "Spectral flux and energy from a time-frequency raster.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "audio".into(),
            label: "Audio, Triad Synth & Speech".into(),
            icon: "audio".into(),
            ontology_prefix: "audio".into(),
            description: "Triad formant synthesis, PCM capture, and neural audio latents.".into(),
            enabled_by_default: true,
            family: "audio".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "audio:tools".into(),
                    label: "Triad Synthesis & Audio".into(),
                    icon: "audio".into(),
                    description: "Triad formant synthesis, PCM capture, and neural audio latents."
                        .into(),
                },
                session_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "audio:dsp".into(),
                    label: "Live audio DSP".into(),
                    icon: "audio".into(),
                    description:
                        "Curated Audio.* DSP scalars — epistemic τ/FM, bin→Hz, MIDI, grid."
                            .into(),
                },
                dsp_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "audio:fx".into(),
                    label: "Live audio FX".into(),
                    icon: "audio".into(),
                    description:
                        "Oscillator, ADSR, biquad, LFO, delay/reverb/comp/EQ, transport, meters."
                            .into(),
                },
                fx_tools,
            ),
        ],
    ));
}
