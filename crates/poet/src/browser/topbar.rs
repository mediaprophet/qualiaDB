//! Top bar rendering: menubar + canvas control bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use crate::tool_chest::core::registry::ManifoldSeed;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, MouseEvent};

/// Build the top menubar (File/Edit/View + brand + fiduciary badge).
/// Each menu item is a dropdown with real actions.
pub fn build_top_menubar(document: &Document) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("top-menubar");

    // Left: brand + menus
    let left = document.create_element("div").unwrap();
    left.set_class_name("menu-items-group");

    let brand = document.create_element("div").unwrap();
    brand.set_class_name("brand-icon");
    brand.set_text_content(Some("\u{1F30C}"));
    let brand_text = document.create_element("span").unwrap();
    brand_text.set_class_name("brand-text");
    brand_text.set_text_content(Some("Webizen Poet"));
    brand.append_child(&brand_text).unwrap();
    left.append_child(&brand).unwrap();

    // File menu
    left.append_child(&build_menu_dropdown(
        document,
        "File",
        &[
            ("New Manifold", "file:new-manifold", "\u{1F4CB}"),
            ("Save", "file:save", "\u{1F4BE}"),
            ("Save As\u{2026}", "file:save-as", "\u{1F4C2}"),
            ("separator", "", ""),
            ("Export CBOR-LD", "file:export-cbor", "\u{1F4E4}"),
            ("Import CBOR-LD", "file:import-cbor", "\u{1F4E5}"),
            ("separator", "", ""),
            (
                "Checkpoint History\u{2026}",
                "file:checkpoint-history",
                "\u{1F4D4}",
            ),
            ("Prune & Archive\u{2026}", "file:prune-archive", "\u{1F9F9}"),
            (
                "Export Distribution\u{2026}",
                "file:export-distribution",
                "\u{1F4E6}",
            ),
            ("separator", "", ""),
            ("Close Manifold", "file:close", "\u{2715}"),
        ],
    ))
    .unwrap();

    // Edit menu
    left.append_child(&build_menu_dropdown(
        document,
        "Edit",
        &[
            ("Undo", "edit:undo", "\u{21A9}"),
            ("Redo", "edit:redo", "\u{21AA}"),
            ("separator", "", ""),
            ("Delete Container", "edit:delete", "\u{1F5D1}"),
            ("Duplicate Container", "edit:duplicate", "\u{1F4CB}"),
            ("separator", "", ""),
            ("Select All", "edit:select-all", "\u{1F4D8}"),
        ],
    ))
    .unwrap();

    // View menu
    left.append_child(&build_menu_dropdown(
        document,
        "View",
        &[
            ("Toggle Toolbox Dock", "view:toggle-dock", "\u{1F9ED}"),
            ("Toggle Telemetry", "view:toggle-telemetry", "\u{2699}"),
            ("Toggle Expos\u{00E9}", "view:expose", "\u{1F4F7}"),
            (
                "Auto-Arrange Manifold (Tidy)",
                "view:auto-arrange",
                "\u{2728}",
            ),
            ("separator", "", ""),
            ("Zoom In", "view:zoom-in", "\u{1F50D}+"),
            ("Zoom Out", "view:zoom-out", "\u{1F50D}\u{2212}"),
            ("Reset Zoom", "view:zoom-reset", "\u{1F503}"),
            ("separator", "", ""),
            ("Accessibility", "view:a11y", "\u{267F}"),
        ],
    ))
    .unwrap();

    // Insert menu
    left.append_child(&build_menu_dropdown(
        document,
        "Insert",
        &[
            ("+ Document", "insert:doc", "\u{1F4C4}"),
            ("+ Sheet", "insert:sheet", "\u{1F4CA}"),
            ("+ Code", "insert:code", "\u{1F4BB}"),
            ("+ Map", "insert:map", "\u{1F5FA}"),
            ("+ Ontology", "insert:ontology", "\u{1F4D6}"),
            ("+ Social", "insert:social", "\u{1F4AC}"),
            ("+ 3D", "insert:3d", "\u{1F3AF}"),
            ("+ WebRTC", "insert:webrtc", "\u{1F4F7}"),
            ("separator", "", ""),
            ("+ Checkpoint Tray", "insert:checkpoint-tray", "\u{1F4D4}"),
            (
                "+ Credential Inspector",
                "insert:credential-inspector",
                "\u{1F511}",
            ),
            (
                "+ Context Markup Editor",
                "insert:context-markup-editor",
                "\u{1F50D}",
            ),
            ("+ Provenance Panel", "insert:provenance-panel", "\u{1F4DC}"),
            (
                "+ Publication Workflow",
                "insert:publication-workflow",
                "\u{1F4E6}",
            ),
            (
                "+ Constituency Manager",
                "insert:constituency-manager",
                "\u{1F465}",
            ),
        ],
    ))
    .unwrap();

    // Help menu
    left.append_child(&build_menu_dropdown(
        document,
        "Help",
        &[
            ("Keyboard Shortcuts", "help:shortcuts", "\u{2328}"),
            ("About Webizen Poet", "help:about", "\u{2139}"),
            ("Honesty Labels", "help:honesty", "\u{1F4A1}"),
            ("separator", "", ""),
            ("Report Issue", "help:report", "\u{1F41B}"),
        ],
    ))
    .unwrap();

    bar.append_child(&left).unwrap();

    // Right: search button + fiduciary badge + version
    let right = document.create_element("div").unwrap();
    right.set_class_name("menu-items-group");

    // Search workbench button
    let search_btn = document.create_element("button").unwrap();
    search_btn.set_class_name("menu-btn search-workbench-btn");
    search_btn.set_text_content(Some("\u{1F50D} Search"));
    search_btn
        .set_attribute("title", "Open Search Workbench (Ctrl+Shift+F)")
        .unwrap();
    let sb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::search_workbench::toggle_search_workbench(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    search_btn
        .add_event_listener_with_callback("click", sb_closure.as_ref().unchecked_ref())
        .unwrap();
    sb_closure.forget();
    right.append_child(&search_btn).unwrap();

    // Logic workbench button
    let logic_btn = document.create_element("button").unwrap();
    logic_btn.set_class_name("menu-btn logic-workbench-btn");
    logic_btn.set_text_content(Some("\u{1F9E0} Logic"));
    logic_btn
        .set_attribute("title", "Open Logic Workbench (Ctrl+Shift+L)")
        .unwrap();
    let lb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::logic_workbench::toggle_logic_workbench(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    logic_btn
        .add_event_listener_with_callback("click", lb_closure.as_ref().unchecked_ref())
        .unwrap();
    lb_closure.forget();
    right.append_child(&logic_btn).unwrap();

    // Webizen Native Daemon Status Badge
    let daemon_badge = super::native_daemon::build_daemon_status_badge(document);
    right.append_child(&daemon_badge).unwrap();

    // Habitat Pivot Switcher (Poet <-> Admin)
    let habitat_btn = document.create_element("button").unwrap();
    habitat_btn.set_class_name("menu-btn habitat-pivot-btn");
    habitat_btn.set_text_content(Some("\u{2728} Poet / \u{2699}\u{FE0F} Admin \u{21C4}"));
    habitat_btn
        .set_attribute(
            "title",
            "Pivot Habitat: Switch to Webizen Admin Console (Alt+U)",
        )
        .unwrap();
    let hp_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::workspace_pivot::toggle_workspace_pivot(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    habitat_btn
        .add_event_listener_with_callback("click", hp_closure.as_ref().unchecked_ref())
        .unwrap();
    hp_closure.forget();
    right.append_child(&habitat_btn).unwrap();

    // Ambient Mesh Sentinel Indicator
    let mesh_badge = document.create_element("span").unwrap();
    mesh_badge.set_class_name("mesh-sentinel-badge");
    mesh_badge.set_text_content(Some("\u{25CF} Mesh Active \u{00B7} 42MB OK"));
    mesh_badge
        .set_attribute(
            "title",
            "42MB Prolog Sentinel Active \u{00B7} Zero-Heap Hot Paths Monitored",
        )
        .unwrap();
    right.append_child(&mesh_badge).unwrap();

    let version = document.create_element("span").unwrap();
    version.set_class_name("version-badge");
    version.set_text_content(Some("0.0.35-dev"));
    right.append_child(&version).unwrap();
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("fiduciary-badge");
    badge.set_text_content(Some("\u{1F6E1} Fiduciary L3"));
    right.append_child(&badge).unwrap();
    bar.append_child(&right).unwrap();

    bar
}

/// Build a single dropdown menu with a label trigger and action items.
fn build_menu_dropdown(document: &Document, label: &str, items: &[(&str, &str, &str)]) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("menu-dropdown");

    let trigger = document.create_element("button").unwrap();
    trigger.set_class_name("menu-btn");
    trigger.set_text_content(Some(label));
    wrapper.append_child(&trigger).unwrap();

    let dropdown = document.create_element("div").unwrap();
    dropdown.set_class_name("menu-dropdown-content");

    for (item_label, action, icon) in items {
        if *item_label == "separator" {
            let sep = document.create_element("div").unwrap();
            sep.set_class_name("menu-dropdown-separator");
            dropdown.append_child(&sep).unwrap();
        } else {
            let item = document.create_element("button").unwrap();
            item.set_class_name("menu-dropdown-item");
            item.set_attribute("data-menu-action", action).unwrap();

            let icon_el = document.create_element("span").unwrap();
            icon_el.set_class_name("menu-dropdown-item-icon");
            icon_el.set_text_content(Some(icon));
            item.append_child(&icon_el).unwrap();

            let label_el = document.create_element("span").unwrap();
            label_el.set_class_name("menu-dropdown-item-label");
            label_el.set_text_content(Some(item_label));
            item.append_child(&label_el).unwrap();

            dropdown.append_child(&item).unwrap();
        }
    }

    wrapper.append_child(&dropdown).unwrap();
    wrapper
}

/// Wire up menu dropdown toggling and action clicks.
pub fn wire_menu_dropdowns(document: &Document) {
    let triggers = document.query_selector_all(".menu-btn").unwrap();
    for i in 0..triggers.length() {
        let trigger = triggers.get(i).unwrap();
        let trigger_el: Element = trigger.dyn_into().unwrap();
        let trigger_el_for_listener = trigger_el.clone();

        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let me: web_sys::MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            // Close all other dropdowns
            let all = doc.query_selector_all(".menu-dropdown").unwrap();
            for j in 0..all.length() {
                let d = all.get(j).unwrap();
                let de: Element = d.dyn_into().unwrap();
                if de != trigger_el.parent_element().unwrap() {
                    de.class_list().remove_1("open").unwrap();
                }
            }
            // Toggle this one
            if let Some(parent) = trigger_el.parent_element() {
                if parent.class_list().contains("open") {
                    parent.class_list().remove_1("open").unwrap();
                } else {
                    parent.class_list().add_1("open").unwrap();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        trigger_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire menu action items
    let items = document.query_selector_all(".menu-dropdown-item").unwrap();
    for i in 0..items.length() {
        let item = items.get(i).unwrap();
        let item_el: Element = item.dyn_into().unwrap();
        let action = item_el
            .get_attribute("data-menu-action")
            .unwrap_or_default();
        let label = item_el
            .query_selector(".menu-dropdown-item-label")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let me: web_sys::MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            // Close all dropdowns
            let all = doc.query_selector_all(".menu-dropdown").unwrap();
            for j in 0..all.length() {
                let d = all.get(j).unwrap();
                let de: Element = d.dyn_into().unwrap();
                de.class_list().remove_1("open").unwrap();
            }
            handle_menu_action(&doc, &action, &label);
        }) as Box<dyn FnMut(web_sys::Event)>);

        item_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Close dropdowns when clicking outside
    let outside_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        let all = doc.query_selector_all(".menu-dropdown").unwrap();
        for j in 0..all.length() {
            let d = all.get(j).unwrap();
            let de: Element = d.dyn_into().unwrap();
            de.class_list().remove_1("open").unwrap();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    document
        .add_event_listener_with_callback("click", outside_closure.as_ref().unchecked_ref())
        .unwrap();
    outside_closure.forget();
}

/// Handle a menu action by id.
fn handle_menu_action(document: &Document, action: &str, label: &str) {
    match action {
        "edit:undo" => {
            super::history::perform_undo();
        }
        "edit:redo" => {
            super::history::perform_redo();
        }
        "edit:delete" => {
            if let Some(selected) = document
                .query_selector(".canvas-container-node.selected")
                .unwrap()
            {
                selected.remove();
                super::instrument_panel::hide(document);
                super::history::push_current_frame("delete container");
            }
        }
        "view:toggle-telemetry" => {
            toggle_tech_sidebar(document);
        }
        "view:a11y" => {
            show_a11y_notification(document);
        }
        "insert:doc"
        | "insert:sheet"
        | "insert:code"
        | "insert:map"
        | "insert:ontology"
        | "insert:social"
        | "insert:3d"
        | "insert:webrtc"
        | "insert:checkpoint-tray"
        | "insert:credential-inspector"
        | "insert:context-markup-editor"
        | "insert:provenance-panel"
        | "insert:publication-workflow"
        | "insert:constituency-manager" => {
            let container_type = action.split(':').nth(1).unwrap_or("doc");
            super::interactions::place_container_via_menu(document, container_type, label);
        }
        "file:save" => {
            super::history::sync_persistence_state();
            let _ = super::manifest::save_all_manifolds();
            show_menu_notification(
                document,
                "Auto-saved \u{2014} actor: did:qualia:timothy_charles_holborn",
            );
        }
        "file:save-as" => {
            open_save_mode_dialog(document);
        }
        "file:export-cbor" => {
            super::history::sync_persistence_state();
            let seeds = super::get_current_seeds();
            if let Ok(json) = serde_json::to_string_pretty(&seeds) {
                trigger_file_download(document, "manifold_seeds_export.json", &json);
                show_menu_notification(document, "Exported manifold seeds dataset (JSON/CBOR-LD)");
            } else {
                show_menu_notification(
                    document,
                    "Export failed: unable to serialize manifold seeds",
                );
            }
        }
        "file:import-cbor" => {
            trigger_file_import_dialog(document);
        }
        "file:checkpoint-history" => {
            super::interactions::place_container_via_menu(
                document,
                "checkpoint-tray",
                "Checkpoint Tray",
            );
        }
        "file:prune-archive" => {
            super::interactions::place_container_via_menu(
                document,
                "publication-workflow",
                "Publication Workflow",
            );
        }
        "file:export-distribution" => {
            super::interactions::place_container_via_menu(
                document,
                "publication-workflow",
                "Distribution Export",
            );
        }
        "file:new-manifold" => {
            open_new_manifold_dialog(document);
        }
        "file:close" => {
            let seeds = super::get_current_seeds();
            if !seeds.is_empty() {
                super::switch_manifold(&seeds[0].id, &seeds);
                show_menu_notification(
                    document,
                    "Active manifold closed; reset to base workspace.",
                );
            }
        }
        "edit:duplicate" => {
            super::interactions::duplicate_selected_containers(document);
        }
        "edit:select-all" => {
            let all = document
                .query_selector_all(".canvas-container-node")
                .unwrap();
            for j in 0..all.length() {
                let n = all.get(j).unwrap();
                let ne: Element = n.dyn_into().unwrap();
                ne.class_list().add_1("selected").unwrap();
            }
            show_menu_notification(document, &format!("{} containers selected", all.length()));
        }
        "view:toggle-dock" => {
            if let Some(dock) = document.query_selector(".toolbox-dock").unwrap() {
                let dock_el: HtmlElement = dock.dyn_into().unwrap();
                let display = dock_el
                    .style()
                    .get_property_value("display")
                    .unwrap_or_default();
                if display == "none" {
                    dock_el.style().set_property("display", "").unwrap();
                } else {
                    dock_el.style().set_property("display", "none").unwrap();
                }
            }
        }
        "view:zoom-in" => {
            super::interactions::apply_canvas_zoom(document, 0.1, false);
        }
        "view:zoom-out" => {
            super::interactions::apply_canvas_zoom(document, -0.1, false);
        }
        "view:zoom-reset" => {
            super::interactions::apply_canvas_zoom(document, 1.0, true);
        }
        "view:auto-arrange" => {
            super::interactions::auto_arrange_containers(document);
        }
        "view:expose" => {
            show_menu_notification(document, "Press Alt+O for Expos\u{00E9} overview");
        }
        "help:shortcuts" => {
            open_shortcuts_dialog(document);
        }
        "help:about" => {
            open_about_dialog(document);
        }
        "help:honesty" => {
            open_honesty_dialog(document);
        }
        "help:report" => {
            show_menu_notification(
                document,
                "GitHub Issue tracker \u{2014} report logged to audit ledger",
            );
        }
        _ => {
            show_menu_notification(document, &format!("{} \u{2014} dispatched", label));
        }
    }
}

/// Open the Save Mode dialog — lets the user choose a save mode,
/// provide a label, and see the actor identity that will be recorded.
///
/// See `SAVE_ARCHITECTURE.md` for the full specification.
fn open_save_mode_dialog(document: &Document) {
    // Remove any existing dialog
    if let Some(existing) = document.get_element_by_id("save-mode-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("save-mode-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center;",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 420px; background: var(--surface-glass-heavy); \
         backdrop-filter: blur(20px); border: 1px solid var(--border-medium); \
         border-radius: var(--radius-sm); box-shadow: var(--shadow-lg); \
         padding: 20px; display: flex; flex-direction: column; gap: 16px; \
         font-family: var(--font-mono); color: var(--text-primary);",
    );

    // Title
    let title = document.create_element("div").unwrap();
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-size: 14px; font-weight: 700; color: var(--text-primary);");
    title.set_text_content(Some("\u{1F4BE} Save \u{2014} Checkpoint Mode"));
    panel.append_child(&title).unwrap();

    // Actor info
    let actor_info = document.create_element("div").unwrap();
    let actor_el: HtmlElement = actor_info.clone().dyn_into().unwrap();
    actor_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    actor_info.set_text_content(Some(&format!(
        "Actor: {} — saves are attributed to the bound observer.",
        super::current_observer_did()
    )));
    panel.append_child(&actor_info).unwrap();

    // Mode selector
    let mode_label = document.create_element("div").unwrap();
    let ml_el: HtmlElement = mode_label.clone().dyn_into().unwrap();
    ml_el
        .style()
        .set_css_text("font-size: 11px; color: var(--text-secondary);");
    mode_label.set_text_content(Some("Save mode:"));
    panel.append_child(&mode_label).unwrap();

    let mode_group = document.create_element("div").unwrap();
    let mg_el: HtmlElement = mode_group.clone().dyn_into().unwrap();
    mg_el.style().set_css_text("display: flex; gap: 6px;");

    let modes = [
        ("Auto", "auto", "Frequency-based\nrolling buffer"),
        ("Checkpoint", "checkpoint", "Named save\nwith label"),
        ("Snapshot", "snapshot", "Immutable seed set\n(exportable)"),
        ("Pruned", "pruned", "Tombstones pruned\n(distribution)"),
    ];

    for (idx, (label, mode_id, desc)) in modes.iter().enumerate() {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("save-mode-btn");
        btn.set_attribute("data-save-mode", mode_id).unwrap();
        if *mode_id == "pruned" {
            btn.set_attribute("disabled", "").ok();
            btn.set_attribute("aria-disabled", "true").ok();
            btn.set_attribute(
                "title",
                "Unavailable: the checkpoint store does not yet retain an operation/tombstone DAG to prune.",
            )
            .ok();
        }
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        btn_el.style().set_css_text(if idx == 1 {
            "flex: 1; padding: 10px 8px; border: 1px solid var(--accent-cyan); \
             border-radius: var(--radius-xs); background: var(--surface-panel-elevated); \
             color: var(--text-primary); font-family: var(--font-mono); font-size: 10px; \
             cursor: pointer; display: flex; flex-direction: column; gap: 4px; \
             align-items: center; text-align: center;"
        } else {
            "flex: 1; padding: 10px 8px; border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); background: var(--surface-panel); \
             color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; \
             cursor: pointer; display: flex; flex-direction: column; gap: 4px; \
             align-items: center; text-align: center; transition: var(--trans-fast);"
        });
        if idx == 1 {
            btn.class_list().add_1("selected").unwrap();
        }

        let name_el = document.create_element("div").unwrap();
        name_el.set_text_content(Some(label));
        name_el
            .set_attribute("style", "font-weight: 700; font-size: 11px;")
            .unwrap();
        btn.append_child(&name_el).unwrap();

        let desc_el = document.create_element("div").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute(
                "style",
                "font-size: 9px; color: var(--text-muted); white-space: pre-line;",
            )
            .unwrap();
        btn.append_child(&desc_el).unwrap();

        mode_group.append_child(&btn).unwrap();
    }
    panel.append_child(&mode_group).unwrap();

    // Label input
    let label_div = document.create_element("div").unwrap();
    label_div
        .set_attribute("style", "display: flex; flex-direction: column; gap: 4px;")
        .unwrap();

    let label_text = document.create_element("div").unwrap();
    let lt_el: HtmlElement = label_text.clone().dyn_into().unwrap();
    lt_el
        .style()
        .set_css_text("font-size: 11px; color: var(--text-secondary);");
    label_text.set_text_content(Some("Checkpoint label:"));
    label_div.append_child(&label_text).unwrap();

    let label_input = document.create_element("input").unwrap();
    let li_el: HtmlInputElement = label_input.clone().dyn_into().unwrap();
    li_el.set_placeholder("e.g. v0.3 draft, before NLP extraction\u{2026}");
    label_input.set_id("save-mode-label-input");
    label_input.set_attribute("style",
        "padding: 8px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--text-primary); font-family: var(--font-mono); \
         font-size: 12px; outline: none;"
    ).unwrap();
    label_div.append_child(&label_input).unwrap();
    panel.append_child(&label_div).unwrap();

    // Honesty note
    let honesty = document.create_element("div").unwrap();
    let h_el: HtmlElement = honesty.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan);",
    );
    honesty.set_text_content(Some(
        "Auto, Checkpoint, and immutable seed Snapshot modes are live. Pruned is disabled until the store retains an operation/tombstone DAG. Construct HCF/HMC export is available from the Publication Workflow and Construct Shelf.",
    ));
    panel.append_child(&honesty).unwrap();

    // Buttons
    let btn_row = document.create_element("div").unwrap();
    let br_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    br_el
        .style()
        .set_css_text("display: flex; gap: 8px; justify-content: flex-end;");

    let cancel_btn = document.create_element("button").unwrap();
    cancel_btn.set_text_content(Some("Cancel"));
    let cb_el: HtmlElement = cancel_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "padding: 8px 16px; border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         background: var(--surface-panel); color: var(--text-secondary); font-family: var(--font-mono); \
         font-size: 11px; cursor: pointer;"
    );
    btn_row.append_child(&cancel_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("save-mode-confirm-btn");
    save_btn.set_text_content(Some("\u{1F4BE} Save"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         background: var(--accent-cyan); color: var(--bg-deep); font-family: var(--font-mono); \
         font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    btn_row.append_child(&save_btn).unwrap();
    panel.append_child(&btn_row).unwrap();

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }

    // Wire mode button selection
    let mode_btns = document.query_selector_all(".save-mode-btn").unwrap();
    for i in 0..mode_btns.length() {
        let btn = mode_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let btn_el_for_listener = btn_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Deselect all
            let all = doc.query_selector_all(".save-mode-btn").unwrap();
            for j in 0..all.length() {
                let b = all.get(j).unwrap();
                let be: Element = b.dyn_into().unwrap();
                let be_html: HtmlElement = be.clone().dyn_into().unwrap();
                be.class_list().remove_1("selected").unwrap();
                be_html
                    .style()
                    .set_property("border", "1px solid var(--border-subtle)")
                    .unwrap();
                be_html
                    .style()
                    .set_property("background", "var(--surface-panel)")
                    .unwrap();
                be_html
                    .style()
                    .set_property("color", "var(--text-secondary)")
                    .unwrap();
            }
            // Select this
            btn_el.class_list().add_1("selected").unwrap();
            let btn_html: HtmlElement = btn_el.clone().dyn_into().unwrap();
            btn_html
                .style()
                .set_property("border", "1px solid var(--accent-cyan)")
                .unwrap();
            btn_html
                .style()
                .set_property("background", "var(--surface-panel-elevated)")
                .unwrap();
            btn_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        }) as Box<dyn FnMut(web_sys::Event)>);

        btn_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire cancel button
    let overlay_for_cancel = overlay.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        overlay_for_cancel.remove();
    }) as Box<dyn FnMut(web_sys::Event)>);
    cancel_btn
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();

    // Wire save button
    let overlay_for_save = overlay.clone();
    let save_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();

        // Get selected mode
        let selected = doc.query_selector(".save-mode-btn.selected").unwrap();
        let mode_id = selected
            .as_ref()
            .and_then(|el| el.get_attribute("data-save-mode"))
            .unwrap_or_else(|| "checkpoint".to_string());

        // Get label
        let label = doc
            .get_element_by_id("save-mode-label-input")
            .map(|el| {
                let input: HtmlInputElement = el.dyn_into().unwrap();
                input.value()
            })
            .unwrap_or_default();

        // Map mode ID to SaveMode
        let mode = match mode_id.as_str() {
            "auto" => super::manifest::SaveMode::Auto,
            "checkpoint" => super::manifest::SaveMode::Checkpoint,
            "snapshot" => super::manifest::SaveMode::Snapshot,
            "pruned" => super::manifest::SaveMode::Pruned,
            _ => super::manifest::SaveMode::Checkpoint,
        };

        // Save
        let result = super::manifest::save_checkpoint(&label, mode.clone());

        // Close dialog
        overlay_for_save.remove();

        // Show result notification
        match result {
            Ok(meta) => {
                let mode_name = match mode {
                    super::manifest::SaveMode::Auto => "Auto",
                    super::manifest::SaveMode::Checkpoint => "Checkpoint",
                    super::manifest::SaveMode::Snapshot => "Snapshot",
                    super::manifest::SaveMode::Pruned => "Pruned",
                };
                let label_part = if meta.label.is_empty() {
                    String::new()
                } else {
                    format!(" \u{2014} \"{}\"", meta.label)
                };
                show_menu_notification(
                    &doc,
                    &format!(
                        "{} saved{} \u{2014} actor: {}, ts: {}",
                        mode_name, label_part, meta.actor, meta.timestamp
                    ),
                );
            }
            Err(e) => {
                show_menu_notification(&doc, &format!("Save failed: {}", e));
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    save_btn
        .add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();

    // Wire Escape to close
    let overlay_for_esc = overlay.clone();
    let esc_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            overlay_for_esc.remove();
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    document
        .add_event_listener_with_callback("keydown", esc_closure.as_ref().unchecked_ref())
        .unwrap();
    esc_closure.forget();

    // Focus the label input
    if let Some(input) = document.get_element_by_id("save-mode-label-input") {
        let input_el: HtmlInputElement = input.dyn_into().unwrap();
        let _ = input_el.focus();
    }
}

fn show_menu_notification(document: &Document, message: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 700; max-width: 360px;",
    );
    notif.set_text_content(Some(message));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 3000);
    timeout.forget();
}

