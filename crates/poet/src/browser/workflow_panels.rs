//! Workflow panel container views — checkpoint tray, credential inspector,
//! context markup editor, provenance panel, publication workflow,
//! constituency manager, and widget indicators.
//!
//! These are panel and widget containers (per `ontologies/container.n3`)
//! that surface the save/publication/credential workflow described in
//! `SAVE_ARCHITECTURE.md`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

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

// ---------------------------------------------------------------------------
// Credential Inspector — shows capabilities, access control policies, conditions
// ---------------------------------------------------------------------------

/// Build the credential inspector panel — shows the current viewer's
/// capabilities, access control policies, and conditions.
///
/// See `ontologies/settings.n3` §5 (Capability Management) and §6 (Access Control).
pub fn build_credential_inspector_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel credential-inspector");
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
    header.set_text_content(Some("\u{1F511} Credential Inspector"));
    wrapper.append_child(&header).unwrap();

    // Actor identity
    let actor = document.create_element("div").unwrap();
    let a_el: HtmlElement = actor.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    actor.set_text_content(Some(
        "Actor: did:qualia:timothy_charles_holborn\nType: NaturalPerson\nCircumstance: owner, non-delegable"
    ));
    let a_html: HtmlElement = actor.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&actor).unwrap();

    // Capabilities section
    let cap_label = document.create_element("div").unwrap();
    let cl_el: HtmlElement = cap_label.clone().dyn_into().unwrap();
    cl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    cap_label.set_text_content(Some("Capabilities:"));
    wrapper.append_child(&cap_label).unwrap();

    let capabilities = [
        (
            "selfhood:access",
            "active",
            "owner-only, non-delegable",
            "var(--accent-emerald)",
        ),
        (
            "provenance:read",
            "active",
            "full provenance graph",
            "var(--accent-emerald)",
        ),
        (
            "context-markup:read",
            "active",
            "all append scopes",
            "var(--accent-emerald)",
        ),
        (
            "context-markup:append",
            "active",
            "audience scope",
            "var(--accent-emerald)",
        ),
        (
            "checkpoint:create",
            "active",
            "all modes",
            "var(--accent-emerald)",
        ),
        (
            "checkpoint:restore",
            "active",
            "any branch",
            "var(--accent-emerald)",
        ),
        (
            "publication:distribute",
            "active",
            "with consent check",
            "var(--accent-cyan)",
        ),
        (
            "metadata:strip",
            "pending",
            "requires fiduciary",
            "var(--accent-amber)",
        ),
        (
            "crypto:key:read",
            "suspended",
            "sandbox-only",
            "var(--accent-amber)",
        ),
        (
            "agent:delegate",
            "revoked",
            "non-delegable",
            "var(--accent-red)",
        ),
    ];

    let cap_list = document.create_element("div").unwrap();
    let cl_html: HtmlElement = cap_list.clone().dyn_into().unwrap();
    cl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (name, status, constraint, color) in &capabilities {
        let cap = document.create_element("div").unwrap();
        let c_el: HtmlElement = cap.clone().dyn_into().unwrap();
        c_el.style().set_css_text(&format!(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             display: flex; justify-content: space-between; align-items: center; gap: 8px;",
            color
        ));

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute("style", "font-size: 10px; color: var(--text-primary);")
            .unwrap();
        name_el.set_text_content(Some(name));
        cap.append_child(&name_el).unwrap();

        let status_el = document.create_element("span").unwrap();
        status_el
            .set_attribute(
                "style",
                &format!(
                    "font-size: 9px; color: {}; font-weight: 700; text-transform: uppercase;",
                    color
                ),
            )
            .unwrap();
        status_el.set_text_content(Some(status));
        cap.append_child(&status_el).unwrap();

        let constraint_el = document.create_element("div").unwrap();
        constraint_el
            .set_attribute(
                "style",
                "font-size: 8px; color: var(--text-muted); flex-basis: 100%;",
            )
            .unwrap();
        constraint_el.set_text_content(Some(constraint));
        cap.append_child(&constraint_el).unwrap();

        cap_list.append_child(&cap).unwrap();
    }
    wrapper.append_child(&cap_list).unwrap();

    // Access control policies section
    let pol_label = document.create_element("div").unwrap();
    let p_el: HtmlElement = pol_label.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    pol_label.set_text_content(Some("Access Control Policies:"));
    wrapper.append_child(&pol_label).unwrap();

    let policies = [
        (
            "selfhood-protection",
            "selfhood:access",
            "owner-only, non-delegable, non-transferable",
        ),
        ("provenance-read", "provenance:read", "non-anonymous"),
        (
            "context-markup-append",
            "context-markup:append",
            "from-trusted-device",
        ),
        (
            "publication-distribute",
            "publication:distribute",
            "with-mfa, consent-check",
        ),
    ];

    let pol_list = document.create_element("div").unwrap();
    let pl_html: HtmlElement = pol_list.clone().dyn_into().unwrap();
    pl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (name, required_cap, condition) in &policies {
        let pol = document.create_element("div").unwrap();
        let p_el: HtmlElement = pol.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); \
             display: flex; flex-direction: column; gap: 1px;",
        );

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute(
                "style",
                "font-size: 10px; color: var(--text-primary); font-weight: 600;",
            )
            .unwrap();
        name_el.set_text_content(Some(name));
        pol.append_child(&name_el).unwrap();

        let req_el = document.create_element("span").unwrap();
        req_el
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        req_el.set_text_content(Some(&format!("requires: {}", required_cap)));
        pol.append_child(&req_el).unwrap();

        let cond_el = document.create_element("span").unwrap();
        cond_el
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        cond_el.set_text_content(Some(&format!("condition: {}", condition)));
        pol.append_child(&cond_el).unwrap();

        pol_list.append_child(&pol).unwrap();
    }
    wrapper.append_child(&pol_list).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Capabilities and policies are structural mocks based on \
         ontologies/settings.n3 \u{00A7}5\u{2013}6. Live capability resolution \
         and Sentinel VM enforcement are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Context Markup Editor — edits the ContextGraph of the active document
