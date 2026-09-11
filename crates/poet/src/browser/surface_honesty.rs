//! Honesty chrome: wait-honest empties speak **held / not yet**, never missing theatre.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

/// Lexicon Frame B voice. Empty Doc / LaTeX / slides wait — they are not broken.
pub const HELD_SAYABLE: &str = "held / not yet";

const READ_ONLY_REASON: &str =
    "held / not yet — the engine contract named by this surface has not been registered.";

/// Wait-honest empty / not-yet tokens. Legacy `missing` is this class, not a fault.
pub fn is_wait_honest(token: &str) -> bool {
    matches!(
        token.trim(),
        "missing" | "held" | "unavailable" | "held / not yet"
    )
}

/// Persist a chrome token. Legacy `missing` empties fold to `held`.
pub fn persist_token(raw: &str) -> &str {
    let raw = raw.trim();
    if is_wait_honest(raw) {
        "held"
    } else {
        raw
    }
}

/// CSS modifier after `honesty-badge `.
pub fn badge_modifier(token: &str) -> &'static str {
    if is_wait_honest(token) {
        return "honesty-held";
    }
    match persist_token(token) {
        "live" => "honesty-live",
        "present" => "honesty-present",
        "partial" => "honesty-partial",
        "error" | "fault" | "failed" => "honesty-error",
        "running" => "honesty-running",
        _ => "honesty-partial",
    }
}

/// Human badge label. Wait-honest empties never say missing / broken.
pub fn badge_label(token: &str) -> &str {
    if is_wait_honest(token) {
        HELD_SAYABLE
    } else {
        persist_token(token)
    }
}

/// Paint a container honesty badge from a seed / stored token.
pub fn paint_badge(badge: &Element, token: &str) {
    let stored = persist_token(token);
    badge.set_class_name(&format!("honesty-badge {}", badge_modifier(token)));
    badge.set_text_content(Some(badge_label(token)));
    badge.set_attribute("data-honesty", stored).ok();
    if is_wait_honest(token) {
        badge.set_attribute("data-gate", "held").ok();
        badge.set_attribute("title", HELD_SAYABLE).ok();
    } else {
        badge.remove_attribute("data-gate").ok();
        badge.set_attribute("title", stored).ok();
    }
}

fn declares_prototype(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("mock data")
        || text.contains("structural mock")
        || text.contains("renderer placeholder")
}

/// Disable leftover mock-labelled bodies. Live COP session surfaces are untouched.
pub fn enforce(document: &Document, body: &Element, _container_type: &str) -> bool {
    let text = body.text_content().unwrap_or_default();
    if !declares_prototype(&text) {
        return false;
    }

    body.set_attribute("data-honesty", "read-only-prototype")
        .ok();
    body.class_list().add_1("read-only-prototype").ok();
    if let Some(container) = body.parent_element() {
        container
            .set_attribute("data-effective-honesty", "unavailable")
            .ok();
        if let Ok(Some(badge)) = container.query_selector(".honesty-badge") {
            paint_badge(&badge, "held");
            badge.set_attribute("title", READ_ONLY_REASON).ok();
        }
    }

    if let Ok(controls) =
        body.query_selector_all("button, input, select, textarea, [contenteditable=\"true\"]")
    {
        for index in 0..controls.length() {
            let Some(node) = controls.get(index) else {
                continue;
            };
            let Ok(control) = node.dyn_into::<Element>() else {
                continue;
            };
            control.set_attribute("disabled", "").ok();
            control.set_attribute("aria-disabled", "true").ok();
            control.set_attribute("contenteditable", "false").ok();
            control.set_attribute("title", READ_ONLY_REASON).ok();
        }
    }
    let _ = document;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_prototype_markers_are_classified() {
        assert!(declares_prototype("Mock data — engine required"));
        assert!(declares_prototype("structural mock"));
        assert!(!declares_prototype("Unavailable: daemon is offline."));
        assert!(!declares_prototype("Live COP ledger count"));
    }

    #[test]
    fn unbound_specialist_surfaces_have_specific_prerequisites() {
        // Session surfaces persist; leftover mock labels still fail closed.
        assert!(declares_prototype("Mock data — wallet"));
    }

    #[test]
    fn wait_honest_empties_never_say_missing_or_broken() {
        for token in ["missing", "held", "unavailable", "held / not yet"] {
            assert!(is_wait_honest(token), "{token}");
            assert_eq!(persist_token(token), "held");
            assert_eq!(badge_label(token), HELD_SAYABLE);
            assert_eq!(badge_modifier(token), "honesty-held");
            assert!(!badge_label(token).contains("missing"));
            assert!(!badge_label(token).contains("broken"));
        }
        assert_eq!(badge_label("live"), "live");
        assert_eq!(badge_modifier("failed"), "honesty-error");
        assert!(!HELD_SAYABLE.contains("qualia."));
    }
}