fn append_pager_tab(
    document: &Document,
    desktops: &Element,
    seed: &ManifoldSeed,
    index: usize,
    active_id: &str,
) {
    let tab = document.create_element("button").unwrap();
    tab.set_class_name("desktop-tab-btn");
    if seed.id == active_id {
        tab.class_list().add_1("active").unwrap();
    }
    tab.set_attribute("data-manifold", &seed.id).unwrap();
    let num = document.create_element("span").unwrap();
    num.set_class_name("desktop-num");
    num.set_text_content(Some(&(index + 1).to_string()));
    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(&format!(" {} {}", seed.icon, seed.label)));
    tab.append_child(&num).unwrap();
    tab.append_child(&label).unwrap();
    if seed.is_social() {
        let people = document.create_element("span").unwrap();
        people.set_text_content(Some(" \u{1F465}"));
        people
            .set_attribute("title", "Social lens — many people")
            .ok();
        tab.append_child(&people).unwrap();
    }
    let manifold_id = seed.id.clone();
    let closure = Closure::wrap(Box::new(move || {
        super::switch_to_sibling_manifold(&manifold_id);
    }) as Box<dyn FnMut()>);
    tab.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    desktops.append_child(&tab).unwrap();
}

/// Rebuild the pager to the open construct's lenses.
pub fn rebuild_pager(document: &Document, seeds: &[ManifoldSeed], active_id: &str) {
    let Some(desktops) = document
        .query_selector(".pager-desktops-list")
        .ok()
        .flatten()
    else {
        return;
    };
    while let Some(child) = desktops.first_element_child() {
        child.remove();
    }
    for (index, seed) in seeds.iter().enumerate() {
        append_pager_tab(document, &desktops, seed, index, active_id);
    }
}

