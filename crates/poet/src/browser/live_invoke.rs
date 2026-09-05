//! Shared live host-invoke controls for POET containers.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::native_daemon::{daemon_invoke, is_daemon_connected};

pub fn status_line(document: &Document) -> Element {
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-live-status", "true").ok();
    let el: HtmlElement = status.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); white-space: pre-wrap;",
    );
    status
}

fn coerce_field(raw: &str) -> serde_json::Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return serde_json::Value::Null;
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return serde_json::Value::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return serde_json::Value::Bool(false);
    }
    if let Ok(integer) = trimmed.parse::<i64>() {
        return serde_json::json!(integer);
    }
    if let Ok(number) = trimmed.parse::<f64>() {
        if number.is_finite() {
            return serde_json::json!(number);
        }
    }
    serde_json::Value::String(trimmed.to_string())
}

fn map_field_key(key: &str, value: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    match (key, value) {
        ("sex", serde_json::Value::String(sex)) if sex.eq_ignore_ascii_case("female") => vec![
            ("sex_female".into(), serde_json::Value::Bool(true)),
            ("sex_male".into(), serde_json::Value::Bool(false)),
        ],
        ("sex", serde_json::Value::String(sex)) if sex.eq_ignore_ascii_case("male") => vec![
            ("sex_female".into(), serde_json::Value::Bool(false)),
            ("sex_male".into(), serde_json::Value::Bool(true)),
        ],
        ("sys_bp", _) => vec![("systolic_bp".into(), value.clone())],
        ("chf", _) => vec![("congestive_heart_failure".into(), value.clone())],
        ("modality", serde_json::Value::String(modality)) => vec![(
            "modality".into(),
            serde_json::Value::String(modality.to_ascii_lowercase()),
        )],
        _ => vec![(key.to_string(), value.clone())],
    }
}

pub fn collect_cop_fields(root: &Element) -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    if let Ok(inputs) = root.query_selector_all("[data-cop-field]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.get(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-cop-field") else {
                continue;
            };
            let coerced = coerce_field(&input.value());
            if coerced.is_null() {
                continue;
            }
            for (mapped, value) in map_field_key(&key, &coerced) {
                fields.insert(mapped, value);
            }
        }
    }
    if let Some(title) = root
        .query_selector("[data-cop-title]")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        let value = title.value();
        if !value.trim().is_empty() {
            fields.insert("title".into(), serde_json::Value::String(value));
        }
    }
    fields
}

fn nearest_cop_root(button: &Element) -> Option<Element> {
    if let Some(root) = button.closest("[data-cop-family]").ok().flatten() {
        return Some(root);
    }
    button
        .parent_element()
        .and_then(|parent| parent.parent_element())
        .and_then(|grand| grand.query_selector("[data-cop-family]").ok().flatten())
}

fn merge_args(
    base: serde_json::Value,
    extra: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    match base {
        serde_json::Value::Object(mut map) => {
            for (key, value) in extra {
                map.insert(key, value);
            }
            serde_json::Value::Object(map)
        }
        other => other,
    }
}

pub fn invoke_button(
    document: &Document,
    label: &str,
    capability: &'static str,
    args: serde_json::Value,
    status: &Element,
) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_text_content(Some(label));
    button.set_attribute("type", "button").ok();
    button
        .set_attribute("data-live-capability", capability)
        .ok();
    if !is_daemon_connected() {
        button.set_attribute("disabled", "").ok();
        button
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    let status = status.clone();
    let button_for_click = button.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !is_daemon_connected() {
            status.set_text_content(Some("Unavailable: start the local QualiaDB daemon."));
            return;
        }
        let extra = nearest_cop_root(&button_for_click)
            .map(|root| collect_cop_fields(&root))
            .unwrap_or_default();
        let args = merge_args(args.clone(), extra);
        status.set_text_content(Some(&format!("Running {capability}…")));
        let status_async = status.clone();
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
                            .unwrap_or("Native invoke failed."),
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
    button
}

pub fn action_bar(
    document: &Document,
    actions: &[(&str, &'static str, serde_json::Value)],
) -> Element {
    let bar = document.create_element("div").unwrap();
    let bar_el: HtmlElement = bar.clone().dyn_into().unwrap();
    bar_el
        .style()
        .set_css_text("display: flex; flex-wrap: wrap; gap: 6px; margin: 6px 0;");
    let status = status_line(document);
    for (label, capability, args) in actions {
        bar.append_child(&invoke_button(
            document,
            label,
            capability,
            args.clone(),
            &status,
        ))
        .unwrap();
    }
    let wrap = document.create_element("div").unwrap();
    wrap.append_child(&bar).unwrap();
    wrap.append_child(&status).unwrap();
    wrap
}
