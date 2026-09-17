//! Pure a11y helpers for instrument chrome (no Document).
//!
//! Live-region politeness, selection/held announcements, and named
//! actions. Dispatch names must not be Host IDs or icon-only glyphs.

/// `aria-live` politeness for instrument status regions.
pub fn live_polite() -> &'static str {
    "polite"
}

/// Screen-reader copy when a catalogue or graph item is selected.
pub fn announce_select(label: &str) -> String {
    format!("selected {label}")
}

/// Screen-reader copy when an action is held. Empty why uses the open-pack prompt.
pub fn announce_held(why: &str) -> String {
    let why = why.trim();
    if why.is_empty() {
        super::HELD_OPEN_PACK.to_string()
    } else {
        why.to_string()
    }
}

/// Accessible action names are non-empty words — never `Host.` and never icon-only.
pub fn action_name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && !name.contains("Host.") && name.chars().any(|c| c.is_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announce_select_includes_label() {
        assert_eq!(announce_select("unit-convert"), "selected unit-convert");
    }

    #[test]
    fn empty_held_uses_open_pack_copy() {
        assert_eq!(announce_held(""), super::super::HELD_OPEN_PACK);
        assert_eq!(announce_held("   "), super::super::HELD_OPEN_PACK);
    }

    #[test]
    fn host_dot_action_name_is_not_ok() {
        assert!(!action_name_ok("Host."));
        assert!(!action_name_ok("✓"));
    }

    #[test]
    fn inspect_action_name_is_ok() {
        assert!(action_name_ok("inspect"));
    }
}
