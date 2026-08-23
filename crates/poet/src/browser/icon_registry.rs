//! Webizen Icon Registry: Unicode + PUA + 10D Interactive Glyphs
//!
//! Provides a zero-heap, constant-time icon resolution layer for Poet,
//! mapping domain concepts (NQuin, Tensor10D, Deontic modalities, etc.)
//! to Private Use Area (U+E000..U+E1FF) codepoints with a 4-tier degradation chain:
//!
//! Tier 1: PUA Glyph (rendered with Webizen Icons font)
//! Tier 2: Standard Unicode Fallback Emoji / Symbol
//! Tier 3: Inline SVG / CSS representation
//! Tier 4: Plaintext ASCII / Semantic Label

/// Canonical 60-bit FNV-1a hash matching QualiaDB q_hash specification.
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

/// Private Use Area (PUA) codepoint newtype (U+E000..U+E1FF block within BMP PUA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PuaCodepoint(pub u32);

impl PuaCodepoint {
    pub const PUA_START: u32 = 0xE000;
    pub const PUA_END: u32 = 0xE1FF;

    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        self.0 >= Self::PUA_START && self.0 <= Self::PUA_END
    }

    #[inline(always)]
    pub const fn as_char(&self) -> char {
        match char::from_u32(self.0) {
            Some(c) => c,
            None => '\u{FFFD}',
        }
    }
}

/// Category classification for icons in the Webizen ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconCategory {
    System,
    Modality,
    Toolbox,
    ToolChain,
    Governance,
    Status,
    Clinical,
    Media,
}

impl IconCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Modality => "Modality",
            Self::Toolbox => "Toolbox",
            Self::ToolChain => "ToolChain",
            Self::Governance => "Governance",
            Self::Status => "Status",
            Self::Clinical => "Clinical",
            Self::Media => "Media",
        }
    }
}

/// Interactive state for dynamic icon rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconState {
    #[default]
    Default,
    Hover,
    Active,
    Disabled,
    Processing,
    Error,
    Selected,
}

impl IconState {
    pub const fn css_class_suffix(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Hover => "wi-hover",
            Self::Active => "wi-active",
            Self::Disabled => "wi-disabled",
            Self::Processing => "wi-spin",
            Self::Error => "wi-error",
            Self::Selected => "wi-selected",
        }
    }
}

/// Size presets for standard UI surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconSize {
    Xs,
    #[default]
    Sm,
    Md,
    Lg,
    Xl,
}

impl IconSize {
    pub const fn css_class(&self) -> &'static str {
        match self {
            Self::Xs => "wi-xs",
            Self::Sm => "wi-sm",
            Self::Md => "wi-md",
            Self::Lg => "wi-lg",
            Self::Xl => "wi-xl",
        }
    }

    pub const fn pixel_size(&self) -> u32 {
        match self {
            Self::Xs => 10,
            Self::Sm => 14,
            Self::Md => 18,
            Self::Lg => 24,
            Self::Xl => 32,
        }
    }
}

/// Metadata descriptor for a registered icon.
#[derive(Debug, Clone, Copy)]
pub struct IconEntry {
    pub id: &'static str,
    pub id_hash: u64,
    pub pua: char,
    pub unicode_fallback: char,
    pub ascii_label: &'static str,
    pub category: IconCategory,
    pub description: &'static str,
}

macro_rules! define_icon {
    ($id:expr, $pua:expr, $fallback:expr, $ascii:expr, $cat:ident, $desc:expr) => {
        IconEntry {
            id: $id,
            id_hash: q_hash($id),
            pua: $pua,
            unicode_fallback: $fallback,
            ascii_label: $ascii,
            category: IconCategory::$cat,
            description: $desc,
        }
    };
}

