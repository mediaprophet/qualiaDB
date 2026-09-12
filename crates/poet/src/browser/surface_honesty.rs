//! Honesty chrome: wait-honest empties speak **held / not yet**, never missing theatre.
//!
//! User-visible wait states never say "unavailable" / "Unavailable".
//! Live ALL_BOUND binds (Pulse.*, SHACL.*, GraphDatabase.*) must not false-held
//! when the daemon is connected. Mesh / Job have no Host family — honest held.
//! No Host invent.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

/// Lexicon Frame B voice. Empty Doc / LaTeX / slides wait — they are not broken.
pub const HELD_SAYABLE: &str = "held / not yet";

/// Short whys for Catalog-adjacent / dock / topbar / footer wait chrome.
pub const MESH_HELD_WHY: &str = "no live mesh bind";
pub const PULSE_HELD_WHY: &str = "Pulse waits on the local daemon";
pub const AURA_HELD_WHY: &str = "SHACL waits on the local daemon";
pub const JOB_HELD_WHY: &str = "no live job-queue bind";
pub const GRAPH_HELD_WHY: &str = "graph waits on the local daemon";
pub const MERKLE_HELD_WHY: &str = "Merkle DAG not mounted";
pub const GAS_HELD_WHY: &str = "meter not mounted";
pub const STRATA_HELD_WHY: &str = "strata lens not mounted";
pub const VIBE_UI_HELD_WHY: &str = "Vibe UI host not mounted";
pub const DAG_HELD_WHY: &str = "Merkle-CRDT DAG not connected";
pub const QUADS_HELD_WHY: &str = "container quads not connected";
pub const CONN_ONTO_HELD_WHY: &str = "connection ontology not connected";
pub const TELEMETRY_NOTE: &str = "held / not yet — telemetry and DAG wait on a live backend";
pub const PULSE_LIVE_EMPTY: &str = "waiting for the next Pulse event — daemon connected";
pub const AURA_LIVE_EMPTY: &str = "waiting for a SHACL report — daemon connected";
pub const WAITING_BADGE: &str = "waiting";

const READ_ONLY_REASON: &str =
    "held / not yet — the engine contract named by this surface has not been registered.";

/// `held / not yet — {why}` (or the sayable alone when why is empty).
pub fn held_why(why: &str) -> String {
    let why = why.trim();
    if why.is_empty() {
        HELD_SAYABLE.to_string()
    } else if why.starts_with(HELD_SAYABLE) {
        why.to_string()
    } else {
        format!("{HELD_SAYABLE} — {why}")
    }
}

pub fn copy_avoids_unavailable(text: &str) -> bool {
    !text.to_ascii_lowercase().contains("unavailable")
}

pub fn mesh_badge_copy() -> String {
    format!("\u{25CF} Mesh \u{00B7} {}", held_why(MESH_HELD_WHY))
}

pub fn mesh_badge_title() -> String {
    held_why(MESH_HELD_WHY)
}

/// Pulse empty body. Daemon up + Pulse.* ALL_BOUND → wait, never false-held.
pub fn pulse_empty_copy(daemon_connected: bool) -> String {
    if daemon_connected {
        PULSE_LIVE_EMPTY.to_string()
    } else {
        held_why(PULSE_HELD_WHY)
    }
}

pub fn pulse_badge_copy(daemon_connected: bool, event_count: usize) -> String {
    if event_count > 0 {
        format!("{event_count} events")
    } else if daemon_connected {
        WAITING_BADGE.to_string()
    } else {
        HELD_SAYABLE.to_string()
    }
}

/// Aura / SHACL empty body. Daemon up + SHACL.* ALL_BOUND → wait, never false-held.
pub fn aura_empty_copy(daemon_connected: bool) -> String {
    if daemon_connected {
        AURA_LIVE_EMPTY.to_string()
    } else {
        held_why(AURA_HELD_WHY)
    }
}

pub fn aura_badge_copy(daemon_connected: bool, result_count: usize, passed: usize) -> String {
    if result_count > 0 {
        format!("{passed}/{result_count} valid")
    } else if daemon_connected {
        WAITING_BADGE.to_string()
    } else {
        HELD_SAYABLE.to_string()
    }
}

/// Job queue has no Job.* Host family — empty is honest held, never invented live.
pub fn job_empty_copy() -> String {
    held_why(JOB_HELD_WHY)
}

pub fn job_badge_copy(job_count: usize, running: usize) -> String {
    if job_count > 0 {
        format!("{running} running")
    } else {
        HELD_SAYABLE.to_string()
    }
}