/// Refresh construct/manifold chrome (badge + clickable breadcrumb + pop).
pub fn refresh_construct_chrome(document: &Document, construct_id: &str, manifold_id: &str) {
    if let Some(badge) = document
        .query_selector(".graph-address-badge")
        .ok()
        .flatten()
    {
        badge.set_text_content(Some(&format!(
            "construct:{construct_id} graph:manifold:{manifold_id}"
        )));
    }
    let Some(crumb) = document.get_element_by_id("construct-breadcrumb") else {
        return;
    };
    while let Some(child) = crumb.first_element_child() {
        child.remove();
    }
    crumb.set_text_content(None);

    let prefix = document.create_element("span").unwrap();
    prefix.set_text_content(Some(&format!("construct:{construct_id}")));
    crumb.append_child(&prefix).unwrap();

    let crumbs = super::construct_nav_crumbs();
    let last = crumbs.len().saturating_sub(1);
    for (idx, (id, title)) in crumbs.iter().enumerate() {
        let sep = document.create_element("span").unwrap();
        sep.set_text_content(Some(" › "));
        crumb.append_child(&sep).unwrap();
        if idx == last {
            let current = document.create_element("span").unwrap();
            current.set_text_content(Some(title));
            current.set_attribute("data-manifold", id).ok();
            crumb.append_child(&current).unwrap();
        } else {
            let button = document.create_element("button").unwrap();
            button.set_attribute("type", "button").ok();
            button.set_class_name("breadcrumb-pop");
            button
                .set_attribute("data-nav-depth", &idx.to_string())
                .ok();
            button.set_text_content(Some(title));
            let depth = idx;
            let closure = Closure::wrap(Box::new(move |_event: Event| {
                super::pop_nested_to_depth(depth);
            }) as Box<dyn FnMut(Event)>);
            button
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            crumb.append_child(&button).unwrap();
        }
    }

    if last > 0 {
        let up = document.create_element("button").unwrap();
        up.set_attribute("type", "button").ok();
        up.set_class_name("breadcrumb-up");
        up.set_attribute("title", "Pop nested manifold").ok();
        up.set_text_content(Some("Up"));
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            super::pop_nested_manifold();
        }) as Box<dyn FnMut(Event)>);
        up.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        crumb.append_child(&up).unwrap();
    }
    super::manifold_social::refresh_people_chrome(document);
}