/// Compile-time registry of all foundational Webizen / Poet icons.
pub const ALL_ICONS: &[IconEntry] = &[
    // ── System & Foundation (0xE000 - 0xE00F) ──────────────────────────
    define_icon!("webizen-logo", '\u{E000}', '🌐', "Webizen", System, "Webizen network emblem"),
    define_icon!("nquin", '\u{E001}', '🧬', "NQuin", System, "48-byte Super-Quin semantic datum"),
    define_icon!("tensor-10d", '\u{E002}', '🔮', "Tensor10D", System, "10-dimensional manifold state vector"),
    define_icon!("sanctuary", '\u{E003}', '🛡', "Sanctuary", System, "Zero-telemetry cryptographic vault"),
    define_icon!("did-q42", '\u{E004}', '🔑', "DID:Q42", System, "Topological hardware DID pointer"),
    define_icon!("tool-chest", '\u{E005}', '🧰', "ToolChest", System, "Docked palette and domain tools"),
    define_icon!("aura-tray", '\u{E006}', '✨', "AuraTray", System, "Ambient reactive dock status tray"),

    // ── Modalities & Formal Logic (0xE010 - 0xE04F) ────────────────────
    define_icon!("deontic-obligate", '\u{E010}', '⚖', "Obligate", Modality, "Deontic obligation operator O(φ)"),
    define_icon!("deontic-permit", '\u{E011}', '✓', "Permit", Modality, "Deontic permission operator P(φ)"),
    define_icon!("deontic-forbid", '\u{E012}', '⊘', "Forbid", Modality, "Deontic prohibition operator F(φ)"),
    define_icon!("epistemic-knows", '\u{E020}', '💡', "Knows", Modality, "Epistemic knowledge K_a(φ)"),
    define_icon!("epistemic-believes", '\u{E021}', '💭', "Believes", Modality, "Doxastic belief B_a(φ)"),
    define_icon!("epistemic-common", '\u{E022}', '👥', "CommonKnowledge", Modality, "Epistemic common knowledge C(φ)"),
    define_icon!("paraconsistent-isolate", '\u{E030}', '🔒', "Isolate", Modality, "Paraconsistent quarantine context"),
    define_icon!("paraconsistent-score", '\u{E031}', '⚡', "ContradictionScore", Modality, "Contradiction severity metric"),
    define_icon!("paraconsistent-merge", '\u{E032}', '🔀', "MergeContext", Modality, "Paraconsistent context ratification"),
    define_icon!("ltl-globally", '\u{E040}', '⊡', "Globally", Modality, "LTL Globally operator G(φ)"),
    define_icon!("ltl-finally", '\u{E041}', '◇', "Finally", Modality, "LTL Finally operator F(φ)"),
    define_icon!("ltl-next", '\u{E042}', '○', "Next", Modality, "LTL Next operator X(φ)"),
    define_icon!("ltl-until", '\u{E043}', '⊔', "Until", Modality, "LTL Until operator φ U ψ"),
    define_icon!("ltl-release", '\u{E044}', '⊓', "Release", Modality, "LTL Release operator φ R ψ"),

    // ── Virtual Manifolds & Workbenches (0xE050 - 0xE05F) ───────────────
    define_icon!("manifold-research", '\u{E050}', '🔬', "Research", Toolbox, "Research manifold: hypermedia synthesis"),
    define_icon!("manifold-social", '\u{E051}', '👥', "Social", Toolbox, "Social manifold: ERP & multi-agent mesh"),
    define_icon!("manifold-knowledge", '\u{E052}', '📚', "Knowledge", Toolbox, "Knowledge manifold: Solid Pods & RDF-Star"),
    define_icon!("manifold-projects", '\u{E053}', '📋', "Projects", Toolbox, "Projects manifold: agile & sprint delivery"),
    define_icon!("manifold-rights", '\u{E054}', '⚖', "Rights", Toolbox, "Rights manifold: fiduciary & legal engineering"),
    define_icon!("manifold-sanctuary", '\u{E055}', '🛡', "Sanctuary", Toolbox, "Sanctuary manifold: zero-telemetry vault"),
    define_icon!("manifold-media", '\u{E056}', '🎨', "Media", Toolbox, "Media manifold: 3D CCF meshes & audio"),
    define_icon!("manifold-communications", '\u{E057}', '✉', "Communications", Toolbox, "Communications manifold: CML inboxes"),
    define_icon!("manifold-settings", '\u{E058}', '⚙', "Settings", Toolbox, "Settings manifold: 42MB Sentinel governance"),
    define_icon!("manifold-vibe", '\u{E059}', '⚡', "Vibe", Toolbox, "Vibe manifold: reactive cell compiler"),

    // ── Toolboxes & Body Views (0xE060 - 0xE07F) ────────────────────────
    define_icon!("tb-word-processor", '\u{E060}', '📝', "Document", Toolbox, "CML Rich Word Processor & Editor"),
    define_icon!("tb-graphics", '\u{E061}', '🖌', "Graphics", Toolbox, "Vector & Raster Graphics Suite"),
    define_icon!("tb-code-ide", '\u{E062}', '💻', "IDE", Toolbox, "VibeScript / Rust Code IDE"),
    define_icon!("tb-clinical", '\u{E063}', '🩺', "Clinical", Clinical, "Clinical Engine: Framingham, CHA2DS2, SCORE2"),
    define_icon!("tb-solid", '\u{E064}', '📦', "SolidPod", Governance, "W3C Solid Pod Inspector & Exporter"),
    define_icon!("tb-search", '\u{E065}', '🔍', "Search", Toolbox, "SPARQL / SLG Query Workbench"),
    define_icon!("tb-logic", '\u{E066}', '📐', "Logic", Modality, "Modal Logic & Deontic Rule Forge"),
    define_icon!("tb-cooperative", '\u{E067}', '🤝', "Cooperative", Governance, "Cooperative Economics & Ledger"),
    define_icon!("tb-audio-synth", '\u{E068}', '🎵', "AudioSynth", Media, "WASM WebAudio Synthesis Engine"),

    // ── Status & Dynamic Telemetry (0xE080 - 0xE09F) ────────────────────
    define_icon!("state-connected", '\u{E080}', '🟢', "Connected", Status, "Daemon connected & healthy"),
    define_icon!("state-inferring", '\u{E081}', '🟣', "Inferring", Status, "Autoregressive neural inference active"),
    define_icon!("state-error", '\u{E082}', '🔴', "Error", Status, "Sentinel or execution fault detected"),
    define_icon!("state-locked", '\u{E083}', '🔒', "Locked", Status, "Sanctuary cryptographic lock engaged"),
    define_icon!("state-syncing", '\u{E084}', '🔄', "Syncing", Status, "P2P CRDT replication in progress"),
    define_icon!("state-standalone", '\u{E085}', '🟡', "Standalone", Status, "Running offline / standalone WASM"),
    define_icon!("state-tidy", '\u{E086}', '✨', "Tidy", Status, "Collision-free smart auto-arrangement"),
];

