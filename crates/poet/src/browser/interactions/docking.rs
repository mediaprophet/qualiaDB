//! Tool Chest docking, flyouts, chain activation, and selector controls.

use super::*;

/// Wire up toolbox dock button selection + flyout panel, and family
/// header collapse/expand.
pub fn wire_toolbox_dock(document: &Document) {
    // Wire family header collapse/expand
    let headers = document.query_selector_all(".dock-family-header").unwrap();
    for i in 0..headers.length() {
        let header = headers.get(i).unwrap();
        let header_el: Element = header.dyn_into().unwrap();
        let family_id = header_el.get_attribute("data-family").unwrap_or_default();
        let header_for_toggle = header_el.clone();

        let closure = Closure::wrap(Box::new(move |e: Event| {
            let me: MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            // Find the children container for this family
            if let Some(section) = doc
                .query_selector(&format!(
                    ".dock-family-section[data-family=\"{}\"]",
                    family_id
                ))
                .unwrap()
            {
                if let Some(children) = section.query_selector(".dock-family-children").unwrap() {
                    if children.class_list().contains("expanded") {
                        children.class_list().remove_1("expanded").unwrap();
                        let _ = header_for_toggle.set_attribute("aria-expanded", "false");
                        if let Ok(buttons) = children.query_selector_all(".toolbox-dock-btn") {
                            for index in 0..buttons.length() {
                                if let Some(node) = buttons.get(index) {
                                    if let Ok(button) = node.dyn_into::<Element>() {
                                        let _ = button.class_list().remove_1("active");
                                    }
                                }
                            }
                        }
                        super::super::docks::hide_flyout(&doc);
                    } else {
                        children.class_list().add_1("expanded").unwrap();
                        let _ = header_for_toggle.set_attribute("aria-expanded", "true");
                        // Open first toolbox in this family
                        if let Ok(Some(first_btn)) = children.query_selector(".toolbox-dock-btn") {
                            let tb_id = first_btn.get_attribute("data-toolbox").unwrap_or_default();
                            if !tb_id.is_empty() {
                                if let Ok(all) = doc.query_selector_all(".toolbox-dock-btn") {
                                    for index in 0..all.length() {
                                        if let Some(node) = all.get(index) {
                                            if let Ok(button) = node.dyn_into::<Element>() {
                                                let _ = button.class_list().remove_1("active");
                                            }
                                        }
                                    }
                                }
                                let _ = first_btn.class_list().add_1("active");
                                super::super::docks::show_flyout(&doc, &tb_id);
                                wire_flyout_tools(&doc);
                            }
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        header_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire toolbox button selection + flyout
    let buttons = document.query_selector_all(".toolbox-dock-btn").unwrap();
    for i in 0..buttons.length() {
        let btn = buttons.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let toolbox_id = btn_el.get_attribute("data-toolbox").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();

            // Toggle active state on all dock buttons
            let all = doc.query_selector_all(".toolbox-dock-btn").unwrap();
            let mut clicked_is_active = false;
            for j in 0..all.length() {
                let n = all.get(j).unwrap();
                let ne: Element = n.dyn_into().unwrap();
                if ne.get_attribute("data-toolbox").as_deref() == Some(&toolbox_id) {
                    clicked_is_active = ne.class_list().contains("active");
                    ne.class_list().remove_1("active").unwrap();
                } else {
                    ne.class_list().remove_1("active").unwrap();
                }
            }

            if clicked_is_active {
                // Was active — hide flyout
                super::super::docks::hide_flyout(&doc);
            } else {
                // Was not active — activate and show flyout
                let all2 = doc.query_selector_all(".toolbox-dock-btn").unwrap();
                for j in 0..all2.length() {
                    let n = all2.get(j).unwrap();
                    let ne: Element = n.dyn_into().unwrap();
                    if ne.get_attribute("data-toolbox").as_deref() == Some(&toolbox_id) {
                        ne.class_list().add_1("active").unwrap();
                        break;
                    }
                }
                super::super::docks::show_flyout(&doc, &toolbox_id);
                // Wire tool button clicks in the flyout
                wire_flyout_tools(&doc);
            }
        }) as Box<dyn FnMut(Event)>);

        btn_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Apply a real workspace grid layout for the selected toolbox anchor.
pub fn apply_toolbox_position(document: &Document, position: &str) {
    let position = match position {
        "top" | "right" | "bottom" => position,
        _ => "left",
    };
    if let Ok(Some(dock)) = document.query_selector(".toolbox-dock") {
        dock.set_class_name(&format!("toolbox-dock dock-pos-{}", position));
    }
    if let Ok(Some(workspace)) = document.query_selector(".main-workspace") {
        workspace.set_class_name(&format!("main-workspace dock-layout-{}", position));
    }
    if let Ok(Some(flyout)) = document.query_selector(".toolbox-flyout") {
        flyout.set_class_name(&format!("toolbox-flyout dock-{}", position));
    }
    if let Ok(buttons) = document.query_selector_all(".dock-pos-btn") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.get(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<HtmlElement>() else {
                continue;
            };
            let active = button.get_attribute("data-pos").as_deref() == Some(position);
            let _ = button.style().set_property(
                "color",
                if active {
                    "var(--accent-cyan)"
                } else {
                    "var(--text-muted)"
                },
            );
            let _ = button.style().set_property(
                "border-color",
                if active {
                    "var(--accent-cyan)"
                } else {
                    "transparent"
                },
            );
            let _ = button.set_attribute("aria-pressed", if active { "true" } else { "false" });
        }
    }
}

/// Wire tool button clicks and tool-chain selection/drag inside the flyout panel.
/// Placement is local; every other action is routed through the shared honest
/// action dispatcher.
/// Tool-chain labels are clickable (activate on focused surface) and draggable
/// (drag onto a container to activate there).
pub fn wire_flyout_tools(document: &Document) {
    // Wire tool-chain label clicks (select/activate chain)
    let chain_labels = document.query_selector_all(".toolchain-label").unwrap();
    for i in 0..chain_labels.length() {
        let label = chain_labels.get(i).unwrap();
        let label_el: Element = label.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&label_el, "flyout-chain") {
            continue;
        }
        let chain_id = label_el.get_attribute("data-chain-id").unwrap_or_default();
        let chain_id_for_drag = chain_id.clone();

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Toggle selected state on this chain label
            let all_labels = doc.query_selector_all(".toolchain-label").unwrap();
            let mut was_selected = false;
            for j in 0..all_labels.length() {
                let l = all_labels.get(j).unwrap();
                let le: Element = l.dyn_into().unwrap();
                if le.get_attribute("data-chain-id").as_deref() == Some(&chain_id) {
                    was_selected = le.class_list().contains("selected");
                    le.class_list().remove_1("selected").unwrap();
                } else {
                    le.class_list().remove_1("selected").unwrap();
                }
            }
            if !was_selected {
                // Activate this chain
                for j in 0..all_labels.length() {
                    let l = all_labels.get(j).unwrap();
                    let le: Element = l.dyn_into().unwrap();
                    if le.get_attribute("data-chain-id").as_deref() == Some(&chain_id) {
                        le.class_list().add_1("selected").unwrap();
                        break;
                    }
                }
                // Show the chain's tools on the contextual instrument panel
                super::super::instrument_panel::activate_chain(&doc, &chain_id);
            } else {
                // Deactivate — clear chain from instrument panel
                super::super::instrument_panel::deactivate_chain(&doc);
            }
        }) as Box<dyn FnMut(Event)>);

        label_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        // Wire drag start
        let drag_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.stop_propagation();
            if let Some(dt) = de.data_transfer() {
                dt.set_data("application/x-toolchain-id", &chain_id_for_drag)
                    .unwrap();
                dt.set_effect_allowed("copy");
            }
        }) as Box<dyn FnMut(Event)>);

        label_el
            .add_event_listener_with_callback("dragstart", drag_closure.as_ref().unchecked_ref())
            .unwrap();
        drag_closure.forget();
    }

    // Wire container drop zones for tool-chain drag
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let container = containers.get(i).unwrap();
        let container_el: Element = container.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&container_el, "chain-drop") {
            continue;
        }
        let container_el_for_drop = container_el.clone();
        let container_el_for_over = container_el.clone();

        let drop_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.prevent_default();
            if let Some(dt) = de.data_transfer() {
                if let Ok(chain_id) = dt.get_data("application/x-toolchain-id") {
                    if !chain_id.is_empty() {
                        // Activate this chain on this container
                        let doc = web_sys::window().unwrap().document().unwrap();
                        super::super::instrument_panel::activate_chain_on_container(
                            &doc, &chain_id,
                        );
                        // Select the container to show the instrument panel
                        let all = doc.query_selector_all(".canvas-container-node").unwrap();
                        for j in 0..all.length() {
                            let n = all.get(j).unwrap();
                            let ne: Element = n.dyn_into().unwrap();
                            ne.class_list().remove_1("selected").unwrap();
                        }
                        container_el_for_drop
                            .class_list()
                            .add_1("selected")
                            .unwrap();
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        container_el
            .add_event_listener_with_callback("drop", drop_closure.as_ref().unchecked_ref())
            .unwrap();
        drop_closure.forget();

        let dragover_closure = Closure::wrap(Box::new(move |e: Event| {
            let de: web_sys::DragEvent = e.dyn_into().unwrap();
            de.prevent_default();
            if let Some(dt) = de.data_transfer() {
                dt.set_drop_effect("copy");
            }
        }) as Box<dyn FnMut(Event)>);

        container_el_for_over
            .add_event_listener_with_callback("dragover", dragover_closure.as_ref().unchecked_ref())
            .unwrap();
        dragover_closure.forget();
    }

    // Wire tool button clicks
    let tools = document.query_selector_all(".tool-btn").unwrap();
    for i in 0..tools.length() {
        let tool = tools.get(i).unwrap();
        let tool_el: Element = tool.dyn_into().unwrap();
        if !super::super::dom_bindings::claim(&tool_el, "flyout-tool") {
            continue;
        }
        let tool_id = tool_el.get_attribute("data-tool-id").unwrap_or_default();
        let label = tool_el
            .query_selector(".tool-btn-label")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();
        let kind_badge = tool_el
            .query_selector(".tool-btn-kind")
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
            .unwrap_or_default();
        let action = match tool_el.get_attribute("data-action").as_deref() {
            Some("query") => crate::tool_chest::core::intent_bus::ActionType::Query,
            Some("mutate") => crate::tool_chest::core::intent_bus::ActionType::Mutate,
            Some("publish") => crate::tool_chest::core::intent_bus::ActionType::Publish,
            Some("validate") => crate::tool_chest::core::intent_bus::ActionType::Validate,
            Some("navigate") => crate::tool_chest::core::intent_bus::ActionType::Navigate,
            Some("annotate") => crate::tool_chest::core::intent_bus::ActionType::Annotate,
            Some("cancel") => crate::tool_chest::core::intent_bus::ActionType::Cancel,
            _ => crate::tool_chest::core::intent_bus::ActionType::Invoke,
        };

        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if kind_badge == "place" {
                // PlaceContainer tool — extract container type from tool_id
                // tool_id format: "toolbox:place_<type>" e.g. "office:place_doc"
                let container_type = tool_id.split("place_").nth(1).unwrap_or("doc").to_string();
                place_container_on_canvas(&doc, &container_type, &label);
            } else {
                super::super::tool_actions::dispatch(&doc, &tool_id, &label, action);
            }
        }) as Box<dyn FnMut(Event)>);

        tool_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
