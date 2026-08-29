//! Wire inspector — click a connection wire to see its details in a
//! floating inspector panel. Ports the wire inspector concept from the
//! Canvas_Workbench mockup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MouseEvent};

/// Wire up wire click handlers — clicking a wire path selects it and shows
/// the wire inspector panel with connection details. Double-clicking a wire
/// label lets you edit it inline.
pub fn wire_wire_inspector(document: &Document) {
    let paths = document.query_selector_all(".wire-overlay path").unwrap();
    for i in 0..paths.length() {
        let path = paths.get(i).unwrap();
        let path_el: Element = path.dyn_into().unwrap();
        if !super::dom_bindings::claim(&path_el, "wire-inspector") {
            continue;
        }
        let wire_id = path_el.get_attribute("data-id").unwrap_or_default();
        path_el.set_attribute("tabindex", "0").unwrap();
        path_el.set_attribute("role", "button").unwrap();
        path_el
            .set_attribute("aria-label", "Open semantic wire editor")
            .unwrap();
        let path_clone = path_el.clone();
        let wire_id_for_click = wire_id.clone();

        let closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();

            // Deselect all other wires
            let all_paths = doc.query_selector_all(".wire-overlay path").unwrap();
            for j in 0..all_paths.length() {
                let p = all_paths.get(j).unwrap();
                let pe: Element = p.dyn_into().unwrap();
                pe.class_list().remove_1("wire-selected").unwrap();
            }

            // Select this wire
            path_clone.class_list().add_1("wire-selected").unwrap();

            // Also deselect all containers (wires and containers are mutually exclusive selection)
            let all_containers = doc.query_selector_all(".canvas-container-node").unwrap();
            for j in 0..all_containers.length() {
                let c = all_containers.get(j).unwrap();
                let ce: Element = c.dyn_into().unwrap();
                ce.class_list().remove_1("selected").unwrap();
            }

            show_inspector(&doc, &wire_id_for_click);
        }) as Box<dyn FnMut(MouseEvent)>);

        path_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        let wire_id_key = wire_id.clone();
        let key_closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if event.key() == "Enter" || event.key() == " " {
                event.prevent_default();
                let doc = web_sys::window().unwrap().document().unwrap();
                show_inspector(&doc, &wire_id_key);
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        path_el
            .add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
            .unwrap();
        key_closure.forget();
    }

    // Wire label text clicks (select the corresponding wire) and double-clicks (edit label)
    let labels = document.query_selector_all(".wire-label-text").unwrap();
    for i in 0..labels.length() {
        let label = labels.get(i).unwrap();
        let label_el: Element = label.dyn_into().unwrap();
        if !super::dom_bindings::claim(&label_el, "wire-label") {
            continue;
        }
        let label_clone = label_el.clone();
        label_el.set_attribute("tabindex", "0").unwrap();
        label_el.set_attribute("role", "button").unwrap();
        label_el
            .set_attribute(
                "aria-label",
                "Wire predicate; press Enter to inspect or F2 to rename",
            )
            .unwrap();

        // Single click — select the wire and show inspector
        let click_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();

            // Deselect all wires
            let all_paths = doc.query_selector_all(".wire-overlay path").unwrap();
            for j in 0..all_paths.length() {
                let p = all_paths.get(j).unwrap();
                let pe: Element = p.dyn_into().unwrap();
                pe.class_list().remove_1("wire-selected").unwrap();
            }

            let wire_id = label_clone
                .get_attribute("data-wire-id")
                .unwrap_or_default();
            if let Some(svg) = label_clone.parent_element() {
                if let Ok(Some(path_el)) =
                    svg.query_selector(&format!("path[data-id=\"{}\"]", wire_id))
                {
                    path_el.class_list().add_1("wire-selected").unwrap();
                    show_inspector(&doc, &wire_id);
                }
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        label_el
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        // Double click — edit the label inline
        let label_clone2 = label_el.clone();
        let dblclick_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            edit_wire_label(&doc, &label_clone2);
        }) as Box<dyn FnMut(MouseEvent)>);

        label_el
            .add_event_listener_with_callback("dblclick", dblclick_closure.as_ref().unchecked_ref())
            .unwrap();
        dblclick_closure.forget();

        let label_for_key = label_el.clone();
        let key_closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if event.key() == "F2" {
                event.prevent_default();
                edit_wire_label(&doc, &label_for_key);
            } else if event.key() == "Enter" || event.key() == " " {
                event.prevent_default();
                let wire_id = label_for_key
                    .get_attribute("data-wire-id")
                    .unwrap_or_default();
                show_inspector(&doc, &wire_id);
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        label_el
            .add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
            .unwrap();
        key_closure.forget();
    }
}

