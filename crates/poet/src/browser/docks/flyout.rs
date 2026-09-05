//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Toolbox flyout: tool-chains, widgets, and gated tool buttons.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::Document;

// ---------------------------------------------------------------------------
// Flyout panel — shows tool-chains and tools for the active toolbox
// ---------------------------------------------------------------------------

/// Show or replace the flyout panel for the given toolbox id.
/// Removes any existing flyout first.
pub fn show_flyout(document: &Document, toolbox_id: &str) {
    // Remove existing flyout
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }

    let view = super::model::find_stored_toolbox_view(toolbox_id);

    let view = match view {
        Some(v) => v,
        None => return,
    };

    let curr_pos = if let Ok(Some(dock_el)) = document.query_selector(".toolbox-dock") {
        if dock_el.class_list().contains("dock-pos-top") {
            "top"
        } else if dock_el.class_list().contains("dock-pos-right") {
            "right"
        } else if dock_el.class_list().contains("dock-pos-bottom") {
            "bottom"
        } else {
            "left"
        }
    } else {
        "left"
    };

    let flyout = document.create_element("div").unwrap();
    flyout.set_class_name(&format!("toolbox-flyout dock-{}", curr_pos));
    flyout.set_attribute("data-toolbox-id", toolbox_id).ok();
    crate::browser::surface_aspects::mark(&flyout, "entrance");

    // Header: Icon + Title + Ontology Badge + Close button
    let header = document.create_element("div").unwrap();
    header.set_class_name("toolbox-flyout-header");

    let header_left = document.create_element("div").unwrap();
    header_left.set_class_name("flyout-header-left");

    let tb_icon = document.create_element("span").unwrap();
    tb_icon.set_class_name("flyout-header-icon");
    tb_icon.set_text_content(Some(super::glyphs::toolbox_glyph(&view.metadata.id)));
    header_left.append_child(&tb_icon).unwrap();

    let title_wrap = document.create_element("div").unwrap();
    title_wrap.set_class_name("flyout-title-wrap");

    let title = document.create_element("div").unwrap();
    title.set_class_name("flyout-title-text");
    title.set_text_content(Some(&view.metadata.label));
    title_wrap.append_child(&title).unwrap();

    let desc = document.create_element("div").unwrap();
    desc.set_class_name("flyout-desc-text");
    desc.set_text_content(Some(&view.metadata.description));
    title_wrap.append_child(&desc).unwrap();

    header_left.append_child(&title_wrap).unwrap();
    header.append_child(&header_left).unwrap();

    let header_right = document.create_element("div").unwrap();
    header_right.set_class_name("flyout-header-right");

    let ont_badge = document.create_element("span").unwrap();
    ont_badge.set_class_name("flyout-ont-badge");
    ont_badge.set_text_content(Some(&format!("{}:", view.metadata.ontology_prefix)));
    header_right.append_child(&ont_badge).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_class_name("flyout-close-btn");
    close_btn.set_attribute("title", "Close Drawer").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));

    let close_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        hide_flyout(&doc);
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();

    header_right.append_child(&close_btn).unwrap();
    header.append_child(&header_right).unwrap();

    flyout.append_child(&header).unwrap();

    // Tool-chains & Interactive Domain Controls
    let chains_scroll = document.create_element("div").unwrap();
    chains_scroll.set_class_name("toolbox-flyout-body");

    for chain in &view.chains {
        let group = document.create_element("div").unwrap();
        group.set_class_name("toolchain-group");

        let chain_header = document.create_element("div").unwrap();
        chain_header.set_class_name("toolchain-label");
        chain_header
            .set_attribute("data-chain-id", &chain.metadata.id)
            .unwrap();
        chain_header
            .set_attribute("data-toolbox-id", &view.metadata.id)
            .unwrap();
        chain_header.set_attribute("draggable", "true").unwrap();
        chain_header
            .set_attribute(
                "title",
                "Click to activate on focused surface, or drag onto a container",
            )
            .unwrap();

        let chain_icon = document.create_element("span").unwrap();
        chain_icon.set_class_name("toolchain-label-icon");
        chain_icon.set_text_content(Some("\u{2630}"));
        chain_header.append_child(&chain_icon).unwrap();

        let chain_text = document.create_element("span").unwrap();
        chain_text.set_class_name("toolchain-label-text");
        chain_text.set_text_content(Some(&chain.metadata.label));
        chain_header.append_child(&chain_text).unwrap();

        if !chain.metadata.description.is_empty() {
            let chain_hint = document.create_element("span").unwrap();
            chain_hint.set_class_name("toolchain-label-hint");
            chain_hint.set_text_content(Some(&chain.metadata.description));
            chain_header.append_child(&chain_hint).unwrap();
        }

        group.append_child(&chain_header).unwrap();

        // Widgets container
        let widgets_box = document.create_element("div").unwrap();
        widgets_box.set_class_name("toolchain-widgets-container");

        if !chain.widgets.is_empty() {
            for widget in &chain.widgets {
                widgets_box.append_child(&widget.render(document)).unwrap();
            }
        } else {
            // Fallback for tools without rich widgets
            for tool in &chain.tools {
                let btn = document.create_element("button").unwrap();
                btn.set_class_name("tool-btn");
                btn.set_attribute("data-tool-id", &tool.id).unwrap();
                btn.set_attribute("data-chain-id", &chain.metadata.id)
                    .unwrap();
                btn.set_attribute("data-action", &tool.action.to_string())
                    .unwrap();
                if crate::browser::tool_actions::requires_daemon(&tool.id) {
                    btn.set_attribute("data-requires-daemon", "true").unwrap();
                }
                let gated = crate::browser::tool_actions::current_disabled_reason(&tool.id);
                if gated.is_some() {
                    btn.set_attribute("disabled", "").unwrap();
                    btn.set_attribute("aria-disabled", "true").unwrap();
                    if let Some(reason) = gated {
                        btn.set_attribute("data-disabled-reason", reason).unwrap();
                    }
                }
                let copy = crate::browser::tool_copy::decorate(
                    &btn,
                    &tool.id,
                    &tool.label,
                    &tool.description,
                    tool.capability_scope.as_deref(),
                    gated,
                );

                let icon_el = document.create_element("span").unwrap();
                icon_el.set_class_name("tool-btn-icon");
                icon_el.set_text_content(Some(super::glyphs::tool_glyph(&tool.icon)));
                btn.append_child(&icon_el).unwrap();

                let label_el = document.create_element("span").unwrap();
                label_el.set_class_name("tool-btn-label");
                label_el.set_text_content(Some(&copy.label));
                btn.append_child(&label_el).unwrap();

                let kind_el = document.create_element("span").unwrap();
                kind_el.set_class_name("tool-btn-kind");
                kind_el.set_text_content(Some(super::glyphs::kind_label(tool.kind)));
                btn.append_child(&kind_el).unwrap();

                widgets_box.append_child(&btn).unwrap();
            }
        }

        group.append_child(&widgets_box).unwrap();
        chains_scroll.append_child(&group).unwrap();
    }

    flyout.append_child(&chains_scroll).unwrap();

    // Append to the workspace (so it positions relative to the dock)
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace.append_child(&flyout).unwrap();
    } else if let Some(body) = document.body() {
        body.append_child(&flyout).unwrap();
    }
}

/// Hide the flyout panel.
pub fn hide_flyout(document: &Document) {
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }
}
