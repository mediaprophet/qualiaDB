//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Checkpoint tray, localStorage history, and restore chrome.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Checkpoint Tray — shows checkpoint history as a vertical timeline
// ---------------------------------------------------------------------------

/// Build the checkpoint tray panel — shows checkpoint history with branch
/// points, actor, timestamp, save mode, and label.
///
/// See `SAVE_ARCHITECTURE.md` §3 (Checkpoint data structure) and §4
/// (Bifurcation model).
pub fn build_checkpoint_tray_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel checkpoint-tray");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; \
         padding: 8px; overflow-y: auto; font-family: var(--font-mono);",
    );

    // Header
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 11px; font-weight: 700; color: var(--text-primary); \
         padding-bottom: 6px; border-bottom: 1px solid var(--border-subtle);",
    );
    header.set_text_content(Some("\u{1F4D4} Checkpoint History"));
    wrapper.append_child(&header).unwrap();

    // Branch indicator
    let branch = document.create_element("div").unwrap();
    let b_el: HtmlElement = branch.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "font-size: 10px; color: var(--accent-cyan); padding: 4px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    branch.set_text_content(Some("\u{1F33C} Branch: main"));
    wrapper.append_child(&branch).unwrap();

    // Timeline
    let timeline = document.create_element("div").unwrap();
    timeline.set_class_name("checkpoint-timeline");
    let t_el: HtmlElement = timeline.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 12px;",
    );

    // Load checkpoint history from localStorage
    let history = load_checkpoint_history(document);

    if history.is_empty() {
        let empty = document.create_element("div").unwrap();
        empty.set_class_name("container-placeholder");
        let e_el: HtmlElement = empty.clone().dyn_into().unwrap();
        e_el.style()
            .set_css_text("font-size: 10px; color: var(--text-muted); padding: 12px 4px;");
        empty.set_text_content(Some(
            "No checkpoints yet. Use File \u{203A} Save As\u{2026} to create one.",
        ));
        timeline.append_child(&empty).unwrap();
    } else {
        for cp in &history {
            let entry = build_checkpoint_entry(document, cp);
            timeline.append_child(&entry).unwrap();
        }
    }

    wrapper.append_child(&timeline).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "display: flex; gap: 6px; padding-top: 8px; \
         border-top: 1px solid var(--border-subtle);",
    );

    let restore_btn = document.create_element("button").unwrap();
    restore_btn.set_text_content(Some("\u{21A9} Restore"));
    style_workflow_button(&restore_btn, false);
    actions.append_child(&restore_btn).unwrap();

    let branch_btn = document.create_element("button").unwrap();
    branch_btn.set_text_content(Some("\u{1F33C} Branch"));
    style_workflow_button(&branch_btn, false);
    actions.append_child(&branch_btn).unwrap();

    let export_btn = document.create_element("button").unwrap();
    export_btn.set_text_content(Some("\u{1F4E4} Export"));
    style_workflow_button(&export_btn, false);
    actions.append_child(&export_btn).unwrap();

    wrapper.append_child(&actions).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Checkpoint chain is live (actor + timestamp + label). \
         Branching, restore, and export are present \u{2014} engine wiring pending. \
         See SAVE_ARCHITECTURE.md.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

/// Checkpoint metadata loaded from localStorage.
#[derive(Clone, Debug)]
struct CheckpointEntry {
    id: String,
    label: String,
    actor: String,
    timestamp: String,
    save_mode: String,
    #[allow(dead_code)]
    parent: Option<String>,
}

/// Load checkpoint history from localStorage.
fn load_checkpoint_history(_document: &Document) -> Vec<CheckpointEntry> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    let history_json = match storage.get_item("qualia-ui:manifest:checkpoint-history") {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    // Parse the comma-separated JSON objects
    // Each object is: {"id":"...","label":"...","actor":"...","timestamp":"...","save_mode":"...","parent_checkpoint":null}
    let mut entries = Vec::new();
    let parts: Vec<&str> = history_json.split("},{").collect();
    for (i, part) in parts.iter().enumerate() {
        let json_str: String = if i == 0 {
            if part.starts_with('{') {
                part.to_string()
            } else {
                format!("{{{}}}", part)
            }
        } else if i == parts.len() - 1 {
            if part.ends_with('}') {
                part.to_string()
            } else {
                format!("{{{}", part)
            }
        } else {
            format!("{{{}}}", part)
        };

        let entry = parse_checkpoint_json(&json_str);
        if let Some(e) = entry {
            entries.push(e);
        }
    }

    // Reverse to show newest first
    entries.reverse();
    entries
}