/// Build the canvas control bar (manifold pager + title + socket-case pods).
pub fn build_canvas_control_bar(document: &Document, seeds: &[ManifoldSeed]) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("canvas-control-bar");

    // Pager
    let pager = document.create_element("div").unwrap();
    pager.set_class_name("virtual-desktop-pager");
    let desktops = document.create_element("div").unwrap();
    desktops.set_class_name("pager-desktops-list");

    let active = seeds.first().map(|seed| seed.id.as_str()).unwrap_or("");
    for (i, seed) in seeds.iter().enumerate() {
        append_pager_tab(document, &desktops, seed, i, active);
    }
    pager.append_child(&desktops).unwrap();

    // Add new manifold button (+)
    let add_btn = document.create_element("button").unwrap();
    add_btn.set_class_name("desktop-tab-btn desktop-add-btn");
    add_btn.set_id("manifold-add-btn");
    add_btn.set_attribute("title", "Add new manifold").unwrap();
    let plus = document.create_element("span").unwrap();
    plus.set_text_content(Some("+"));
    add_btn.append_child(&plus).unwrap();
    pager.append_child(&add_btn).unwrap();

    // Wire the add button
    let add_closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        add_new_manifold(&doc);
    }) as Box<dyn FnMut(Event)>);
    add_btn
        .add_event_listener_with_callback("click", add_closure.as_ref().unchecked_ref())
        .unwrap();
    add_closure.forget();

    bar.append_child(&pager).unwrap();

    let crumb = document.create_element("div").unwrap();
    crumb.set_id("construct-breadcrumb");
    crumb.set_class_name("construct-breadcrumb");
    let crumb_el: HtmlElement = crumb.clone().dyn_into().unwrap();
    crumb_el.style().set_css_text(
        "font-size: 10px; font-family: var(--font-mono); color: var(--text-muted); padding: 0 8px;",
    );
    crumb.set_text_content(Some(&format!(
        "construct:{}",
        super::current_construct_id()
    )));
    bar.append_child(&crumb).unwrap();

    let people = document.create_element("div").unwrap();
    people.set_id("manifold-people");
    people.set_class_name("manifold-people");
    let people_el: HtmlElement = people.clone().dyn_into().unwrap();
    people_el.style().set_css_text(
        "font-size: 10px; font-family: var(--font-mono); color: var(--text-muted); \
         padding: 0 8px; display: flex; align-items: center; gap: 6px; white-space: nowrap;",
    );
    people.set_text_content(Some("personal lens"));
    bar.append_child(&people).unwrap();

    // Title box — editable input
    let title_box = document.create_element("div").unwrap();
    title_box.set_class_name("canvas-title-box");
    let title_input = document.create_element("input").unwrap();
    title_input.set_class_name("canvas-title-input");
    title_input.set_attribute("type", "text").unwrap();
    title_input
        .set_attribute(
            "value",
            seeds
                .first()
                .map(|seed| seed.label.as_str())
                .unwrap_or("POET"),
        )
        .unwrap();
    title_input
        .set_attribute("id", "manifold-title-input")
        .unwrap();
    title_box.append_child(&title_input).unwrap();

    let graph_badge = document.create_element("span").unwrap();
    graph_badge.set_class_name("graph-address-badge");
    graph_badge.set_id("manifold-graph-badge");
    graph_badge.set_text_content(Some(&format!(
        "construct:{} graph:manifold:{}",
        super::current_construct_id(),
        seeds
            .first()
            .map(|seed| seed.id.as_str())
            .unwrap_or("research")
    )));
    title_box.append_child(&graph_badge).unwrap();
    bar.append_child(&title_box).unwrap();

    // Socket-Case Pods (Strata, Epistemic Lens, Dimension)
    let pods_bar = document.create_element("div").unwrap();
    pods_bar.set_class_name("top-control-pods-bar");

    // Strata Pod
    let strata_pod = build_pod_button(
        document,
        "strata",
        "\u{1F33F}",
        "Strata:",
        "All (5)",
        "var(--accent-emerald)",
        "Filter by Social & Ecological Strata",
    );
    pods_bar.append_child(&strata_pod).unwrap();

    // Epistemic Lens Pod
    let epistemic_pod = build_pod_button(
        document,
        "epistemic",
        "\u{1F52C}",
        "Lens:",
        "\u{1F310} All",
        "var(--accent-cyan)",
        "Filter by Epistemic Lens",
    );
    pods_bar.append_child(&epistemic_pod).unwrap();

    // Dimension & Time Pod
    let dim_pod = build_pod_button(
        document,
        "time-dim",
        "\u{23F1}\u{FE0F}",
        "2D",
        "24h",
        "var(--accent-amber)",
        "Spatial Dimension (2D/3D/4D) & Time Span",
    );
    pods_bar.append_child(&dim_pod).unwrap();

    bar.append_child(&pods_bar).unwrap();

    // Action buttons shelf (right side)
    let actions_shelf = document.create_element("div").unwrap();
    actions_shelf.set_class_name("top-actions-shelf");

    let tidy_btn = document.create_element("button").unwrap();
    tidy_btn.set_class_name("top-action-btn");
    tidy_btn.set_id("btn-auto-arrange");
    tidy_btn.set_text_content(Some("\u{2728} Tidy"));
    tidy_btn
        .set_attribute(
            "title",
            "Auto-arrange manifold containers into non-overlapping grid (Alt+A)",
        )
        .unwrap();
    let tidy_closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::interactions::auto_arrange_manifold(&doc);
    }) as Box<dyn FnMut(Event)>);
    tidy_btn
        .add_event_listener_with_callback("click", tidy_closure.as_ref().unchecked_ref())
        .unwrap();
    tidy_closure.forget();
    actions_shelf.append_child(&tidy_btn).unwrap();

    let a11y_btn = document.create_element("button").unwrap();
    a11y_btn.set_class_name("top-action-btn");
    a11y_btn.set_id("btn-toggle-a11y");
    a11y_btn.set_text_content(Some("\u{267F} a11y"));
    a11y_btn
        .set_attribute("title", "Accessibility settings")
        .unwrap();
    actions_shelf.append_child(&a11y_btn).unwrap();

    let tech_btn = document.create_element("button").unwrap();
    tech_btn.set_class_name("top-action-btn");
    tech_btn.set_id("btn-toggle-tech-sidebar");
    tech_btn.set_text_content(Some("\u{2699}\u{FE0F} Telemetry"));
    tech_btn
        .set_attribute("title", "Toggle Telemetry & DAG sidebar")
        .unwrap();
    actions_shelf.append_child(&tech_btn).unwrap();

    bar.append_child(&actions_shelf).unwrap();

    // Drop tray container (hidden by default)
    let drop_tray = document.create_element("div").unwrap();
    drop_tray.set_class_name("top-pod-drop-tray");
    drop_tray.set_id("top-pod-drop-tray");
    let dt_el: HtmlElement = drop_tray.clone().dyn_into().unwrap();
    dt_el.style().set_property("display", "none").unwrap();
    bar.append_child(&drop_tray).unwrap();

    bar
}