/// Replace a wire label <text> element with an inline <foreignObject> input
/// for editing. On Enter or blur, commit the new label text.
fn edit_wire_label(document: &Document, label_el: &Element) {
    let current_text = label_el.text_content().unwrap_or_default();
    let y = label_el.get_attribute("y").unwrap_or_default();

    // Get the parent SVG
    let parent = match label_el.parent_element() {
        Some(p) => p,
        None => return,
    };

    // Create a foreignObject with an HTML input
    let fo = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "foreignObject")
        .unwrap();
    fo.set_attribute("x", "0").unwrap();
    fo.set_attribute("y", &format!("{}", y.parse::<f32>().unwrap_or(0.0) - 14.0))
        .unwrap();
    fo.set_attribute("width", "300").unwrap();
    fo.set_attribute("height", "28").unwrap();
    fo.set_attribute("class", "wire-label-editor").unwrap();
    let wire_id = label_el.get_attribute("data-wire-id").unwrap_or_default();
    fo.set_attribute("data-wire-id", &wire_id).unwrap();

    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").unwrap();
    input.set_attribute("value", &current_text).unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.style().set_css_text(
        "width: 100%; padding: 2px 6px; font-size: 11px; font-family: var(--font-mono); \
         background: var(--surface-glass-heavy); border: 1px solid var(--accent-cyan); \
         border-radius: var(--radius-xs); color: var(--text-primary); outline: none;",
    );

    fo.append_child(&input).unwrap();

    // Replace the label with the editor
    let label_clone = label_el.clone();
    if let Some(parent_svg) = parent.dyn_ref::<web_sys::Element>() {
        parent_svg.replace_child(&fo, label_el).unwrap();
    }

    // Focus the input
    let _ = input_el.focus();
    input_el.select();

    // Commit on Enter
    let input_for_enter: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    let fo_for_enter = fo.clone();
    let label_for_enter = label_clone.clone();
    let enter_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" {
            let doc = web_sys::window().unwrap().document().unwrap();
            let new_text = input_for_enter.value();
            commit_label_edit(&doc, &fo_for_enter, &label_for_enter, &new_text);
        } else if e.key() == "Escape" {
            let doc = web_sys::window().unwrap().document().unwrap();
            commit_label_edit(&doc, &fo_for_enter, &label_for_enter, &current_text);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    input
        .add_event_listener_with_callback("keydown", enter_closure.as_ref().unchecked_ref())
        .unwrap();
    enter_closure.forget();

    // Commit on blur
    let input_for_blur: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    let fo_for_blur = fo.clone();
    let label_for_blur = label_clone.clone();
    let blur_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        let new_text = input_for_blur.value();
        commit_label_edit(&doc, &fo_for_blur, &label_for_blur, &new_text);
    }) as Box<dyn FnMut(web_sys::Event)>);
    input
        .add_event_listener_with_callback("blur", blur_closure.as_ref().unchecked_ref())
        .unwrap();
    blur_closure.forget();
}

