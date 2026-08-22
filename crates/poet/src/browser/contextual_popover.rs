//! Contextual RDF & CML Popover Engine.
//!
//! Provides text-selection annotation with 12 CML categories, certainty scoring,
//! `<q-entity>` and `<q-relation>` tag generation, and interactive hover inspector
//! cards linked to personal ontology IRIs.
//!
//! Aligned with `06_HYPERMEDIA_LIBRARY_FOUNDATION_SPEC.md` and `ontologies/document.n3`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

use super::cml_document::CmlCategory;

/// Wire the contextual CML popover and entity hover inspectors on all contenteditable editors.
pub fn wire_contextual_popover(document: &Document) {
    let editors = document
        .query_selector_all(".doc-editor[contenteditable=\"true\"]")
        .unwrap();
    for i in 0..editors.length() {
        let editor = editors.get(i).unwrap();
        let editor_el: Element = editor.dyn_into().unwrap();

        // Mouseup selection handler
        let closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            let window = web_sys::window().unwrap();
            let doc = window.document().unwrap();
            let sel = match window.get_selection() {
                Ok(Some(s)) => s,
                _ => return,
            };

            // Check if there's a non-empty text selection
            let text: String = sel.to_string().as_string().unwrap_or_default();
            if text.trim().is_empty() {
                hide_popover(&doc);
                return;
            }

            // Check that the selection is within this contenteditable
            let anchor_node = match sel.anchor_node() {
                Some(n) => n,
                None => return,
            };
            let anchor_el = match anchor_node.parent_element() {
                Some(e) => e,
                None => return,
            };
            if !anchor_el.closest(".doc-editor").unwrap().is_some() {
                hide_popover(&doc);
                return;
            }

            // Position the popover near the mouse pointer
            show_popover(&doc, e.client_x() as i32, e.client_y() as i32, &text);
        }) as Box<dyn FnMut(MouseEvent)>);

        editor_el
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire hover inspector for existing <q-entity> tags
    wire_entity_hover_inspectors(document);

    // Hide popover when clicking outside
    let doc_clone = document.clone();
    let dismiss_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
        let target: Element = match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            Some(t) => t,
            None => return,
        };
        // Don't hide if clicking inside the popover or inspector
        if target.closest("#rdf-popover").unwrap().is_some() 
            || target.closest("#cml-entity-inspector").unwrap().is_some() {
            return;
        }
        hide_popover(&doc_clone);
        hide_inspector(&doc_clone);
    }) as Box<dyn FnMut(MouseEvent)>);
    document
        .add_event_listener_with_callback("mousedown", dismiss_closure.as_ref().unchecked_ref())
        .unwrap();
    dismiss_closure.forget();
}

/// Wire hover inspector popup for all `<q-entity>` tags.
pub fn wire_entity_hover_inspectors(document: &Document) {
    let entities = document.query_selector_all("q-entity").unwrap();
    for i in 0..entities.length() {
        let entity = entities.get(i).unwrap();
        let entity_el: Element = entity.dyn_into().unwrap();
        let entity_el_clone = entity_el.clone();

        let hover_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let cat_code = entity_el_clone.get_attribute("data-category")
                .or_else(|| entity_el_clone.get_attribute("data-entity-type"))
                .unwrap_or_else(|| "entity".into());
            let iri = entity_el_clone.get_attribute("data-iri")
                .unwrap_or_else(|| "did:qualia:entity:unresolved".into());
            let certainty = entity_el_clone.get_attribute("data-certainty")
                .unwrap_or_else(|| "95".into());
            let prov = entity_el_clone.get_attribute("data-provenance")
                .unwrap_or_else(|| "Gazetteer:System".into());
            let label = entity_el_clone.text_content().unwrap_or_default();

            show_entity_inspector(&doc, e.client_x() as i32, e.client_y() as i32, 
                &cat_code, &iri, &certainty, &prov, &label, &entity_el_clone);
        }) as Box<dyn FnMut(MouseEvent)>);

        entity_el
            .add_event_listener_with_callback("mouseenter", hover_closure.as_ref().unchecked_ref())
            .unwrap();
        hover_closure.forget();
    }
}

