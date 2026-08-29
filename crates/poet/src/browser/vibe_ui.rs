//! VibeScript-Driven Dynamic UI Engine & DOM Reconciler.
//!
//! Enables authoring, evaluating, and live-reloading UI components (such as
//! dock furniture, collapsible trays, metrics, cards, and containers) directly
//! in VibeScript (`.vibe`) without recompiling the Rust host.
//!
//! Aligned with `vibescript-core.md` and `poet-mindware-workbench-ui.md`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Document, Element, Event, HtmlElement, HtmlTextAreaElement};

use vibe::{Budget, Engine, Env, LocalHost, Program, Value, parse_program};

// ---------------------------------------------------------------------------
// Vibe UI AST & Representation
// ---------------------------------------------------------------------------

/// A structured UI component node generated from VibeScript evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum VibeUiNode {
    /// A collapsible top-level dock panel (e.g. Aura Tray, Pulse Stream).
    DockPanel {
        id: String,
        title: String,
        badge: Option<String>,
        collapsed: bool,
        flex_grow: bool,
        children: Vec<VibeUiNode>,
    },
    /// A collapsible sub-tray nested inside a dock panel.
    Subtray {
        id: String,
        title: String,
        badge: Option<String>,
        collapsed: bool,
        children: Vec<VibeUiNode>,
    },
    /// A SHACL shape validation entry.
    ShaclShape {
        shape: String,
        conformant: bool,
        node_count: u32,
        violations: Vec<String>,
    },
    /// A key-value metric row.
    Metric {
        label: String,
        value: String,
        color: Option<String>,
    },
    /// An interactive action button.
    Button {
        id: String,
        label: String,
        action: String,
        class_name: String,
    },
    /// Formatted text block.
    Text { content: String, style: String },
    /// Generic container box with children.
    Container {
        class_name: String,
        style: String,
        children: Vec<VibeUiNode>,
    },
}

impl VibeUiNode {
    /// Convert a VibeScript `Value` (Record or Map) into a `VibeUiNode`.
    pub fn from_value(val: &Value) -> Option<Self> {
        match val {
            Value::Record(map) => Self::from_map(map),
            Value::List(list) => {
                let children: Vec<VibeUiNode> = list.iter().filter_map(Self::from_value).collect();
                Some(VibeUiNode::Container {
                    class_name: "vibe-ui-group".into(),
                    style: "".into(),
                    children,
                })
            }
            Value::String(s) => Some(VibeUiNode::Text {
                content: s.clone(),
                style: "".into(),
            }),
            _ => None,
        }
    }

