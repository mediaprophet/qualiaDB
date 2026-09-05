//! Shared helpers for the logic workbench panels.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
};

pub(super) fn make_textarea(document: &Document, id: &str, value: &str, height: &str) -> Element {
    let ta = document.create_element("textarea").unwrap();
    ta.set_id(id);
    let ta_el: HtmlTextAreaElement = ta.clone().dyn_into().unwrap();
    ta_el.set_value(value);
    let html: HtmlElement = ta.clone().dyn_into().unwrap();
    html.style().set_css_text(&format!(
        "width: 100%; box-sizing: border-box; height: {}; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 10px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--accent-cyan); \
         line-height: 1.5; resize: vertical;",
        height
    ));
    ta
}

pub(super) fn make_button(document: &Document, id: &str, label: &str, primary: bool) -> Element {
    let btn = document.create_element("button").unwrap();
    btn.set_id(id);
    btn.set_text_content(Some(label));
    let el: HtmlElement = btn.clone().dyn_into().unwrap();
    if primary {
        btn.set_attribute("data-primary-action", "true").unwrap();
        el.style().set_css_text(
            "padding: 8px 16px; background: var(--accent-violet); color: #fff; \
             border: 1px solid var(--accent-violet); border-radius: var(--radius-xs); \
             font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
        );
        btn.set_attribute("data-enabled-title", label).unwrap();
        let contract = super::requests::capability_contract_for_button(id);
        if let Some((attribute, value)) = contract {
            btn.set_attribute("data-requires-daemon", "true").unwrap();
            btn.set_attribute(attribute, value).unwrap();
        }
        let capability_available = contract.is_some_and(|(attribute, value)| {
            crate::browser::native_daemon::is_daemon_connected()
                && match attribute {
                    "data-capability-id" => {
                        crate::browser::native_daemon::native_capability_available(value)
                    }
                    "data-capability-prefix" => true,
                    _ => false,
                }
        });
        if !capability_available {
            btn.set_attribute("disabled", "").unwrap();
            btn.set_attribute("aria-disabled", "true").unwrap();
            let reason = if contract.is_some() {
                "Unavailable until a connected daemon advertises this native capability."
            } else {
                "Unavailable: this panel's native capability binding is still incomplete."
            };
            btn.set_attribute("title", reason).unwrap();
        }
    } else {
        el.style().set_css_text(
            "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
        );
    }
    btn
}

pub(super) fn make_results_area(document: &Document, id: &str, placeholder: &str) -> Element {
    let results = document.create_element("div").unwrap();
    results.set_id(id);
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    let initial = if placeholder.to_ascii_lowercase().contains("mock") {
        "No result yet. Availability is shown on the evaluation control."
    } else {
        placeholder
    };
    results.set_text_content(Some(initial));
    results
}

pub(super) fn make_section_label(document: &Document, text: &str) -> Element {
    let lbl = document.create_element("div").unwrap();
    let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "font-size: 10px; font-weight: 700; color: var(--text-secondary); \
         text-transform: uppercase; letter-spacing: 0.3px;",
    );
    lbl.set_text_content(Some(text));
    lbl
}

pub(super) fn make_tool_panel(document: &Document, tool_id: &str, visible: bool) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("logic-tool-panel");
    panel.set_attribute("data-tool", tool_id).unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(&format!(
        "display: {}; flex-direction: column; gap: 12px;",
        if visible { "flex" } else { "none" }
    ));
    panel
}

pub(super) fn make_select(document: &Document, id: &str, options: &[(&str, &str)]) -> Element {
    let sel = document.create_element("select").unwrap();
    sel.set_id(id);
    let s_el: HtmlElement = sel.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    for (key, display) in options {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", key).unwrap();
        opt.set_text_content(Some(display));
        sel.append_child(&opt).unwrap();
    }
    sel
}

pub(super) fn make_text_input(document: &Document, id: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_id(id);
    input.set_attribute("type", "text").unwrap();
    input.set_attribute("placeholder", placeholder).unwrap();
    let el: HtmlInputElement = input.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "flex: 1; padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    input
}

pub(super) fn show_logic_notification(document: &Document, message: &str) {
    if message.to_ascii_lowercase().contains("mock") {
        crate::browser::interactions::show_tool_status(
            document,
            "Logic workbench",
            "Unavailable: this operation has no registered native execution contract yet.",
            "unavailable",
        );
        return;
    }
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; \
         background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 10002; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!("\u{1F9E0} {}", message)));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    crate::browser::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

pub(super) fn field_value(document: &Document, id: &str) -> String {
    let Some(element) = document.get_element_by_id(id) else {
        return String::new();
    };
    if let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() {
        input.value()
    } else if let Ok(select) = element.clone().dyn_into::<HtmlSelectElement>() {
        select.value()
    } else if let Ok(textarea) = element.dyn_into::<HtmlTextAreaElement>() {
        textarea.value()
    } else {
        String::new()
    }
}

pub(super) fn show_mock_results(document: &Document, results_id: &str, tool_name: &str) {
    let results = match document.get_element_by_id(results_id) {
        Some(r) => r,
        None => return,
    };
    let (capability, args) = match super::requests::logic_request(document, tool_name) {
        Ok(request) => request,
        Err(reason) => {
            results.set_attribute("data-honesty", "unavailable").ok();
            results.set_text_content(Some(&format!("Unavailable: {reason}")));
            return;
        }
    };
    if !crate::browser::native_daemon::is_daemon_connected() {
        results.set_attribute("data-honesty", "unavailable").ok();
        results.set_text_content(Some(
            "Unavailable: start the local QualiaDB daemon to evaluate this panel against the live graph.",
        ));
        if capability.starts_with("Inference.") {
            results.set_attribute("data-inference-trail", "1").ok();
            let trail = crate::browser::diag_glow::build_inference_trail(
                document,
                "unavailable",
                capability,
            );
            let _ = results.append_child(&trail);
        }
        return;
    }

    results.set_attribute("data-honesty", "running").ok();
    results.set_attribute("data-beat", "dwell").ok();
    let inference = capability.starts_with("Inference.");
    results.set_text_content(Some(&format!(
        "Running {capability} against the live graph…"
    )));
    if inference {
        results.set_attribute("data-inference-trail", "1").ok();
        let trail =
            crate::browser::diag_glow::build_inference_trail(document, "running", capability);
        let _ = results.append_child(&trail);
    }
    let results_id = results_id.to_string();
    let capability_owned = capability.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let response = crate::browser::native_daemon::daemon_invoke(&capability_owned, args).await;
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(results) = document.get_element_by_id(&results_id) else {
            return;
        };
        match response {
            Ok(response) if response.ok => {
                results.set_attribute("data-honesty", "live").ok();
                results.set_text_content(Some(&response.value));
                if capability_owned.starts_with("Inference.") {
                    results.set_attribute("data-inference-trail", "1").ok();
                    let trail = crate::browser::diag_glow::build_inference_trail(
                        &document,
                        "live",
                        &capability_owned,
                    );
                    let _ = results.append_child(&trail);
                }
            }
            Ok(response) => {
                results.set_attribute("data-honesty", "error").ok();
                results.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Native evaluation failed without a diagnostic."),
                ));
                if capability_owned.starts_with("Inference.") {
                    let trail = crate::browser::diag_glow::build_inference_trail(
                        &document,
                        "error",
                        &capability_owned,
                    );
                    let _ = results.append_child(&trail);
                }
            }
            Err(error) => {
                results.set_attribute("data-honesty", "error").ok();
                results.set_text_content(Some(&error));
            }
        }
    });
}
