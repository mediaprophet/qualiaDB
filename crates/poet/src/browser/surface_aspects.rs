//! Layout · Stage · Timeline **aspects** on Poet surfaces.
//!
//! Three readings of one studio surface: structure, depth, and named time.
//! They are not copies of each other. Do not call them "twins" — twin infers
//! identical, which is almost always misleading. Do not call them "planes"
//! (acoustic-plane and network control/data plane are other vocabularies).
//! Not a credential "digital twin". Named beats only (entrance · dwell · exit).
//! Not legal FormationStage. Machine tokens: layout/stage/timeline; labels UTF-8.

use web_sys::{Document, Element};

pub const BEATS: [&str; 3] = ["entrance", "dwell", "exit"];
pub const ASPECTS: [(&str, &str); 3] = [
    ("layout", "Layout"),
    ("stage", "Stage"),
    ("timeline", "Timeline"),
];

/// Mark a shipped surface as having all three aspects + a named beat.
pub fn mark(el: &Element, beat: &str) {
    let beat = if BEATS.contains(&beat) {
        beat
    } else {
        "entrance"
    };
    el.set_attribute("data-aspect-surface", "1").ok();
    el.set_attribute("data-beat", beat).ok();
    el.set_attribute("data-aspect-layout", "1").ok();
    el.set_attribute("data-aspect-stage", "1").ok();
    el.set_attribute("data-aspect-timeline", "1").ok();
}

/// Compact Layout / Stage / Timeline chips. UTF-8 labels; machine tokens in data-aspect.
pub fn chip_row(document: &Document) -> Element {
    let row = document.create_element("span").unwrap();
    row.set_class_name("aspect-chip-row");
    row.set_attribute("aria-label", "Layout Stage Timeline")
        .ok();
    for (token, label) in ASPECTS {
        let chip = document.create_element("span").unwrap();
        chip.set_class_name("aspect-chip");
        chip.set_attribute("data-aspect", token).ok();
        chip.set_attribute("title", label).ok();
        chip.set_text_content(Some(label));
        row.append_child(&chip).unwrap();
    }
    row
}

/// Shells that must carry aspects (Stage 7 regression list).
pub fn required_shells() -> &'static [&'static str] {
    &[
        "app",
        "main-workspace",
        "canvas-viewport-container",
        "toolbox-dock",
        "right-dock",
        "bottom-statusbar",
        "dock-panel",
        "vibe-console",
        "contextual-instrument-panel",
        "construct-shelf",
        "g-coord-map",
        "q-cell-widget",
        "native-render-preview",
        "canvas-container-node",
        "cmd-palette-panel",
        "top-menubar",
        "canvas-control-bar",
        "toolbox-flyout",
        "save-mode-dialog",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_are_named_only() {
        assert_eq!(BEATS, ["entrance", "dwell", "exit"]);
        assert!(!BEATS.iter().any(|b| *b == "tween" || *b == "free"));
    }

    #[test]
    fn required_shells_cover_stage7() {
        let shells = required_shells();
        for need in [
            "app",
            "toolbox-dock",
            "right-dock",
            "g-coord-map",
            "q-cell-widget",
            "native-render-preview",
            "canvas-container-node",
            "top-menubar",
            "canvas-control-bar",
            "toolbox-flyout",
            "save-mode-dialog",
        ] {
            assert!(shells.contains(&need), "missing {need}");
        }
    }

    #[test]
    fn aspect_tokens_are_not_twin_or_plane_or_formation_stage() {
        for (token, label) in ASPECTS {
            assert_ne!(token, "FormationStage");
            assert!(!token.contains("twin"));
            assert!(!token.contains("plane"));
            let folded = label.to_lowercase();
            assert!(!folded.contains("twin"));
            assert!(!folded.contains("plane"));
        }
    }
}
