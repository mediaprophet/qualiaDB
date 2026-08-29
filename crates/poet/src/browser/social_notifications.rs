//! Recipient-controlled local mention notification inbox.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement};

use super::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_social_notifications(document: &Document, _workspace: &Element) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-notifications", "").ok();
    root.set_inner_html(
        "<strong>Mention notifications</strong>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         These are local inbox receipts, not proof that another host delivered or displayed a message.</p>",
    );
    let refresh = button(document, "Refresh mentions");
    root.append_child(&refresh).unwrap();
    let status = document.create_element("span").unwrap();
    status
        .set_attribute("data-social-notification-status", "")
        .ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-social-notification-list", "").ok();
    root.append_child(&list).unwrap();
    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_notifications(&click_root);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    refresh_notifications(&root);
    root
}

pub fn refresh_all_notifications() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(roots) = document.query_selector_all("[data-social-notifications]") else {
        return;
    };
    for index in 0..roots.length() {
        if let Some(root) = roots
            .item(index)
            .and_then(|node| node.dyn_into::<Element>().ok())
        {
            refresh_notifications(&root);
        }
    }
}

fn refresh_notifications(root: &Element) {
    let actor = actor(root);
    if !valid_did(&actor) {
        set_status(
            root,
            " Enter your DID in the Social header, then refresh mentions.",
        );
        return;
    }
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "social_notification".into(),
            ..Default::default()
        })
        .await
        {
            Ok(response) if response.ok => render(&root, &response.data, &actor),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Mention notification query rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn render(root: &Element, data: &serde_json::Value, actor: &str) {
    let Some(list) = root
        .query_selector("[data-social-notification-list]")
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
    let mut unread = 0usize;
    for record in records.iter().rev() {
        let Some(fields) = record.get("fields").and_then(serde_json::Value::as_object) else {
            continue;
        };
        if field(fields, "recipient") != actor {
            continue;
        }
        shown += 1;
        let is_unread = field(fields, "status") == "unread";
        unread += usize::from(is_unread);
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} · #{} · from {} · message {} ",
            field(fields, "status"),
            field(fields, "thread"),
            field(fields, "from"),
            field(fields, "source_message_id")
        )));
        if is_unread {
            let action = button(&document, "Mark read");
            let root_click = root.clone();
            let record = record.clone();
            let actor = actor.to_string();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                mark_read(&root_click, &record, &actor);
            }) as Box<dyn FnMut(_)>);
            action
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&action).unwrap();
        }
        list.append_child(&row).unwrap();
    }
    set_status(
        root,
        &format!(" {unread} unread / {shown} local mention receipt(s)."),
    );
}

fn mark_read(root: &Element, record: &serde_json::Value, actor: &str) {
    let mut fields = record
        .get("fields")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    if field(&fields, "recipient") != actor {
        set_status(root, " Only the notification recipient may mark it read.");
        return;
    }
    fields.insert("status".into(), serde_json::json!("read"));
    fields.insert("acted_by".into(), serde_json::json!(actor));
    fields.insert(
        "acted_at".into(),
        serde_json::json!(((js_sys::Date::now() / 1000.0) as u64).to_string()),
    );
    let request = NativeRecordUpsertRequest {
        family: "social_notification".into(),
        title: record
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Mention notification")
            .into(),
        id: record
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        fields,
    };
    set_status(root, " Recording read state…");
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(request).await {
            Ok(response) if response.ok => refresh_notifications(&root),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Read transition rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn actor(root: &Element) -> String {
    root.parent_element()
        .and_then(|workspace| {
            workspace
                .query_selector("[data-social-field=\"from\"]")
                .ok()
                .flatten()
        })
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn field<'a>(fields: &'a serde_json::Map<String, serde_json::Value>, key: &str) -> &'a str {
    fields
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn valid_did(value: &str) -> bool {
    value.starts_with("did:") && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn set_status(root: &Element, text: &str) {
    if let Ok(Some(status)) = root.query_selector("[data-social-notification-status]") {
        status.set_text_content(Some(text));
    }
}
