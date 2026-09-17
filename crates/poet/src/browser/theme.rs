//! Theme engine — QPrime presets, presentation levels, sanctuary mode, WCAG.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Poet-owned theme engine. Replaces the hardcoded CSS
//! string with CSS custom property injection driven by theme presets.

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

// ---------------------------------------------------------------------------
// Theme preset enum (7 canonical presets)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemePreset {
    FiduciaryDark,
    CommonsLight,
    Sanctuary,
    Infosphere,
    HumanWarmth,
    TwilightBlue,
    MidnightSlate,
}

impl ThemePreset {
    pub fn id(self) -> &'static str {
        match self {
            Self::FiduciaryDark => "fiduciary-dark",
            Self::CommonsLight => "commons-light",
            Self::Sanctuary => "sanctuary",
            Self::Infosphere => "infosphere",
            Self::HumanWarmth => "human-warmth",
            Self::TwilightBlue => "twilight-blue",
            Self::MidnightSlate => "midnight-slate",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::FiduciaryDark => "Fiduciary Dark",
            Self::CommonsLight => "Commons Light",
            Self::Sanctuary => "Sanctuary",
            Self::Infosphere => "Infosphere",
            Self::HumanWarmth => "Human Warmth",
            Self::TwilightBlue => "Twilight Blue",
            Self::MidnightSlate => "Midnight Slate",
        }
    }

    pub fn all() -> &'static [ThemePreset] {
        &[
            Self::FiduciaryDark,
            Self::CommonsLight,
            Self::Sanctuary,
            Self::Infosphere,
            Self::HumanWarmth,
            Self::TwilightBlue,
            Self::MidnightSlate,
        ]
    }
}

// ---------------------------------------------------------------------------
// Presentation level (P0–P6)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PresentationLevel {
    P0, // flat
    P1, // subtle depth
    P2, // glassmorphism
    P3, // parallax
    P4, // spatial board
    P5, // room
    P6, // full hyperspace
}

impl PresentationLevel {
    pub fn id(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
            Self::P5 => "P5",
            Self::P6 => "P6",
        }
    }
}

// ---------------------------------------------------------------------------
// Spatial grammar
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialGrammar {
    List,
    Board,
    Globe,
    Room,
    Strata,
    Hyperspace,
}

impl SpatialGrammar {
    pub fn id(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Board => "board",
            Self::Globe => "globe",
            Self::Room => "room",
            Self::Strata => "strata",
            Self::Hyperspace => "hyperspace",
        }
    }
}

// ---------------------------------------------------------------------------
// Theme state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ThemeState {
    pub preset: ThemePreset,
    pub presentation: PresentationLevel,
    pub spatial_grammar: SpatialGrammar,
    pub spatialize_allowed: bool,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub text_scale_percent: u8,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            preset: ThemePreset::FiduciaryDark,
            presentation: PresentationLevel::P2,
            spatial_grammar: SpatialGrammar::Board,
            spatialize_allowed: true,
            reduced_motion: false,
            high_contrast: false,
            text_scale_percent: 100,
        }
    }
}

impl ThemeState {
    /// Sanctuary mode is active when reduced_motion is true or preset is Sanctuary.
    pub fn is_sanctuary(&self) -> bool {
        self.reduced_motion || self.preset == ThemePreset::Sanctuary
    }

