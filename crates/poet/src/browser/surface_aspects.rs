//! Layout · Stage · Timeline **aspects** on Poet surfaces.
//!
//! Not "twins" and not a credential "digital twin". These are three 1:1
//! readings of one studio surface: structure, depth, and named time.
//! Named beats only (entrance · dwell · exit). Not legal FormationStage.
//! Machine tokens are layout/stage/timeline; human labels are UTF-8.

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
        ] {
            assert!(shells.contains(&need), "missing {need}");
        }
    }

    #[test]
    fn aspect_tokens_are_not_formation_stage_or_digital_twin() {
        for (token, label) in ASPECTS {
            assert_ne!(token, "FormationStage");
            assert!(!token.contains("twin"));
            assert!(!label.to_lowercase().contains("twin"));
        }
    }
}