/// Show the CML annotation popover at the given coordinates.
fn show_popover(document: &Document, x: i32, y: i32, selected_text: &str) {
    hide_popover(document);
    hide_inspector(document);

    let popover = document.create_element("div").unwrap();
    popover.set_id("rdf-popover");
    let pop_el: HtmlElement = popover.clone().dyn_into().unwrap();

    let px = x + 10;
    let py = y - 10;
    pop_el.style().set_css_text(&format!(
        "position: fixed; left: {}px; top: {}px; \
         background: var(--surface-glass-heavy); backdrop-filter: blur(24px); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         box-shadow: var(--shadow-lg); z-index: 8500; \
         display: flex; flex-direction: column; \
         min-width: 240px; max-width: 320px; overflow: hidden; \
         transform: translateY(-100%); font-family: var(--font-sans);",
        px, py
    ));

    // Header
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "padding: 8px 12px; font-size: 10px; font-weight: 700; \
         color: var(--accent-cyan); text-transform: uppercase; \
         letter-spacing: 0.6px; border-bottom: 1px solid var(--border-subtle); \
         background: var(--surface-panel); font-family: var(--font-mono); \
         display: flex; align-items: center; justify-content: space-between;",
    );
    header.set_text_content(Some("\u{1F50D} CML Context Markup"));
    
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "font-size: 9px; padding: 2px 6px; background: rgba(0,210,255,0.12); \
         color: var(--accent-cyan); border-radius: 4px; border: 1px solid rgba(0,210,255,0.25);",
    );
    chip.set_text_content(Some("W3C RDF 1.2"));
    header.append_child(&chip).unwrap();
    popover.append_child(&header).unwrap();

    // Selected text preview
    let preview = document.create_element("div").unwrap();
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 6px 12px; font-size: 11px; color: var(--text-secondary); \
         font-style: italic; border-bottom: 1px solid var(--border-subtle); \
         max-height: 44px; overflow: hidden; font-family: var(--font-mono);",
    );
    let preview_text = if selected_text.len() > 50 {
        format!("{}...", &selected_text[..50])
    } else {
        selected_text.to_string()
    };
    preview.set_text_content(Some(&format!("\u{201C}{}\u{2026}\u{201D}", preview_text.trim())));
    popover.append_child(&preview).unwrap();

    // Category Grid container
    let cat_container = document.create_element("div").unwrap();
    let cat_el: HtmlElement = cat_container.clone().dyn_into().unwrap();
    cat_el.style().set_css_text(
        "display: grid; grid-template-columns: 1fr 1fr; gap: 4px; padding: 8px; \
         max-height: 200px; overflow-y: auto;",
    );

    for cat in CmlCategory::all() {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("cml-popover-cat-btn");
        btn.set_attribute("data-cml-cat", cat.code()).unwrap();
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 6px 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); color: var(--text-secondary); font-size: 10px; \
             cursor: pointer; display: flex; align-items: center; gap: 6px; text-align: left; \
             transition: var(--trans-fast); font-family: var(--font-mono);",
        );

        let ic = document.create_element("span").unwrap();
        ic.set_text_content(Some(cat.glyph()));
        let ic_el: HtmlElement = ic.clone().dyn_into().unwrap();
        ic_el.style().set_css_text("font-size: 11px; width: 14px; text-align: center;");
        btn.append_child(&ic).unwrap();

        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(cat.label()));
        let lbl_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lbl_el.style().set_css_text("overflow: hidden; text-overflow: ellipsis; white-space: nowrap;");
        btn.append_child(&lbl).unwrap();

        // Wire click to annotate
        let cat_code = cat.code().to_string();
        let cat_label = cat.label().to_string();
        let click_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            annotate_selection(&doc, &cat_code, &cat_label);
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        cat_container.append_child(&btn).unwrap();
    }
    popover.append_child(&cat_container).unwrap();

    // Footer note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "padding: 6px 10px; font-size: 9px; color: var(--text-muted); \
         border-top: 1px solid var(--border-subtle); background: var(--surface-base); \
         font-family: var(--font-mono); display: flex; justify-content: space-between;",
    );
    note.set_text_content(Some("48-Byte Super-Quin Grounded"));
    popover.append_child(&note).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&popover).unwrap();
    }
}

