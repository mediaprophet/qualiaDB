//! Top menubar and dropdown construction.

use super::*;

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
            ("New Manifold", "file:new-manifold", "\u{1F4CB}", "Alt+N"),
            ("Save Checkpoint", "file:save", "\u{1F4BE}", "Ctrl+S"),
            (
                "Save As\u{2026}",
                "file:save-as",
                "\u{1F4C2}",
                "Ctrl+Shift+S",
            ),
            ("separator", "", "", ""),
            ("header:Import & Export", "", "", ""),
            ("Export CBOR-LD", "file:export-cbor", "\u{1F4E4}", ""),
            ("Import CBOR-LD", "file:import-cbor", "\u{1F4E5}", ""),
            (
                "Export Distribution (HCF)\u{2026}",
                "file:export-distribution",
                "\u{1F4E6}",
                "",
            ),
            ("separator", "", "", ""),
            ("header:Audit & Versioning", "", "", ""),
            (
                "Checkpoint History\u{2026}",
                "file:checkpoint-history",
                "\u{1F4D4}",
                "",
            ),
            (
                "Prune & Archive\u{2026}",
                "file:prune-archive",
                "\u{1F9F9}",
                "",
            ),
            ("separator", "", "", ""),
            ("Close Manifold", "file:close", "\u{2715}", ""),
        ],
    ))
    .unwrap();

    // Edit menu
    left.append_child(&build_menu_dropdown(
        document,
        "Edit",
        &[
            ("Undo Mutation", "edit:undo", "\u{21A9}", "Ctrl+Z"),
            ("Redo Mutation", "edit:redo", "\u{21AA}", "Ctrl+Y"),
            ("separator", "", "", ""),
            ("Delete Selected", "edit:delete", "\u{1F5D1}", "Del"),
            (
                "Duplicate Selected",
                "edit:duplicate",
                "\u{1F4CB}",
                "Ctrl+D",
            ),
            ("separator", "", "", ""),
            (
                "Select All Containers",
                "edit:select-all",
                "\u{1F4D8}",
                "Ctrl+A",
            ),
            ("separator", "", "", ""),
            ("header:Cross-Manifold Operations", "", "", ""),
            (
                "Move Container to Manifold\u{2026}",
                "edit:move-to-manifold",
                "\u{1F4E6}",
                "",
            ),
            (
                "Copy Container to Manifold\u{2026}",
                "edit:copy-to-manifold",
                "\u{1F4CB}",
                "",
            ),
        ],
    ))
    .unwrap();

    // View menu
    left.append_child(&build_menu_dropdown(
        document,
        "View",
        &[
            (
                "Toggle Toolbox Dock",
                "view:toggle-dock",
                "\u{1F9ED}",
                "Alt+T",
            ),
            (
                "Toggle Telemetry & DAG",
                "view:toggle-telemetry",
                "\u{2699}",
                "Alt+D",
            ),
            (
                "Toggle Expos\u{00E9} Overview",
                "view:expose",
                "\u{1F4F7}",
                "Alt+O",
            ),
            (
                "Auto-Arrange Manifold (Tidy)",
                "view:auto-arrange",
                "\u{2728}",
                "Alt+A",
            ),
            ("separator", "", "", ""),
            ("Zoom In", "view:zoom-in", "\u{1F50D}+", "Ctrl++"),
            ("Zoom Out", "view:zoom-out", "\u{1F50D}\u{2212}", "Ctrl+-"),
            (
                "Reset Zoom (100%)",
                "view:zoom-reset",
                "\u{1F503}",
                "Ctrl+0",
            ),
            ("separator", "", "", ""),
            ("Accessibility Settings", "view:a11y", "\u{267F}", ""),
        ],
    ))
    .unwrap();

    // Insert menu
    left.append_child(&build_menu_dropdown(
        document,
        "Insert",
        &[
            ("header:Primary Containers", "", "", ""),
            ("+ Document (CML HyperDoc)", "insert:doc", "\u{1F4C4}", ""),
            ("+ Sheet / Table", "insert:sheet", "\u{1F4CA}", ""),
            ("+ Code / VibeScript", "insert:code", "\u{1F4BB}", ""),
            ("+ GIS Spatial Map", "insert:map", "\u{1F5FA}", ""),
            ("+ Ontology Graph", "insert:ontology", "\u{1F4D6}", ""),
            ("+ Social Channel", "insert:social", "\u{1F4AC}", ""),
            ("+ 3D Viewport", "insert:3d", "\u{1F3AF}", ""),
            ("+ WebRTC AV Stream", "insert:webrtc", "\u{1F4F7}", ""),
            ("separator", "", "", ""),
            ("header:Workflow & Verification Panels", "", "", ""),
            (
                "+ Checkpoint Tray",
                "insert:checkpoint-tray",
                "\u{1F4D4}",
                "",
            ),
            (
                "+ Credential Inspector",
                "insert:credential-inspector",
                "\u{1F511}",
                "",
            ),
            (
                "+ Context Markup Editor",
                "insert:context-markup-editor",
                "\u{1F50D}",
                "",
            ),
            (
                "+ Provenance Panel",
                "insert:provenance-panel",
                "\u{1F4DC}",
                "",
            ),
            (
                "+ Publication Workflow",
                "insert:publication-workflow",
                "\u{1F4E6}",
                "",
            ),
            (
                "+ Constituency Manager",
                "insert:constituency-manager",
                "\u{1F465}",
                "",
            ),
        ],
    ))
    .unwrap();

    // Connectors menu
    left.append_child(&build_menu_dropdown(
        document,
        "Connectors",
        &[
            ("header:Hypermedia Semantic Wires", "", "", ""),
            (
                "Inspect Selected Wire",
                "wire:inspect-selected",
                "\u{1F3F7}\u{FE0F}",
                "Enter",
            ),
            (
                "Edit Wire Predicate Label",
                "wire:edit-label",
                "\u{270F}\u{FE0F}",
                "F2",
            ),
            ("separator", "", "", ""),
            ("header:Wire Modalities", "", "", ""),
            (
                "Active Reactive Flow Wire",
                "wire:modality-active",
                "\u{26A1}",
                "",
            ),
            (
                "Event Stream Signal Wire",
                "wire:modality-event",
                "\u{23F1}\u{FE0F}",
                "",
            ),
            (
                "Ontology Semantic Link Wire",
                "wire:modality-ontology",
                "\u{1F517}",
                "",
            ),
            (
                "Deontic Obligation Wire (O)",
                "wire:modality-deontic",
                "\u{1F6E1}\u{FE0F}",
                "",
            ),
            (
                "Epistemic Knowledge Wire (K)",
                "wire:modality-epistemic",
                "\u{1F52C}",
                "",
            ),
            ("separator", "", "", ""),
            (
                "Delete Selected Wire",
                "wire:delete-selected",
                "\u{1F5D1}",
                "Del",
            ),
        ],
    ))
    .unwrap();

    // Help menu
    left.append_child(&build_menu_dropdown(
        document,
        "Help",
        &[
            (
                "Command Palette",
                "help:command-palette",
                "\u{2318}",
                "Ctrl+K",
            ),
            (
                "Search Workbench",
                "help:search-workbench",
                "\u{1F50D}",
                "Ctrl+Shift+F",
            ),
            (
                "Logic Workbench",
                "help:logic-workbench",
                "\u{1F9E0}",
                "Ctrl+Shift+L",
            ),
            ("separator", "", "", ""),
            ("Keyboard Shortcuts", "help:shortcuts", "\u{2328}", ""),
            ("Honesty Standards", "help:honesty", "\u{1F4A1}", ""),
            ("About Webizen Poet", "help:about", "\u{2139}", ""),
            ("separator", "", "", ""),
            ("Report Issue to GitHub", "help:report", "\u{1F41B}", ""),
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
        super::super::search_workbench::toggle_search_workbench(&doc);
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
        super::super::logic_workbench::toggle_logic_workbench(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    logic_btn
        .add_event_listener_with_callback("click", lb_closure.as_ref().unchecked_ref())
        .unwrap();
    lb_closure.forget();
    right.append_child(&logic_btn).unwrap();

    // Webizen Native Daemon Status Badge
    let daemon_badge = super::super::native_daemon::build_daemon_status_badge(document);
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
        super::super::workspace_pivot::toggle_workspace_pivot(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    habitat_btn
        .add_event_listener_with_callback("click", hp_closure.as_ref().unchecked_ref())
        .unwrap();
    hp_closure.forget();
    right.append_child(&habitat_btn).unwrap();

    // Ambient Mesh Sentinel Indicator
    let mesh_badge = document.create_element("span").unwrap();
    mesh_badge.set_class_name("mesh-sentinel-badge");
    mesh_badge.set_text_content(Some("\u{25CF} Mesh unavailable"));
    mesh_badge
        .set_attribute("title", "Unavailable: live mesh status is not connected")
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

/// A menu item tuple: (Label/Header/Separator, Action ID, Icon emoji/glyph, Shortcut hint)
pub type MenuItemDef<'a> = (&'a str, &'a str, &'a str, &'a str);

/// Build a single dropdown menu with a label trigger and action items.
fn build_menu_dropdown(document: &Document, label: &str, items: &[MenuItemDef]) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("menu-dropdown");

    let trigger = document.create_element("button").unwrap();
    trigger.set_class_name("menu-btn");
    trigger.set_text_content(Some(label));
    trigger.set_attribute("type", "button").ok();
    trigger.set_attribute("aria-haspopup", "true").ok();
    trigger.set_attribute("aria-expanded", "false").ok();
    wrapper.append_child(&trigger).unwrap();

    let dropdown = document.create_element("div").unwrap();
    dropdown.set_class_name("menu-dropdown-content");
    dropdown.set_attribute("role", "menu").ok();

    for &(item_label, action, icon, shortcut) in items {
        if item_label == "separator" {
            let sep = document.create_element("div").unwrap();
            sep.set_class_name("menu-dropdown-separator");
            dropdown.append_child(&sep).unwrap();
        } else if let Some(header_text) = item_label.strip_prefix("header:") {
            let header = document.create_element("div").unwrap();
            header.set_class_name("menu-dropdown-header");
            header.set_text_content(Some(header_text));
            dropdown.append_child(&header).unwrap();
        } else {
            let item = document.create_element("button").unwrap();
            item.set_class_name("menu-dropdown-item");
            item.set_attribute("type", "button").ok();
            item.set_attribute("role", "menuitem").ok();
            item.set_attribute("data-menu-action", action).unwrap();

            let icon_el = document.create_element("span").unwrap();
            icon_el.set_class_name("menu-dropdown-item-icon");
            icon_el.set_text_content(Some(icon));
            item.append_child(&icon_el).unwrap();

            let label_el = document.create_element("span").unwrap();
            label_el.set_class_name("menu-dropdown-item-label");
            label_el.set_text_content(Some(item_label));
            item.append_child(&label_el).unwrap();

            if !shortcut.is_empty() {
                let sc_el = document.create_element("span").unwrap();
                sc_el.set_class_name("menu-dropdown-item-shortcut");
                sc_el.set_text_content(Some(shortcut));
                item.append_child(&sc_el).unwrap();
            }

            dropdown.append_child(&item).unwrap();
        }
    }

    wrapper.append_child(&dropdown).unwrap();
    wrapper
}