/// Commit a wire label edit — replace the foreignObject input with a
/// <text> element containing the new label.
fn commit_label_edit(document: &Document, fo: &Element, original_label: &Element, new_text: &str) {
    let parent = match fo.parent_element() {
        Some(p) => p,
        None => return,
    };

    // Create a new <text> element with the updated label
    let text = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "text")
        .unwrap();
    text.set_attribute("x", &original_label.get_attribute("x").unwrap_or_default())
        .unwrap();
    text.set_attribute("y", &original_label.get_attribute("y").unwrap_or_default())
        .unwrap();
    text.set_attribute("class", "wire-label-text").unwrap();
    let wire_id = original_label
        .get_attribute("data-wire-id")
        .unwrap_or_default();
    text.set_attribute("data-wire-id", &wire_id).unwrap();
    text.set_text_content(Some(new_text));

    if let Ok(Some(path)) = parent.query_selector(&format!("path[data-id=\"{}\"]", wire_id)) {
        let _ = path.set_attribute("data-predicate", new_text);
    }

    // Replace the foreignObject with the new text
    parent.replace_child(&text, fo).unwrap();

    // Re-wire the wire inspector for the new label
    wire_wire_inspector(document);

    // Push a history frame
    super::history::push_current_frame("edit wire label");

    // Show notification
    super::interactions::show_tool_notification(document, "wire-label-edit", "Wire label updated");
}