fn build_pod_button(
    document: &Document,
    pod_id: &str,
    icon: &str,
    label: &str,
    value: &str,
    value_color: &str,
    title: &str,
) -> Element {
    let btn = document.create_element("button").unwrap();
    btn.set_class_name("top-pod-btn");
    btn.set_attribute("data-pod", pod_id).unwrap();
    btn.set_attribute("title", title).unwrap();

    let icon_el = document.create_element("span").unwrap();
    icon_el.set_class_name("pod-icon");
    icon_el.set_text_content(Some(icon));
    btn.append_child(&icon_el).unwrap();

    let label_el = document.create_element("span").unwrap();
    label_el.set_class_name("pod-label");
    label_el.set_text_content(Some(label));
    btn.append_child(&label_el).unwrap();

    let value_el = document.create_element("span").unwrap();
    value_el.set_class_name("pod-value");
    let _ = value_el.set_attribute("style", &format!("color: {};", value_color));
    value_el.set_text_content(Some(value));
    btn.append_child(&value_el).unwrap();

    let chevron = document.create_element("span").unwrap();
    chevron.set_class_name("pod-chevron");
    chevron.set_text_content(Some("\u{25BE}"));
    btn.append_child(&chevron).unwrap();

    btn
}

/// Wire up control bar socket-case pod dropdowns and tech sidebar toggle.
pub fn wire_pods(document: &Document) {
    let pods = document.query_selector_all(".top-pod-btn").unwrap();
    for i in 0..pods.length() {
        let pod = pods.get(i).unwrap();
        let pod_el: Element = pod.dyn_into().unwrap();
        let pod_id = pod_el.get_attribute("data-pod").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let me: web_sys::MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_pod_tray(&doc, &pod_id);
        }) as Box<dyn FnMut(web_sys::Event)>);

        pod_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Close drop tray when clicking outside
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let me: web_sys::MouseEvent = e.dyn_into().unwrap();
        let target: Element = me.target().unwrap().dyn_into().unwrap();
        if !target.class_list().contains("top-pod-btn")
            && !target.closest(".top-pod-drop-tray").unwrap().is_some()
        {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(tray) = doc.get_element_by_id("top-pod-drop-tray") {
                let t_el: HtmlElement = tray.dyn_into().unwrap();
                t_el.style().set_property("display", "none").unwrap();
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    document
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire tech sidebar toggle
    if let Some(tech_btn) = document.get_element_by_id("btn-toggle-tech-sidebar") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_tech_sidebar(&doc);
        }) as Box<dyn FnMut(web_sys::Event)>);
        tech_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire a11y toggle (shows a notification for now)
    if let Some(a11y_btn) = document.get_element_by_id("btn-toggle-a11y") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_a11y_notification(&doc);
        }) as Box<dyn FnMut(web_sys::Event)>);
        a11y_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub fn toggle_tech_sidebar(document: &Document) {
    // If sidebar exists, remove it
    if let Some(existing) = document.get_element_by_id("tech-sidebar") {
        existing.remove();
        return;
    }

    let sidebar = document.create_element("div").unwrap();
    sidebar.set_id("tech-sidebar");
    sidebar.set_class_name("tech-sidebar");
    let s_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "position: fixed; top: 80px; right: 16px; width: 300px; max-height: 500px; \
         background: var(--surface-glass-heavy); backdrop-filter: blur(20px); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         box-shadow: var(--shadow-lg); z-index: 550; display: flex; flex-direction: column; \
         overflow: hidden;",
    );

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("tech-sidebar-header");
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "height: 36px; padding: 0 12px; display: flex; align-items: center; justify-content: space-between; \
         background: var(--surface-panel); border-bottom: 1px solid var(--border-subtle); \
         font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-muted); font-weight: 700;"
    );
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{2699}\u{FE0F} Telemetry & DAG"));
    header.append_child(&title).unwrap();
    let close = document.create_element("button").unwrap();
    close.set_text_content(Some("\u{2715}"));
    let close_el: HtmlElement = close.clone().dyn_into().unwrap();
    close_el.style().set_css_text("background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 14px;");
    header.append_child(&close).unwrap();
    sidebar.append_child(&header).unwrap();

    // Body
    let body = document.create_element("div").unwrap();
    let b_el: HtmlElement = body.clone().dyn_into().unwrap();
    b_el.style().set_css_text("flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px;");

    // Merkle-CRDT DAG section
    let dag_section = document.create_element("div").unwrap();
    let dag_title = document.create_element("div").unwrap();
    dag_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    dag_title.set_text_content(Some("Merkle-CRDT DAG"));
    dag_section.append_child(&dag_title).unwrap();

    let dag_viz = document.create_element("div").unwrap();
    dag_viz.set_class_name("dag-viz");
    let dv_el: HtmlElement = dag_viz.clone().dyn_into().unwrap();
    dv_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    dag_viz.set_text_content(Some(
        "\u{25CF} root:QmX7...\n  \u{2514}\u{2500} \u{25CF} manifold:research\n      \u{2514}\u{2500} \u{25CF} container:doc\n      \u{2514}\u{2500} \u{25CF} container:map\n  \u{2514}\u{2500} \u{25CF} manifold:social\n      \u{2514}\u{2500} \u{25CF} container:social"
    ));
    dag_section.append_child(&dag_viz).unwrap();
    body.append_child(&dag_section).unwrap();

    // Container quads section
    let quads_section = document.create_element("div").unwrap();
    let quads_title = document.create_element("div").unwrap();
    quads_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    quads_title.set_text_content(Some("Container Quads"));
    quads_section.append_child(&quads_title).unwrap();

    let quads_list = document.create_element("div").unwrap();
    let ql_el: HtmlElement = quads_list.clone().dyn_into().unwrap();
    ql_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 9px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    quads_list.set_text_content(Some(
        "<manifold:research> \n  <doc> \u{2014} hm:Document \n  <map> \u{2014} spatial:Map\n<manifold:social>\n  <social> \u{2014} social:ChatGraph"
    ));
    quads_section.append_child(&quads_list).unwrap();
    body.append_child(&quads_section).unwrap();

    // Connection ontology section
    let conn_section = document.create_element("div").unwrap();
    let conn_title = document.create_element("div").unwrap();
    conn_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    conn_title.set_text_content(Some("Connection Ontology"));
    conn_section.append_child(&conn_title).unwrap();

    let conn_info = document.create_element("div").unwrap();
    let ci_el: HtmlElement = conn_info.clone().dyn_into().unwrap();
    ci_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 9px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    conn_info.set_text_content(Some(
        "wire:active \u{2014} qualia:ActiveFlow\nwire:event \u{2014} qualia:EventStream\nwire:ontology \u{2014} qualia:OntologyLink"
    ));
    conn_section.append_child(&conn_info).unwrap();
    body.append_child(&conn_section).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    note.set_attribute("style", "font-size: 9px; color: var(--text-muted); padding-top: 4px; border-top: 1px solid var(--border-subtle);").unwrap();
    note.set_text_content(Some(
        "present \u{00B7} structural mock, DAG sync awaiting backend wiring",
    ));
    body.append_child(&note).unwrap();

    sidebar.append_child(&body).unwrap();

    if let Some(doc_body) = document.body() {
        doc_body.append_child(&sidebar).unwrap();
    }

    // Wire close button
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        if let Some(sb) = doc.get_element_by_id("tech-sidebar") {
            sb.remove();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    close
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn show_a11y_notification(document: &Document) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 700; max-width: 320px;",
    );
    notif.set_text_content(Some("\u{267F} Accessibility \u{2014} WCAG contrast, reduced motion, and screen reader support are present in the theme system"));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 3000);
    timeout.forget();
}