/// Show hover inspector card for an existing `<q-entity>` tag.
fn show_entity_inspector(
    document: &Document,
    x: i32,
    y: i32,
    cat_code: &str,
    iri: &str,
    certainty: &str,
    provenance: &str,
    label: &str,
    entity_element: &Element,
) {
    hide_inspector(document);

    let inspector = document.create_element("div").unwrap();
    inspector.set_id("cml-entity-inspector");
    let ins_el: HtmlElement = inspector.clone().dyn_into().unwrap();

    let cat_obj = CmlCategory::from_code(cat_code).unwrap_or(CmlCategory::NamedEntity);

    let px = x + 12;
    let py = y + 12;
    ins_el.style().set_css_text(&format!(
        "position: fixed; left: {}px; top: {}px; \
         background: var(--surface-panel-elevated); backdrop-filter: blur(20px); \
         border: 1px solid var(--border-medium); border-left: 3px solid {}; \
         border-radius: var(--radius-sm); box-shadow: var(--shadow-lg); z-index: 8600; \
         padding: 10px 14px; min-width: 220px; max-width: 320px; font-family: var(--font-sans); \
         display: flex; flex-direction: column; gap: 6px; font-size: 11px;",
        px, py, cat_obj.color_accent()
    ));

    // Header badge
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
    );
    
    let cat_badge = document.create_element("span").unwrap();
    let badge_el: HtmlElement = cat_badge.clone().dyn_into().unwrap();
    badge_el.style().set_css_text(&format!(
        "padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.06); \
         color: {}; font-weight: 700; font-family: var(--font-mono); font-size: 10px;",
        cat_obj.color_accent()
    ));
    cat_badge.set_text_content(Some(&format!("{} {}", cat_obj.glyph(), cat_obj.label())));
    header.append_child(&cat_badge).unwrap();

    let cert_badge = document.create_element("span").unwrap();
    let cert_el: HtmlElement = cert_badge.clone().dyn_into().unwrap();
    cert_el.style().set_css_text(
        "color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px;",
    );
    cert_badge.set_text_content(Some(&format!("{}% Cert", certainty)));
    header.append_child(&cert_badge).unwrap();
    inspector.append_child(&header).unwrap();

    // Entity Label
    let name_div = document.create_element("div").unwrap();
    let name_el: HtmlElement = name_div.clone().dyn_into().unwrap();
    name_el.style().set_css_text("font-weight: 600; color: var(--text-primary); font-size: 12px;");
    name_div.set_text_content(Some(label.trim()));
    inspector.append_child(&name_div).unwrap();

    // IRI link
    let iri_div = document.create_element("div").unwrap();
    let iri_el: HtmlElement = iri_div.clone().dyn_into().unwrap();
    iri_el.style().set_css_text(
        "color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; \
         word-break: break-all; background: var(--surface-base); padding: 4px 6px; \
         border-radius: 3px; border: 1px solid var(--border-subtle);",
    );
    iri_div.set_text_content(Some(iri));
    inspector.append_child(&iri_div).unwrap();

    // Provenance
    let prov_div = document.create_element("div").unwrap();
    let prov_el: HtmlElement = prov_div.clone().dyn_into().unwrap();
    prov_el.style().set_css_text("color: var(--text-muted); font-size: 9px; font-family: var(--font-mono);");
    prov_div.set_text_content(Some(&format!("Source: {}", provenance)));
    inspector.append_child(&prov_div).unwrap();

    // Action button row
    let btn_row = document.create_element("div").unwrap();
    let row_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    row_el.style().set_css_text(
        "display: flex; gap: 6px; margin-top: 4px; padding-top: 6px; \
         border-top: 1px solid var(--border-subtle);",
    );

    let remove_btn = document.create_element("button").unwrap();
    let r_el: HtmlElement = remove_btn.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; padding: 4px 8px; background: rgba(239,68,68,0.12); border: 1px solid rgba(239,68,68,0.25); \
         border-radius: 4px; color: var(--accent-rose); font-size: 9px; font-family: var(--font-mono); \
         cursor: pointer;",
    );
    remove_btn.set_text_content(Some("\u{1F5D1} Remove"));

    let target_el = entity_element.clone();
    let remove_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let parent = match target_el.parent_node() {
            Some(p) => p,
            None => return,
        };
        let text = target_el.text_content().unwrap_or_default();
        let doc = web_sys::window().unwrap().document().unwrap();
        let text_node = doc.create_text_node(&text);
        parent.replace_child(&text_node, &target_el).unwrap();
        hide_inspector(&doc);
        super::history::push_current_frame("remove entity tag");
    }) as Box<dyn FnMut(MouseEvent)>);
    remove_btn.add_event_listener_with_callback("click", remove_closure.as_ref().unchecked_ref())
        .unwrap();
    remove_closure.forget();
    btn_row.append_child(&remove_btn).unwrap();

    inspector.append_child(&btn_row).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&inspector).unwrap();
    }
}