    fn from_map(map: &BTreeMap<String, Value>) -> Option<Self> {
        let node_type = map
            .get("type")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("container");

        match node_type {
            "dock_panel" | "dock-panel" | "panel" => {
                let id = get_string(map, "id").unwrap_or_else(|| "dock-panel".into());
                let title = get_string(map, "title").unwrap_or_else(|| "Panel".into());
                let badge = get_string(map, "badge");
                let collapsed = get_bool(map, "collapsed").unwrap_or(false);
                let flex_grow = get_bool(map, "flex_grow").unwrap_or(false);
                let children = get_children(map);
                Some(VibeUiNode::DockPanel {
                    id,
                    title,
                    badge,
                    collapsed,
                    flex_grow,
                    children,
                })
            }
            "subtray" | "sub-tray" | "tray" => {
                let id = get_string(map, "id").unwrap_or_else(|| "subtray".into());
                let title = get_string(map, "title").unwrap_or_else(|| "Section".into());
                let badge = get_string(map, "badge");
                let collapsed = get_bool(map, "collapsed").unwrap_or(false);
                let children = get_children(map);
                Some(VibeUiNode::Subtray {
                    id,
                    title,
                    badge,
                    collapsed,
                    children,
                })
            }
            "shacl_shape" | "shacl-shape" | "shape" => {
                let shape = get_string(map, "shape").unwrap_or_else(|| "Shape".into());
                let conformant = get_bool(map, "conformant").unwrap_or(true);
                let node_count = map
                    .get("nodes")
                    .or_else(|| map.get("node_count"))
                    .and_then(|v| match v {
                        Value::I64(n) => Some(*n as u32),
                        Value::U64(n) => Some(*n as u32),
                        _ => None,
                    })
                    .unwrap_or(0);
                let mut violations = Vec::new();
                if let Some(v_val) = map.get("violation").or_else(|| map.get("violations")) {
                    match v_val {
                        Value::String(s) => violations.push(s.clone()),
                        Value::List(l) => {
                            for item in l {
                                if let Value::String(s) = item {
                                    violations.push(s.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Some(VibeUiNode::ShaclShape {
                    shape,
                    conformant,
                    node_count,
                    violations,
                })
            }
            "metric" | "row" => {
                let label = get_string(map, "label").unwrap_or_default();
                let value = get_string(map, "value").unwrap_or_default();
                let color = get_string(map, "color");
                Some(VibeUiNode::Metric {
                    label,
                    value,
                    color,
                })
            }
            "button" | "action" => {
                let id = get_string(map, "id").unwrap_or_else(|| "btn".into());
                let label = get_string(map, "label").unwrap_or_else(|| "Action".into());
                let action = get_string(map, "action").unwrap_or_default();
                let class_name = get_string(map, "class").unwrap_or_else(|| "vibe-run-btn".into());
                Some(VibeUiNode::Button {
                    id,
                    label,
                    action,
                    class_name,
                })
            }
            "text" => {
                let content = get_string(map, "content")
                    .or_else(|| get_string(map, "text"))
                    .unwrap_or_default();
                let style = get_string(map, "style").unwrap_or_default();
                Some(VibeUiNode::Text { content, style })
            }
            _ => {
                let class_name = get_string(map, "class").unwrap_or_default();
                let style = get_string(map, "style").unwrap_or_default();
                let children = get_children(map);
                Some(VibeUiNode::Container {
                    class_name,
                    style,
                    children,
                })
            }
        }
    }
}

fn get_string(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Iri(s) => Some(s.clone()),
        Value::Prefixed(p, l) => Some(format!("{p}:{l}")),
        _ => None,
    })
}

fn get_bool(map: &BTreeMap<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn get_children(map: &BTreeMap<String, Value>) -> Vec<VibeUiNode> {
    if let Some(Value::List(list)) = map.get("children").or_else(|| map.get("items")) {
        list.iter().filter_map(VibeUiNode::from_value).collect()
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Evaluator & Parser
// ---------------------------------------------------------------------------

/// Evaluate a VibeScript source string into a root `VibeUiNode`.
pub fn eval_vibe_ui_script(source: &str) -> Result<VibeUiNode, String> {
    let program: Program = parse_program(source).map_err(|e| {
        format!(
            "VibeScript Parse Error (line {}): {}",
            e.span.start, e.message
        )
    })?;

    let mut host = LocalHost::default();
    let mut env = Env::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let val = engine
        .eval_program(&program, &mut env)
        .map_err(|e| format!("VibeScript Evaluation Error: {}", e.message))?;

    // If the program returned a direct UI node or set a variable named `ui`, `aura_ui`, or `aura_tray`, extract it.
    if let Some(node) = VibeUiNode::from_value(&val) {
        return Ok(node);
    }
    if let Some(ui_val) = env
        .vars
        .get("ui")
        .or_else(|| env.vars.get("aura_ui"))
        .or_else(|| env.vars.get("aura_tray"))
    {
        if let Some(node) = VibeUiNode::from_value(ui_val) {
            return Ok(node);
        }
    }
    // Fallback: look for any variable in env that converts to a VibeUiNode
    for (_name, var_val) in env.vars.iter() {
        if let Some(node) = VibeUiNode::from_value(var_val) {
            return Ok(node);
        }
    }

    Err("Program evaluated successfully but produced no valid UI presentation node.".into())
}

// ---------------------------------------------------------------------------
// DOM Rendering & Reconciler
// ---------------------------------------------------------------------------

/// Render a `VibeUiNode` into a live, styled, interactive DOM element.
pub fn render_vibe_ui_node(document: &Document, node: &VibeUiNode) -> Element {
    match node {
        VibeUiNode::DockPanel {
            title,
            badge,
            collapsed,
            flex_grow,
            children,
            ..
        } => {
            let body = document.create_element("div").unwrap();
            body.set_class_name("dock-panel-body");
            for child in children {
                body.append_child(&render_vibe_ui_node(document, child))
                    .unwrap();
            }
            super::docks::create_collapsible_dock_panel(
                document,
                title,
                badge.as_deref(),
                body,
                !collapsed,
                *flex_grow,
            )
        }

        VibeUiNode::Subtray {
            title,
            badge,
            collapsed,
            children,
            ..
        } => {
            let body = document.create_element("div").unwrap();
            body.set_class_name("dock-subtray-body");
            for child in children {
                body.append_child(&render_vibe_ui_node(document, child))
                    .unwrap();
            }
            super::diagnostics::render_subtray(document, title, badge.as_deref(), body, !collapsed)
        }

        VibeUiNode::ShaclShape {
            shape,
            conformant,
            node_count,
            violations,
        } => {
            let row_container = document.create_element("div").unwrap();
            row_container
                .set_attribute(
                    "style",
                    "margin-top: 3px; font-family: var(--font-mono); font-size: 10px;",
                )
                .unwrap();

            let row = document.create_element("div").unwrap();
            row.set_attribute(
                "style",
                "display: flex; align-items: center; gap: 5px; cursor: pointer;",
            )
            .unwrap();

            let icon = document.create_element("span").unwrap();
            icon.set_text_content(Some(if *conformant { "\u{2705}" } else { "\u{274C}" }));
            row.append_child(&icon).unwrap();

            let label = document.create_element("span").unwrap();
            let l_el: HtmlElement = label.clone().dyn_into().unwrap();
            if *conformant {
                l_el.style()
                    .set_css_text("color: var(--accent-emerald); font-weight: 500;");
                label.set_text_content(Some(&format!("{shape} \u{00B7} {node_count} nodes")));
            } else {
                l_el.style()
                    .set_css_text("color: var(--accent-rose); font-weight: 600;");
                label.set_text_content(Some(&format!(
                    "{shape} \u{00B7} {} violation{}",
                    violations.len(),
                    if violations.len() == 1 { "" } else { "s" }
                )));
            }
            row.append_child(&label).unwrap();
            row_container.append_child(&row).unwrap();

            if !violations.is_empty() {
                let viol_box = document.create_element("div").unwrap();
                let vb_el: HtmlElement = viol_box.clone().dyn_into().unwrap();
                vb_el
                    .style()
                    .set_css_text("padding-left: 18px; margin-top: 2px;");

                for v in violations {
                    let d = document.create_element("div").unwrap();
                    d.set_attribute(
                        "style",
                        "color: var(--accent-rose); font-size: 9px; margin-top: 1px;",
                    )
                    .unwrap();
                    d.set_text_content(Some(&format!("\u{2192} {v}")));
                    viol_box.append_child(&d).unwrap();
                }
                row_container.append_child(&viol_box).unwrap();
            }

            row_container
        }

        VibeUiNode::Metric {
            label,
            value,
            color,
        } => {
            let row = document.create_element("div").unwrap();
            row.set_attribute(
                "style",
                "display: flex; align-items: center; justify-content: space-between; \
                 font-family: var(--font-mono); font-size: 9.5px; padding: 2px 0;",
            )
            .unwrap();

            let l_span = document.create_element("span").unwrap();
            l_span
                .set_attribute("style", "color: var(--text-muted);")
                .unwrap();
            l_span.set_text_content(Some(label));
            row.append_child(&l_span).unwrap();

            let v_span = document.create_element("span").unwrap();
            let v_el: HtmlElement = v_span.clone().dyn_into().unwrap();
            let col = color.as_deref().unwrap_or("var(--text-secondary)");
            v_el.style()
                .set_css_text(&format!("color: {col}; font-weight: 500;"));
            v_span.set_text_content(Some(value));
            row.append_child(&v_span).unwrap();

            row
        }

        VibeUiNode::Button {
            label,
            class_name,
            action,
            ..
        } => {
            let btn = document.create_element("button").unwrap();
            btn.set_class_name(class_name);
            let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
            b_el.style().set_css_text(
                "padding: 2px 8px; font-size: 9px; cursor: pointer; margin-top: 4px;",
            );
            btn.set_text_content(Some(label));

            let act = action.clone();
            let click_closure = Closure::wrap(Box::new(move |_e: Event| {
                web_sys::console::log_1(&format!("[VibeUi Action] Triggered: {act}").into());
            }) as Box<dyn FnMut(Event)>);
            btn.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
                .unwrap();
            click_closure.forget();

            btn
        }

        VibeUiNode::Text { content, style } => {
            let div = document.create_element("div").unwrap();
            if !style.is_empty() {
                div.set_attribute("style", style).unwrap();
            }
            div.set_text_content(Some(content));
            div
        }

        VibeUiNode::Container {
            class_name,
            style,
            children,
        } => {
            let container = document.create_element("div").unwrap();
            if !class_name.is_empty() {
                container.set_class_name(class_name);
            }
            if !style.is_empty() {
                container.set_attribute("style", style).unwrap();
            }
            for child in children {
                container
                    .append_child(&render_vibe_ui_node(document, child))
                    .unwrap();
            }
            container
        }
    }
}

// ---------------------------------------------------------------------------
// Live VibeScript UI Host Component (<q-vibe-ui>)
// ---------------------------------------------------------------------------

/// Render a live, hot-reloadable VibeScript UI host container.
pub fn render_live_vibe_ui(document: &Document, default_source: &str) -> Element {
    let host = document.create_element("q-vibe-ui").unwrap();
    host.set_class_name("q-vibe-ui-host");
    let h_el: HtmlElement = host.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden;",
    );

    // Render area for the evaluated Vibe UI component
    let render_slot = document.create_element("div").unwrap();
    render_slot.set_class_name("vibe-ui-render-slot");
    let rs_el: HtmlElement = render_slot.clone().dyn_into().unwrap();
    rs_el
        .style()
        .set_css_text("flex: 1; overflow-y: auto; display: flex; flex-direction: column;");

    // Initial render
    let initial_node = eval_vibe_ui_script(default_source).unwrap_or_else(|e| {
        VibeUiNode::Text {
            content: format!("VibeScript Error: {e}"),
            style: "color: var(--accent-rose); font-family: var(--font-mono); font-size: 11px; padding: 8px;".into(),
        }
    });
    render_slot
        .append_child(&render_vibe_ui_node(document, &initial_node))
        .unwrap();

    host.append_child(&render_slot).unwrap();

    // Collapsible Live Source Editor Bar
    let editor_tray = document.create_element("div").unwrap();
    editor_tray.set_class_name("vibe-live-editor-tray");
    let et_el: HtmlElement = editor_tray.clone().dyn_into().unwrap();
    et_el.style().set_css_text(
        "border-top: 1px solid var(--border-subtle); background: var(--surface-panel); \
         display: flex; flex-direction: column; max-height: 180px; flex-shrink: 0;",
    );

    let editor_header = document.create_element("div").unwrap();
    editor_header.set_class_name("vibe-editor-header");
    let eh_el: HtmlElement = editor_header.clone().dyn_into().unwrap();
    eh_el.style().set_css_text(
        "height: 24px; padding: 0 8px; font-size: 9px; font-weight: 700; text-transform: uppercase; \
         letter-spacing: 0.06em; color: var(--text-muted); display: flex; align-items: center; \
         justify-content: space-between; cursor: pointer; user-select: none; background: var(--surface-base);",
    );

    let ed_title = document.create_element("span").unwrap();
    ed_title.set_text_content(Some("\u{26A1} Live VibeScript UI Editor"));
    editor_header.append_child(&ed_title).unwrap();

    let ed_badge = document.create_element("span").unwrap();
    let eb_el: HtmlElement = ed_badge.clone().dyn_into().unwrap();
    eb_el.style().set_css_text(
        "font-size: 8px; padding: 1px 4px; border-radius: 2px; background: var(--surface-panel-elevated); color: var(--accent-cyan);",
    );
    ed_badge.set_text_content(Some("Zero-Compile"));
    editor_header.append_child(&ed_badge).unwrap();

    editor_tray.append_child(&editor_header).unwrap();

    let editor_body = document.create_element("div").unwrap();
    let eb_body_el: HtmlElement = editor_body.clone().dyn_into().unwrap();
    eb_body_el.style().set_css_text(
        "display: none; padding: 6px; flex-direction: column; gap: 4px; background: var(--surface-base);",
    );

    let textarea = document.create_element("textarea").unwrap();
    textarea.set_class_name("vibe-editor-textarea");
    let ta_el: HtmlTextAreaElement = textarea.clone().dyn_into().unwrap();
    ta_el.set_value(default_source);
    ta_el.style().set_css_text(
        "width: 100%; height: 110px; font-family: var(--font-mono); font-size: 9.5px; \
         background: #030508; color: #a5b4fc; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 4px; resize: none; box-sizing: border-box;",
    );
    editor_body.append_child(&textarea).unwrap();

    let apply_btn = document.create_element("button").unwrap();
    apply_btn.set_class_name("vibe-run-btn");
    let ab_el: HtmlElement = apply_btn.clone().dyn_into().unwrap();
    ab_el
        .style()
        .set_css_text("align-self: flex-end; padding: 2px 8px; font-size: 9px;");
    apply_btn.set_text_content(Some("\u{25B6} Hot-Reload UI"));
    editor_body.append_child(&apply_btn).unwrap();

    editor_tray.append_child(&editor_body).unwrap();
    host.append_child(&editor_tray).unwrap();

    // Toggle editor visibility
    let editor_open = Rc::new(Cell::new(false));
    let eo_c = editor_open.clone();
    let eb_body_c = editor_body.clone();
    let toggle_editor_closure = Closure::wrap(Box::new(move |_e: Event| {
        let next = !eo_c.get();
        eo_c.set(next);
        let b: HtmlElement = eb_body_c.clone().dyn_into().unwrap();
        b.style()
            .set_property("display", if next { "flex" } else { "none" })
            .unwrap();
    }) as Box<dyn FnMut(Event)>);
    editor_header
        .add_event_listener_with_callback("click", toggle_editor_closure.as_ref().unchecked_ref())
        .unwrap();
    toggle_editor_closure.forget();

    // Wire Hot-Reload Action
    let doc_clone = document.clone();
    let slot_clone = render_slot.clone();
    let ta_clone = textarea.clone();
    let reload_closure = Closure::wrap(Box::new(move |_e: Event| {
        let ta: HtmlTextAreaElement = ta_clone.clone().dyn_into().unwrap();
        let src = ta.value();

        // Clear existing slot children
        while let Some(child) = slot_clone.first_child() {
            slot_clone.remove_child(&child).unwrap();
        }

        match eval_vibe_ui_script(&src) {
            Ok(node) => {
                let rendered = render_vibe_ui_node(&doc_clone, &node);
                slot_clone.append_child(&rendered).unwrap();
            }
            Err(err) => {
                let err_el = doc_clone.create_element("div").unwrap();
                err_el
                    .set_attribute(
                        "style",
                        "color: var(--accent-rose); font-family: var(--font-mono); font-size: 10px; padding: 10px;",
                    )
                    .unwrap();
                err_el.set_text_content(Some(&format!("\u{274C} {err}")));
                slot_clone.append_child(&err_el).unwrap();
            }
        }
    }) as Box<dyn FnMut(Event)>);
    apply_btn
        .add_event_listener_with_callback("click", reload_closure.as_ref().unchecked_ref())
        .unwrap();
    reload_closure.forget();

    host
}

/// Default VibeScript UI program defining the Aura Tray schema and sub-trays.
pub fn default_aura_tray_vibe_script() -> &'static str {
    r#"// Aura Tray Definition in VibeScript 0.1
let aura_ui = {
  type: "dock_panel",
  title: "Aura Tray",
  badge: "3/4 Valid",
  collapsed: false,
  children: [
    {
      type: "subtray",
      title: "SHACL Shapes",
      badge: "3/4",
      collapsed: false,
      children: [
        { type: "shacl_shape", shape: "soc:PeerShape", conformant: true, nodes: 42 },
        { type: "shacl_shape", shape: "soc:AgreementShape", conformant: true, nodes: 8 },
        { type: "shacl_shape", shape: "health:RecordShape", conformant: false, nodes: 15, violation: "missing `health:hasConsent` on 2 nodes" },
        { type: "shacl_shape", shape: "rights:FiduciaryShape", conformant: true, nodes: 3 }
      ]
    },
    {
      type: "subtray",
      title: "Ontologies & Schemas",
      badge: "5 Active",
      collapsed: false,
      children: [
        { type: "metric", label: "q42:", value: "Qualia Core & did:q42 Topologies", color: "var(--accent-cyan)" },
        { type: "metric", label: "soc:", value: "Social Agreements & Commons", color: "var(--accent-cyan)" },
        { type: "metric", label: "health:", value: "Clinical & Biomedical Modalities", color: "var(--accent-cyan)" },
        { type: "metric", label: "rights:", value: "Fiduciary Agency & Guardianship", color: "var(--accent-cyan)" },
        { type: "metric", label: "vibe:", value: "VibeScript 0.1 AST & Effects", color: "var(--accent-cyan)" }
      ]
    },
    {
      type: "subtray",
      title: "Super-Quin Sentinel",
      badge: "42MB Cap",
      collapsed: true,
      children: [
        { type: "metric", label: "Certainty:", value: "96% (Epistemic Halo)", color: "var(--accent-emerald)" },
        { type: "metric", label: "Hot Path:", value: "Zero-Heap · 48B Quin", color: "var(--accent-cyan)" },
        { type: "button", label: "📦 Export .hcf", action: "export_hcf" }
      ]
    }
  ]
};
return aura_ui;
"#
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_vibe_ui_script_valid() {
        let script = r#"
            let panel = {
                type: "dock_panel",
                title: "Test Panel",
                badge: "Active",
                collapsed: false,
                children: [
                    { type: "metric", label: "Status", value: "OK" }
                ]
            };
            return panel;
        "#;
        let res = eval_vibe_ui_script(script);
        assert!(
            res.is_ok(),
            "Failed to eval Vibe UI script: {:?}",
            res.err()
        );
        let node = res.unwrap();
        match node {
            VibeUiNode::DockPanel {
                title,
                badge,
                children,
                ..
            } => {
                assert_eq!(title, "Test Panel");
                assert_eq!(badge, Some("Active".into()));
                assert_eq!(children.len(), 1);
            }
            _ => panic!("Expected DockPanel node"),
        }
    }

    #[test]
    fn test_default_aura_tray_script_eval() {
        let script = default_aura_tray_vibe_script();
        let res = eval_vibe_ui_script(script);
        assert!(
            res.is_ok(),
            "Failed to eval default Aura script: {:?}",
            res.err()
        );
        if let VibeUiNode::DockPanel {
            title, children, ..
        } = res.unwrap()
        {
            assert_eq!(title, "Aura Tray");
            assert_eq!(children.len(), 3);
        } else {
            panic!("Expected DockPanel");
        }
    }
}
