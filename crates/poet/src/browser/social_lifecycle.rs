//! Channels and explicit relationship-request transitions.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_social_lifecycle(document: &Document, workspace: &Element) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-lifecycle", "").ok();
    style(&root, "border:1px solid var(--border-medium);border-radius:6px;padding:8px;display:flex;flex-direction:column;gap:7px;");
    let title = document.create_element("strong").unwrap();
    title.set_text_content(Some("Channels, membership and relationships"));
    root.append_child(&title).unwrap();
    root.append_child(&input(document, "actor", "Your DID for social actions"))
        .unwrap();

    let channel_form = document.create_element("div").unwrap();
    style(
        &channel_form,
        "display:grid;grid-template-columns:1fr 1fr;gap:5px;",
    );
    for (key, placeholder) in [
        ("channel-id", "Channel id"),
        ("topic", "Semantic topic IRI"),
        ("visibility", "public | private | restricted"),
        ("membership", "open | request | invite"),
    ] {
        channel_form
            .append_child(&input(document, key, placeholder))
            .unwrap();
    }
    let create_channel = button(document, "Create channel");
    channel_form.append_child(&create_channel).unwrap();
    root.append_child(&channel_form).unwrap();

    let request_form = document.create_element("div").unwrap();
    style(&request_form, "display:flex;gap:5px;");
    request_form
        .append_child(&input(document, "request-to", "Connection target DID"))
        .unwrap();
    let send_request = button(document, "Request connection");
    request_form.append_child(&send_request).unwrap();
    root.append_child(&request_form).unwrap();

    let lists = document.create_element("div").unwrap();
    style(
        &lists,
        "display:grid;grid-template-columns:1fr 1fr;gap:7px;",
    );
    for (attribute, empty) in [
        ("data-social-channels", "No channels loaded."),
        ("data-social-requests", "No requests loaded."),
    ] {
        let list = document.create_element("div").unwrap();
        list.set_attribute(attribute, "").ok();
        list.set_text_content(Some(empty));
        lists.append_child(&list).unwrap();
    }
    root.append_child(&lists).unwrap();
    let refresh = button(document, "Refresh channels and requests");
    root.append_child(&refresh).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "font:9px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let local_root = root.clone();
    let local_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        create_channel_record(&local_root, &local_workspace);
    }) as Box<dyn FnMut(_)>);
    create_channel
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let local_root = root.clone();
    let local_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        create_request(&local_root, &local_workspace);
    }) as Box<dyn FnMut(_)>);
    send_request
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let local_root = root.clone();
    let local_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_lifecycle(&local_root, &local_workspace);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    refresh_lifecycle(&root, workspace);
    root
}

fn create_channel_record(root: &Element, workspace: &Element) {
    let actor = value(root, "actor");
    let channel_id = value(root, "channel-id");
    let topic = value(root, "topic");
    if !valid_did(&actor) || channel_id.trim().is_empty() || !topic.contains(':') {
        set_status(
            root,
            "Enter your DID, a channel id, and a semantic topic IRI.",
        );
        return;
    }
    let fields = serde_json::Map::from_iter([
        ("channel_id".into(), serde_json::json!(channel_id.trim())),
        ("creator_did".into(), serde_json::json!(actor.trim())),
        (
            "visibility".into(),
            serde_json::json!(defaulted(&value(root, "visibility"), "private")),
        ),
        (
            "membership".into(),
            serde_json::json!(defaulted(&value(root, "membership"), "invite")),
        ),
        ("semantic_topic_iri".into(), serde_json::json!(topic.trim())),
    ]);
    save_record(
        root,
        workspace,
        "channel",
        channel_id.trim(),
        None,
        fields,
        "Channel created with explicit membership semantics.",
    );
}

fn create_request(root: &Element, workspace: &Element) {
    let from = value(root, "actor");
    let to = value(root, "request-to");
    if !valid_did(&from) || !valid_did(&to) || from.trim() == to.trim() {
        set_status(root, "Enter distinct requesting and receiving DIDs.");
        return;
    }
    let fields = serde_json::Map::from_iter([
        ("from".into(), serde_json::json!(from.trim())),
        ("to".into(), serde_json::json!(to.trim())),
        ("status".into(), serde_json::json!("pending")),
    ]);
    save_record(
        root,
        workspace,
        "social_request",
        &format!("{} → {}", from.trim(), to.trim()),
        None,
        fields,
        "Connection request persisted; acceptance was not inferred.",
    );
}

