//! Human review queue for persisted local-model assertions.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement};

use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_agent_review_queue(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-agent-review", "").ok();
    root.set_inner_html(
        "<h4 style=\"margin:0\">Model assertion review</h4>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         Approval records human review; it does not execute tools or publish graph claims.</p>",
    );
    let reviewer = document.create_element("input").unwrap();
    reviewer.set_attribute("data-agent-reviewer", "").ok();
    reviewer.set_attribute("placeholder", "Reviewer DID").ok();
    root.append_child(&reviewer).unwrap();
    let refresh = button(document, "Refresh review queue");
    root.append_child(&refresh).unwrap();
    let status = document.create_element("span").unwrap();
    status.set_attribute("data-agent-review-status", "").ok();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(" Review queue not loaded."));
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-agent-review-list", "").ok();
    root.append_child(&list).unwrap();

    let root_click = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_queue(&root_click);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    refresh_queue(&root);
    root
}

fn refresh_queue(root: &Element) {
    set_status(root, " Loading model assertions…");
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "project_agent".into(),
            kind: "turn".into(),
            ..Default::default()
        })
        .await
        {
            Ok(response) if response.ok => render_queue(&root, &response.data),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Review queue request was rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn render_queue(root: &Element, data: &serde_json::Value) {
    let Some(list) = root
        .query_selector("[data-agent-review-list]")
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
    let mut shown = 0usize;
    for record in records.iter().rev() {
        let Some(fields) = record.get("fields").and_then(serde_json::Value::as_object) else {
            continue;
        };
        let review = field(fields, "review_status");
        if !review.is_empty() && review != "pending" {
            continue;
        }
        shown += 1;
        let row = document.create_element("article").unwrap();
        row.set_attribute(
            "style",
            "border:1px solid var(--border-medium);padding:6px;margin-top:5px;border-radius:5px",
        )
        .ok();
        let prompt = field(fields, "prompt");
        let response = field(fields, "response");
        let title = record
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Model turn");
        row.set_inner_html(&format!(
            "<strong>{}</strong><div>Prompt: {}</div><div>Response: {}</div>",
            escape(title),
            escape(prompt),
            escape(response)
        ));
        for (label, decision) in [("Approve", "approved"), ("Reject", "rejected")] {
            let action = button(&document, label);
            let root_click = root.clone();
            let id = record
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = title.to_string();
            let updated = fields.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                decide(&root_click, &id, &title, updated.clone(), decision);
            }) as Box<dyn FnMut(_)>);
            action
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&action).unwrap();
        }
        list.append_child(&row).unwrap();
    }
    set_status(root, &format!(" {shown} assertion(s) awaiting review."));
}

fn decide(
    root: &Element,
    id: &str,
    title: &str,
    mut fields: serde_json::Map<String, serde_json::Value>,
    decision: &'static str,
) {
    let reviewer = root
        .query_selector("[data-agent-reviewer]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default();
    if !reviewer.trim().starts_with("did:") {
        set_status(root, " Enter a reviewer DID before deciding.");
        return;
    }
    fields.insert("review_status".into(), serde_json::json!(decision));
    fields.insert("reviewed_by".into(), serde_json::json!(reviewer.trim()));
    fields.insert(
        "reviewed_at".into(),
        serde_json::json!(((js_sys::Date::now() / 1000.0) as u64).to_string()),
    );
    let request = NativeRecordUpsertRequest {
        family: "project_agent".into(),
        title: title.into(),
        id: Some(id.into()),
        fields,
    };
    set_status(root, " Recording review decision…");
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(request).await {
            Ok(response) if response.ok => refresh_queue(&root),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Review was rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
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

fn set_status(root: &Element, message: &str) {
    if let Ok(Some(status)) = root.query_selector("[data-agent-review-status]") {
        status.set_text_content(Some(message));
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