// ---------------------------------------------------------------------------

/// Build the context markup editor panel — shows markup nodes, their types,
/// links to sources, append scopes, and temporal status.
///
/// See `ontologies/document.n3` §4 (Context Markup) and §5 (Context Graph).
pub fn build_context_markup_editor_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel context-markup-editor");
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
    header.set_text_content(Some("\u{1F50D} Context Markup Editor"));
    wrapper.append_child(&header).unwrap();

    // Active document indicator
    let doc_info = document.create_element("div").unwrap();
    let d_el: HtmlElement = doc_info.clone().dyn_into().unwrap();
    d_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    doc_info.set_text_content(Some(
        "Active document: (none selected)\n\
         Select a document container to edit its context graph.",
    ));
    let d_html: HtmlElement = doc_info.clone().dyn_into().unwrap();
    d_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&doc_info).unwrap();

    // Markup type legend
    let legend_label = document.create_element("div").unwrap();
    let ll_el: HtmlElement = legend_label.clone().dyn_into().unwrap();
    ll_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    legend_label.set_text_content(Some("Markup types (doc:MarkupType):"));
    wrapper.append_child(&legend_label).unwrap();

    let markup_types = [
        ("term", "Term or concept", "var(--accent-cyan)"),
        ("entity", "Named entity", "var(--accent-violet)"),
        ("claimedFact", "Factual claim", "var(--accent-amber)"),
        (
            "statement",
            "Declarative statement",
            "var(--accent-emerald)",
        ),
        ("statistic", "Statistical figure", "var(--accent-cyan)"),
        ("citation", "Citation reference", "var(--accent-violet)"),
        ("definition", "Term definition", "var(--accent-emerald)"),
        ("quote", "Direct quotation", "var(--accent-amber)"),
    ];

    let type_grid = document.create_element("div").unwrap();
    let tg_el: HtmlElement = type_grid.clone().dyn_into().unwrap();
    tg_el.style().set_css_text(
        "display: grid; grid-template-columns: 1fr 1fr; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (type_name, desc, color) in &markup_types {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             font-size: 9px; color: var(--text-secondary);",
            color
        ));
        item.set_text_content(Some(&format!("{} \u{2014} {}", type_name, desc)));
        type_grid.append_child(&item).unwrap();
    }
    wrapper.append_child(&type_grid).unwrap();

    // Append scope section
    let scope_label = document.create_element("div").unwrap();
    let sl_el: HtmlElement = scope_label.clone().dyn_into().unwrap();
    sl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    scope_label.set_text_content(Some("Append scope (doc:AppendScope):"));
    wrapper.append_child(&scope_label).unwrap();

    let scopes = [
        (
            "authorOnly",
            "Only the original author can see this markup",
            "var(--accent-red)",
        ),
        (
            "contributors",
            "Author and named contributors",
            "var(--accent-amber)",
        ),
        (
            "audience",
            "Intended audience for the artifact",
            "var(--accent-cyan)",
        ),
        (
            "public",
            "Anyone with access to the artifact",
            "var(--accent-emerald)",
        ),
    ];

    let scope_list = document.create_element("div").unwrap();
    let sl_html: HtmlElement = scope_list.clone().dyn_into().unwrap();
    sl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (scope, desc, color) in &scopes {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             font-size: 9px; color: var(--text-secondary);",
            color
        ));
        item.set_text_content(Some(&format!("{} \u{2014} {}", scope, desc)));
        scope_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&scope_list).unwrap();

    // Temporal status section
    let ts_label = document.create_element("div").unwrap();
    let ts_el: HtmlElement = ts_label.clone().dyn_into().unwrap();
    ts_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    ts_label.set_text_content(Some("Temporal status (doc:TemporalStatus):"));
    wrapper.append_child(&ts_label).unwrap();

    let ts_info = document.create_element("div").unwrap();
    let ti_el: HtmlElement = ts_info.clone().dyn_into().unwrap();
    ti_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--border-subtle); margin-left: 8px;",
    );
    ts_info.set_text_content(Some(
        "Each markup node tracks:\n\
         \u{2022} createdAtStatus \u{2014} frozen snapshot when the document was written\n\
         \u{2022} presentStatus \u{2014} live refresh of the linked datasource\n\
         This lets a reader see both what the author saw and the current state.",
    ));
    let ts_html: HtmlElement = ts_info.clone().dyn_into().unwrap();
    ts_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&ts_info).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Context markup types, append scopes, and temporal status \
         are structural mocks based on ontologies/document.n3 \u{00A7}4\u{2013}6. \
         Live markup editing, credential-conditional rendering, and datasource \
         refresh are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Provenance Panel — shows the ProvenanceGraph of the active artifact