/// Zero-heap, constant-time lookup of an icon entry by 60-bit hash.
#[inline]
pub fn icon_entry_by_hash(id_hash: u64) -> Option<&'static IconEntry> {
    let mut i = 0;
    while i < ALL_ICONS.len() {
        if ALL_ICONS[i].id_hash == id_hash {
            return Some(&ALL_ICONS[i]);
        }
        i += 1;
    }
    None
}

/// Zero-heap lookup of an icon entry by string ID.
#[inline]
pub fn icon_entry(id: &str) -> Option<&'static IconEntry> {
    icon_entry_by_hash(q_hash(id))
}

/// Hot-path glyph retrieval by hash: returns PUA char if found, or '?' fallback.
#[inline]
pub fn icon_char_fast(id_hash: u64) -> char {
    match icon_entry_by_hash(id_hash) {
        Some(entry) => entry.pua,
        None => '?',
    }
}

/// Convenient glyph lookup by string ID.
#[inline]
pub fn icon_char(id: &str) -> char {
    icon_char_fast(q_hash(id))
}

/// Retrieve the standard Unicode fallback character (e.g. for non-PUA or Solid exports).
#[inline]
pub fn icon_fallback(id: &str) -> char {
    match icon_entry(id) {
        Some(entry) => entry.unicode_fallback,
        None => '▫',
    }
}