/// Annotate the current selection with a `<q-entity>` custom element.
fn annotate_selection(document: &Document, cat_code: &str, cat_label: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let sel = match window.get_selection() {
        Ok(Some(s)) => s,
        _ => return,
    };

    if sel.range_count() == 0 {
        return;
    }

    let range = match sel.get_range_at(0) {
        Ok(r) => r,
        Err(_) => return,
    };

    let selected_text = sel.to_string().as_string().unwrap_or_default();
    let slug = selected_text.trim().to_lowercase().replace(' ', "_");
    let auto_iri = format!("did:qualia:entity:{}", slug);

    let cat_obj = CmlCategory::from_code(cat_code).unwrap_or(CmlCategory::NamedEntity);

    // Create the q-entity wrapper element
    let q_entity = document.create_element("q-entity").unwrap();
    q_entity.set_class_name("cml-entity-tag");
    q_entity.set_attribute("data-category", cat_code).unwrap();
    q_entity.set_attribute("data-entity-type", cat_code).unwrap();
    q_entity.set_attribute("data-iri", &auto_iri).unwrap();
    q_entity.set_attribute("data-certainty", "95").unwrap();
    q_entity.set_attribute("data-provenance", "CML:AuthorManual").unwrap();
    q_entity.set_attribute("title", &format!("{}: {}", cat_label, auto_iri)).unwrap();

    let q_el: HtmlElement = q_entity.clone().dyn_into().unwrap();
    q_el.style().set_css_text(&format!(
        "border-bottom: 2px solid {}; cursor: pointer; \
         background: rgba(255,255,255,0.04); border-radius: 3px; padding: 0 4px; \
         display: inline-flex; align-items: center; gap: 3px; font-weight: 500;",
        cat_obj.color_accent()
    ));

    // Surround the selection contents with the q-entity element
    if range.surround_contents(&q_entity).is_err() {
        return;
    }

    // Append small category glyph badge
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("cml-entity-badge");
    let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
    b_el.style().set_css_text(&format!(
        "font-size: 9px; padding: 1px 3px; border-radius: 2px; \
         background: {}; color: #000; font-weight: 700; line-height: 1;",
        cat_obj.color_accent()
    ));
    badge.set_text_content(Some(cat_obj.glyph()));
    q_entity.append_child(&badge).unwrap();

    // Clear the selection
    sel.remove_all_ranges().unwrap();

    // Hide the popovers
    hide_popover(document);

    // Re-wire hover inspectors for new entity
    wire_entity_hover_inspectors(document);

    // Push a history frame
    super::history::push_current_frame("annotate cml entity");
}

/// Hide the CML popover if present.
fn hide_popover(document: &Document) {
    if let Some(existing) = document.get_element_by_id("rdf-popover") {
        existing.remove();
    }
}

/// Hide the entity inspector card if present.
fn hide_inspector(document: &Document) {
    if let Some(existing) = document.get_element_by_id("cml-entity-inspector") {
        existing.remove();
    }
}