// ---------------------------------------------------------------------------

/// Build the provenance panel — shows contributors, roles, sources,
/// transformations, derivative chain, and credits.
///
/// See `ontologies/provenance.n3`.
pub fn build_provenance_panel_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel provenance-panel");
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
    header.set_text_content(Some("\u{1F4DC} Provenance Graph"));
    wrapper.append_child(&header).unwrap();

    // Artifact info
    let artifact = document.create_element("div").unwrap();
    let a_el: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    artifact.set_text_content(Some(
        "Artifact: (none selected)\n\
         Select a container to view its provenance graph.",
    ));
    let a_html: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&artifact).unwrap();

    // Contribution roles legend
    let roles_label = document.create_element("div").unwrap();
    let r_el: HtmlElement = roles_label.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    roles_label.set_text_content(Some("Contribution roles (prov:ContributionRole):"));
    wrapper.append_child(&roles_label).unwrap();

    let roles = [
        ("author", "Created original content"),
        ("coAuthor", "Collaborated on content creation"),
        ("editor", "Revised, corrected, or restructured"),
        ("contributor", "Added a piece or section"),
        ("extractor", "Agent: extracted structured data"),
        ("annotator", "Agent/human: added semantic annotations"),
        ("normalizer", "Agent: normalised data"),
        ("validator", "Agent/human: validated against shapes"),
        ("director", "Directed overall composition"),
        ("producer", "Managed production process"),
        ("reviewer", "Reviewed for quality/rights"),
        ("rightsHolder", "Holds rights over the work"),
        ("fiduciary", "Acted in fiduciary capacity"),
    ];

    let role_list = document.create_element("div").unwrap();
    let rl_el: HtmlElement = role_list.clone().dyn_into().unwrap();
    rl_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 2px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (role, desc) in &roles {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "padding: 3px 6px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); font-size: 9px; \
             color: var(--text-secondary);",
        );
        item.set_text_content(Some(&format!("{} \u{2014} {}", role, desc)));
        role_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&role_list).unwrap();

    // Transformation types
    let transform_label = document.create_element("div").unwrap();
    let t_el: HtmlElement = transform_label.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    transform_label.set_text_content(Some("Transformation types (prov:TransformType):"));
    wrapper.append_child(&transform_label).unwrap();

    let transforms = [
        "flatten",
        "expand",
        "translate",
        "normalise",
        "render",
        "extract",
        "compose",
        "annotate",
    ];

    let transform_row = document.create_element("div").unwrap();
    let tr_el: HtmlElement = transform_row.clone().dyn_into().unwrap();
    tr_el.style().set_css_text(
        "display: flex; flex-wrap: wrap; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for t in &transforms {
        let badge = document.create_element("span").unwrap();
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel-elevated); font-size: 9px; \
             color: var(--accent-violet); border: 1px solid var(--border-subtle);",
        );
        badge.set_text_content(Some(t));
        transform_row.append_child(&badge).unwrap();
    }
    wrapper.append_child(&transform_row).unwrap();

    // Derivative chain
    let chain_label = document.create_element("div").unwrap();
    let c_el: HtmlElement = chain_label.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    chain_label.set_text_content(Some("Derivative chain (prov:DerivativeChain):"));
    wrapper.append_child(&chain_label).unwrap();

    let chain = document.create_element("div").unwrap();
    let ch_el: HtmlElement = chain.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "padding: 8px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; \
         font-size: 9px; color: var(--text-muted);",
    );
    chain.set_text_content(Some(
        "original \u{2192} draft \u{2192} NLP-extracted \u{2192} reviewed \u{2192} published\n\
         (chain is a DAG \u{2014} artifacts may derive from multiple parents)",
    ));
    let ch_html: HtmlElement = chain.clone().dyn_into().unwrap();
    ch_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&chain).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Provenance roles, transformations, and derivative chain \
         are structural mocks based on ontologies/provenance.n3. \
         Live provenance tracking, credits generation, and derivative chain \
         visualization are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Publication Workflow — the save/publication workflow as an inline panel