/// Retrieve the ASCII/semantic label for screen readers or plaintext logs.
#[inline]
pub fn icon_label(id: &str) -> &'static str {
    match icon_entry(id) {
        Some(entry) => entry.ascii_label,
        None => "[Icon]",
    }
}

/// Formats a degraded representation according to the 4-tier contract:
/// - Tier 1: PUA character
/// - Tier 2: Standard Unicode character
/// - Tier 3: SVG identifier / class name
/// - Tier 4: Plaintext ASCII label
pub fn format_degraded(id: &str, tier: u8) -> String {
    let entry = icon_entry(id);
    match tier {
        1 => entry.map(|e| e.pua.to_string()).unwrap_or_else(|| "?".to_string()),
        2 => entry.map(|e| e.unicode_fallback.to_string()).unwrap_or_else(|| "▫".to_string()),
        3 => format!("wi wi-{}", id),
        _ => entry.map(|e| format!("[{}]", e.ascii_label)).unwrap_or_else(|| format!("[{}]", id)),
    }
}

/// Generates an accessible HTML icon span with `.wi` classes and aria attributes.
pub fn icon_span(id: &str, state: IconState, size: IconSize) -> String {
    let entry = icon_entry(id);
    let glyph = entry.map(|e| e.pua).unwrap_or('?');
    let label = entry.map(|e| e.ascii_label).unwrap_or(id);
    let state_class = state.css_class_suffix();
    let size_class = size.css_class();
    
    if state_class.is_empty() {
        format!(
            r#"<span class="wi {}" aria-hidden="true" title="{}">{}</span>"#,
            size_class, label, glyph
        )
    } else {
        format!(
            r#"<span class="wi {} {}" aria-hidden="true" title="{}">{}</span>"#,
            size_class, state_class, label, glyph
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_icon_hashes_unique() {
        for i in 0..ALL_ICONS.len() {
            for j in (i + 1)..ALL_ICONS.len() {
                assert_ne!(
                    ALL_ICONS[i].id_hash,
                    ALL_ICONS[j].id_hash,
                    "Hash collision between '{}' and '{}'",
                    ALL_ICONS[i].id,
                    ALL_ICONS[j].id
                );
            }
        }
    }

    #[test]
    fn test_pua_codepoints_in_valid_range() {
        for icon in ALL_ICONS {
            let cp = PuaCodepoint(icon.pua as u32);
            assert!(
                cp.is_valid(),
                "Icon '{}' has PUA codepoint U+{:04X} which is outside U+E000..U+E1FF",
                icon.id,
                icon.pua as u32
            );
        }
    }

    #[test]
    fn test_fallback_characters_present() {
        for icon in ALL_ICONS {
            assert!(
                icon.unicode_fallback != '\0',
                "Icon '{}' must have a non-null Unicode fallback character",
                icon.id
            );
            assert!(
                !icon.ascii_label.is_empty(),
                "Icon '{}' must have a non-empty ASCII label",
                icon.id
            );
        }
    }

    #[test]
    fn test_icon_lookup_resolution() {
        assert_eq!(icon_char("nquin"), '\u{E001}');
        assert_eq!(icon_fallback("nquin"), '🧬');
        assert_eq!(icon_label("nquin"), "NQuin");
        assert_eq!(icon_char("tensor-10d"), '\u{E002}');
        assert_eq!(icon_char("unknown_id_xyz"), '?');
    }

    #[test]
    fn test_degraded_formats() {
        assert_eq!(format_degraded("nquin", 1), "\u{E001}");
        assert_eq!(format_degraded("nquin", 2), "🧬");
        assert_eq!(format_degraded("nquin", 3), "wi wi-nquin");
        assert_eq!(format_degraded("nquin", 4), "[NQuin]");
    }

    #[test]
    fn test_icon_span_html() {
        let span = icon_span("nquin", IconState::Processing, IconSize::Md);
        assert!(span.contains("wi-md"));
        assert!(span.contains("wi-spin"));
        assert!(span.contains("title=\"NQuin\""));
    }
}
