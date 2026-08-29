//! Shared live native-render preview panel for standalone POET containers.

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

pub fn build(document: &Document, kind: &str, width: u32, height: u32) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("native-render-preview");
    panel.set_attribute("data-render-kind", kind).unwrap();

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
        "No native frame requested. A connected renderer provider is required.",
    ));
    panel.append_child(&status).unwrap();

    let image = document.create_element("img").unwrap();
    image.set_class_name("native-render-preview-image");
    image
        .set_attribute("alt", &format!("Live {kind} render preview"))
        .unwrap();
    image.set_attribute("hidden", "").unwrap();
    panel.append_child(&image).unwrap();

    let kind = kind.to_string();
    let status_for_click = status.clone();
    let image_for_click = image.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        status_for_click
            .set_attribute("data-honesty", "running")
            .ok();
        status_for_click.set_text_content(Some("Rendering on the native GPU host…"));
        image_for_click.set_attribute("hidden", "").ok();
        let kind = kind.clone();
        let status = status_for_click.clone();
        let image = image_for_click.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match super::native_daemon::daemon_render_preview(&kind, width, height).await {
                Ok(response) if response.ok => {
                    let Some(data_uri) = response
                        .data_uri
                        .filter(|uri| uri.starts_with("data:image/png;base64,"))
                    else {
                        status.set_attribute("data-honesty", "error").ok();
                        status.set_text_content(Some(
                            "Renderer reported success without a valid PNG data URI.",
                        ));
                        return;
                    };
                    image.set_attribute("src", &data_uri).ok();
                    image.remove_attribute("hidden").ok();
                    status.set_attribute("data-honesty", "live").ok();
                    status.set_text_content(Some(&format!(
                        "Live {}×{} frame · {} nodes · {} edges · {} faces",
                        response.width,
                        response.height,
                        response.node_count,
                        response.edge_count,
                        response.face_count
                    )));
                }
                Ok(response) => {
                    status.set_attribute("data-honesty", "unavailable").ok();
                    status.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("The native renderer returned no frame."),
                    ));
                }
                Err(error) => {
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

    let panel_style: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_style.style().set_css_text(
        "display:flex;flex-direction:column;gap:6px;padding:6px;border-top:1px solid var(--border-subtle);",
    );
    panel
}