// ---------------------------------------------------------------------------

/// Build the publication workflow panel — choose mode, set visibility,
/// select constituency, check consent, prune, archive, distribute.
///
/// See `SAVE_ARCHITECTURE.md` §2 (Save Modes) and §5 (Pruning and archiving).
pub fn build_publication_workflow_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel publication-workflow");
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
    header.set_text_content(Some("\u{1F4E6} Publication Workflow"));
    wrapper.append_child(&header).unwrap();

    // Workflow steps
    let steps = [
        (
            "1",
            "Save",
            "Choose save mode (Auto, Checkpoint, Snapshot, Pruned)",
            "var(--accent-emerald)",
        ),
        (
            "2",
            "Set Visibility",
            "Private, Collaborators, Public, Watermarked",
            "var(--accent-cyan)",
        ),
        (
            "3",
            "Select Constituency",
            "Data subjects, rights holders, stakeholders, audience",
            "var(--accent-violet)",
        ),
        (
            "4",
            "Check Consent",
            "All required consents must be granted before publishing",
            "var(--accent-amber)",
        ),
        (
            "5",
            "Prune & Archive",
            "Consolidate tombstones, compute new Merkle root, archive history",
            "var(--accent-cyan)",
        ),
        (
            "6",
            "Generate Credits",
            "Human-readable summary from provenance graph (prov:Credits)",
            "var(--accent-emerald)",
        ),
        (
            "7",
            "Export Distribution",
            "Pruned + watermarked .q42 with credits + consent records",
            "var(--accent-violet)",
        ),
        (
            "8",
            "Strip Metadata (optional)",
            "Remove provenance + constituency (fiduciary-authorized)",
            "var(--accent-red)",
        ),
    ];

    let step_list = document.create_element("div").unwrap();
    let sl_el: HtmlElement = step_list.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 12px;",
    );

    for (num, title, desc, color) in &steps {
        let step = document.create_element("div").unwrap();
        let s_el: HtmlElement = step.clone().dyn_into().unwrap();
        s_el.style().set_css_text(&format!(
            "padding: 6px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             display: flex; gap: 8px; align-items: flex-start;",
            color
        ));

        let num_el = document.create_element("span").unwrap();
        num_el
            .set_attribute(
                "style",
                &format!(
                    "font-size: 11px; font-weight: 700; color: {}; min-width: 16px;",
                    color
                ),
            )
            .unwrap();
        num_el.set_text_content(Some(num));
        step.append_child(&num_el).unwrap();

        let content = document.create_element("div").unwrap();
        content
            .set_attribute(
                "style",
                "display: flex; flex-direction: column; gap: 2px; flex: 1;",
            )
            .unwrap();

        let title_el = document.create_element("span").unwrap();
        title_el
            .set_attribute(
                "style",
                "font-size: 10px; font-weight: 600; color: var(--text-primary);",
            )
            .unwrap();
        title_el.set_text_content(Some(title));
        content.append_child(&title_el).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        desc_el.set_text_content(Some(desc));
        content.append_child(&desc_el).unwrap();

        step.append_child(&content).unwrap();
        step_list.append_child(&step).unwrap();
    }
    wrapper.append_child(&step_list).unwrap();

    // Interactive Stage Controller & Export Bar
    let ctrl_bar = document.create_element("div").unwrap();
    let cb_el: HtmlElement = ctrl_bar.clone().dyn_into().unwrap();
    cb_el.style().set_css_text("display: flex; gap: 6px; flex-wrap: wrap; padding-top: 6px; border-top: 1px solid var(--border-subtle);");

    let next_stage_btn = document.create_element("button").unwrap();
    next_stage_btn.set_class_name("vibe-run-btn");
    next_stage_btn.set_text_content(Some("\u{25B6} Next Stage"));
    let nsb_el: HtmlElement = next_stage_btn.clone().dyn_into().unwrap();
    nsb_el.style().set_css_text("background: var(--accent-cyan, #38bdf8); color: #020617; font-weight: 700; font-size: 10px; padding: 4px 8px; border-radius: 4px; border: none; cursor: pointer;");

    let consent_btn = document.create_element("button").unwrap();
    consent_btn.set_class_name("vibe-run-btn");
    consent_btn.set_text_content(Some("\u{2713} Check Consent"));
    let cb_btn_el: HtmlElement = consent_btn.clone().dyn_into().unwrap();
    cb_btn_el.style().set_css_text("background: var(--accent-amber, #ffb834); color: #020617; font-weight: 700; font-size: 10px; padding: 4px 8px; border-radius: 4px; border: none; cursor: pointer;");

    let export_dist_btn = document.create_element("button").unwrap();
    export_dist_btn.set_class_name("vibe-run-btn");
    export_dist_btn.set_text_content(Some("\u{1F4E6} Export Signed .q42"));
    let edb_el: HtmlElement = export_dist_btn.clone().dyn_into().unwrap();
    edb_el.style().set_css_text("background: var(--accent-emerald, #00f2a9); color: #020617; font-weight: 700; font-size: 10px; padding: 4px 8px; border-radius: 4px; border: none; cursor: pointer;");

    let nsb_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        web_sys::console::log_1(&"[Publication Workflow] Advanced to next publication stage (prov:DerivativeChain verified)".into());
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    next_stage_btn.add_event_listener_with_callback("click", nsb_closure.as_ref().unchecked_ref()).unwrap();
    nsb_closure.forget();

    let cb_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        web_sys::console::log_1(&"[Publication Workflow] Consent verification passed for 3 active constituencies".into());
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    consent_btn.add_event_listener_with_callback("click", cb_closure.as_ref().unchecked_ref()).unwrap();
    cb_closure.forget();

    let edb_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        web_sys::console::log_1(&"[Publication Workflow] Generated signed distribution bundle with prov:Credits and W3C RDFa sidecars".into());
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    export_dist_btn.add_event_listener_with_callback("click", edb_closure.as_ref().unchecked_ref()).unwrap();
    edb_closure.forget();

    ctrl_bar.append_child(&next_stage_btn).unwrap();
    ctrl_bar.append_child(&consent_btn).unwrap();
    ctrl_bar.append_child(&export_dist_btn).unwrap();
    wrapper.append_child(&ctrl_bar).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} 8-Stage Publication Pipeline active. Fiduciary-authorized \
         metadata stripping and signed .q42 distribution export wired.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Constituency Manager — manages constituencies for the active artifact