/// Graph footer. Daemon up → caller paints live quin count; never false-held.
pub fn graph_held_copy() -> String {
    held_why(GRAPH_HELD_WHY)
}

pub fn merkle_held_copy() -> String {
    held_why(MERKLE_HELD_WHY)
}

pub fn gas_held_copy() -> String {
    held_why(GAS_HELD_WHY)
}

pub fn strata_held_copy() -> String {
    held_why(STRATA_HELD_WHY)
}

pub fn vibe_ui_held_copy() -> String {
    held_why(VIBE_UI_HELD_WHY)
}

pub fn dag_held_copy() -> String {
    held_why(DAG_HELD_WHY)
}

pub fn quads_held_copy() -> String {
    held_why(QUADS_HELD_WHY)
}

pub fn conn_ontology_held_copy() -> String {
    held_why(CONN_ONTO_HELD_WHY)
}

/// Persist wait-honest chrome as `held` (legacy `unavailable` folds here).
pub fn honesty_attr(token: &str) -> &str {
    if is_wait_honest(token) {
        "held"
    } else {
        persist_token(token)
    }
}

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
        assert!(!declares_prototype(&held_why("daemon is offline")));
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

    #[test]
    fn wait_honest_chrome_never_says_unavailable() {
        let connected_pulse = pulse_empty_copy(true);
        let held_pulse = pulse_empty_copy(false);
        let connected_aura = aura_empty_copy(true);
        let held_aura = aura_empty_copy(false);
        let samples = [
            mesh_badge_copy(),
            mesh_badge_title(),
            connected_pulse,
            held_pulse,
            pulse_badge_copy(true, 0),
            pulse_badge_copy(false, 0),
            connected_aura,
            held_aura,
            aura_badge_copy(true, 0, 0),
            aura_badge_copy(false, 0, 0),
            job_empty_copy(),
            job_badge_copy(0, 0),
            graph_held_copy(),
            merkle_held_copy(),
            gas_held_copy(),
            strata_held_copy(),
            vibe_ui_held_copy(),
            dag_held_copy(),
            quads_held_copy(),
            conn_ontology_held_copy(),
            TELEMETRY_NOTE.to_string(),
            READ_ONLY_REASON.to_string(),
        ];
        for text in &samples {
            assert!(copy_avoids_unavailable(text), "{text}");
            assert!(!text.contains("Unavailable"), "{text}");
        }
        assert_eq!(pulse_empty_copy(true), PULSE_LIVE_EMPTY);
        assert!(!pulse_empty_copy(true).contains(HELD_SAYABLE));
        assert_eq!(aura_empty_copy(true), AURA_LIVE_EMPTY);
        assert!(!aura_empty_copy(true).contains(HELD_SAYABLE));
        assert_eq!(pulse_badge_copy(true, 0), WAITING_BADGE);
        assert_eq!(aura_badge_copy(true, 0, 0), WAITING_BADGE);
        assert_eq!(honesty_attr("unavailable"), "held");
        assert_eq!(honesty_attr("held / not yet"), "held");
        assert_eq!(honesty_attr("live"), "live");
    }

    #[test]
    fn live_all_bound_empties_are_not_false_held() {
        assert_eq!(pulse_badge_copy(true, 3), "3 events");
        assert_eq!(aura_badge_copy(true, 4, 2), "2/4 valid");
        assert_eq!(job_badge_copy(2, 1), "1 running");
        assert!(job_empty_copy().starts_with(HELD_SAYABLE));
        assert!(mesh_badge_copy().contains(HELD_SAYABLE));
        assert!(graph_held_copy().contains(GRAPH_HELD_WHY));
    }

    #[test]
    fn catalog_adjacent_chrome_literals_avoid_unavailable() {
        let blobs = [
            include_str!("topbar/menu.rs"),
            include_str!("docks/right.rs"),
            include_str!("docks/statusbar.rs"),
            include_str!("pulse_stream.rs"),
            include_str!("topbar/pods.rs"),
        ];
        for blob in blobs {
            assert!(
                !blob.contains("Unavailable"),
                "user-facing Unavailable leftover in chrome source"
            );
            assert!(
                !blob.contains("Some(\"unavailable\")"),
                "bare unavailable text leftover in chrome source"
            );
            assert!(
                !blob.contains("\"unavailable\".to_string()"),
                "unavailable badge leftover in chrome source"
            );
        }
    }
}
