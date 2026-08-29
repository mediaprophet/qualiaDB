//! Persistent connector execution receipts with bounded pure-operation retry.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element};

use crate::browser::native_daemon::{
    daemon_invoke, daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_connector_runs(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-connector-runs", "").ok();
    root.set_inner_html(
        "<strong>Connector execution history</strong>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         Failed Pure operations may be retried up to three attempts. Cold or unknown-effect operations require a fresh explicit run.</p>",
    );
    let refresh = button(document, "Refresh run history");
    root.append_child(&refresh).unwrap();
    let status = document.create_element("span").unwrap();
    status.set_attribute("data-connector-run-status", "").ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-connector-run-list", "").ok();
    root.append_child(&list).unwrap();
    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_runs(&click_root);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    refresh_runs(&root);
    root
}

pub fn execute_and_record(
    status: &Element,
    output: Option<Element>,
    connector_id: String,
    capability_id: String,
    raw_args: String,
    effect_class: String,
    attempt: u8,
) {
    let Ok(args) = serde_json::from_str::<serde_json::Value>(&raw_args) else {
        status.set_text_content(Some("Connector arguments must be a JSON object."));
        return;
    };
    if !args.is_object() || !(1..=3).contains(&attempt) {
        status.set_text_content(Some(
            "Connector run requires object arguments and attempt 1..=3.",
        ));
        return;
    }
    status.set_text_content(Some("Invoking connector and recording its receipt…"));
    let status = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let started_at = unix_now();
        let invocation = daemon_invoke(&capability_id, args).await;
        let finished_at = unix_now();
        let (run_status, diagnostic, value) = match invocation {
            Ok(response) if response.ok => ("succeeded", String::new(), response.value),
            Ok(response) => (
                "failed",
                response
                    .diagnostic
                    .unwrap_or_else(|| "Connector returned a typed failure.".into()),
                response.value,
            ),
            Err(error) => ("failed", error, String::new()),
        };
        if let Some(output) = output {
            output.set_text_content(Some(if run_status == "succeeded" {
                &value
            } else {
                &diagnostic
            }));
        }
        let fields = serde_json::Map::from_iter([
            ("connector_id".into(), serde_json::json!(connector_id)),
            ("capability_id".into(), serde_json::json!(capability_id)),
            ("status".into(), serde_json::json!(run_status)),
            ("attempt".into(), serde_json::json!(attempt.to_string())),
            (
                "started_at".into(),
                serde_json::json!(started_at.to_string()),
            ),
            (
                "finished_at".into(),
                serde_json::json!(finished_at.to_string()),
            ),
            ("effect_class".into(), serde_json::json!(effect_class)),
            ("probe_args".into(), serde_json::json!(raw_args)),
            (
                "diagnostic".into(),
                serde_json::json!(bounded(&diagnostic, 900)),
            ),
            ("value".into(), serde_json::json!(bounded(&value, 900))),
        ]);
        let receipt = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "project_connector_run".into(),
            title: format!("Connector {run_status} · attempt {attempt}"),
            id: None,
            fields,
        })
        .await;
        status.set_text_content(Some(match receipt {
            Ok(response) if response.ok && run_status == "succeeded" => {
                "Connector completed; execution receipt persisted."
            }
            Ok(response) if response.ok => "Connector failed; failure receipt persisted.",
            _ => "Connector finished, but its execution receipt could not be persisted.",
        }));
        refresh_all();
    });
}

fn refresh_runs(root: &Element) {
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "project_connector_run".into(),
            ..Default::default()
        })
        .await
        {
            Ok(response) if response.ok => render(&root, &response.data),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Run history rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn render(root: &Element, data: &serde_json::Value) {
    let Some(list) = root
        .query_selector("[data-connector-run-list]")
        .ok()
        .flatten()
    else {
        return;
    };
    list.set_inner_html("");
    let document = root.owner_document().unwrap();
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for record in records.iter().rev().take(50) {
        let Some(fields) = record.get("fields").and_then(serde_json::Value::as_object) else {
            continue;
        };
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} · {} · attempt {} · {} ",
            field(fields, "connector_id"),
            field(fields, "status"),
            field(fields, "attempt"),
            field(fields, "diagnostic")
        )));
        let attempt = field(fields, "attempt").parse::<u8>().unwrap_or(3);
        if field(fields, "status") == "failed"
            && field(fields, "effect_class").eq_ignore_ascii_case("pure")
            && attempt < 3
        {
            let retry = button(&document, "Retry pure operation");
            let root_click = root.clone();
            let connector_id = field(fields, "connector_id").to_string();
            let capability_id = field(fields, "capability_id").to_string();
            let raw = field(fields, "probe_args").to_string();
            let effect = field(fields, "effect_class").to_string();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                let status = root_click
                    .query_selector("[data-connector-run-status]")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| root_click.clone());
                execute_and_record(
                    &status,
                    None,
                    connector_id.clone(),
                    capability_id.clone(),
                    raw.clone(),
                    effect.clone(),
                    attempt + 1,
                );
            }) as Box<dyn FnMut(_)>);
            retry
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&retry).unwrap();
        }
        list.append_child(&row).unwrap();
    }
    set_status(
        root,
        &format!(" {} retained run receipt(s).", records.len()),
    );
}

fn refresh_all() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(roots) = document.query_selector_all("[data-connector-runs]") else {
        return;
    };
    for index in 0..roots.length() {
        if let Some(root) = roots
            .item(index)
            .and_then(|node| node.dyn_into::<Element>().ok())
        {
            refresh_runs(&root);
        }
    }
}

fn unix_now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
fn bounded(value: &str, max: usize) -> String {
    if value.len() <= max {
        value.into()
    } else {
        let mut end = max;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &value[..end])
    }
}
fn field<'a>(fields: &'a serde_json::Map<String, serde_json::Value>, key: &str) -> &'a str {
    fields
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}
fn set_status(root: &Element, text: &str) {
    if let Ok(Some(status)) = root.query_selector("[data-connector-run-status]") {
        status.set_text_content(Some(text));
    }
}