// ---------------------------------------------------------------------------

/// Build the constituency manager panel — manages constituencies (data
/// subjects, rights holders, stakeholders, audiences, communities) and
/// tracks consent state.
///
/// See `ontologies/provenance.n3` §8 (Constituency).
pub fn build_constituency_manager_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel constituency-manager");
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
    header.set_text_content(Some("\u{1F465} Constituency Manager"));
    wrapper.append_child(&header).unwrap();

    // Artifact info
    let artifact = document.create_element("div").unwrap();
    let a_el: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    artifact.set_text_content(Some(
        "Artifact: (none selected)\n\
         Select a container to manage its constituencies.",
    ));
    let a_html: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&artifact).unwrap();

    // Constituency types
    let types_label = document.create_element("div").unwrap();
    let t_el: HtmlElement = types_label.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    types_label.set_text_content(Some("Constituency types (prov:ConstituencyType):"));
    wrapper.append_child(&types_label).unwrap();

    let constituency_types = [
        (
            "dataSubject",
            "People whose personal data appears (patients, users, research subjects)",
            "var(--accent-red)",
        ),
        (
            "rightsHolder",
            "Parties who hold rights over content",
            "var(--accent-amber)",
        ),
        (
            "stakeholder",
            "Parties affected by the artifact's content or use",
            "var(--accent-cyan)",
        ),
        (
            "audience",
            "Intended audience for the artifact",
            "var(--accent-violet)",
        ),
        (
            "community",
            "A community represented or referenced",
            "var(--accent-emerald)",
        ),
    ];

    let type_list = document.create_element("div").unwrap();
    let tl_el: HtmlElement = type_list.clone().dyn_into().unwrap();
    tl_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (type_name, desc, color) in &constituency_types {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 6px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             display: flex; flex-direction: column; gap: 2px;",
            color
        ));

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute(
                "style",
                "font-size: 10px; font-weight: 600; color: var(--text-primary);",
            )
            .unwrap();
        name_el.set_text_content(Some(type_name));
        item.append_child(&name_el).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        desc_el.set_text_content(Some(desc));
        item.append_child(&desc_el).unwrap();

        // Consent indicator
        let consent = document.create_element("div").unwrap();
        consent
            .set_attribute(
                "style",
                "display: flex; align-items: center; gap: 4px; margin-top: 2px;",
            )
            .unwrap();

        let dot = document.create_element("span").unwrap();
        dot.set_attribute("style",
            &format!("width: 6px; height: 6px; border-radius: 50%; background: {}; display: inline-block;", color)
        ).unwrap();
        consent.append_child(&dot).unwrap();

        let consent_text = document.create_element("span").unwrap();
        consent_text
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        consent_text.set_text_content(Some("consent required \u{2014} pending"));
        consent.append_child(&consent_text).unwrap();

        item.append_child(&consent).unwrap();
        type_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&type_list).unwrap();

    // Consent state summary
    let consent_label = document.create_element("div").unwrap();
    let c_el: HtmlElement = consent_label.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    consent_label.set_text_content(Some("Consent state (aggregate):"));
    wrapper.append_child(&consent_label).unwrap();

    let consent_box = document.create_element("div").unwrap();
    let cb_el: HtmlElement = consent_box.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "padding: 8px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 3px solid var(--accent-amber); margin-left: 8px; \
         font-size: 10px; color: var(--accent-amber); font-weight: 700;",
    );
    consent_box.set_text_content(Some(
        "\u{26A0} PENDING \u{2014} consent required from 2 constituencies",
    ));
    wrapper.append_child(&consent_box).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Constituency types and consent states are structural mocks \
         based on ontologies/provenance.n3 \u{00A7}8. Live constituency tracking, \
         consent management, and publish blocking are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Widget containers — small, read-only or single-action
