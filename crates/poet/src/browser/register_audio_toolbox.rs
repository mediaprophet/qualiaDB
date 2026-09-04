//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_audio_toolbox(reg: &mut Registry) {
    register_compact_toolbox(
        reg,
        "audio",
        "Audio, Triad Synth & Speech",
        "audio",
        "p64",
        "audio",
        "Triad formant synthesis, PCM capture, and neural audio latents.",
        "Triad Synthesis & Audio",
        &[
            CompactTool {
                id: "place_audio_session",
                label: "+ Audio session",
                icon: "audio",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place an Audio session (transport + oscillator). Not a nested DAW.",
            },
            CompactTool {
                id: "place_media",
                label: "+ Triad Formant Synthesizer",
                icon: "media",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the live media/audio synthesis surface.",
            },
            CompactTool {
                id: "mic_capture",
                label: "Mic Capture (PCM Stream)",
                icon: "audio",
                kind: ToolKind::RunAction,
                action: ActionType::Invoke,
                description: "Capture a bounded PCM stream.",
            },
            CompactTool {
                id: "neural_latents",
                label: "Neural Audio Latents (P64)",
                icon: "audio",
                kind: ToolKind::RunAction,
                action: ActionType::Invoke,
                description: "Inspect P64 audio latent state.",
            },
        ],
    );
}