/// Show the rich semantic connection inspector for a wire.
pub fn show_inspector(document: &Document, wire_id: &str) {
    hide_inspector(document);

    // Find the wire path in the DOM
    let path_opt = document
        .query_selector(&format!(".wire-overlay path[data-id=\"{}\"]", wire_id))
        .ok()
        .flatten();

    let (src_id, tgt_id, current_pred, current_modality, _current_weight) =
        if let Some(ref path) = path_opt {
            (
                path.get_attribute("data-source-id")
                    .unwrap_or_else(|| "source".into()),
                path.get_attribute("data-target-id")
                    .unwrap_or_else(|| "target".into()),
                path.get_attribute("data-predicate")
                    .unwrap_or_else(|| "doc:references".into()),
                path.get_attribute("data-modality")
                    .unwrap_or_else(|| "active".into()),
                path.get_attribute("data-weight")
                    .unwrap_or_else(|| "1.0".into()),
            )
        } else {
            (
                "source".into(),
                "target".into(),
                "doc:references".into(),
                "active".into(),
                "1.0".into(),
            )
        };

    let panel = document.create_element("div").unwrap();
    panel.set_id("wire-inspector");
    panel.set_class_name("wire-inspector-panel");
    panel
        .set_attribute("data-selected-modality", &current_modality)
        .unwrap();
    panel.set_attribute("role", "dialog").unwrap();
    panel
        .set_attribute("aria-label", "Semantic wire editor")
        .unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 280px; width: 360px; \
         background: var(--surface-glass-heavy); backdrop-filter: blur(24px); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 16px; box-shadow: 0 12px 40px rgba(0,0,0,0.75); z-index: 600; \
         display: flex; flex-direction: column; gap: 10px; font-family: var(--font-sans);",
    );

    // Header
    let header = document.create_element("div").unwrap();
    header
        .set_attribute(
            "style",
            "display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border-subtle); padding-bottom: 8px;",
        )
        .unwrap();
    let title = document.create_element("span").unwrap();
    title.set_attribute(
        "style",
        "font-size: 11px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    ).unwrap();
    title.set_text_content(Some("\u{1F3F7}\u{FE0F} Semantic Wire Semantics"));
    header.append_child(&title).unwrap();

    let close = document.create_element("button").unwrap();
    close.set_text_content(Some("\u{2715}"));
    let close_el: HtmlElement = close.clone().dyn_into().unwrap();
    close_el.style().set_css_text("background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 14px;");
    header.append_child(&close).unwrap();
    panel.append_child(&header).unwrap();

    // Node endpoints row
    let endpoints_row = document.create_element("div").unwrap();
    endpoints_row.set_attribute("style", "display: flex; align-items: center; gap: 6px; font-family: var(--font-mono); font-size: 11px; background: rgba(0,0,0,0.3); padding: 6px 8px; border-radius: 4px; border: 1px solid var(--border-subtle);").unwrap();

    let src_badge = document.create_element("span").unwrap();
    src_badge
        .set_attribute("style", "color: var(--accent-cyan); font-weight: 600;")
        .unwrap();
    src_badge.set_text_content(Some(&format!("[{}]", src_id)));
    endpoints_row.append_child(&src_badge).unwrap();

    let arrow = document.create_element("span").unwrap();
    arrow
        .set_attribute("style", "color: var(--text-muted);")
        .unwrap();
    arrow.set_text_content(Some(" \u{27F6} "));
    endpoints_row.append_child(&arrow).unwrap();

    let tgt_badge = document.create_element("span").unwrap();
    tgt_badge
        .set_attribute("style", "color: var(--accent-emerald); font-weight: 600;")
        .unwrap();
    tgt_badge.set_text_content(Some(&format!("[{}]", tgt_id)));
    endpoints_row.append_child(&tgt_badge).unwrap();

    panel.append_child(&endpoints_row).unwrap();

    // Semantic Predicate Selector
    let pred_group = document.create_element("div").unwrap();
    pred_group
        .set_attribute("style", "display: flex; flex-direction: column; gap: 4px;")
        .unwrap();

    let pred_label = document.create_element("span").unwrap();
    pred_label.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);").unwrap();
    pred_label.set_text_content(Some("Semantic Predicate (RDF / Vibe Relation)"));
    pred_group.append_child(&pred_label).unwrap();

    let pred_select = document.create_element("select").unwrap();
    pred_select.set_id("wire-predicate-select");
    let ps_el: HtmlElement = pred_select.clone().dyn_into().unwrap();
    ps_el.style().set_css_text("width: 100%; font-family: var(--font-mono); font-size: 11px; background: var(--surface-panel-elevated); border: 1px solid var(--border-medium); border-radius: var(--radius-xs); color: var(--text-primary); padding: 5px 8px; outline: none;");

    let predicates = [
        ("doc:references", "doc:references — Hypermedia Reference"),
        (
            "prov:wasDerivedFrom",
            "prov:wasDerivedFrom — W3C PROV Provenance",
        ),
        (
            "vibe:pipesTo",
            "vibe:pipesTo — Reactive Data Stream Pipeline",
        ),
        (
            "rights:authorizes",
            "rights:authorizes — Permission & Delegated Access",
        ),
        (
            "agency:governs",
            "agency:governs — Normative Governance Rule",
        ),
        ("data:feeds", "data:feeds — Real-time Telemetry Feed"),
        (
            "social:mentions",
            "social:mentions — P2P Social Message Mention",
        ),
        (
            "deontic:obligates",
            "deontic:obligates — Modal Obligation (O)",
        ),
        ("deontic:permits", "deontic:permits — Modal Permission (P)"),
        ("deontic:forbids", "deontic:forbids — Modal Prohibition (F)"),
        (
            "epistemic:knows",
            "epistemic:knows — Epistemic Knowledge (K)",
        ),
        (
            "epistemic:believes",
            "epistemic:believes — Doxastic Belief (B)",
        ),
        ("custom", "Custom URI / Freeform Predicate..."),
    ];

    for (p_val, p_lbl) in &predicates {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", p_val).unwrap();
        opt.set_text_content(Some(p_lbl));
        if *p_val == current_pred
            || (*p_val == "custom" && !predicates.iter().any(|(v, _)| *v == current_pred))
        {
            opt.set_attribute("selected", "selected").unwrap();
        }
        pred_select.append_child(&opt).unwrap();
    }
    pred_group.append_child(&pred_select).unwrap();

    let custom_input = document.create_element("input").unwrap();
    custom_input.set_id("wire-custom-predicate-input");
    custom_input.set_attribute("type", "text").unwrap();
    custom_input
        .set_attribute("placeholder", "e.g. https://schema.org/about or custom:rel")
        .unwrap();
    let is_custom_predicate = !predicates.iter().any(|(value, _)| *value == current_pred);
    custom_input
        .set_attribute(
            "value",
            if is_custom_predicate {
                &current_pred
            } else {
                ""
            },
        )
        .unwrap();
    let ci_el: HtmlElement = custom_input.clone().dyn_into().unwrap();
    ci_el.style().set_css_text("width: 100%; font-family: var(--font-mono); font-size: 11px; background: var(--surface-panel-elevated); border: 1px solid var(--border-medium); border-radius: var(--radius-xs); color: var(--text-primary); padding: 4px 8px; outline: none; margin-top: 2px;");
    if !is_custom_predicate {
        ci_el.style().set_property("display", "none").unwrap();
    }
    pred_group.append_child(&custom_input).unwrap();

    panel.append_child(&pred_group).unwrap();

    // Modality Visual Selector
    let modality_group = document.create_element("div").unwrap();
    modality_group
        .set_attribute("style", "display: flex; flex-direction: column; gap: 4px;")
        .unwrap();
    let mod_label = document.create_element("span").unwrap();
    mod_label.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);").unwrap();
    mod_label.set_text_content(Some("Modality & Wire Type"));
    modality_group.append_child(&mod_label).unwrap();

    let mod_row = document.create_element("div").unwrap();
    mod_row
        .set_attribute("style", "display: flex; gap: 4px; flex-wrap: wrap;")
        .unwrap();

    let modalities = [
        ("active", "Active Stream ⚡", "var(--accent-cyan)"),
        ("event", "Event Pulse 📡", "var(--accent-amber)"),
        ("ontology", "Ontology Triple 📖", "var(--accent-violet)"),
        ("deontic", "Deontic Norm ⚖️", "var(--accent-rose)"),
        ("epistemic", "Epistemic 🧭", "var(--accent-emerald)"),
    ];

    for (m_id, m_name, m_color) in &modalities {
        let m_btn = document.create_element("button").unwrap();
        m_btn.set_class_name(&format!(
            "wire-modality-btn {}",
            if *m_id == current_modality {
                "active"
            } else {
                ""
            }
        ));
        m_btn.set_attribute("data-modality", m_id).unwrap();
        let mb_el: HtmlElement = m_btn.clone().dyn_into().unwrap();
        mb_el.style().set_css_text(&format!(
            "flex: 1; padding: 4px 6px; font-size: 10px; font-family: var(--font-mono); \
             background: {}; border: 1px solid {}; border-radius: 3px; color: {}; cursor: pointer; text-align: center; transition: all 0.15s ease;",
            if *m_id == current_modality { "rgba(255,255,255,0.12)" } else { "var(--surface-panel)" },
            if *m_id == current_modality { m_color } else { "var(--border-subtle)" },
            if *m_id == current_modality { m_color } else { "var(--text-secondary)" }
        ));
        m_btn.set_text_content(Some(m_name));

        let mod_row_clone = mod_row.clone();
        let m_id_str = m_id.to_string();
        let click_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            if let Some(panel) = mod_row_clone.closest("#wire-inspector").ok().flatten() {
                let _ = panel.set_attribute("data-selected-modality", &m_id_str);
            }
            if let Ok(btn_list) = mod_row_clone.query_selector_all(".wire-modality-btn") {
                for k in 0..btn_list.length() {
                    if let Some(b) = btn_list
                        .item(k)
                        .and_then(|n| n.dyn_into::<HtmlElement>().ok())
                    {
                        let is_this =
                            b.get_attribute("data-modality").as_deref() == Some(&m_id_str);
                        let _ = b.class_list().toggle_with_force("active", is_this);
                        let _ = b.style().set_property(
                            "background",
                            if is_this {
                                "rgba(255,255,255,0.12)"
                            } else {
                                "var(--surface-panel)"
                            },
                        );
                        let _ = b.style().set_property(
                            "border-color",
                            if is_this {
                                "var(--accent-cyan)"
                            } else {
                                "var(--border-subtle)"
                            },
                        );
                        let _ = b.style().set_property(
                            "color",
                            if is_this {
                                "var(--accent-cyan)"
                            } else {
                                "var(--text-secondary)"
                            },
                        );
                    }
                }
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        m_btn
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        mod_row.append_child(&m_btn).unwrap();
    }
    modality_group.append_child(&mod_row).unwrap();
    panel.append_child(&modality_group).unwrap();

    // Live RDF Triple Grounding Preview
    let triple_box = document.create_element("div").unwrap();
    triple_box.set_id("wire-triple-preview");
    triple_box.set_attribute("style", "background: rgba(0,0,0,0.4); border: 1px solid var(--border-medium); border-radius: 4px; padding: 8px; font-family: var(--font-mono); font-size: 10px; color: var(--accent-emerald); line-height: 1.4; word-break: break-all;").unwrap();
    triple_box.set_text_content(Some(&format!(
        "<\u{1F517} did:q42:{}> <{}> <\u{1F517} did:q42:{}> .",
        src_id, current_pred, tgt_id
    )));
    panel.append_child(&triple_box).unwrap();

    // Keep custom predicate visibility and the RDF grounding preview live.
    let custom_for_select: web_sys::HtmlInputElement = custom_input.clone().dyn_into().unwrap();
    let src_for_select = src_id.clone();
    let tgt_for_select = tgt_id.clone();
    let select_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Ok(select) = event
            .target()
            .unwrap()
            .dyn_into::<web_sys::HtmlSelectElement>()
        else {
            return;
        };
        let value = select.value();
        let custom = value == "custom";
        let _ = custom_for_select
            .style()
            .set_property("display", if custom { "block" } else { "none" });
        let predicate = if custom {
            custom_for_select.value()
        } else {
            value
        };
        update_triple_preview(&src_for_select, &tgt_for_select, &predicate);
        if custom {
            let _ = custom_for_select.focus();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    pred_select
        .add_event_listener_with_callback("change", select_closure.as_ref().unchecked_ref())
        .unwrap();
    select_closure.forget();

    let src_for_input = src_id.clone();
    let tgt_for_input = tgt_id.clone();
    let custom_for_input: web_sys::HtmlInputElement = custom_input.clone().dyn_into().unwrap();
    let input_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        update_triple_preview(&src_for_input, &tgt_for_input, &custom_for_input.value());
    }) as Box<dyn FnMut(web_sys::Event)>);
    custom_input
        .add_event_listener_with_callback("input", input_closure.as_ref().unchecked_ref())
        .unwrap();
    input_closure.forget();

    // Action Buttons
    let actions = document.create_element("div").unwrap();
    actions
        .set_attribute("style", "display: flex; gap: 6px; padding-top: 4px;")
        .unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_attribute("style", "flex: 2; padding: 6px 12px; background: var(--accent-cyan); color: #07090e; font-weight: 700; font-size: 11px; font-family: var(--font-mono); border: none; border-radius: 4px; cursor: pointer; transition: opacity 0.15s;").unwrap();
    save_btn.set_text_content(Some("💾 Apply Semantics"));

    let wire_id_clone = wire_id.to_string();
    let save_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        let selected_predicate = doc
            .get_element_by_id("wire-predicate-select")
            .and_then(|el| el.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_else(|| "doc:references".into());
        let custom_value = doc
            .get_element_by_id("wire-custom-predicate-input")
            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|i| i.value())
            .unwrap_or_else(|| "doc:references".into());
        let input_val = if selected_predicate == "custom" {
            custom_value.trim().to_string()
        } else {
            selected_predicate
        };
        if input_val.is_empty() {
            super::interactions::show_tool_notification(
                &doc,
                "wire-semantics",
                "Predicate cannot be empty",
            );
            return;
        }
        let modality = doc
            .get_element_by_id("wire-inspector")
            .and_then(|panel| panel.get_attribute("data-selected-modality"))
            .unwrap_or_else(|| "active".into());

        if let Ok(Some(path)) = doc.query_selector(&format!(
            ".wire-overlay path[data-id=\"{}\"]",
            wire_id_clone
        )) {
            let _ = path.set_attribute("data-predicate", &input_val);
            let _ = path.set_attribute("data-modality", &modality);
            let _ = path.set_attribute("class", &format!("wire-{} wire-selected", modality));
        }

        // Update corresponding midpoint label text
        if let Ok(Some(label)) = doc.query_selector(&format!(
            ".wire-label-text[data-wire-id=\"{}\"]",
            wire_id_clone
        )) {
            label.set_text_content(Some(&input_val));
        }

        super::interactions::show_tool_notification(
            &doc,
            "wire-semantics",
            &format!("Semantics applied: {}", input_val),
        );
        super::history::push_current_frame("apply wire semantics");
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    save_btn
        .add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();
    actions.append_child(&save_btn).unwrap();

    let delete_btn = document.create_element("button").unwrap();
    delete_btn.set_attribute("style", "flex: 1; padding: 6px 10px; background: rgba(239, 68, 68, 0.15); border: 1px solid var(--accent-rose); color: var(--accent-rose); font-weight: 600; font-size: 11px; font-family: var(--font-mono); border-radius: 4px; cursor: pointer;").unwrap();
    delete_btn.set_text_content(Some("🗑️ Delete"));

    let wire_id_clone2 = wire_id.to_string();
    let delete_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        if let Ok(Some(path)) = doc.query_selector(&format!(
            ".wire-overlay path[data-id=\"{}\"]",
            wire_id_clone2
        )) {
            super::interactions::delete_wire_element(&doc, &path);
        }
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    delete_btn
        .add_event_listener_with_callback("click", delete_closure.as_ref().unchecked_ref())
        .unwrap();
    delete_closure.forget();
    actions.append_child(&delete_btn).unwrap();

    panel.append_child(&actions).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&panel).unwrap();
    }

    // Close button click
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
}