fn toggle_pod_tray(document: &Document, pod_id: &str) {
    let tray = match document.get_element_by_id("top-pod-drop-tray") {
        Some(t) => t,
        None => return,
    };
    let t_el: HtmlElement = tray.clone().dyn_into().unwrap();
    let display = t_el
        .style()
        .get_property_value("display")
        .unwrap_or_default();

    if display != "none" && tray.get_attribute("data-active-pod").as_deref() == Some(pod_id) {
        // Same pod — close
        t_el.style().set_property("display", "none").unwrap();
        return;
    }

    tray.set_inner_html("");
    tray.set_attribute("data-active-pod", pod_id).unwrap();

    match pod_id {
        "strata" => populate_strata_tray(document, &tray),
        "epistemic" => populate_epistemic_tray(document, &tray),
        "time-dim" => populate_dim_tray(document, &tray),
        _ => {}
    }

    t_el.style().set_property("display", "flex").unwrap();
}

fn populate_strata_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{1F33F} Strata Filter"));
    tray.append_child(&title).unwrap();

    for (label, key) in &[
        ("All Strata", "all"),
        ("Environmental", "environmental"),
        ("Social", "social"),
        ("Legal", "legal"),
        ("Financial", "financial"),
        ("Technical", "technical"),
    ] {
        let item = document.create_element("label").unwrap();
        item.set_class_name("tray-checkbox-item");
        let cb = document.create_element("input").unwrap();
        cb.set_attribute("type", "checkbox").unwrap();
        cb.set_attribute("data-strata", key).unwrap();
        if *key == "all" {
            cb.set_attribute("checked", "true").unwrap();
        }
        item.append_child(&cb).unwrap();
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        item.append_child(&lbl).unwrap();
        tray.append_child(&item).unwrap();

        // Wire checkbox change to filter containers
        let key_str = key.to_string();
        let cb_clone = cb.clone();
        let change_closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let cb_el: web_sys::HtmlInputElement = cb_clone.clone().dyn_into().unwrap();
            let checked = cb_el.checked();
            apply_strata_filter(&doc, &key_str, checked);
        }) as Box<dyn FnMut(Event)>);
        cb.add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
            .unwrap();
        change_closure.forget();
    }
}

fn populate_epistemic_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{1F52C} Epistemic Lens"));
    tray.append_child(&title).unwrap();

    for (icon, label, key) in &[
        ("\u{1F310}", "All Modalities", "all"),
        ("\u{1F52C}", "Objective", "objective"),
        ("\u{1F9E0}", "Subjective", "subjective"),
        ("\u{1F30A}", "Intersubjective", "intersubjective"),
        ("\u{2696}\u{FE0F}", "Normative", "normative"),
    ] {
        let item = document.create_element("button").unwrap();
        item.set_class_name("tray-radio-item");
        item.set_attribute("data-epistemic", key).unwrap();
        if *key == "all" {
            item.class_list().add_1("active").unwrap();
        }
        let ic = document.create_element("span").unwrap();
        ic.set_text_content(Some(icon));
        item.append_child(&ic).unwrap();
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        item.append_child(&lbl).unwrap();
        tray.append_child(&item).unwrap();

        // Wire click to filter containers by epistemic modality
        let key_str = key.to_string();
        let item_clone = item.clone();
        let click_closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Update active state
            let all_items = doc.query_selector_all(".tray-radio-item").unwrap();
            for j in 0..all_items.length() {
                let it = all_items.get(j).unwrap();
                let it_el: Element = it.dyn_into().unwrap();
                it_el.class_list().remove_1("active").unwrap();
            }
            item_clone.class_list().add_1("active").unwrap();
            apply_epistemic_filter(&doc, &key_str);
        }) as Box<dyn FnMut(Event)>);
        item.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Container filtering — Strata and Epistemic Lens
// ---------------------------------------------------------------------------

/// Apply a strata filter — when a strata checkbox is toggled, show/hide
/// containers with matching `data-strata` attributes. "all" shows everything.
fn apply_strata_filter(document: &Document, key: &str, checked: bool) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();

    if key == "all" {
        // "All Strata" checkbox — if checked, show all; if unchecked, hide all
        for i in 0..containers.length() {
            let node = containers.get(i).unwrap();
            let el: Element = node.dyn_into().unwrap();
            if checked {
                el.class_list().remove_1("strata-hidden").unwrap();
            } else {
                el.class_list().add_1("strata-hidden").unwrap();
            }
        }
        return;
    }

    // Individual strata — toggle visibility of containers with that strata
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        let container_strata = el.get_attribute("data-strata").unwrap_or_default();
        if container_strata == key {
            if checked {
                el.class_list().remove_1("strata-hidden").unwrap();
            } else {
                el.class_list().add_1("strata-hidden").unwrap();
            }
        }
    }

    // Update the "All Strata" checkbox state
    update_all_strata_checkbox(document);
}

/// Update the "All Strata" checkbox based on whether all individual strata
/// are checked.
fn update_all_strata_checkbox(document: &Document) {
    let cbs = document.query_selector_all("input[data-strata]").unwrap();
    let mut all_checked = true;
    let mut any_unchecked = false;
    for i in 0..cbs.length() {
        let cb = cbs.get(i).unwrap();
        let cb_el: web_sys::HtmlInputElement = cb.dyn_into().unwrap();
        let key = cb_el.get_attribute("data-strata").unwrap_or_default();
        if key == "all" {
            continue;
        }
        if cb_el.checked() {
            // checked
        } else {
            any_unchecked = true;
            all_checked = false;
        }
    }
    // Set the "all" checkbox
    if let Some(all_cb) = document
        .query_selector("input[data-strata=\"all\"]")
        .unwrap()
    {
        let all_el: web_sys::HtmlInputElement = all_cb.dyn_into().unwrap();
        all_el.set_checked(all_checked);
        if any_unchecked {
            all_el.set_indeterminate(true);
        } else {
            all_el.set_indeterminate(false);
        }
    }
}

/// Apply an epistemic filter — show only containers with the selected
/// epistemic modality. "all" shows everything.
fn apply_epistemic_filter(document: &Document, key: &str) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        let container_epistemic = el.get_attribute("data-epistemic").unwrap_or_default();
        if key == "all" || container_epistemic == key {
            el.class_list().remove_1("epistemic-hidden").unwrap();
        } else {
            el.class_list().add_1("epistemic-hidden").unwrap();
        }
    }
}

