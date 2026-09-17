//! Incoming relationship/channel decisions and recent social activity.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement};

use super::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_social_inbox(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-inbox", "").ok();
    root.set_inner_html(
        "<strong>Invitations and activity inbox</strong>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         Decisions are DID-attributed ledger transitions. Channel invites can only originate from the channel creator.</p>",
    );
    for (key, placeholder) in [
        ("actor", "Your DID"),
        ("channel", "Invitation-only channel id"),
        ("invitee", "Invitee DID"),
    ] {
        let input = document.create_element("input").unwrap();
        input.set_attribute("data-social-inbox-field", key).ok();
        input.set_attribute("placeholder", placeholder).ok();
        root.append_child(&input).unwrap();
    }
    let invite = button(document, "Invite to channel");
    let refresh = button(document, "Refresh inbox");
    root.append_child(&invite).unwrap();
    root.append_child(&refresh).unwrap();
    let status = document.create_element("span").unwrap();
    status.set_attribute("data-social-inbox-status", "").ok();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(" Inbox not loaded."));
    root.append_child(&status).unwrap();
    let list = document.create_element("div").unwrap();
    list.set_attribute("data-social-inbox-list", "").ok();
    root.append_child(&list).unwrap();
    let activity = document.create_element("div").unwrap();
    activity.set_attribute("data-social-activity-list", "").ok();
    root.append_child(&activity).unwrap();

    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        create_invite(&click_root);
    }) as Box<dyn FnMut(_)>);
    invite
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_inbox(&click_root);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    refresh_inbox(&root);
    root
}

fn create_invite(root: &Element) {
    let from = value(root, "actor");
    let to = value(root, "invitee");
    let channel = value(root, "channel");
    if !valid_did(&from) || !valid_did(&to) || from == to || channel.trim().is_empty() {
        set_status(
            root,
            " Enter distinct creator/invitee DIDs and a channel id.",
        );
        return;
    }
    let fields = serde_json::Map::from_iter([
        ("from".into(), serde_json::json!(from.trim())),
        ("to".into(), serde_json::json!(to.trim())),
        ("status".into(), serde_json::json!("pending")),
        ("request_type".into(), serde_json::json!("channel-invite")),
        (
            "scope".into(),
            serde_json::json!(format!("channel:{}", channel.trim())),
        ),
    ]);
    let request = NativeRecordUpsertRequest {
        family: "social_request".into(),
        title: format!("Invite {} to #{}", to.trim(), channel.trim()),
        id: None,
        fields,
    };
    set_status(root, " Persisting invitation…");
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(request).await {
            Ok(response) if response.ok => refresh_inbox(&root),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Invitation rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn refresh_inbox(root: &Element) {
    let actor = value(root, "actor");
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let requests = daemon_records_query(NativeRecordQueryRequest {
            family: "social_request".into(),
            ..Default::default()
        })
        .await;
        let activity = daemon_records_query(NativeRecordQueryRequest {
            family: "pulse_event".into(),
            ..Default::default()
        })
        .await;
        if let Ok(response) = requests {
            if response.ok {
                render_requests(&root, &response.data, actor.trim());
            }
        }
        if let Ok(response) = activity {
            if response.ok {
                render_activity(&root, &response.data);
            }
        }
    });
}

fn render_requests(root: &Element, data: &serde_json::Value, actor: &str) {
    let Some(list) = root
        .query_selector("[data-social-inbox-list]")
        .ok()
        .flatten()
    else {
        return;
    };
    list.set_inner_html("<small>Incoming decisions</small>");
    let document = root.owner_document().unwrap();
    let mut shown = 0usize;
    for record in records(data).iter().rev() {
        let Some(fields) = record.get("fields").and_then(serde_json::Value::as_object) else {
            continue;
        };
        if field(fields, "to") != actor || field(fields, "status") != "pending" {
            continue;
        }
        shown += 1;
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} · {} · {} ",
            field(fields, "from"),
            field(fields, "request_type"),
            field(fields, "scope")
        )));
        for (label, next) in [
            ("Accept", "accepted"),
            ("Deny", "denied"),
            ("Block", "blocked"),
        ] {
            let action = button(&document, label);
            let root_click = root.clone();
            let record = record.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                decide(&root_click, &record, next);
            }) as Box<dyn FnMut(_)>);
            action
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&action).unwrap();
        }
        list.append_child(&row).unwrap();
    }
    set_status(root, &format!(" {shown} incoming request(s)."));
}

fn decide(root: &Element, record: &serde_json::Value, next: &'static str) {
    let actor = value(root, "actor");
    let mut fields = record
        .get("fields")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    if actor.trim() != field(&fields, "to") {
        set_status(root, " Only the receiving DID may decide this request.");
        return;
    }
    fields.insert("status".into(), serde_json::json!(next));
    fields.insert("acted_by".into(), serde_json::json!(actor.trim()));
    fields.insert(
        "acted_at".into(),
        serde_json::json!(((js_sys::Date::now() / 1000.0) as u64).to_string()),
    );
    let request_type = field(&fields, "request_type").to_string();
    let scope = field(&fields, "scope").to_string();
    let from = field(&fields, "from").to_string();
    let to = field(&fields, "to").to_string();
    let request = NativeRecordUpsertRequest {
        family: "social_request".into(),
        title: record
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Social request")
            .into(),
        id: record
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        fields,
    };
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(request).await {
            Ok(response) if response.ok => {
                if next == "accepted"
                    && matches!(
                        request_type.as_str(),
                        "channel-membership" | "channel-invite"
                    )
                {
                    let (participant, authorised_by) = if request_type == "channel-invite" {
                        (to, from)
                    } else {
                        (from, to)
                    };
                    let member = NativeRecordUpsertRequest {
                        family: "manifold_participant".into(),
                        title: "Accepted channel membership".into(),
                        id: None,
                        fields: serde_json::Map::from_iter([
                            ("manifold".into(), serde_json::json!(scope)),
                            ("participant".into(), serde_json::json!(participant)),
                            ("role".into(), serde_json::json!("member")),
                            ("status".into(), serde_json::json!("active")),
                            ("authorised_by".into(), serde_json::json!(authorised_by)),
                        ]),
                    };
                    if !daemon_records_upsert(member)
                        .await
                        .is_ok_and(|response| response.ok)
                    {
                        set_status(&root, " Request decided, but membership activation failed.");
                        return;
                    }
                }
                refresh_inbox(&root);
            }
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Decision rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn render_activity(root: &Element, data: &serde_json::Value) {
    let Some(list) = root
        .query_selector("[data-social-activity-list]")
        .ok()
        .flatten()
    else {
        return;
    };
    list.set_inner_html("<small>Recent persisted Pulse activity</small>");
    for record in records(data).iter().rev().take(10) {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let row = root
            .owner_document()
            .unwrap()
            .create_element("div")
            .unwrap();
        row.set_text_content(Some(&format!(
            "{} · {}",
            field_opt(fields, "channel"),
            field_opt(fields, "payload_type")
        )));
        list.append_child(&row).unwrap();
    }
}

fn records(data: &serde_json::Value) -> &[serde_json::Value] {
    data.get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn field<'a>(fields: &'a serde_json::Map<String, serde_json::Value>, key: &str) -> &'a str {
    fields
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn field_opt<'a>(
    fields: Option<&'a serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> &'a str {
    fields
        .and_then(|f| f.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-social-inbox-field=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
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
    if let Ok(Some(status)) = root.query_selector("[data-social-inbox-status]") {
        status.set_text_content(Some(text));
    }
}