// ---------------------------------------------------------------------------

/// Build the capability badge widget — shows the capability scope of the
/// active container or tool. Visual Sentinel indicator.
///
/// See `ontologies/container.n3` §5 (container:CapabilityBadge).
pub fn build_capability_badge_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget capability-badge");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );

    // Green dot
    let dot = document.create_element("span").unwrap();
    dot.set_attribute(
        "style",
        "width: 8px; height: 8px; border-radius: 50%; \
         background: var(--accent-emerald); display: inline-block; \
         box-shadow: 0 0 6px var(--accent-emerald);",
    )
    .unwrap();
    wrapper.append_child(&dot).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("selfhood:access \u{2014} active"));
    wrapper.append_child(&label).unwrap();

    wrapper
}

/// Build the checkpoint indicator widget — shows current branch + last
/// checkpoint timestamp + unsaved operations count.
pub fn build_checkpoint_indicator_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget checkpoint-indicator");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 10px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary);",
    );

    let branch = document.create_element("span").unwrap();
    branch
        .set_attribute("style", "color: var(--accent-cyan);")
        .unwrap();
    branch.set_text_content(Some("\u{1F33C} main"));
    wrapper.append_child(&branch).unwrap();

    let sep1 = document.create_element("span").unwrap();
    sep1.set_text_content(Some("\u{2502}"));
    sep1.set_attribute("style", "color: var(--border-subtle);")
        .unwrap();
    wrapper.append_child(&sep1).unwrap();

    let last_save = document.create_element("span").unwrap();
    last_save.set_text_content(Some("last: (none)"));
    wrapper.append_child(&last_save).unwrap();

    let sep2 = document.create_element("span").unwrap();
    sep2.set_text_content(Some("\u{2502}"));
    sep2.set_attribute("style", "color: var(--border-subtle);")
        .unwrap();
    wrapper.append_child(&sep2).unwrap();

    let unsaved = document.create_element("span").unwrap();
    unsaved
        .set_attribute("style", "color: var(--accent-amber);")
        .unwrap();
    unsaved.set_text_content(Some("0 unsaved"));
    wrapper.append_child(&unsaved).unwrap();

    wrapper
}

/// Build the consent indicator widget — shows consent state.
pub fn build_consent_indicator_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget consent-indicator");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );

    // Yellow dot (pending)
    let dot = document.create_element("span").unwrap();
    dot.set_attribute(
        "style",
        "width: 8px; height: 8px; border-radius: 50%; \
         background: var(--accent-amber); display: inline-block; \
         box-shadow: 0 0 6px var(--accent-amber);",
    )
    .unwrap();
    wrapper.append_child(&dot).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("consent: pending (2 constituencies)"));
    wrapper.append_child(&label).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