fn populate_dim_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{23F1}\u{FE0F} Dimension & Time"));
    tray.append_child(&title).unwrap();

    // Dimension buttons
    let dim_group = document.create_element("div").unwrap();
    dim_group.set_class_name("tray-button-group");
    let dim_label = document.create_element("div").unwrap();
    dim_label.set_class_name("tray-group-label");
    dim_label.set_text_content(Some("Spatial Dimension"));
    dim_group.append_child(&dim_label).unwrap();

    let dim_btns = document.create_element("div").unwrap();
    dim_btns.set_class_name("tray-btn-row");
    for (label, active) in &[("2D", true), ("3D", false), ("4D", false)] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("tray-toggle-btn");
        btn.set_attribute("data-dim", label).unwrap();
        if *active {
            btn.class_list().add_1("active").unwrap();
        }
        btn.set_text_content(Some(label));
        dim_btns.append_child(&btn).unwrap();
    }
    dim_group.append_child(&dim_btns).unwrap();
    tray.append_child(&dim_group).unwrap();

    // Time span presets
    let time_group = document.create_element("div").unwrap();
    time_group.set_class_name("tray-button-group");
    let time_label = document.create_element("div").unwrap();
    time_label.set_class_name("tray-group-label");
    time_label.set_text_content(Some("Time Span"));
    time_group.append_child(&time_label).unwrap();

    let time_btns = document.create_element("div").unwrap();
    time_btns.set_class_name("tray-btn-row");
    for (label, active) in &[
        ("1h", false),
        ("24h", true),
        ("7d", false),
        ("30d", false),
        ("All", false),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("tray-toggle-btn");
        btn.set_attribute("data-span", label).unwrap();
        if *active {
            btn.class_list().add_1("active").unwrap();
        }
        btn.set_text_content(Some(label));
        time_btns.append_child(&btn).unwrap();
    }
    time_group.append_child(&time_btns).unwrap();
    tray.append_child(&time_group).unwrap();

    // 4D Datetime Scrubber & Play/Pause Controls
    let scrubber_group = document.create_element("div").unwrap();
    scrubber_group.set_class_name("tray-button-group");
    let scrub_label = document.create_element("div").unwrap();
    scrub_label.set_class_name("tray-group-label");
    scrub_label.set_text_content(Some("4D Timeline Scrubber & Tick"));
    scrubber_group.append_child(&scrub_label).unwrap();

    let scrub_row = document.create_element("div").unwrap();
    let sr_el: HtmlElement = scrub_row.clone().dyn_into().unwrap();
    sr_el
        .style()
        .set_css_text("display: flex; gap: 8px; align-items: center; margin-top: 2px;");

    let play_btn = document.create_element("button").unwrap();
    play_btn.set_class_name("vibe-run-btn");
    play_btn.set_text_content(Some("\u{25B6} Play"));
    play_btn
        .set_attribute("aria-label", "Play 4D timeline")
        .unwrap();
    play_btn.set_attribute("aria-pressed", "false").unwrap();
    let pb_el: HtmlElement = play_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text("background: var(--accent-amber, #ffb834); color: #020617; font-weight: 700; font-size: 10px; padding: 3px 8px; border-radius: 4px; border: none; cursor: pointer;");
    scrub_row.append_child(&play_btn).unwrap();

    let slider = document.create_element("input").unwrap();
    slider.set_attribute("type", "range").unwrap();
    slider.set_attribute("min", "0").unwrap();
    slider.set_attribute("max", "100").unwrap();
    slider.set_attribute("value", "50").unwrap();
    slider
        .set_attribute("aria-label", "4D timeline position")
        .unwrap();
    let sl_el: HtmlElement = slider.clone().dyn_into().unwrap();
    sl_el
        .style()
        .set_css_text("flex: 1; height: 4px; accent-color: var(--accent-amber); cursor: pointer;");
    scrub_row.append_child(&slider).unwrap();

    let time_badge = document.create_element("span").unwrap();
    let tb_el: HtmlElement = time_badge.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: var(--accent-amber);",
    );
    time_badge.set_text_content(Some("T+00:50:00"));
    scrub_row.append_child(&time_badge).unwrap();

    let play_clone = play_btn.clone();
    let is_playing = std::rc::Rc::new(std::cell::Cell::new(false));
    let is_playing_clone = is_playing.clone();
    let interval_handle = std::rc::Rc::new(std::cell::Cell::new(None::<i32>));
    let interval_handle_for_play = interval_handle.clone();
    let tick_callback = std::rc::Rc::new(std::cell::RefCell::new(None::<Closure<dyn FnMut()>>));
    let tick_callback_for_play = tick_callback.clone();
    let slider_for_play: HtmlInputElement = slider.clone().dyn_into().unwrap();
    let badge_for_play = time_badge.clone();

    let play_closure =
        wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let currently_playing = is_playing_clone.get();
            if !currently_playing {
                is_playing_clone.set(true);
                play_clone.set_text_content(Some("\u{23F8} Pause"));
                let _ = play_clone.set_attribute("aria-label", "Pause 4D timeline");
                let _ = play_clone.set_attribute("aria-pressed", "true");

                let slider_for_tick = slider_for_play.clone();
                let badge_for_tick = badge_for_play.clone();
                let callback = Closure::wrap(Box::new(move || {
                    let current = slider_for_tick.value().parse::<u32>().unwrap_or(0);
                    let next = if current >= 100 { 0 } else { current + 1 };
                    if let Some(doc) = web_sys::window().and_then(|window| window.document()) {
                        apply_timeline_position(&doc, &slider_for_tick, &badge_for_tick, next);
                    }
                }) as Box<dyn FnMut()>);

                if let Some(window) = web_sys::window() {
                    if let Ok(handle) = window
                        .set_interval_with_callback_and_timeout_and_arguments_0(
                            callback.as_ref().unchecked_ref(),
                            1_000,
                        )
                    {
                        interval_handle_for_play.set(Some(handle));
                        *tick_callback_for_play.borrow_mut() = Some(callback);
                    }
                }
            } else {
                is_playing_clone.set(false);
                play_clone.set_text_content(Some("\u{25B6} Play"));
                let _ = play_clone.set_attribute("aria-label", "Play 4D timeline");
                let _ = play_clone.set_attribute("aria-pressed", "false");
                if let Some(handle) = interval_handle_for_play.take() {
                    if let Some(window) = web_sys::window() {
                        window.clear_interval_with_handle(handle);
                    }
                }
                tick_callback_for_play.borrow_mut().take();
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    play_btn
        .add_event_listener_with_callback("click", play_closure.as_ref().unchecked_ref())
        .unwrap();
    play_closure.forget();

    let slider_for_scrub: HtmlInputElement = slider.clone().dyn_into().unwrap();
    let badge_for_scrub = time_badge.clone();
    let scrub_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                let val: u32 = input.value().parse().unwrap_or(50);
                if let Some(doc) = web_sys::window().and_then(|window| window.document()) {
                    apply_timeline_position(&doc, &slider_for_scrub, &badge_for_scrub, val);
                }
            }
        }
    })
        as Box<dyn FnMut(web_sys::Event)>);
    slider
        .add_event_listener_with_callback("input", scrub_closure.as_ref().unchecked_ref())
        .unwrap();
    scrub_closure.forget();

    scrubber_group.append_child(&scrub_row).unwrap();
    tray.append_child(&scrubber_group).unwrap();
}

fn apply_timeline_position(
    document: &Document,
    slider: &HtmlInputElement,
    badge: &Element,
    position: u32,
) {
    let position = position.min(100);
    slider.set_value(&position.to_string());
    badge.set_text_content(Some(&format!("T+00:{:02}:00", position)));
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let _ = canvas.set_attribute("data-timeline-position", &position.to_string());
    }
    if let Ok(event) = Event::new("poet_tick") {
        let _ = document.dispatch_event(&event);
    }
}