/// Wire up tray toggle buttons (dimension, time span, epistemic radio items).
/// These are dynamically created when a pod drop-tray opens, so this function
/// should be called after tray population. It uses event delegation on the
/// document to handle dynamically created buttons.
pub fn wire_selector_buttons(document: &Document) {
    // Wire tray toggle buttons via event delegation
    let closure = Closure::wrap(Box::new(move |e: Event| {
        let target: Element = match e.target() {
            Some(t) => t.dyn_into().unwrap(),
            None => return,
        };
        if !target.class_list().contains("tray-toggle-btn") {
            return;
        }

        // Find sibling buttons in the same row and deactivate them
        if let Some(parent) = target.parent_element() {
            let siblings = parent.query_selector_all(".tray-toggle-btn").unwrap();
            for j in 0..siblings.length() {
                let s = siblings.get(j).unwrap();
                let se: Element = s.dyn_into().unwrap();
                se.class_list().remove_1("active").unwrap();
            }
        }
        target.class_list().add_1("active").unwrap();
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire epistemic radio items
    let closure2 = Closure::wrap(Box::new(move |e: Event| {
        let target: Element = match e.target() {
            Some(t) => t.dyn_into().unwrap(),
            None => return,
        };
        if !target.class_list().contains("tray-radio-item") {
            return;
        }

        // Deactivate all radio items in the same tray
        let doc = web_sys::window().unwrap().document().unwrap();
        let all = doc.query_selector_all(".tray-radio-item").unwrap();
        for j in 0..all.length() {
            let n = all.get(j).unwrap();
            let ne: Element = n.dyn_into().unwrap();
            ne.class_list().remove_1("active").unwrap();
        }
        target.class_list().add_1("active").unwrap();
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback("click", closure2.as_ref().unchecked_ref())
        .unwrap();
    closure2.forget();
}