fn update_triple_preview(source: &str, target: &str, predicate: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(preview) = document.get_element_by_id("wire-triple-preview") {
        preview.set_text_content(Some(&format!(
            "<🔗 did:q42:{}> <{}> <🔗 did:q42:{}> .",
            source, predicate, target
        )));
    }
}

/// Show a real semantic editor for a container resource.
pub fn show_semantic_dialog_for_container(document: &Document, container_id: &str) {
    hide_inspector(document);
    let Ok(Some(container)) = document.query_selector(&format!(
        ".canvas-container-node[data-id=\"{}\"]",
        container_id
    )) else {
        return;
    };

    let current_type = container
        .get_attribute("data-semantic-type")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            format!(
                "poet:{}",
                container
                    .get_attribute("data-container-type")
                    .unwrap_or_else(|| "Container".into())
            )
        });
    let current_uri = container
        .get_attribute("data-semantic-uri")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("did:q42:{}", container_id));

    let panel = document.create_element("div").unwrap();
    panel.set_id("wire-inspector");
    panel.set_class_name("wire-inspector-panel container-semantics-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel
        .set_attribute("aria-label", "Container semantic identity editor")
        .unwrap();
    let panel_html: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_html.style().set_css_text(
        "position:fixed;bottom:40px;right:280px;width:360px;background:var(--surface-glass-heavy);\
         border:1px solid var(--border-medium);border-radius:var(--radius-sm);padding:16px;\
         box-shadow:0 12px 40px rgba(0,0,0,.75);z-index:600;display:flex;flex-direction:column;gap:10px;"
    );

    let title = document.create_element("strong").unwrap();
    title.set_text_content(Some("🏷️ Container Semantic Identity"));
    title
        .set_attribute(
            "style",
            "font:700 11px var(--font-mono);color:var(--accent-cyan);text-transform:uppercase",
        )
        .unwrap();
    panel.append_child(&title).unwrap();

    let id_badge = document.create_element("code").unwrap();
    id_badge.set_text_content(Some(container_id));
    id_badge
        .set_attribute(
            "style",
            "font-size:10px;color:var(--text-muted);word-break:break-all",
        )
        .unwrap();
    panel.append_child(&id_badge).unwrap();

    let type_input = labelled_input(
        document,
        "Semantic class",
        "container-semantic-type",
        &current_type,
    );
    let uri_input = labelled_input(
        document,
        "Resource URI / DID",
        "container-semantic-uri",
        &current_uri,
    );
    panel.append_child(&type_input).unwrap();
    panel.append_child(&uri_input).unwrap();

    let preview = document.create_element("code").unwrap();
    preview.set_id("container-semantic-preview");
    preview.set_text_content(Some(&format!("<{}> a <{}> .", current_uri, current_type)));
    preview.set_attribute("style", "padding:8px;background:rgba(0,0,0,.4);color:var(--accent-emerald);font-size:10px;word-break:break-all").unwrap();
    panel.append_child(&preview).unwrap();

    for input_id in ["container-semantic-type", "container-semantic-uri"] {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let Some(doc) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let semantic_type = input_value(&doc, "container-semantic-type");
            let uri = input_value(&doc, "container-semantic-uri");
            if let Some(preview) = doc.get_element_by_id("container-semantic-preview") {
                preview.set_text_content(Some(&format!("<{}> a <{}> .", uri, semantic_type)));
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        if let Some(input) = document.get_element_by_id(input_id) {
            input
                .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
    }

    let actions = document.create_element("div").unwrap();
    actions
        .set_attribute("style", "display:flex;gap:6px")
        .unwrap();
    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Apply identity"));
    save.set_attribute("style", "flex:1;padding:7px;border:0;border-radius:4px;background:var(--accent-cyan);color:#071018;font-weight:700;cursor:pointer").unwrap();
    let container_for_save = container.clone();
    let save_closure = Closure::wrap(Box::new(move |_event: MouseEvent| {
        let Some(doc) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let semantic_type = input_value(&doc, "container-semantic-type");
        let uri = input_value(&doc, "container-semantic-uri");
        if semantic_type.trim().is_empty() || uri.trim().is_empty() {
            super::interactions::show_tool_notification(
                &doc,
                "container-semantics",
                "Class and URI are required",
            );
            return;
        }
        let _ = container_for_save.set_attribute("data-semantic-type", semantic_type.trim());
        let _ = container_for_save.set_attribute("data-semantic-uri", uri.trim());
        super::history::push_current_frame("apply container semantics");
        super::interactions::show_tool_notification(
            &doc,
            "container-semantics",
            "Container semantic identity updated",
        );
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    save.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();
    actions.append_child(&save).unwrap();

    let close = document.create_element("button").unwrap();
    close.set_text_content(Some("Cancel"));
    close.set_attribute("style", "padding:7px 12px;border:1px solid var(--border-medium);border-radius:4px;background:transparent;color:var(--text-secondary);cursor:pointer").unwrap();
    let close_closure = Closure::wrap(Box::new(move |_event: MouseEvent| {
        let Some(doc) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        hide_inspector(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    close
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    actions.append_child(&close).unwrap();
    panel.append_child(&actions).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&panel).unwrap();
    }
}

fn labelled_input(document: &Document, label: &str, id: &str, value: &str) -> Element {
    let group = document.create_element("label").unwrap();
    group.set_attribute("style", "display:flex;flex-direction:column;gap:4px;font:700 10px var(--font-mono);color:var(--text-muted);text-transform:uppercase").unwrap();
    group.set_text_content(Some(label));
    let input = document.create_element("input").unwrap();
    input.set_id(id);
    input.set_attribute("value", value).unwrap();
    input.set_attribute("style", "padding:6px 8px;background:var(--surface-panel-elevated);border:1px solid var(--border-medium);border-radius:4px;color:var(--text-primary);font:11px var(--font-mono);text-transform:none").unwrap();
    group.append_child(&input).unwrap();
    group
}

fn input_value(document: &Document, id: &str) -> String {
    document
        .get_element_by_id(id)
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn hide_inspector(document: &Document) {
    if let Some(existing) = document.get_element_by_id("wire-inspector") {
        existing.remove();
    }
}

/// Public hide — removes the wire inspector panel if present.
pub fn hide() {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            hide_inspector(&doc);
        }
    }
}