/// Parse a single checkpoint JSON object.
fn parse_checkpoint_json(json: &str) -> Option<CheckpointEntry> {
    let id = extract_json_string(json, "id")?;
    let label = extract_json_string(json, "label").unwrap_or_default();
    let actor = extract_json_string(json, "actor").unwrap_or_default();
    let timestamp = extract_json_string(json, "timestamp").unwrap_or_default();
    let save_mode = extract_json_string(json, "save_mode").unwrap_or_default();
    let parent = extract_json_string(json, "parent_checkpoint");

    Some(CheckpointEntry {
        id,
        label,
        actor,
        timestamp,
        save_mode,
        parent,
    })
}

/// Extract a string value from a JSON key (simple parser, no dependency).
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Build a single checkpoint entry in the timeline.
fn build_checkpoint_entry(document: &Document, cp: &CheckpointEntry) -> Element {
    let entry = document.create_element("div").unwrap();
    entry.set_class_name("checkpoint-entry");
    let e_el: HtmlElement = entry.clone().dyn_into().unwrap();

    let mode_color = match cp.save_mode.as_str() {
        "auto" => "var(--text-muted)",
        "checkpoint" => "var(--accent-cyan)",
        "snapshot" => "var(--accent-violet)",
        "pruned" => "var(--accent-emerald)",
        _ => "var(--text-secondary)",
    };

    e_el.style().set_css_text(&format!(
        "padding: 6px 8px; border-radius: var(--radius-xs); \
         background: var(--surface-panel); border-left: 3px solid {}; \
         display: flex; flex-direction: column; gap: 2px; cursor: pointer; \
         transition: var(--trans-fast);",
        mode_color
    ));

    // Label line
    let label_line = document.create_element("div").unwrap();
    let l_el: HtmlElement = label_line.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("font-size: 11px; color: var(--text-primary); font-weight: 600;");
    let label_text = if cp.label.is_empty() {
        format!("[{}] {}", cp.save_mode, cp.id)
    } else {
        format!("[{}] {}", cp.save_mode, cp.label)
    };
    label_line.set_text_content(Some(&label_text));
    entry.append_child(&label_line).unwrap();

    // Actor + timestamp line
    let meta_line = document.create_element("div").unwrap();
    let m_el: HtmlElement = meta_line.clone().dyn_into().unwrap();
    m_el.style()
        .set_css_text("font-size: 9px; color: var(--text-muted);");
    let actor_short = if cp.actor.len() > 30 {
        &cp.actor[..30]
    } else {
        &cp.actor
    };
    let ts_short = if cp.timestamp.len() > 19 {
        &cp.timestamp[..19]
    } else {
        &cp.timestamp
    };
    meta_line.set_text_content(Some(&format!("{} \u{2014} {}", actor_short, ts_short)));
    entry.append_child(&meta_line).unwrap();

    entry
}

/// Style a workflow button (secondary or primary).
fn style_workflow_button(el: &Element, primary: bool) {
    let html_el: HtmlElement = el.clone().dyn_into().unwrap();
    if primary {
        html_el.style().set_css_text(
            "padding: 6px 12px; border: 1px solid var(--accent-cyan); \
             border-radius: var(--radius-xs); background: var(--accent-cyan); \
             color: var(--bg-deep); font-family: var(--font-mono); \
             font-size: 10px; font-weight: 700; cursor: pointer;",
        );
    } else {
        html_el.style().set_css_text(
            "padding: 6px 12px; border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); background: var(--surface-panel); \
             color: var(--text-secondary); font-family: var(--font-mono); \
             font-size: 10px; cursor: pointer; transition: var(--trans-fast);",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_json_string, parse_checkpoint_json};

    #[test]
    fn extracts_quoted_json_string() {
        assert_eq!(
            extract_json_string(r#"{"id":"cp-1","label":"save"}"#, "id").as_deref(),
            Some("cp-1")
        );
    }

    #[test]
    fn parse_checkpoint_requires_id() {
        assert!(parse_checkpoint_json(r#"{"label":"no-id"}"#).is_none());
        let entry = parse_checkpoint_json(
            r#"{"id":"cp-1","label":"nightly","actor":"did:q","timestamp":"2026-09-05T00:00:00Z","save_mode":"local"}"#,
        )
        .unwrap();
        assert_eq!(entry.id, "cp-1");
        assert_eq!(entry.label, "nightly");
        assert_eq!(entry.save_mode, "local");
    }
}