    /// Generate CSS custom property overrides for this theme state.
    pub fn css_custom_properties(&self) -> String {
        let mut css = String::new();
        css.push_str(":root {\n");

        // Text scale
        css.push_str(&format!(
            "  --text-scale: {};\n",
            self.text_scale_percent as f32 / 100.0
        ));

        // Presentation level controls depth of glassmorphism
        let glass_opacity = match self.presentation {
            PresentationLevel::P0 => 1.0,
            PresentationLevel::P1 => 0.95,
            PresentationLevel::P2 => 0.82,
            PresentationLevel::P3 => 0.75,
            PresentationLevel::P4 => 0.70,
            PresentationLevel::P5 => 0.65,
            PresentationLevel::P6 => 0.55,
        };
        css.push_str(&format!("  --glass-opacity: {};\n", glass_opacity));

        // Sanctuary mode: zero motion, spatialize disabled
        if self.is_sanctuary() {
            css.push_str("  --motion-duration: 0s;\n");
            css.push_str("  --motion-enabled: 0;\n");
            css.push_str("  --spatialize: 0;\n");
        } else {
            css.push_str("  --motion-duration: 0.3s;\n");
            css.push_str("  --motion-enabled: 1;\n");
            if self.spatialize_allowed {
                css.push_str("  --spatialize: 1;\n");
            } else {
                css.push_str("  --spatialize: 0;\n");
            }
        }

        // High contrast: boost border brightness
        if self.high_contrast {
            css.push_str("  --border-subtle: #3e5170;\n");
            css.push_str("  --border-medium: #5a7090;\n");
            css.push_str("  --border-bright: #7a9ac0;\n");
            css.push_str("  --text-muted: #8a9ab0;\n");
        }

        // Preset-specific colour overrides
        match self.preset {
            ThemePreset::FiduciaryDark => {
                // Default — no overrides needed, CSS already uses these tokens
            }
            ThemePreset::CommonsLight => {
                css.push_str("  --canvas-bg: #f0f2f5;\n");
                css.push_str("  --canvas-grid-line: rgba(0,100,180,0.08);\n");
                css.push_str("  --surface-base: #ffffff;\n");
                css.push_str("  --surface-panel: #f5f7fa;\n");
                css.push_str("  --surface-panel-elevated: #ffffff;\n");
                css.push_str("  --surface-glass: rgba(255,255,255,0.88);\n");
                css.push_str("  --surface-glass-heavy: rgba(255,255,255,0.95);\n");
                css.push_str("  --border-subtle: #d0d8e0;\n");
                css.push_str("  --border-medium: #b0bcc8;\n");
                css.push_str("  --border-bright: #8090a0;\n");
                css.push_str("  --text-primary: #1a2230;\n");
                css.push_str("  --text-secondary: #3a4858;\n");
                css.push_str("  --text-muted: #6a7888;\n");
            }
            ThemePreset::Sanctuary => {
                css.push_str("  --canvas-bg: #050608;\n");
                css.push_str("  --canvas-grid-line: rgba(100,100,120,0.04);\n");
                css.push_str("  --surface-base: #080a0e;\n");
                css.push_str("  --surface-panel: #0c0e12;\n");
                css.push_str("  --surface-glass: rgba(8,10,14,0.95);\n");
                css.push_str("  --border-subtle: #141820;\n");
                css.push_str("  --border-medium: #1c2028;\n");
                css.push_str("  --accent-cyan: #4a6080;\n");
                css.push_str("  --accent-emerald: #3a6050;\n");
                css.push_str("  --accent-amber: #5a4a30;\n");
                css.push_str("  --accent-violet: #4a3a60;\n");
            }
            ThemePreset::Infosphere => {
                css.push_str("  --canvas-bg: #080c14;\n");
                css.push_str("  --accent-cyan: #00e0ff;\n");
                css.push_str("  --accent-emerald: #00ffaa;\n");
                css.push_str("  --accent-amber: #ffaa00;\n");
                css.push_str("  --accent-violet: #aa00ff;\n");
            }
            ThemePreset::HumanWarmth => {
                css.push_str("  --canvas-bg: #0e0a08;\n");
                css.push_str("  --surface-base: #14100c;\n");
                css.push_str("  --surface-panel: #1a1610;\n");
                css.push_str("  --surface-glass: rgba(20,16,12,0.85);\n");
                css.push_str("  --border-subtle: #28201a;\n");
                css.push_str("  --accent-cyan: #4a9ab0;\n");
                css.push_str("  --accent-emerald: #5a8a6a;\n");
                css.push_str("  --accent-amber: #d09040;\n");
                css.push_str("  --accent-violet: #8a5a8a;\n");
            }
            ThemePreset::TwilightBlue => {
                css.push_str("  --canvas-bg: #0a0e18;\n");
                css.push_str("  --surface-base: #101830;\n");
                css.push_str("  --surface-panel: #182040;\n");
                css.push_str("  --surface-glass: rgba(16,24,48,0.85);\n");
                css.push_str("  --border-subtle: #202848;\n");
                css.push_str("  --accent-cyan: #4080ff;\n");
                css.push_str("  --accent-emerald: #40b080;\n");
            }
            ThemePreset::MidnightSlate => {
                css.push_str("  --canvas-bg: #06080c;\n");
                css.push_str("  --surface-base: #0a0e14;\n");
                css.push_str("  --surface-panel: #10141c;\n");
                css.push_str("  --surface-glass: rgba(10,14,20,0.90);\n");
                css.push_str("  --border-subtle: #181c24;\n");
                css.push_str("  --accent-cyan: #306090;\n");
                css.push_str("  --accent-emerald: #306050;\n");
            }
        }

        css.push_str("}\n");
        css
    }

