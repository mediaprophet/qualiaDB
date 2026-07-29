use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // ── AI & Inference ────────────────────────────────────────────────────
        QApp {
            id: "chat",
            name: "Neuro-Symbolic Chat",
            tagline: "Conversational AI",
            desc: "Phase 8 bifurcated LLM with real-time Webizen Sentinel oversight. \
                   LogitStream → ControlStream ring buffers gate every token; all output requires ≥1 NQuin provenance citation.",
            icon: "chat-dots",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
        QApp {
            id: "llm-harness",
            name: "LLM Model Harness",
            tagline: "Model Testing",
            desc: "Load GGUF models via memmap2, run autoregressive inference on the wgpu shader, \
                   inspect logit vectors, measure throughput, and compare quantisation tiers (Q4_K_M vs Q8_0).",
            icon: "cpu-fill",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
        QApp {
            id: "lora-manager",
            name: "LoRA Adapter Manager",
            tagline: "Neural Adaptation",
            desc: "Zero-copy LoRA multiplexing: load adapters from NQuin bits 63–48, blend up to 8 \
                   adapters per token via the fused WGSL shader, and manage the LRU adapter cache.",
            icon: "layers-half",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
        QApp {
            id: "agent-config",
            name: "Agent Configuration",
            tagline: "Inference Runtime",
            desc: "Configure AgentBackend (Local / Remote / Hybrid), ModelLifecycle state machine, \
                   128 MB RAM cap enforcement, and Nym mixnet routing for Remote-mode API calls.",
            icon: "robot",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
        QApp {
            id: "inference-monitor",
            name: "Inference Monitor",
            tagline: "Real-time Telemetry",
            desc: "Live dashboard for ThermalGovernor readings, VRAM utilisation, tokens/sec \
                   throughput, Sentinel anomaly events, and DenyRollback injection counts per session.",
            icon: "activity",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
        QApp {
            id: "model-lifecycle",
            name: "Model Lifecycle",
            tagline: "Model Management",
            desc: "Download, cache, swap, and retire GGUF models. Verify SHA-256 checksums, manage \
                   resident-model LRU policy, and track version compatibility with the LoRA adapter registry.",
            icon: "arrow-repeat",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Ai,
        },
    ]
}
