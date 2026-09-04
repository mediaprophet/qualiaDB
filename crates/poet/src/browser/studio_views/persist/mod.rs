//! Studio surfaces are Scene / Audio / Animation sessions, not a nested DAW.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::native_daemon::{daemon_invoke, is_daemon_connected};

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

fn banner(document: &Document, text: &str) -> Element {
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(text));
    let el: HtmlElement = note.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 8px;",
    );
    note
}

fn invoke_on_click(
    button: &Element,
    capability: &'static str,
    args: serde_json::Value,
    status: Element,
) {
    if !is_daemon_connected() {
        button.set_attribute("disabled", "").ok();
        button
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !is_daemon_connected() {
            status.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to run this session capability.",
            ));
            return;
        }
        status.set_text_content(Some(&format!("Running {capability}…")));
        let status_async = status.clone();
        let args = args.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_invoke(capability, args).await {
                Ok(response) if response.ok => {
                    status_async.set_attribute("data-honesty", "live").ok();
                    status_async.set_text_content(Some(&response.value));
                }
                Ok(response) => {
                    status_async.set_attribute("data-honesty", "error").ok();
                    status_async.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Native session invoke failed."),
                    ));
                }
                Err(error) => {
                    status_async.set_attribute("data-honesty", "error").ok();
                    status_async.set_text_content(Some(&error));
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}


mod sessions;
mod editors;
mod surfaces;

pub use editors::*;
pub use sessions::*;
pub use surfaces::*;

mod tests {
    #[test]
    fn studio_sessions_are_not_a_nested_daw() {
        let families = [
            "studio_scene",
            "studio_audio",
            "studio_animation",
            "studio_asset",
        ];
        assert_eq!(families.len(), 4);
    }
}