fn save_record(
    root: &Element,
    workspace: &Element,
    family: &'static str,
    title: &str,
    id: Option<String>,
    fields: serde_json::Map<String, serde_json::Value>,
    success: &'static str,
) {
    set_status(root, "Saving social lifecycle transition…");
    let request_type = field(Some(&fields), "request_type");
    let membership_activation = (family == "social_request"
        && matches!(request_type, "channel-membership" | "channel-invite")
        && field(Some(&fields), "status") == "accepted")
        .then(|| {
            (
                field(Some(&fields), "scope").to_string(),
                if request_type == "channel-invite" {
                    field(Some(&fields), "to")
                } else {
                    field(Some(&fields), "from")
                }
                .to_string(),
                if request_type == "channel-invite" {
                    field(Some(&fields), "from")
                } else {
                    field(Some(&fields), "to")
                }
                .to_string(),
            )
        });
    let root = root.clone();
    let workspace = workspace.clone();
    let title = title.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(NativeRecordUpsertRequest {
            family: family.into(),
            title,
            id,
            fields,
        })
        .await
        {
            Ok(response) if response.ok => {
                if let Some((manifold, participant, authorised_by)) = membership_activation {
                    let member_fields = serde_json::Map::from_iter([
                        ("manifold".into(), serde_json::json!(manifold)),
                        ("participant".into(), serde_json::json!(participant)),
                        ("role".into(), serde_json::json!("member")),
                        ("status".into(), serde_json::json!("active")),
                        ("authorised_by".into(), serde_json::json!(authorised_by)),
                    ]);
                    let member = daemon_records_upsert(NativeRecordUpsertRequest {
                        family: "manifold_participant".into(),
                        title: "Accepted channel membership".into(),
                        id: None,
                        fields: member_fields,
                    })
                    .await;
                    if !member.is_ok_and(|response| response.ok) {
                        set_status(&root, "Request accepted, but membership activation failed.");
                        return;
                    }
                }
                set_status(&root, success);
                refresh_lifecycle(&root, &workspace);
            }
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Transition rejected."),
            ),
            Err(error) => set_status(&root, &error),
        }
    });
}

fn refresh_lifecycle(root: &Element, workspace: &Element) {
    let root = root.clone();
    let workspace = workspace.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let channels = daemon_records_query(NativeRecordQueryRequest {
            family: "channel".into(),
            ..Default::default()
        })
        .await;
        let requests = daemon_records_query(NativeRecordQueryRequest {
            family: "social_request".into(),
            ..Default::default()
        })
        .await;
        if let Ok(response) = channels {
            if response.ok {
                render_channels(&root, &workspace, &response.data);
            }
        }
        if let Ok(response) = requests {
            if response.ok {
                render_requests(&root, &workspace, &response.data);
            }
        }
    });
}

fn render_channels(root: &Element, workspace: &Element, data: &serde_json::Value) {
    let Some(list) = root.query_selector("[data-social-channels]").ok().flatten() else {
        return;
    };
    list.set_inner_html("<small>Channels</small>");
    let document = root.owner_document().unwrap();
    for record in records(data) {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let id = field(fields, "channel_id");
        let membership = field(fields, "membership");
        let row = document.create_element("div").unwrap();
        let open = button(
            &document,
            &format!("#{id} · {}/{}", field(fields, "visibility"), membership),
        );
        let open_workspace = workspace.clone();
        let id = id.to_string();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            if let Some(input) = open_workspace
                .query_selector("[data-social-field=\"thread\"]")
                .ok()
                .flatten()
                .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
            {
                input.set_value(&id);
            }
        }) as Box<dyn FnMut(_)>);
        open.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        row.append_child(&open).unwrap();
        if membership == "open" {
            let join = button(&document, "Join");
            let root = root.clone();
            let workspace = workspace.clone();
            let id = field(fields, "channel_id").to_string();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                join_open_channel(&root, &workspace, &id);
            }) as Box<dyn FnMut(_)>);
            join.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&join).unwrap();
        } else if membership == "request" {
            let request = button(&document, "Request join");
            let root = root.clone();
            let workspace = workspace.clone();
            let id = field(fields, "channel_id").to_string();
            let creator = field(fields, "creator_did").to_string();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                request_channel_join(&root, &workspace, &id, &creator);
            }) as Box<dyn FnMut(_)>);
            request
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&request).unwrap();
        }
        list.append_child(&row).unwrap();
    }
}

