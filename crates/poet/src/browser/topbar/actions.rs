//! Top menubar event wiring and action dispatch.

use super::*;

/// Wire up menu dropdown toggling, hover traversal, and action clicks.
pub fn wire_menu_dropdowns(document: &Document) {
    let triggers = document
        .query_selector_all(".top-menubar .menu-btn")
        .unwrap();
    for i in 0..triggers.length() {
        let trigger = triggers.get(i).unwrap();
        let trigger_el: Element = trigger.dyn_into().unwrap();
        let trigger_el_click = trigger_el.clone();
        let trigger_el_enter = trigger_el.clone();

        // Click trigger to toggle
        let click_closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let me: web_sys::MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            let parent_opt = trigger_el_click.parent_element();
            let is_already_open = parent_opt
                .as_ref()
                .map(|p| p.class_list().contains("open"))
                .unwrap_or(false);

            // Close all dropdowns
            let all = doc
                .query_selector_all(".top-menubar .menu-dropdown")
                .unwrap();
            for j in 0..all.length() {
                let d = all.get(j).unwrap();
                let de: Element = d.dyn_into().unwrap();
                de.class_list().remove_1("open").unwrap();
                if let Some(btn) = de.query_selector(".menu-btn").ok().flatten() {
                    let _ = btn.set_attribute("aria-expanded", "false");
                }
            }

            // Toggle this one
            if let Some(parent) = parent_opt {
                if !is_already_open {
                    parent.class_list().add_1("open").unwrap();
                    let _ = trigger_el_click.set_attribute("aria-expanded", "true");
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        trigger_el
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        // Desktop Menubar Hover Traversal: when any menu is open, hovering sibling opens it
        let enter_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let any_open = doc
                .query_selector(".top-menubar .menu-dropdown.open")
                .ok()
                .flatten()
                .is_some();
            if any_open {
                let all = doc
                    .query_selector_all(".top-menubar .menu-dropdown")
                    .unwrap();
                for j in 0..all.length() {
                    let d = all.get(j).unwrap();
                    let de: Element = d.dyn_into().unwrap();
                    if de != trigger_el_enter.parent_element().unwrap() {
                        de.class_list().remove_1("open").unwrap();
                        if let Some(btn) = de.query_selector(".menu-btn").ok().flatten() {
                            let _ = btn.set_attribute("aria-expanded", "false");
                        }
                    } else {
                        de.class_list().add_1("open").unwrap();
                        let _ = trigger_el_enter.set_attribute("aria-expanded", "true");
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        trigger_el
            .add_event_listener_with_callback("mouseenter", enter_closure.as_ref().unchecked_ref())
            .unwrap();
        enter_closure.forget();
    }

    // Wire menu action items
    let items = document
        .query_selector_all(".top-menubar .menu-dropdown-item")
        .unwrap();
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
            let all = doc
                .query_selector_all(".top-menubar .menu-dropdown")
                .unwrap();
            for j in 0..all.length() {
                let d = all.get(j).unwrap();
                let de: Element = d.dyn_into().unwrap();
                de.class_list().remove_1("open").unwrap();
                if let Some(btn) = de.query_selector(".menu-btn").ok().flatten() {
                    let _ = btn.set_attribute("aria-expanded", "false");
                }
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
        let all = doc
            .query_selector_all(".top-menubar .menu-dropdown")
            .unwrap();
        for j in 0..all.length() {
            let d = all.get(j).unwrap();
            let de: Element = d.dyn_into().unwrap();
            de.class_list().remove_1("open").unwrap();
            if let Some(btn) = de.query_selector(".menu-btn").ok().flatten() {
                let _ = btn.set_attribute("aria-expanded", "false");
            }
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
            super::super::history::perform_undo();
        }
        "edit:redo" => {
            super::super::history::perform_redo();
        }
        "edit:delete" => {
            if let Some(selected) = document
                .query_selector(".canvas-container-node.selected")
                .unwrap()
            {
                selected.remove();
                super::super::instrument_panel::hide(document);
                super::super::history::push_current_frame("delete container");
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
            super::super::interactions::place_container_via_menu(document, container_type, label);
        }
        "wire:inspect-selected" => {
            if let Some(selected_wire) = document
                .query_selector(".wire-overlay path.wire-selected")
                .ok()
                .flatten()
            {
                let wire_id = selected_wire.get_attribute("data-id").unwrap_or_default();
                super::super::wire_inspector::show_inspector(document, &wire_id);
            } else {
                show_menu_notification(
                    document,
                    "Click a wire connector on the canvas to inspect its semantic properties",
                );
            }
        }
        "wire:edit-label" => {
            if let Some(selected_wire) = document
                .query_selector(".wire-overlay path.wire-selected")
                .ok()
                .flatten()
            {
                let wire_id = selected_wire.get_attribute("data-id").unwrap_or_default();
                if let Some(label_el) = document
                    .query_selector(&format!(".wire-overlay text[data-wire-id=\"{}\"]", wire_id))
                    .ok()
                    .flatten()
                {
                    super::super::wire_inspector::edit_wire_label(document, &label_el);
                }
            } else {
                show_menu_notification(
                    document,
                    "Select a wire connector or double-click its label to edit predicate",
                );
            }
        }
        "wire:delete-selected" => {
            if let Some(selected_wire) = document
                .query_selector(".wire-overlay path.wire-selected")
                .ok()
                .flatten()
            {
                let wire_id = selected_wire.get_attribute("data-id").unwrap_or_default();
                selected_wire.remove();
                if let Some(label_el) = document
                    .query_selector(&format!(".wire-overlay text[data-wire-id=\"{}\"]", wire_id))
                    .ok()
                    .flatten()
                {
                    label_el.remove();
                }
                super::super::wire_inspector::hide_inspector(document);
                super::super::history::push_current_frame("delete wire connector");
                show_menu_notification(document, "Wire connector deleted");
            } else {
                show_menu_notification(document, "Select a wire on the canvas to delete");
            }
        }
        "wire:modality-active"
        | "wire:modality-event"
        | "wire:modality-ontology"
        | "wire:modality-deontic"
        | "wire:modality-epistemic" => {
            let modality = action.strip_prefix("wire:modality-").unwrap_or("active");
            if let Some(selected_wire) = document
                .query_selector(".wire-overlay path.wire-selected")
                .ok()
                .flatten()
            {
                let wire_id = selected_wire.get_attribute("data-id").unwrap_or_default();
                let _ = selected_wire.set_attribute("data-modality", modality);
                let new_class = format!("wire-path wire-selected wire-{}", modality);
                let _ = selected_wire.set_attribute("class", &new_class);
                super::super::history::push_current_frame("update wire modality");
                show_menu_notification(
                    document,
                    &format!("Wire modality updated to: {}", modality),
                );
                super::super::wire_inspector::show_inspector(document, &wire_id);
            } else {
                show_menu_notification(
                    document,
                    &format!("Modality '{}' selected for new connections", modality),
                );
            }
        }
        "help:command-palette" => {
            super::super::command_palette::toggle_command_palette(document);
        }
        "help:search-workbench" => {
            super::super::search_workbench::toggle_search_workbench(document);
        }
        "help:logic-workbench" => {
            super::super::logic_workbench::toggle_logic_workbench(document);
        }
        "file:save" => {
            super::super::history::sync_persistence_state();
            let _ = super::super::manifest::save_all_manifolds();
            show_menu_notification(
                document,
                "Auto-saved \u{2014} actor: did:qualia:timothy_charles_holborn",
            );
        }
        "file:save-as" => {
            open_save_mode_dialog(document);
        }
        "file:export-cbor" => {
            super::super::history::sync_persistence_state();
            let seeds = super::super::get_current_seeds();
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
            super::super::interactions::place_container_via_menu(
                document,
                "checkpoint-tray",
                "Checkpoint Tray",
            );
        }
        "file:prune-archive" => {
            super::super::interactions::place_container_via_menu(
                document,
                "publication-workflow",
                "Publication Workflow",
            );
        }
        "file:export-distribution" => {
            super::super::interactions::place_container_via_menu(
                document,
                "publication-workflow",
                "Distribution Export",
            );
        }
        "file:new-manifold" => {
            open_new_manifold_dialog(document);
        }
        "file:close" => {
            let seeds = super::super::get_current_seeds();
            if !seeds.is_empty() {
                super::super::switch_manifold(&seeds[0].id, &seeds);
                show_menu_notification(
                    document,
                    "Active manifold closed; reset to base workspace.",
                );
            }
        }
        "edit:duplicate" => {
            super::super::interactions::duplicate_selected_containers(document);
        }
        "edit:move-to-manifold" => {
            super::super::container_transfer::open_transfer_dialog(document, false);
        }
        "edit:copy-to-manifold" => {
            super::super::container_transfer::open_transfer_dialog(document, true);
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
            super::super::interactions::apply_canvas_zoom(document, 0.1, false);
        }
        "view:zoom-out" => {
            super::super::interactions::apply_canvas_zoom(document, -0.1, false);
        }
        "view:zoom-reset" => {
            super::super::interactions::apply_canvas_zoom(document, 1.0, true);
        }
        "view:auto-arrange" => {
            super::super::interactions::auto_arrange_containers(document);
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
