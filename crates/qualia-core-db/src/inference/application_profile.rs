//! Application profiles — **how** inference is used, not only **which** GPU path.
//!
//! Timothy (2026-07-10): on a local device, work need not be live. High-stakes
//! multi-system health eval / differential analysis can run overnight and deliver
//! a verified HTML (or email body). Different applications want different modes:
//!
//! | Profile | Latency | Mid-decode | Post-turn | Decode budget | Timeout |
//! |---------|---------|------------|-----------|---------------|---------|
//! | **Interactive** | low | optional | light | 256 | 30s |
//! | **LiveFast** | lowest | off (FastVerify) | graph heal | 256 | 30s |
//! | **BatchOvernight** | irrelevant | off | full HTML+CML | 2048 | 8h |
//!
//! No Ollama API: all profiles stay in-process Qualia (P64 + resident GEMV + graph).

use std::sync::atomic::{AtomicU8, Ordering};

use crate::inference_modes::{set_inference_mode, InferenceMode};
#[cfg(not(target_arch = "wasm32"))]
use crate::llm_bench::{set_decode_budget_override, set_inference_timeout_override_ms};

#[cfg(target_arch = "wasm32")]
fn set_decode_budget_override(_n: u32) {}
#[cfg(target_arch = "wasm32")]
fn set_inference_timeout_override_ms(_ms: u64) {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplicationProfile {
    /// Chat / UI — path selector + portable defaults.
    Interactive = 0,
    /// Streaming-feel: FastVerify (generate then post-heal).
    LiveFast = 1,
    /// Overnight / high-stakes batch: long budget, HTML verification surface.
    BatchOvernight = 2,
}

impl ApplicationProfile {
    pub const ALL: [ApplicationProfile; 3] = [
        ApplicationProfile::Interactive,
        ApplicationProfile::LiveFast,
        ApplicationProfile::BatchOvernight,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::LiveFast => "live-fast",
            Self::BatchOvernight => "batch",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "interactive" | "chat" | "ui" | "0" => Some(Self::Interactive),
            "live-fast" | "live_fast" | "live" | "fast" | "1" => Some(Self::LiveFast),
            "batch" | "batch-overnight" | "overnight" | "offline" | "async" | "email" | "2" => {
                Some(Self::BatchOvernight)
            }
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Interactive => {
                "interactive chat: path-selected GPU backend, 256-tok budget, 30s wall-clock"
            }
            Self::LiveFast => {
                "live-fast: FastVerify (uninterrupted decode → post graph/CML heal), still local"
            }
            Self::BatchOvernight => {
                "batch/overnight: up to 2048 tokens, 8h wall-clock, HTML+CML verify — multi-system eval, email-ready"
            }
        }
    }
}

static PROFILE: AtomicU8 = AtomicU8::new(ApplicationProfile::Interactive as u8);

pub fn active_application_profile() -> ApplicationProfile {
    if let Ok(s) = std::env::var("QUALIA_APP_PROFILE") {
        if let Some(p) = ApplicationProfile::parse(&s) {
            return p;
        }
    }
    match PROFILE.load(Ordering::Relaxed) {
        1 => ApplicationProfile::LiveFast,
        2 => ApplicationProfile::BatchOvernight,
        _ => ApplicationProfile::Interactive,
    }
}

/// Apply profile: inference mode, budgets, timeouts, HTML return for batch.
pub fn set_application_profile(profile: ApplicationProfile) {
    PROFILE.store(profile as u8, Ordering::Relaxed);
    apply_application_profile(profile);
    log::info!(
        "APP_PROFILE|{}|{}",
        profile.as_str(),
        profile.description()
    );
}

pub fn apply_application_profile(profile: ApplicationProfile) {
    match profile {
        ApplicationProfile::Interactive => {
            set_decode_budget_override(0); // production 256
            set_inference_timeout_override_ms(0); // 30s default
            if std::env::var("QUALIA_INFERENCE_MODE").is_err() {
                set_inference_mode(InferenceMode::Portable);
            }
        }
        ApplicationProfile::LiveFast => {
            set_decode_budget_override(0);
            set_inference_timeout_override_ms(0);
            if std::env::var("QUALIA_INFERENCE_MODE").is_err() {
                set_inference_mode(InferenceMode::FastVerify);
            }
        }
        ApplicationProfile::BatchOvernight => {
            // Long-form reasoning for differential / multi-system jobs.
            set_decode_budget_override(2048);
            set_inference_timeout_override_ms(8 * 60 * 60 * 1000); // 8 hours
            // Always post-verify + HTML surface for email / archival.
            std::env::set_var("QUALIA_RETURN_VERIFY_HTML", "1");
            if std::env::var("QUALIA_INFERENCE_MODE").is_err() {
                set_inference_mode(InferenceMode::FastVerify);
            }
            // Ensure fact graph is warm.
            let n = crate::quant_graph_grounding::seed_facts_from_bundled();
            log::info!("APP_PROFILE|batch|facts_seeded|{n}|budget=2048|timeout=8h|html=1");
        }
    }
}

/// Bootstrap from env `QUALIA_APP_PROFILE` (call early with path selector).
pub fn bootstrap_application_profile() -> ApplicationProfile {
    let p = active_application_profile();
    apply_application_profile(p);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_profiles() {
        assert_eq!(
            ApplicationProfile::parse("overnight"),
            Some(ApplicationProfile::BatchOvernight)
        );
        assert_eq!(
            ApplicationProfile::parse("live-fast"),
            Some(ApplicationProfile::LiveFast)
        );
    }
}
