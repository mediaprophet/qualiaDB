//! Copy and move selected containers between Poet manifolds.

use std::collections::{BTreeSet, HashMap, HashSet};

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement};

use crate::tool_chest::core::registry::{ManifoldSeed, SeedConnection};

pub fn open_transfer_dialog(document: &Document, copy: bool) {
    let return_focus = document.active_element();
    let selected = selected_container_ids(document);
    if selected.is_empty() {
        super::interactions::show_tool_status(
            document,
            if copy {
                "Copy containers"
            } else {
                "Move containers"
            },
            "Select one or more containers first.",
            "info",
        );
        return;
    }
    let current_id = super::current_manifold_id();
    let targets: Vec<_> = super::visible_seeds()
        .into_iter()
        .filter(|seed| seed.id != current_id)
        .collect();
    if targets.is_empty() {
        super::interactions::show_tool_status(
            document,
            "Cross-manifold operation",
            "This construct has no other manifold available.",
            "error",
        );
        return;
    }
    if let Some(existing) = document.get_element_by_id("container-transfer-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("container-transfer-dialog");
    overlay.set_class_name("dialog-overlay");
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dialog-panel container-transfer-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();

    let header = document.create_element("div").unwrap();
    header.set_class_name("dialog-header");
    let title = document.create_element("div").unwrap();
    title.set_class_name("dialog-title");
    title.set_text_content(Some(if copy {
        "\u{1F4CB} Copy to manifold"
    } else {
        "\u{1F4E6} Move to manifold"
    }));
    header.append_child(&title).unwrap();
    let close = document.create_element("button").unwrap();
    close.set_class_name("dialog-close-btn");
    close.set_attribute("type", "button").unwrap();
    close.set_attribute("aria-label", "Close").unwrap();
    close.set_text_content(Some("\u{2715}"));
    header.append_child(&close).unwrap();
    panel.append_child(&header).unwrap();

    let body = document.create_element("div").unwrap();
    body.set_class_name("dialog-body");
    let summary = document.create_element("div").unwrap();
    summary.set_class_name("transfer-summary");
    summary.set_text_content(Some(&format!(
        "{} selected container{} from {}",
        selected.len(),
        if selected.len() == 1 { "" } else { "s" },
        current_id
    )));
    body.append_child(&summary).unwrap();
    let label = document.create_element("label").unwrap();
    label.set_class_name("form-group");
    let label_text = document.create_element("span").unwrap();
    label_text.set_class_name("form-label");
    label_text.set_text_content(Some("Destination manifold"));
    label.append_child(&label_text).unwrap();
    let select = document.create_element("select").unwrap();
    select.set_id("container-transfer-target");
    select.set_class_name("form-select");
    for target in &targets {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", &target.id).unwrap();
        option.set_text_content(Some(&format!("{} {}", target.icon, target.label)));
        select.append_child(&option).unwrap();
    }
    label.append_child(&select).unwrap();
    body.append_child(&label).unwrap();

    let open_label = document.create_element("label").unwrap();
    open_label.set_class_name("transfer-open-target");
    let open = document.create_element("input").unwrap();
    open.set_id("container-transfer-open-target");
    open.set_attribute("type", "checkbox").unwrap();
    open.set_attribute("checked", "checked").unwrap();
    open_label.append_child(&open).unwrap();
    let open_text = document.create_element("span").unwrap();
    open_text.set_text_content(Some("Open the destination after transfer"));
    open_label.append_child(&open_text).unwrap();
    body.append_child(&open_label).unwrap();
    panel.append_child(&body).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_class_name("dialog-footer");
    let cancel = document.create_element("button").unwrap();
    cancel.set_class_name("btn btn-secondary");
    cancel.set_text_content(Some("Cancel"));
    let confirm = document.create_element("button").unwrap();
    confirm.set_class_name("btn btn-primary");
    confirm.set_text_content(Some(if copy {
        "Copy containers"
    } else {
        "Move containers"
    }));
    footer.append_child(&cancel).unwrap();
    footer.append_child(&confirm).unwrap();
    panel.append_child(&footer).unwrap();
    overlay.append_child(&panel).unwrap();
    document.body().unwrap().append_child(&overlay).unwrap();
    super::accessibility::wire_modal_accessibility(
        document,
        &overlay,
        &panel,
        return_focus,
        Some(select.clone()),
    );

    for button in [close, cancel] {
        let overlay = overlay.clone();
        let closure =
            Closure::wrap(Box::new(move |_event: Event| overlay.remove()) as Box<dyn FnMut(Event)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let overlay_for_confirm = overlay.clone();
    let closure = Closure::wrap(Box::new(move |_event: Event| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let target_id = document
            .get_element_by_id("container-transfer-target")
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        let open_target = document
            .get_element_by_id("container-transfer-open-target")
            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.checked())
            .unwrap_or(true);
        overlay_for_confirm.remove();
        execute_transfer(
            &document,
            &current_id,
            &target_id,
            &selected,
            copy,
            open_target,
        );
    }) as Box<dyn FnMut(Event)>);
    confirm
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn selected_container_ids(document: &Document) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let Ok(nodes) = document.query_selector_all(".canvas-container-node.selected[data-id]") else {
        return ids;
    };
    for index in 0..nodes.length() {
        if let Some(id) = nodes
            .get(index)
            .and_then(|node| node.dyn_into::<Element>().ok())
            .and_then(|element| element.get_attribute("data-id"))
        {
            ids.insert(id);
        }
    }
    ids
}

fn execute_transfer(
    document: &Document,
    source_id: &str,
    target_id: &str,
    selected: &BTreeSet<String>,
    copy: bool,
    open_target: bool,
) {
    if target_id.is_empty() || target_id == source_id {
        super::interactions::show_tool_status(
            document,
            "Cross-manifold operation",
            "Choose a different destination.",
            "error",
        );
        return;
    }
    super::history::sync_persistence_state();
    let result = super::CURRENT_SEEDS.with(|slot| {
        let mut seeds = slot.borrow_mut();
        let source_index = seeds.iter().position(|seed| seed.id == source_id)?;
        let target_index = seeds.iter().position(|seed| seed.id == target_id)?;
        let source_snapshot = seeds[source_index].clone();
        let mut updated_source = source_snapshot.clone();
        let mut updated_target = seeds[target_index].clone();
        let count = transfer_models(&mut updated_source, &mut updated_target, selected, copy);
        seeds[target_index] = updated_target;
        if !copy {
            seeds[source_index] = updated_source.clone();
        }
        Some((count, updated_source))
    });
    let Some((count, updated_source)) = result else {
        super::interactions::show_tool_status(
            document,
            "Cross-manifold operation",
            "The source or destination manifold is unavailable.",
            "error",
        );
        return;
    };
    if count == 0 {
        super::interactions::show_tool_status(
            document,
            "Cross-manifold operation",
            "The selected containers are no longer available.",
            "error",
        );
        return;
    }

    if !copy {
        super::history::commit_external_seed(updated_source.clone(), "move containers to manifold");
        super::rerender_canvas(&updated_source);
    }
    let _ = super::manifest::save_all_manifolds();
    if open_target {
        super::switch_to_sibling_manifold(target_id);
    }
    super::interactions::show_tool_status(
        document,
        if copy {
            "Containers copied"
        } else {
            "Containers moved"
        },
        &format!(
            "{} container{} transferred to {}. Internal wires were preserved.",
            count,
            if count == 1 { "" } else { "s" },
            target_id
        ),
        "success",
    );
}

fn transfer_models(
    source: &mut ManifoldSeed,
    target: &mut ManifoldSeed,
    selected: &BTreeSet<String>,
    copy: bool,
) -> usize {
    let selected_indices: Vec<usize> = source
        .containers
        .iter()
        .enumerate()
        .filter_map(|(index, container)| selected.contains(&container.id).then_some(index))
        .collect();
    if selected_indices.is_empty() {
        return 0;
    }
    let selected_set: HashSet<usize> = selected_indices.iter().copied().collect();
    let mut target_ids: HashSet<String> = target
        .containers
        .iter()
        .map(|container| container.id.clone())
        .collect();
    let mut old_to_target = HashMap::new();
    for (serial, old_index) in selected_indices.iter().copied().enumerate() {
        let mut container = source.containers[old_index].clone();
        container.id = unique_id(&container.id, &target.id, &mut target_ids);
        container.x += 28.0 + serial as f32 * 12.0;
        container.y += 28.0 + serial as f32 * 12.0;
        container.z = target.containers.len() as f32 + 100.0;
        old_to_target.insert(old_index, target.containers.len());
        target.containers.push(container);
    }
    let mut target_wire_ids: HashSet<String> = target
        .connections
        .iter()
        .map(|wire| wire.id.clone())
        .collect();
    for connection in &source.connections {
        if let (Some(&from), Some(&to)) = (
            old_to_target.get(&connection.from),
            old_to_target.get(&connection.to),
        ) {
            let mut wire = connection.clone();
            wire.id = unique_id(&wire.id, &target.id, &mut target_wire_ids);
            wire.from = from;
            wire.to = to;
            target.connections.push(wire);
        }
    }

    if !copy {
        let mut old_to_remaining = HashMap::new();
        let mut remaining = Vec::with_capacity(source.containers.len() - selected_indices.len());
        for (old_index, container) in source.containers.drain(..).enumerate() {
            if !selected_set.contains(&old_index) {
                old_to_remaining.insert(old_index, remaining.len());
                remaining.push(container);
            }
        }
        source.containers = remaining;
        source.connections = source
            .connections
            .iter()
            .filter_map(|connection| {
                let (&from, &to) = (
                    old_to_remaining.get(&connection.from)?,
                    old_to_remaining.get(&connection.to)?,
                );
                Some(SeedConnection {
                    from,
                    to,
                    ..connection.clone()
                })
            })
            .collect();
    }
    selected_indices.len()
}

fn unique_id(base: &str, target_id: &str, existing: &mut HashSet<String>) -> String {
    let stem = format!("{}-{}", base, target_id);
    let mut candidate = stem.clone();
    let mut serial = 2usize;
    while existing.contains(&candidate) {
        candidate = format!("{stem}-{serial}");
        serial += 1;
    }
    existing.insert(candidate.clone());
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::registry::SeedContainer;

    #[test]
    fn move_preserves_internal_wires_and_remaps_source() {
        let mut source = ManifoldSeed {
            id: "a".into(),
            ..Default::default()
        };
        source.containers = vec![
            SeedContainer {
                id: "one".into(),
                ..SeedContainer::new("doc", "One", 0.0, 0.0, 300.0, 200.0)
            },
            SeedContainer {
                id: "two".into(),
                ..SeedContainer::new("doc", "Two", 20.0, 20.0, 300.0, 200.0)
            },
            SeedContainer {
                id: "stay".into(),
                ..SeedContainer::new("doc", "Stay", 40.0, 40.0, 300.0, 200.0)
            },
        ];
        source.connections = vec![
            SeedConnection {
                id: "internal".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "p".into(),
            },
            SeedConnection {
                id: "external".into(),
                from: 1,
                to: 2,
                wire_type: "active".into(),
                label: "q".into(),
            },
        ];
        let mut target = ManifoldSeed {
            id: "b".into(),
            ..Default::default()
        };
        let selected = BTreeSet::from(["one".to_string(), "two".to_string()]);

        assert_eq!(
            transfer_models(&mut source, &mut target, &selected, false),
            2
        );
        assert_eq!(source.containers.len(), 1);
        assert!(source.connections.is_empty());
        assert_eq!(target.containers.len(), 2);
        assert_eq!(target.connections.len(), 1);
        assert_eq!(
            (target.connections[0].from, target.connections[0].to),
            (0, 1)
        );
    }
}
