//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Left Tool Chest dock: family groups, 4-way anchors, and quick spawn.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::toolbox::Toolbox;

// ---------------------------------------------------------------------------
// Dock builder with 4-Way Docking Architecture
// ---------------------------------------------------------------------------

/// Build the toolbox dock from a populated registry with 4-way docking anchor controls.
pub fn build_toolbox_dock(document: &Document, toolboxes: &[Toolbox]) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("toolbox-dock dock-pos-left");
    crate::browser::surface_aspects::mark(&dock, "entrance");

    // Dock Header: Brand + 4-Way Docking Anchor Bar
    let dock_header = document.create_element("div").unwrap();
    dock_header.set_class_name("dock-master-header");
    let dh_el: HtmlElement = dock_header.clone().dyn_into().unwrap();
    dh_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );

    let title_span = document.create_element("span").unwrap();
    let ts_el: HtmlElement = title_span.clone().dyn_into().unwrap();
    ts_el.style().set_css_text(
        "font-size: 9px; font-weight: 700; color: var(--accent-cyan); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title_span.set_text_content(Some("\u{1F9F0} Tool Chest"));
    dock_header.append_child(&title_span).unwrap();

    // 4-Way Dock Anchor Controls
    let anchor_bar = document.create_element("div").unwrap();
    anchor_bar.set_class_name("dock-anchor-bar");
    let ab_el: HtmlElement = anchor_bar.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("display: flex; gap: 2px;");

    let positions = [
        ("left", "\u{25C0}"),
        ("top", "\u{25B2}"),
        ("right", "\u{25B6}"),
        ("bottom", "\u{25BC}"),
    ];

    for (pos_id, glyph) in &positions {
        let pos_btn = document.create_element("button").unwrap();
        pos_btn.set_class_name("dock-pos-btn");
        pos_btn.set_attribute("data-pos", pos_id).unwrap();
        pos_btn
            .set_attribute("title", &format!("Dock {}", pos_id))
            .unwrap();
        pos_btn
            .set_attribute("aria-label", &format!("Dock Tool Chest {}", pos_id))
            .unwrap();
        pos_btn
            .set_attribute(
                "aria-pressed",
                if *pos_id == "left" { "true" } else { "false" },
            )
            .unwrap();
        let pb_el: HtmlElement = pos_btn.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "padding: 1px 3px; font-size: 8px; background: transparent; border: 1px solid transparent; \
             border-radius: 2px; color: var(--text-muted); cursor: pointer; transition: var(--trans-fast);",
        );
        pos_btn.set_text_content(Some(glyph));

        let pos_str = pos_id.to_string();
        let pos_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    crate::browser::interactions::apply_toolbox_position(&doc, &pos_str);
                }
            }

            if let Some(storage) = web_sys::window()
                .and_then(|w| w.local_storage().ok())
                .flatten()
            {
                let _ = storage.set_item("qualia_dock_pos", &pos_str);
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        pos_btn
            .add_event_listener_with_callback("click", pos_closure.as_ref().unchecked_ref())
            .unwrap();
        pos_closure.forget();

        anchor_bar.append_child(&pos_btn).unwrap();
    }
    dock_header.append_child(&anchor_bar).unwrap();
    dock.append_child(&dock_header).unwrap();
    dock.append_child(&crate::browser::tool_proficiency::render_switcher(document))
        .unwrap();
    crate::browser::tool_proficiency::restore(document);

    // Quick Spawn Tiles in Tool Chest
    let quick_grid = document.create_element("div").unwrap();
    quick_grid.set_class_name("dock-quick-grid");
    let qg_el: HtmlElement = quick_grid.clone().dyn_into().unwrap();
    qg_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; padding: 4px 6px; \
         border-bottom: 1px solid var(--border-subtle); margin-bottom: 6px;",
    );

    let quick_containers = [
        ("doc", "📄 Doc"),
        ("sheet", "📊 Sheet"),
        ("code", "💻 Script"),
        ("anatomy", "🫀 Anatomy"),
        ("dual_studio", "Studio"),
        ("audio_session", "Audio session"),
        ("3d", "🧊 3D Scene"),
        ("social", "💬 Social"),
        ("agent_console", "🤖 Local help"),
        ("integrations", "🔌 Connectors"),
        ("webrtc", "📹 Swarm"),
        ("finance", "💰 Finance"),
    ];

    for (c_type, c_lbl) in &quick_containers {
        let q_btn = document.create_element("button").unwrap();
        q_btn.set_class_name("dock-quick-spawn-btn");
        let qb_el: HtmlElement = q_btn.clone().dyn_into().unwrap();
        qb_el.style().set_css_text(
            "display: flex; align-items: center; justify-content: center; gap: 4px; \
             padding: 4px 2px; font-size: 10px; font-family: var(--font-mono); font-weight: 600; \
             background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: 4px; color: var(--text-secondary); cursor: pointer; transition: all 0.15s ease;",
        );
        q_btn.set_text_content(Some(c_lbl));

        let c_type_str = c_type.to_string();
        let c_lbl_str = c_lbl.to_string();
        let click_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    crate::browser::interactions::place_container_via_menu(
                        &doc,
                        &c_type_str,
                        &format!("+ {}", c_lbl_str),
                    );
                }
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        q_btn
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        quick_grid.append_child(&q_btn).unwrap();
    }
    dock.append_child(&quick_grid).unwrap();

    let families = super::model::family_order();
    let mut first_toolbox = true;

    for family in &families {
        // Find toolboxes in this family
        let family_toolboxes: Vec<&Toolbox> = toolboxes
            .iter()
            .filter(|tb| tb.metadata().family == family.id)
            .collect();

        if family_toolboxes.is_empty() {
            continue;
        }

        // Family section
        let section = document.create_element("div").unwrap();
        section.set_class_name("dock-family-section");
        section.set_attribute("data-family", &family.id).unwrap();

        // Family header (collapsible)
        let header = document.create_element("button").unwrap();
        header.set_class_name("dock-family-header");
        header.set_attribute("data-family", &family.id).unwrap();
        header.set_attribute("title", &family.label).unwrap();
        header
            .set_attribute(
                "aria-expanded",
                if first_toolbox { "true" } else { "false" },
            )
            .unwrap();

        let family_icon = document.create_element("span").unwrap();
        family_icon.set_class_name("dock-family-icon");
        family_icon.set_text_content(Some(&family.icon));
        header.append_child(&family_icon).unwrap();

        let family_label = document.create_element("span").unwrap();
        family_label.set_class_name("dock-family-label");
        family_label.set_text_content(Some(&family.label));
        header.append_child(&family_label).unwrap();

        let chevron = document.create_element("span").unwrap();
        chevron.set_class_name("dock-family-chevron");
        chevron.set_text_content(Some("\u{25BE}"));
        header.append_child(&chevron).unwrap();

        section.append_child(&header).unwrap();

        // Toolbox buttons (children, shown by default for first family)
        let children = document.create_element("div").unwrap();
        children.set_class_name("dock-family-children");
        if first_toolbox {
            children.class_list().add_1("expanded").unwrap();
        }

        for toolbox in &family_toolboxes {
            let meta = toolbox.metadata();
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("toolbox-dock-btn");
            if first_toolbox {
                first_toolbox = false;
            }
            btn.set_attribute("data-toolbox", &meta.id).unwrap();
            btn.set_attribute("aria-label", &meta.label).unwrap();
            btn.set_text_content(Some(super::glyphs::toolbox_glyph(&meta.id)));

            let tooltip = document.create_element("span").unwrap();
            tooltip.set_class_name("dock-tooltip");
            tooltip.set_text_content(Some(&meta.label));
            btn.append_child(&tooltip).unwrap();

            children.append_child(&btn).unwrap();
        }

        section.append_child(&children).unwrap();
        dock.append_child(&section).unwrap();
    }

    dock
}
