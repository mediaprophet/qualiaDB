//! Capability honesty chip — Present / Partial / Scaffold / … labels for product surfaces.
//!
//! Matches comprehensive UI plan §5.3 and webizen-ui-implementation-subagents U1-C.
//! Never label a scaffold or experimental path as Ready.

#![allow(non_snake_case)]
use dioxus::prelude::*;

/// Product honesty level for a capability surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HonestyLevel {
    /// Present + tested path on this profile.
    Ready,
    /// Works with known caveats.
    Partial,
    /// Academic / placeholder QApp or unfinished pane.
    Scaffold,
    /// Needs GGUF / P64 / weights before use.
    NeedsModel,
    /// Biosense / vault / agent tools require explicit consent.
    NeedsConsent,
    /// WASM profile, feature flag, or hardware missing.
    Unavailable,
}

impl HonestyLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Partial => "Partial",
            Self::Scaffold => "Scaffold",
            Self::NeedsModel => "Needs model",
            Self::NeedsConsent => "Needs consent",
            Self::Unavailable => "Unavailable",
        }
    }

    pub fn bg_fg(self) -> (&'static str, &'static str) {
        match self {
            Self::Ready => ("#064e3b", "#a7f3d0"),
            Self::Partial => ("#1e3a5f", "#93c5fd"),
            Self::Scaffold => ("#374151", "#d1d5db"),
            Self::NeedsModel => ("#78350f", "#fde68a"),
            Self::NeedsConsent => ("#4c1d95", "#e9d5ff"),
            Self::Unavailable => ("#450a0a", "#fecaca"),
        }
    }
}

/// Compact honesty chip for headers and status strips.
#[component]
pub fn HonestyChip(level: HonestyLevel, #[props(default)] detail: String) -> Element {
    let (bg, fg) = level.bg_fg();
    let label = level.label();
    let title = if detail.is_empty() {
        label.to_string()
    } else {
        format!("{label}: {detail}")
    };
    let style = format!(
        "display:inline-flex; align-items:center; gap:4px; font-size:11px; font-weight:600; \
         letter-spacing:0.03em; text-transform:uppercase; color:{fg}; background:{bg}; \
         padding:3px 10px; border-radius:999px; white-space:nowrap; max-width:100%; \
         overflow:hidden; text-overflow:ellipsis;"
    );
    rsx! {
        span {
            style: "{style}",
            title: "{title}",
            "{label}"
            if !detail.is_empty() {
                span {
                    style: "font-weight:500; text-transform:none; letter-spacing:0; opacity:0.9; margin-left:4px;",
                    "· {detail}"
                }
            }
        }
    }
}