fn request_channel_join(root: &Element, workspace: &Element, channel_id: &str, creator: &str) {
    let actor = value(root, "actor");
    if !valid_did(&actor) || !valid_did(creator) || actor.trim() == creator {
        set_status(
            root,
            "Enter a non-owner DID before requesting channel membership.",
        );
        return;
    }
    let fields = serde_json::Map::from_iter([
        ("from".into(), serde_json::json!(actor.trim())),
        ("to".into(), serde_json::json!(creator)),
        ("status".into(), serde_json::json!("pending")),
        (
            "request_type".into(),
            serde_json::json!("channel-membership"),
        ),
        (
            "scope".into(),
            serde_json::json!(format!("channel:{channel_id}")),
        ),
    ]);
    save_record(
        root,
        workspace,
        "social_request",
        &format!("Join #{channel_id}"),
        None,
        fields,
        "Channel membership request persisted for the creator to decide.",
    );
}

fn join_open_channel(root: &Element, workspace: &Element, channel_id: &str) {
    let actor = value(root, "actor");
    if !valid_did(&actor) {
        set_status(root, "Enter your DID before joining a channel.");
        return;
    }
    let fields = serde_json::Map::from_iter([
        (
            "manifold".into(),
            serde_json::json!(format!("channel:{channel_id}")),
        ),
        ("participant".into(), serde_json::json!(actor.trim())),
        ("role".into(), serde_json::json!("member")),
        ("status".into(), serde_json::json!("active")),
    ]);
    save_record(
        root,
        workspace,
        "manifold_participant",
        &format!("{channel_id} · {}", actor.trim()),
        None,
        fields,
        "Joined the open channel; membership is now explicit.",
    );
}

fn render_requests(root: &Element, workspace: &Element, data: &serde_json::Value) {
    let Some(list) = root.query_selector("[data-social-requests]").ok().flatten() else {
        return;
    };
    list.set_inner_html("<small>Relationship requests</small>");
    let document = root.owner_document().unwrap();
    for record in records(data) {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let status = field(fields, "status");
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} → {} · {status} ",
            field(fields, "from"),
            field(fields, "to")
        )));
        if status == "pending" {
            for next in ["accepted", "denied"] {
                let action = button(&document, next);
                let root = root.clone();
                let workspace = workspace.clone();
                let record = record.clone();
                let next = next.to_string();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                    transition_request(&root, &workspace, &record, &next);
                }) as Box<dyn FnMut(_)>);
                action
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
                row.append_child(&action).unwrap();
            }
        }
        list.append_child(&row).unwrap();
    }
}

fn transition_request(root: &Element, workspace: &Element, record: &serde_json::Value, next: &str) {
    let actor = value(root, "actor");
    let mut fields = record
        .get("fields")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    if actor.trim() != field(Some(&fields), "to") {
        set_status(root, "Only the receiving DID can transition this request.");
        return;
    }
    fields.insert("status".into(), serde_json::json!(next));
    fields.insert("acted_by".into(), serde_json::json!(actor.trim()));
    fields.insert(
        "acted_at".into(),
        serde_json::json!((js_sys::Date::now() / 1000.0).floor().to_string()),
    );
    save_record(
        root,
        workspace,
        "social_request",
        record
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Relationship request"),
        record
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        fields,
        "Relationship request transitioned with an explicit actor receipt.",
    );
}

fn records(data: &serde_json::Value) -> &[serde_json::Value] {
    data.get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn field<'a>(fields: Option<&'a serde_json::Map<String, serde_json::Value>>, key: &str) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-social-lifecycle-field=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}
fn input(document: &Document, key: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_attribute("data-social-lifecycle-field", key).ok();
    input.set_attribute("placeholder", placeholder).ok();
    input
}
fn set_status(root: &Element, text: &str) {
    if let Some(status) = root.query_selector("[role=\"status\"]").ok().flatten() {
        status.set_text_content(Some(text));
    }
}
fn valid_did(value: &str) -> bool {
    value.trim().starts_with("did:") && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}
fn defaulted(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.trim().into()
    }
}
fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}
fn style(element: &Element, css: &str) {
    element
        .clone()
        .dyn_into::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text(css);
}
