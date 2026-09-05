//! Shared live native-render preview panel for standalone POET containers.
//!
//! Still / clip / scene are handle kinds over the live `/render/preview` bind
//! (B-007). No sibling preview op. Unbound looks gated, never stub-broken.

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{Document, Element};

const HANDLE_KINDS: &[(&str, &str, &str)] = &[
    ("still", "Still", "2d"),
    ("clip", "Clip", "film"),
    ("scene", "Scene", "3d"),
];

fn surface_for_kind(kind: &str) -> &'static str {
    match kind {
        "clip" => "film",
        "scene" | "submanifold" => "3d",
        "map" => "map",
        "media" | "still" => "2d",
        _ => "cg",
    }
}

fn initial_handle(kind: &str) -> &'static str {
    match kind {
        "clip" => "clip",
        "scene" | "submanifold" => "scene",
        _ => "still",
    }
}

/// Compact studio dock used by the right-hand Preview panel.
pub fn build_studio_dock(document: &Document) -> Element {
    let wrap = document.create_element("div").unwrap();
    wrap.set_class_name("studio-preview-dock");
    wrap.set_attribute("data-shape", "container").ok();
    super::surface_aspects::mark(&wrap, "entrance");
    wrap.set_attribute("data-media-surface", "2d").ok();
    wrap.append_child(&build(document, "still", 640, 360))
        .unwrap();
    wrap
}

pub fn build(document: &Document, kind: &str, width: u32, height: u32) -> Element {
    let handle0 = initial_handle(kind);
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("native-render-preview");
    panel.set_attribute("data-render-kind", kind).unwrap();
    panel.set_attribute("data-handle-kind", handle0).unwrap();
    panel
        .set_attribute("data-media-surface", surface_for_kind(handle0))
        .unwrap();
    super::surface_aspects::mark(&panel, "entrance");
    panel.set_attribute("data-honesty", "unavailable").ok();

    let handles = document.create_element("div").unwrap();
    handles.set_class_name("preview-handle-row");
    handles.set_attribute("role", "tablist").ok();
    handles
        .set_attribute("aria-label", "Preview handle kind")
        .ok();
    for (id, label, _surface) in HANDLE_KINDS {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("preview-handle-tab");
        tab.set_attribute("type", "button").unwrap();
        tab.set_attribute("role", "tab").ok();
        tab.set_attribute("data-handle-kind", id).unwrap();
        tab.set_attribute(
            "aria-pressed",
            if *id == handle0 { "true" } else { "false" },
        )
        .unwrap();
        if *id == handle0 {
            tab.class_list().add_1("active").ok();
        }
        tab.set_text_content(Some(label));
        handles.append_child(&tab).unwrap();
    }
    panel.append_child(&handles).unwrap();

    let kind_label = document.create_element("div").unwrap();
    kind_label.set_class_name("preview-kind-label");
    kind_label.set_text_content(Some(&handle_caption(handle0)));
    panel.append_child(&kind_label).unwrap();

    let button = document.create_element("button").unwrap();
    button.set_class_name("vibe-run-btn native-render-preview-btn");
    button.set_text_content(Some("Render live GPU preview"));
    button
        .set_attribute("data-requires-daemon", "true")
        .unwrap();
    button
        .set_attribute("data-enabled-title", "Render a live native PNG preview")
        .unwrap();
    if !super::native_daemon::is_daemon_connected() {
        button.set_attribute("disabled", "").unwrap();
        button.set_attribute("aria-disabled", "true").unwrap();
        button
            .set_attribute(
                "title",
                "Unavailable until a local QualiaDB daemon with webizen-render is connected.",
            )
            .unwrap();
    }
    panel.append_child(&button).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("native-render-preview-status");
    status.set_attribute("role", "status").unwrap();
    status.set_attribute("data-honesty", "unavailable").unwrap();
    status.set_text_content(Some(
        "No native frame requested. A connected renderer provider is required. Still, clip, and scene are handle kinds on the same Render preview bind — not a second pipeline.",
    ));
    panel.append_child(&status).unwrap();

    let stage = document.create_element("div").unwrap();
    stage.set_class_name("preview-stage");
    let image = document.create_element("img").unwrap();
    image.set_class_name("native-render-preview-image");
    image
        .set_attribute("alt", &format!("Live {handle0} render preview"))
        .unwrap();
    image.set_attribute("hidden", "").unwrap();
    stage.append_child(&image).unwrap();
    panel.append_child(&stage).unwrap();

    let timeline = document.create_element("div").unwrap();
    timeline.set_class_name("preview-timeline");
    timeline.set_attribute("aria-hidden", "true").ok();
    panel.append_child(&timeline).unwrap();

    wire_handle_tabs(&panel, &kind_label, &image);
    wire_render_click(
        &button,
        &panel,
        &status,
        &image,
        kind.to_string(),
        width,
        height,
    );

    panel
}

