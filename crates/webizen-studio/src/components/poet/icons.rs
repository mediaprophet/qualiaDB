//! Webizen Studio: Unified Icon Component & Registry
//!
//! Provides the `IconBadge` Dioxus component, mapping semantic icon identifiers
//! to Webizen Icons (`.wi`) PUA glyphs with automated fallback rendering.

use dioxus::prelude::*;

/// 60-bit FNV-1a hash matching QualiaDB q_hash.
pub const fn q_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

#[derive(Debug, Clone, Copy)]
pub struct IconDescriptor {
    pub id: &'static str,
    pub id_hash: u64,
    pub pua: char,
    pub fallback: char,
    pub label: &'static str,
}

macro_rules! icon_desc {
    ($id:expr, $pua:expr, $fallback:expr, $label:expr) => {
        IconDescriptor {
            id: $id,
            id_hash: q_hash($id),
            pua: $pua,
            fallback: $fallback,
            label: $label,
        }
    };
}

pub const ICONS: &[IconDescriptor] = &[
    icon_desc!("webizen-logo", '\u{E000}', '🌐', "Webizen"),
    icon_desc!("nquin", '\u{E001}', '🧬', "NQuin"),
    icon_desc!("tensor-10d", '\u{E002}', '🔮', "Tensor10D"),
    icon_desc!("sanctuary", '\u{E003}', '🛡', "Sanctuary"),
    icon_desc!("did-q42", '\u{E004}', '🔑', "DID:Q42"),
    icon_desc!("tool-chest", '\u{E005}', '🧰', "ToolChest"),
    icon_desc!("aura-tray", '\u{E006}', '✨', "AuraTray"),
    icon_desc!("deontic-obligate", '\u{E010}', '⚖', "Obligate"),
    icon_desc!("deontic-permit", '\u{E011}', '✓', "Permit"),
    icon_desc!("deontic-forbid", '\u{E012}', '⊘', "Forbid"),
    icon_desc!("epistemic-knows", '\u{E020}', '💡', "Knows"),
    icon_desc!("paraconsistent-isolate", '\u{E030}', '🔒', "Isolate"),
    icon_desc!("ltl-globally", '\u{E040}', '⊡', "Globally"),
    icon_desc!("manifold-research", '\u{E050}', '🔬', "Research"),
    icon_desc!("manifold-social", '\u{E051}', '👥', "Social"),
    icon_desc!("manifold-knowledge", '\u{E052}', '📚', "Knowledge"),
    icon_desc!("manifold-projects", '\u{E053}', '📋', "Projects"),
    icon_desc!("manifold-rights", '\u{E054}', '⚖', "Rights"),
    icon_desc!("manifold-sanctuary", '\u{E055}', '🛡', "Sanctuary"),
    icon_desc!("manifold-media", '\u{E056}', '🎨', "Media"),
    icon_desc!("manifold-communications", '\u{E057}', '✉', "Communications"),
    icon_desc!("manifold-settings", '\u{E058}', '⚙', "Settings"),
    icon_desc!("manifold-vibe", '\u{E059}', '⚡', "Vibe"),
    icon_desc!("tb-word-processor", '\u{E060}', '📝', "Document"),
    icon_desc!("tb-graphics", '\u{E061}', '🖌', "Graphics"),
    icon_desc!("tb-code-ide", '\u{E062}', '💻', "IDE"),
    icon_desc!("tb-clinical", '\u{E063}', '🩺', "Clinical"),
    icon_desc!("tb-solid", '\u{E064}', '📦', "SolidPod"),
    icon_desc!("tb-search", '\u{E065}', '🔍', "Search"),
    icon_desc!("tb-logic", '\u{E066}', '📐', "Logic"),
    icon_desc!("state-connected", '\u{E080}', '🟢', "Connected"),
    icon_desc!("state-inferring", '\u{E081}', '🟣', "Inferring"),
    icon_desc!("state-error", '\u{E082}', '🔴', "Error"),
    icon_desc!("state-locked", '\u{E083}', '🔒', "Locked"),
    icon_desc!("state-syncing", '\u{E084}', '🔄', "Syncing"),
    icon_desc!("state-tidy", '\u{E086}', '✨', "Tidy"),
];

#[inline]
pub fn lookup_icon(id: &str) -> Option<&'static IconDescriptor> {
    let hash = q_hash(id);
    let mut i = 0;
    while i < ICONS.len() {
        if ICONS[i].id_hash == hash {
            return Some(&ICONS[i]);
        }
        i += 1;
    }
    None
}

#[component]
pub fn IconBadge(
    id: &'static str,
    #[props(default)] state: Option<String>,
    #[props(default = "sm".to_string())] size: String,
    #[props(default)] extra_class: Option<String>,
) -> Element {
    let descriptor = lookup_icon(id);
    let glyph = descriptor.map(|d| d.pua).unwrap_or('?');
    let label = descriptor.map(|d| d.label).unwrap_or(id);

    let size_class = format!("wi-{}", size);
    let state_class = state.map(|s| format!("wi-{}", s)).unwrap_or_default();
    let extra = extra_class.unwrap_or_default();

    rsx! {
        span {
            class: "wi {size_class} {state_class} {extra}",
            title: "{label}",
            aria_hidden: "true",
            "{glyph}"
        }
    }
}
