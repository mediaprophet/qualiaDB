//! CSS asset composition for the browser UI shell.
//!
//! Keep style concerns in focused files under `styles/`. Source order is part
//! of the cascade contract, so append new assets only at a deliberate boundary.

pub const CSS: &str = concat!(
    include_str!("styles/01-tokens.css"),
    include_str!("styles/02-chrome.css"),
    include_str!("styles/03-controls.css"),
    include_str!("styles/04-workspace.css"),
    include_str!("styles/05-containers.css"),
    include_str!("styles/06-canvas-social.css"),
    include_str!("styles/07-workbenches.css"),
    include_str!("styles/08-responsive.css"),
    include_str!("styles/09-icons-dialogs.css"),
    include_str!("styles/10-sheet.css"),
    include_str!("styles/11-health-base.css"),
    include_str!("styles/12-health-vitals.css"),
    include_str!("styles/13-health-inspection.css"),
    include_str!("styles/14-health-disclosure.css"),
    include_str!("styles/15-studio-chrome.css"),
);

#[cfg(test)]
mod tests {
    use super::CSS;

    #[test]
    fn composed_stylesheet_keeps_key_boundaries_in_order() {
        let tokens = CSS.find(":root {").expect("design tokens");
        let chrome = CSS
            .find("/* === Top Menu Bar === */")
            .expect("chrome styles");
        let containers = CSS
            .find("/* === Canvas Containers (Glassmorphism) === */")
            .expect("container styles");
        let health = CSS
            .find("/* Person-controlled health workspace */")
            .expect("health styles");
        let studio = CSS
            .find("/* === Studio chrome (davinci / monet) === */")
            .expect("studio chrome");

        assert!(tokens < chrome && chrome < containers && containers < health && health < studio);
        assert!(CSS.contains(".health-doc-snippet"));
        assert!(CSS.contains(".diag-glow-token"));
        assert!(CSS.contains(".preview-handle-tab"));
        assert!(CSS.contains("--beat-entrance"));
        assert!(CSS.contains("[data-volume-state=\"committed\"]"));
        assert!(CSS.contains("[data-media-surface=\"film\"]"));
        assert!(CSS.contains(".aspect-chip"));
        assert!(CSS.contains(".lexicon-chip"));
        assert!(CSS.contains(".lexicon-held-gate"));
        assert!(CSS.contains("[data-recipe=\"arrive\"]"));
        assert!(CSS.contains("--chip-living"));
        assert!(CSS.contains(".tool-tip"));
        assert!(CSS.contains(".tool-proficiency-switcher"));
        assert!(!CSS.contains(".twin-chip"));
        assert!(!CSS.contains("Twin chips"));
        assert!(!CSS.contains("plane-chip"));
    }
}
