//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Canvas container chrome: header, body slot, ports, and resize handle.

use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

use super::attrs::{container_type_filter_attrs, container_type_tag, media_surface_for};
use super::body::fill_body;

/// Build a single container node on the canvas.
pub fn build_container(document: &Document, container: &SeedContainer) -> Element {
    let el = document.create_element("div").unwrap();
    el.set_class_name(&format!(
        "canvas-container-node container-card container-kind-{}",
        container.kind.class_suffix()
    ));
    let container_id = if container.id.is_empty() {
        crate::browser::canvas_state::next_container_id(&container.container_type)
    } else {
        container.id.clone()
    };
    el.set_attribute("data-id", &container_id).unwrap();
    el.set_attribute("data-shape", "container").unwrap();
    crate::browser::surface_aspects::mark(&el, "entrance");
    el.set_attribute(
        "data-media-surface",
        media_surface_for(&container.container_type),
    )
    .unwrap();
    el.set_attribute("data-container-type", &container.container_type)
        .unwrap();
    el.set_attribute("data-semantic-type", &container.semantic_type)
        .unwrap();
    el.set_attribute("data-semantic-uri", &container.semantic_uri)
        .unwrap();
    if !container.target_manifold.is_empty() {
        el.set_attribute("data-target-manifold", &container.target_manifold)
            .unwrap();
    }
    if !container.target_construct.is_empty() {
        el.set_attribute("data-target-construct", &container.target_construct)
            .unwrap();
    }
    el.set_attribute("role", "group").unwrap();
    el.set_attribute("tabindex", "0").unwrap();
    el.set_attribute(
        "aria-label",
        &format!("{} {} container", container.title, container.container_type),
    )
    .unwrap();

    // Set strata and epistemic data attributes for filtering
    let (strata, epistemic) = container_type_filter_attrs(&container.container_type);
    el.set_attribute("data-strata", strata).unwrap();
    el.set_attribute("data-epistemic", epistemic).unwrap();

    let style = format!(
        "left: {}px; top: {}px; width: {}px; height: {}px; z-index: {};",
        container.x.round() as i32,
        container.y.round() as i32,
        container.width.round() as i32,
        container.height.round() as i32,
        container.z.round() as i32
    );
    el.set_attribute("style", &style).unwrap();

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("container-header");

    let title_group = document.create_element("div").unwrap();
    title_group.set_class_name("container-title-group");

    // Type tag
    let tag = document.create_element("span").unwrap();
    let (tag_class, tag_label) = container_type_tag(&container.container_type);
    tag.set_class_name(&format!("container-type-tag {}", tag_class));
    tag.set_text_content(Some(tag_label));
    title_group.append_child(&tag).unwrap();

    // Title
    let title = document.create_element("span").unwrap();
    title.set_class_name("container-title");
    title.set_text_content(Some(&container.title));
    title_group.append_child(&title).unwrap();

    // Honesty badge
    let badge = document.create_element("span").unwrap();
    badge.set_class_name(&format!("honesty-badge honesty-{}", container.honesty));
    badge.set_text_content(Some(&container.honesty));
    title_group.append_child(&badge).unwrap();

    title_group
        .append_child(&crate::browser::surface_aspects::chip_row(document))
        .unwrap();

    header.append_child(&title_group).unwrap();

    // Shared lifecycle chrome (settings, minimise, close).
    let actions = crate::browser::container_chrome::build_header_actions(document);
    header.append_child(&actions).unwrap();
    el.append_child(&header).unwrap();

    // Body
    let body = document.create_element("div").unwrap();
    body.set_class_name("container-body");

    fill_body(document, container, &body);

    if !container.content_html.is_empty() {
        if let Ok(Some(editor)) = body.query_selector(".doc-editor") {
            editor.set_inner_html(&container.content_html);
        }
    }
    el.append_child(&body).unwrap();
    crate::browser::tool_widgets::restore_container_settings(&el, &container.tool_settings);
    crate::browser::view_state::restore(&el, &container.view_state);
    crate::browser::surface_honesty::enforce(document, &body, &container.container_type);

    // Connection ports
    let port_in = document.create_element("button").unwrap();
    port_in.set_class_name("container-port port-in");
    port_in.set_attribute("type", "button").unwrap();
    port_in
        .set_attribute(
            "aria-label",
            "Input port: connect an incoming semantic wire",
        )
        .unwrap();
    port_in.set_attribute("data-port", "in").unwrap();
    port_in
        .set_attribute("title", "Input Port: drop incoming reactive wire here")
        .unwrap();
    el.append_child(&port_in).unwrap();

    let port_out = document.create_element("button").unwrap();
    port_out.set_class_name("container-port port-out");
    port_out.set_attribute("type", "button").unwrap();
    port_out
        .set_attribute("aria-label", "Output port: start a semantic wire")
        .unwrap();
    port_out.set_attribute("data-port", "out").unwrap();
    port_out
        .set_attribute(
            "title",
            "Output Port: drag to connect reactive wire to another container",
        )
        .unwrap();
    el.append_child(&port_out).unwrap();

    // Resize handle
    let resizer = document.create_element("div").unwrap();
    resizer.set_class_name("container-resizer resize-handle");
    resizer
        .set_attribute("title", "Drag to resize container")
        .unwrap();
    resizer.set_attribute("role", "separator").unwrap();
    resizer
        .set_attribute("aria-label", "Resize container")
        .unwrap();
    el.append_child(&resizer).unwrap();

    crate::browser::container_chrome::restore_chrome_state(&el, container);

    el
}