    /// Inject theme CSS custom properties into the document head.
    pub fn apply(&self, document: &Document) {
        let css_text = self.css_custom_properties();

        // Check if theme style tag already exists
        let existing = document.get_element_by_id("theme-override");
        if let Some(el) = existing {
            el.set_text_content(Some(&css_text));
        } else {
            let style = document.create_element("style").unwrap();
            style.set_id("theme-override");
            style.set_text_content(Some(&css_text));
            if let Some(head) = document.head() {
                head.append_child(&style).unwrap();
            }
        }

        // Set data attributes on body for CSS selector targeting
        if let Some(body) = document.body() {
            let body_el: HtmlElement = body.dyn_into().unwrap();
            body_el
                .set_attribute("data-theme", self.preset.id())
                .unwrap();
            body_el
                .set_attribute("data-presentation", self.presentation.id())
                .unwrap();
            body_el
                .set_attribute("data-spatial", self.spatial_grammar.id())
                .unwrap();
            if self.is_sanctuary() {
                body_el.set_attribute("data-sanctuary", "true").unwrap();
            } else {
                body_el.remove_attribute("data-sanctuary").unwrap();
            }
            if self.high_contrast {
                body_el.set_attribute("data-high-contrast", "true").unwrap();
            } else {
                body_el.remove_attribute("data-high-contrast").unwrap();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WCAG contrast ratio calculation
// ---------------------------------------------------------------------------

/// Linearise an sRGB channel value (0–255) to the 0–1 range.
fn linearise(channel: u8) -> f32 {
    let v = channel as f32 / 255.0;
    if v <= 0.03928 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance of an RGB colour (0–1).
pub fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * linearise(r) + 0.7152 * linearise(g) + 0.0722 * linearise(b)
}

/// WCAG contrast ratio between two luminance values (1–21).
pub fn contrast_ratio(l1: f32, l2: f32) -> f32 {
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Parse a hex colour (#RRGGBB) into (r, g, b).
pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Check if a foreground/background pair meets WCAG AA (4.5:1 for normal text).
pub fn meets_wcag_aa(fg_hex: &str, bg_hex: &str) -> bool {
    let (fr, fg, fb) = match parse_hex(fg_hex) {
        Some(c) => c,
        None => return false,
    };
    let (br, bg, bb) = match parse_hex(bg_hex) {
        Some(c) => c,
        None => return false,
    };
    let ratio = contrast_ratio(
        relative_luminance(fr, fg, fb),
        relative_luminance(br, bg, bb),
    );
    ratio >= 4.5
}

/// Check if a foreground/background pair meets WCAG AAA (7.0:1 for normal text).
pub fn meets_wcag_aaa(fg_hex: &str, bg_hex: &str) -> bool {
    let (fr, fg, fb) = match parse_hex(fg_hex) {
        Some(c) => c,
        None => return false,
    };
    let (br, bg, bb) = match parse_hex(bg_hex) {
        Some(c) => c,
        None => return false,
    };
    let ratio = contrast_ratio(
        relative_luminance(fr, fg, fb),
        relative_luminance(br, bg, bb),
    );
    ratio >= 7.0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_ids() {
        assert_eq!(ThemePreset::FiduciaryDark.id(), "fiduciary-dark");
        assert_eq!(ThemePreset::Sanctuary.id(), "sanctuary");
        assert_eq!(ThemePreset::all().len(), 7);
    }

    #[test]
    fn test_sanctuary_mode() {
        let mut state = ThemeState::default();
        assert!(!state.is_sanctuary());

        state.reduced_motion = true;
        assert!(state.is_sanctuary());

        state.reduced_motion = false;
        state.preset = ThemePreset::Sanctuary;
        assert!(state.is_sanctuary());
    }

    #[test]
    fn test_css_custom_properties_sanctuary() {
        let state = ThemeState {
            preset: ThemePreset::Sanctuary,
            reduced_motion: true,
            ..Default::default()
        };
        let css = state.css_custom_properties();
        assert!(css.contains("--motion-duration: 0s"));
        assert!(css.contains("--motion-enabled: 0"));
        assert!(css.contains("--spatialize: 0"));
    }

    #[test]
    fn test_css_custom_properties_commons_light() {
        let state = ThemeState {
            preset: ThemePreset::CommonsLight,
            ..Default::default()
        };
        let css = state.css_custom_properties();
        assert!(css.contains("--canvas-bg: #f0f2f5"));
        assert!(css.contains("--text-primary: #1a2230"));
    }

    #[test]
    fn test_presentation_level_glass_opacity() {
        let p0 = ThemeState {
            presentation: PresentationLevel::P0,
            ..Default::default()
        };
        assert!(p0.css_custom_properties().contains("--glass-opacity: 1"));

        let p6 = ThemeState {
            presentation: PresentationLevel::P6,
            ..Default::default()
        };
        assert!(p6.css_custom_properties().contains("--glass-opacity: 0.55"));
    }

    #[test]
    fn test_wcag_contrast_white_on_black() {
        assert!(meets_wcag_aa("#ffffff", "#000000"));
        assert!(meets_wcag_aaa("#ffffff", "#000000"));
    }

    #[test]
    fn test_wcag_contrast_low_contrast() {
        assert!(!meets_wcag_aa("#777777", "#888888"));
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#07090e"), Some((7, 9, 14)));
        assert_eq!(parse_hex("#ffffff"), Some((255, 255, 255)));
        assert_eq!(parse_hex("invalid"), None);
    }

    #[test]
    fn test_contrast_ratio_bounds() {
        let black = relative_luminance(0, 0, 0);
        let white = relative_luminance(255, 255, 255);
        let ratio = contrast_ratio(white, black);
        assert!((ratio - 21.0).abs() < 0.1);
    }
}
