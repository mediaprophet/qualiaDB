//! Top bar rendering: menubar + canvas control bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use crate::tool_chest::core::registry::ManifoldSeed;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
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

    let version = document.create_element("span").unwrap();
    version.set_class_name("version-badge");
    version.set_text_content(Some("0.0.31-dev"));
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
            show_menu_notification(
                document,
                "CBOR-LD export \u{2014} present, file download pending",
            );
        }
        "file:import-cbor" => {
            show_menu_notification(
                document,
                "CBOR-LD import \u{2014} present, file picker pending",
            );
        }
        "file:checkpoint-history" => {
            show_menu_notification(
                document,
                "Checkpoint history \u{2014} present, tree view pending (see SAVE_ARCHITECTURE.md)",
            );
        }
        "file:prune-archive" => {
            show_menu_notification(document, "Prune & archive \u{2014} present, tombstone pruning pending (see SAVE_ARCHITECTURE.md \u{00A7}5)");
        }
        "file:export-distribution" => {
            show_menu_notification(document, "Export distribution \u{2014} present, .q42 with credits + consent pending (see SAVE_ARCHITECTURE.md \u{00A7}5.3)");
        }
        "file:new-manifold" => {
            show_menu_notification(
                document,
                "New manifold \u{2014} present, creation dialog pending",
            );
        }
        "file:close" => {
            show_menu_notification(
                document,
                "Close manifold \u{2014} present, confirmation pending",
            );
        }
        "edit:duplicate" => {
            show_menu_notification(document, "Duplicate container \u{2014} pending");
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
        "view:zoom-in" | "view:zoom-out" | "view:zoom-reset" => {
            show_menu_notification(
                document,
                &format!("{} \u{2014} pending canvas zoom wiring", label),
            );
        }
        "view:expose" => {
            show_menu_notification(document, "Press Alt+O for Expos\u{00E9} overview");
        }
        "help:shortcuts" => {
            show_menu_notification(document, "Shortcuts: Alt+1-9 manifolds, Alt+O Expos\u{00E9}, Ctrl+K palette, Ctrl+Z/Y undo/redo, Del delete container");
        }
        "help:about" => {
            show_menu_notification(document, "Webizen Poet HyperCanvas \u{2014} Poet workbench, VibeScript engine, local CBOR-LD persistence");
        }
        "help:honesty" => {
            show_menu_notification(
                document,
                "live = wired, partial = some bindings, present = UI only, missing = not built",
            );
        }
        "help:report" => {
            show_menu_notification(document, "Issue reporting \u{2014} pending");
        }
        _ => {
            show_menu_notification(document, &format!("{} \u{2014} pending", label));
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
    actor_info.set_text_content(Some(
        "Actor: did:qualia:timothy_charles_holborn \u{2014} all saves are attributed to the principal."
    ));
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
        ("Snapshot", "snapshot", "Full provenance\n(archival)"),
        ("Pruned", "pruned", "Tombstones pruned\n(distribution)"),
    ];

    for (idx, (label, mode_id, desc)) in modes.iter().enumerate() {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("save-mode-btn");
        btn.set_attribute("data-save-mode", mode_id).unwrap();
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
        "\u{1F4A1} Auto and Checkpoint modes are live. Snapshot and Pruned modes are \
         present (UI records intent) \u{2014} full provenance graph, Merkle root, and \
         tombstone pruning are engine wiring pending. See SAVE_ARCHITECTURE.md.",
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
         font-size: 11px; font-weight: 700; cursor: pointer;"
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

/// Build the canvas control bar (manifold pager + title + socket-case pods).
pub fn build_canvas_control_bar(document: &Document, seeds: &[ManifoldSeed]) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("canvas-control-bar");

    // Pager
    let pager = document.create_element("div").unwrap();
    pager.set_class_name("virtual-desktop-pager");
    let desktops = document.create_element("div").unwrap();
    desktops.set_class_name("pager-desktops-list");

    for (i, seed) in seeds.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("desktop-tab-btn");
        if i == 0 {
            tab.class_list().add_1("active").unwrap();
        }
        tab.set_attribute("data-manifold", &seed.id).unwrap();

        let num = document.create_element("span").unwrap();
        num.set_class_name("desktop-num");
        num.set_text_content(Some(&(i + 1).to_string()));

        let label = document.create_element("span").unwrap();
        label.set_text_content(Some(&format!(" {} {}", seed.icon, seed.label)));

        tab.append_child(&num).unwrap();
        tab.append_child(&label).unwrap();
        desktops.append_child(&tab).unwrap();
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

    // Title box — editable input
    let title_box = document.create_element("div").unwrap();
    title_box.set_class_name("canvas-title-box");
    let title_input = document.create_element("input").unwrap();
    title_input.set_class_name("canvas-title-input");
    title_input.set_attribute("type", "text").unwrap();
    title_input.set_attribute("value", &seeds[0].label).unwrap();
    title_input
        .set_attribute("id", "manifold-title-input")
        .unwrap();
    title_box.append_child(&title_input).unwrap();

    let graph_badge = document.create_element("span").unwrap();
    graph_badge.set_class_name("graph-address-badge");
    graph_badge.set_id("manifold-graph-badge");
    graph_badge.set_text_content(Some(&format!("graph:manifold:{}", seeds[0].id)));
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

    // Wire pod interactions
    wire_pods(document);

    // Wire title rename
    wire_title_rename(document, seeds);

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

fn wire_pods(document: &Document) {
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

fn toggle_tech_sidebar(document: &Document) {
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
}

fn wire_title_rename(document: &Document, seeds: &[ManifoldSeed]) {
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
    use crate::tool_chest::core::registry::{ManifoldSeed, SeedContainer};

    // Generate a unique manifold ID
    let manifold_id = format!("manifold-{}", js_sys::Date::now() as u64);
    let label = format!("Manifold {}", {
        let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
        tabs.length() + 1
    });

    // Create an empty seed
    let seed = ManifoldSeed {
        id: manifold_id.clone(),
        label: label.clone(),
        icon: "\u{1F30C}".into(),
        containers: Vec::<SeedContainer>::new(),
        connections: Vec::new(),
        ..Default::default()
    };

    // Add a new tab button for this manifold
    let desktops = match document.query_selector(".pager-desktops-list").unwrap() {
        Some(d) => d,
        None => return,
    };

    let tab = document.create_element("button").unwrap();
    tab.set_class_name("desktop-tab-btn");
    tab.set_attribute("data-manifold", &manifold_id).unwrap();

    let num = document.create_element("span").unwrap();
    num.set_class_name("desktop-num");
    let tab_count = document
        .query_selector_all(".desktop-tab-btn")
        .unwrap()
        .length();
    num.set_text_content(Some(&(tab_count).to_string()));

    let lbl = document.create_element("span").unwrap();
    lbl.set_text_content(Some(&format!(" {} {}", seed.icon, seed.label)));

    tab.append_child(&num).unwrap();
    tab.append_child(&lbl).unwrap();

    // Insert before the + button
    if let Some(add_btn) = document.get_element_by_id("manifold-add-btn") {
        desktops.insert_before(&tab, Some(&add_btn)).unwrap();
    } else {
        desktops.append_child(&tab).unwrap();
    }

    // Wire the tab to switch to this manifold
    let seed_clone = seed.clone();
    let manifold_id_clone = manifold_id.clone();
    let tab_closure = Closure::wrap(Box::new(move || {
        switch_to_new_manifold(&manifold_id_clone, &seed_clone);
    }) as Box<dyn FnMut()>);
    tab.add_event_listener_with_callback("click", tab_closure.as_ref().unchecked_ref())
        .unwrap();
    tab_closure.forget();

    // Switch to the new manifold immediately
    switch_to_new_manifold(&manifold_id, &seed);

    // Show notification
    show_menu_notification(document, &format!("Created \u{201C}{}\u{201D}", label));
}

/// Switch to a newly created manifold — updates active tab, re-renders
/// canvas, and wires interactions.
fn switch_to_new_manifold(
    manifold_id: &str,
    seed: &crate::tool_chest::core::registry::ManifoldSeed,
) {
    let document = web_sys::window().unwrap().document().unwrap();

    // Update active tab
    let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        if tab_el.get_attribute("data-manifold").as_deref() == Some(manifold_id) {
            tab_el.class_list().add_1("active").unwrap();
        } else {
            tab_el.class_list().remove_1("active").unwrap();
        }
    }

    // Re-render the canvas with the new (empty) seed
    super::rerender_canvas(seed);
}