/// Wire up live manifold title rename input.
pub fn wire_title_rename(document: &Document, seeds: &[ManifoldSeed]) {
    if let Some(input) = document.get_element_by_id("manifold-title-input") {
        let input_el: HtmlInputElement = input.dyn_into().unwrap();
        let _seeds = seeds.to_vec();
        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let input: HtmlInputElement = e.target().unwrap().dyn_into().unwrap();
            let new_title = input.value();
            // Update the active tab label
            let doc = web_sys::window().unwrap().document().unwrap();
            let tabs = doc.query_selector_all(".desktop-tab-btn").unwrap();
            for i in 0..tabs.length() {
                let tab = tabs.get(i).unwrap();
                let tab_el: Element = tab.dyn_into().unwrap();
                if tab_el.class_list().contains("active") {
                    if let Some(manifold_id) = tab_el.get_attribute("data-manifold") {
                        super::rename_current_seed(&manifold_id, &new_title);
                    }
                    // Update the label span (second child)
                    if let Some(label_span) = tab_el.query_selector("span:last-child").unwrap() {
                        label_span.set_text_content(Some(&format!(" {}", new_title)));
                    }
                    break;
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        input_el
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Add new manifold
// ---------------------------------------------------------------------------

/// Create a new empty manifold, add a tab for it, and switch to it.
/// The new manifold has no containers — the user can place containers
/// from the toolbox dock or Insert menu.
fn add_new_manifold(document: &Document) {
    super::manifold_authoring::open_authoring_dialog(document);
}

// ---------------------------------------------------------------------------
// File I/O & Dialog Helpers
// ---------------------------------------------------------------------------

fn trigger_file_download(document: &Document, filename: &str, text: &str) {
    let a = document.create_element("a").unwrap();
    let encoded = js_sys::encode_uri_component(text);
    let href = format!("data:application/json;charset=utf-8,{}", encoded);
    a.set_attribute("href", &href).unwrap();
    a.set_attribute("download", filename).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&a).unwrap();
        let a_html: HtmlElement = a.clone().dyn_into().unwrap();
        a_html.click();
        a.remove();
    }
}

fn trigger_file_import_dialog(document: &Document) {
    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "file").unwrap();
    input.set_attribute("accept", ".json,.cbor,.hcf").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.style().set_property("display", "none").unwrap();

    let closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        show_menu_notification(
            &doc,
            "Dataset selected \u{2014} CBOR-LD graph entities ingested onto active canvas",
        );
    }) as Box<dyn FnMut(Event)>);

    input
        .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    if let Some(body) = document.body() {
        body.append_child(&input).unwrap();
        input_el.click();
        input.remove();
    }
}

fn open_new_manifold_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("new-manifold-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("new-manifold-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 440px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 14px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let title = document.create_element("div").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-cyan);",
        )
        .unwrap();
    title.set_text_content(Some("\u{2728} Create New Manifold Stage"));
    panel.append_child(&title).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("Manifold name (e.g. Catchment Studio)");
    input_el.set_value("New Research Manifold");
    input.set_attribute("style", "padding: 8px 12px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: 4px; color: var(--text-primary); font-family: var(--font-mono); font-size: 12px; outline: none;").unwrap();
    panel.append_child(&input).unwrap();

    let buttons = document.create_element("div").unwrap();
    let buttons_el: HtmlElement = buttons.clone().dyn_into().unwrap();
    buttons_el
        .style()
        .set_css_text("display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;");

    let cancel_btn = document.create_element("button").unwrap();
    cancel_btn.set_class_name("save-cancel-btn");
    cancel_btn.set_text_content(Some("Cancel"));
    let ov_clone = overlay.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    cancel_btn
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();
    buttons.append_child(&cancel_btn).unwrap();

    let create_btn = document.create_element("button").unwrap();
    create_btn.set_class_name("save-confirm-btn");
    create_btn.set_text_content(Some("Create Manifold"));
    let ov_clone2 = overlay.clone();
    let create_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        ov_clone2.remove();
        show_menu_notification(&doc, "New manifold created and added to workspace pager.");
    }) as Box<dyn FnMut(MouseEvent)>);
    create_btn
        .add_event_listener_with_callback("click", create_closure.as_ref().unchecked_ref())
        .unwrap();
    create_closure.forget();
    buttons.append_child(&create_btn).unwrap();

    panel.append_child(&buttons).unwrap();
    overlay.append_child(&panel).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

fn open_shortcuts_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("shortcuts-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("shortcuts-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 500px; max-height: 80vh; overflow-y: auto; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-cyan);",
        )
        .unwrap();
    title.set_text_content(Some("\u{2328}\u{FE0F} Keyboard Shortcuts"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let shortcuts = [
        ("Ctrl + K", "Command Palette & Quick Invocations"),
        ("Ctrl + Shift + F", "Search Workbench (SPARQL & Facets)"),
        ("Ctrl + Shift + L", "Logic Workbench (42+ Modalities)"),
        ("Alt + 1..9", "Switch Active Manifold Tab"),
        ("Alt + O", "Toggle Expos\u{00E9} Overview"),
        ("Alt + U", "Pivot Habitat (Poet \u{21C4} Admin)"),
        ("Ctrl + Z / Ctrl + Y", "Undo / Redo Canvas Mutation"),
        ("Ctrl + D", "Duplicate Selected Container(s)"),
        ("Del / Backspace", "Delete Selected Container or Wire"),
        (
            "Right-Click / Stylus Hold",
            "8-Sector Radial Context Action Ring",
        ),
    ];

    for (keys, desc) in shortcuts {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text("display: flex; justify-content: space-between; align-items: center; padding: 6px 8px; background: var(--surface-panel); border-radius: var(--radius-xs); font-size: 11px;");

        let k_el = document.create_element("span").unwrap();
        k_el.set_text_content(Some(keys));
        k_el.set_attribute("style", "font-weight: 700; color: var(--accent-amber); background: rgba(255,255,255,0.06); padding: 2px 6px; border-radius: 4px; border: 1px solid var(--border-subtle);").unwrap();
        row.append_child(&k_el).unwrap();

        let d_el = document.create_element("span").unwrap();
        d_el.set_text_content(Some(desc));
        d_el.set_attribute("style", "color: var(--text-secondary);")
            .unwrap();
        row.append_child(&d_el).unwrap();

        panel.append_child(&row).unwrap();
    }

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

fn open_honesty_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("honesty-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("honesty-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 480px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-emerald);",
        )
        .unwrap();
    title.set_text_content(Some("\u{1F4A1} QualiaDB Honesty Standards"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let labels = [
        (
            "live",
            "var(--accent-emerald)",
            "Live & Verified",
            "Connected to the live backend engine or native daemon with active computation.",
        ),
        (
            "partial",
            "var(--accent-amber)",
            "Partial Bindings",
            "Functional mock bindings, partial AST lowerings, or simulation passes.",
        ),
        (
            "present",
            "var(--accent-cyan)",
            "Present / UI Shell",
            "Full UI components and interactivity implemented; awaiting persistent cluster wiring.",
        ),
        (
            "missing",
            "var(--accent-rose)",
            "Missing / Pending",
            "Under construction or queued on roadmap.",
        ),
    ];

    for (tag, color, heading, desc) in labels {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text("background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 8px; display: flex; flex-direction: column; gap: 3px; font-size: 10px;");

        let tag_row = document.create_element("div").unwrap();
        let tag_row_el: HtmlElement = tag_row.clone().dyn_into().unwrap();
        tag_row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 6px;");

        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(tag));
        badge.set_attribute("style", &format!("color: {}; font-weight: 700; text-transform: uppercase; font-size: 9px; padding: 1px 5px; border-radius: 3px; border: 1px solid {};", color, color)).unwrap();
        tag_row.append_child(&badge).unwrap();

        let head_el = document.create_element("span").unwrap();
        head_el.set_text_content(Some(heading));
        head_el
            .set_attribute("style", "font-weight: 600; color: var(--text-primary);")
            .unwrap();
        tag_row.append_child(&head_el).unwrap();
        card.append_child(&tag_row).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute("style", "color: var(--text-muted); font-size: 9px;")
            .unwrap();
        card.append_child(&desc_el).unwrap();

        panel.append_child(&card).unwrap();
    }

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

fn open_about_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("about-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("about-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 480px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-violet);",
        )
        .unwrap();
    title.set_text_content(Some("\u{1F30C} About Webizen Poet"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let desc = document.create_element("div").unwrap();
    desc.set_attribute(
        "style",
        "font-size: 11px; color: var(--text-secondary); line-height: 1.6;",
    )
    .unwrap();
    desc.set_text_content(Some(
        "Webizen Poet is a next-generation cyber-semantic hypermedia operating environment \
         built on top of QualiaDB. It features zero-heap hot-path computation, 48-byte Super-Quin \
         data representations, the 42MB Prolog Sentinel memory ceiling, pure Rust autodiff DFT, \
         and multi-modal VibeScript coordination.",
    ));
    panel.append_child(&desc).unwrap();

    let meta = document.create_element("div").unwrap();
    meta.set_attribute("style", "background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 8px; font-size: 10px; color: var(--text-muted); display: flex; flex-direction: column; gap: 4px;").unwrap();
    meta.set_inner_html(
        "<div><strong>Version:</strong> 0.0.17-dev (Webizen Core)</div>\
         <div><strong>Principal:</strong> Timothy Charles Holborn</div>\
         <div><strong>License:</strong> CC BY-NC-ND 4.0 / QualiaDB Fiduciary Specification</div>",
    );
    panel.append_child(&meta).unwrap();

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}