fn handle_caption(handle: &str) -> String {
    match handle {
        "clip" => "Clip · Timeline handle on live Render preview".into(),
        "scene" => "Scene · Stage / camera handle on live Render preview".into(),
        _ => "Still · Layout frame on live Render preview".into(),
    }
}

fn wire_handle_tabs(panel: &Element, kind_label: &Element, image: &Element) {
    let tabs = panel.query_selector_all(".preview-handle-tab").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap().dyn_into::<Element>().unwrap();
        let panel = panel.clone();
        let kind_label = kind_label.clone();
        let image = image.clone();
        let tab_for_listen = tab.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let handle = tab
                .get_attribute("data-handle-kind")
                .unwrap_or_else(|| "still".into());
            panel.set_attribute("data-handle-kind", &handle).ok();
            panel
                .set_attribute("data-media-surface", surface_for_kind(&handle))
                .ok();
            kind_label.set_text_content(Some(&handle_caption(&handle)));
            image
                .set_attribute("alt", &format!("Live {handle} render preview"))
                .ok();
            let all = panel.query_selector_all(".preview-handle-tab").unwrap();
            for j in 0..all.length() {
                let other = all.get(j).unwrap().dyn_into::<Element>().unwrap();
                let on =
                    other.get_attribute("data-handle-kind").as_deref() == Some(handle.as_str());
                other
                    .set_attribute("aria-pressed", if on { "true" } else { "false" })
                    .ok();
                let _ = other.class_list().toggle_with_force("active", on);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        tab_for_listen
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn wire_render_click(
    button: &Element,
    panel: &Element,
    status: &Element,
    image: &Element,
    fallback_kind: String,
    width: u32,
    height: u32,
) {
    let status_for_click = status.clone();
    let image_for_click = image.clone();
    let panel_for_click = panel.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let handle = panel_for_click
            .get_attribute("data-handle-kind")
            .unwrap_or_else(|| fallback_kind.clone());
        panel_for_click
            .set_attribute("data-honesty", "running")
            .ok();
        panel_for_click.set_attribute("data-beat", "dwell").ok();
        status_for_click
            .set_attribute("data-honesty", "running")
            .ok();
        status_for_click
            .set_text_content(Some(&format!("Rendering {handle} on the native GPU host…")));
        image_for_click.set_attribute("hidden", "").ok();
        let status = status_for_click.clone();
        let image = image_for_click.clone();
        let panel = panel_for_click.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match super::native_daemon::daemon_render_preview(&handle, width, height).await {
                Ok(response) if response.ok => {
                    let Some(data_uri) = response
                        .data_uri
                        .filter(|uri| uri.starts_with("data:image/png;base64,"))
                    else {
                        panel.set_attribute("data-honesty", "error").ok();
                        status.set_attribute("data-honesty", "error").ok();
                        status.set_text_content(Some(
                            "Renderer reported success without a valid PNG data URI.",
                        ));
                        return;
                    };
                    image.set_attribute("src", &data_uri).ok();
                    image.remove_attribute("hidden").ok();
                    panel.set_attribute("data-honesty", "live").ok();
                    panel.set_attribute("data-beat", "dwell").ok();
                    status.set_attribute("data-honesty", "live").ok();
                    status.set_text_content(Some(&format!(
                        "Live {handle} · {}×{} · {} nodes · {} edges · {} faces",
                        response.width,
                        response.height,
                        response.node_count,
                        response.edge_count,
                        response.face_count
                    )));
                }
                Ok(response) => {
                    panel.set_attribute("data-honesty", "unavailable").ok();
                    status.set_attribute("data-honesty", "unavailable").ok();
                    status.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("The native renderer returned no frame."),
                    ));
                }
                Err(error) => {
                    panel.set_attribute("data-honesty", "error").ok();
                    status.set_attribute("data-honesty", "error").ok();
                    status.set_text_content(Some(&error));
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_kinds_map_to_media_surfaces() {
        assert_eq!(surface_for_kind("still"), "2d");
        assert_eq!(surface_for_kind("clip"), "film");
        assert_eq!(surface_for_kind("scene"), "3d");
        assert_eq!(surface_for_kind("map"), "map");
        assert_eq!(initial_handle("submanifold"), "scene");
        assert_eq!(initial_handle("media"), "still");
    }
}
