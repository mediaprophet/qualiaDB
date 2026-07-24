//! Life-domain strip for Wellfair panels — Care / Practice / Rights / etc.
//!
//! Keeps product language consistent without per-panel style drift.

use crate::Route;
use dioxus::prelude::*;

/// Compact domain identity row above a panel title.
///
/// `domain` — short life domain (e.g. "Care").  
/// `chip` — role note (e.g. "Body · local vault").  
/// `show_memory` — link to Lived Memory when the panel can hand off meaning.
#[component]
pub fn DomainChrome(domain: &'static str, chip: &'static str, show_memory: bool) -> Element {
    let accent = match domain {
        "Care" => ("#a5b4fc", "#4c1d95", "rgba(139,92,246,0.12)", "#c4b5fd"),
        "Practice" => ("#6ee7b7", "#065f46", "rgba(16,185,129,0.12)", "#a7f3d0"),
        "Relations" => ("#93c5fd", "#1e3a5f", "rgba(59,130,246,0.12)", "#bfdbfe"),
        "World" => ("#67e8f9", "#155e75", "rgba(34,211,238,0.12)", "#a5f3fc"),
        "Instruments" => ("#94a3b8", "#475569", "rgba(71,85,105,0.2)", "#cbd5e1"),
        "You" => ("#fcd34d", "#78350f", "rgba(245,158,11,0.12)", "#fde68a"),
        _ => ("#a5b4fc", "#4c1d95", "rgba(139,92,246,0.12)", "#c4b5fd"),
    };
    let (label_c, chip_border, chip_bg, chip_c) = accent;

    rsx! {
        div {
            style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.4rem;",
            span {
                style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:{label_c};",
                "{domain}"
            }
            span {
                style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid {chip_border};background:{chip_bg};color:{chip_c};font-weight:700;",
                "{chip}"
            }
            if show_memory {
                Link {
                    to: Route::LibraryRoute {},
                    style: "margin-left:auto;font-size:0.68rem;font-weight:700;color:#7c3aed;text-decoration:none;",
                    title: "Lived Memory — keep by meaning",
                    "→ Memory"
                }
            }
        }
    }
}
