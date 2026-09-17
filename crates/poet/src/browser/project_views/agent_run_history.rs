//! Durable completed/cancelled/failed local-agent run receipts.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element};

use crate::browser::native_daemon::{daemon_records_query, NativeRecordQueryRequest};

pub fn build_agent_run_history(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-agent-run-history", "").ok();
    root.set_inner_html(
        "<strong>Agent run history</strong>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         Terminal operational receipts include completed, cancelled and failed runs; they are separate from conversation turns.</p>",
    );
    let refresh = document.create_element("button").unwrap();
    refresh.set_attribute("type", "button").ok();
    refresh.set_text_content(Some("Refresh run history"));
    root.append_child(&refresh).unwrap();
    let status = document.create_element("span").unwrap();
    status.set_attribute("data-agent-run-status", "").ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-agent-run-list", "").ok();
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

pub fn refresh_all_agent_runs() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(roots) = document.query_selector_all("[data-agent-run-history]") else {
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

fn refresh_runs(root: &Element) {
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "project_agent_run".into(),
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
                    .unwrap_or(" Run history query rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn render(root: &Element, data: &serde_json::Value) {
    let Some(list) = root.query_selector("[data-agent-run-list]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for record in records.iter().rev().take(50) {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let row = root
            .owner_document()
            .unwrap()
            .create_element("div")
            .unwrap();
        row.set_text_content(Some(&format!(
            "{} · {} / {} tokens · {} ms · {}",
            field(fields, "status"),
            field(fields, "tokens_generated"),
            field(fields, "token_budget"),
            field(fields, "duration_ms"),
            field(fields, "agent_did")
        )));
        if !field(fields, "diagnostic").is_empty() {
            row.set_attribute("title", field(fields, "diagnostic")).ok();
        }
        list.append_child(&row).unwrap();
    }
    set_status(
        root,
        &format!(" {} terminal run receipt(s).", records.len()),
    );
}

fn field<'a>(fields: Option<&'a serde_json::Map<String, serde_json::Value>>, key: &str) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn set_status(root: &Element, text: &str) {
    if let Ok(Some(status)) = root.query_selector("[data-agent-run-status]") {
        status.set_text_content(Some(text));
    }
}
