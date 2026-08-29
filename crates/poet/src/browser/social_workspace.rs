//! Human-facing social and communications workspace.

use std::collections::BTreeSet;

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use super::native_daemon::{
    daemon_invoke, daemon_records_query, daemon_records_upsert, is_daemon_connected,
    NativeRecordQueryRequest, NativeRecordUpsertRequest,
};

pub fn build_social_view(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-workspace", "live").ok();
    style(
        &root,
        "display:flex;flex-direction:column;gap:8px;padding:9px;overflow:auto;",
    );

    let intro = document.create_element("div").unwrap();
    intro.set_inner_html(
        "<h3 style=\"margin:0 0 3px\">Social</h3>\
         <p style=\"margin:0;color:var(--text-muted);font-size:10px\">\
         DID-attributed conversation threads with persistent messages and live Pulse notices. \
         Persistence is not represented as end-to-end delivery or signature.</p>",
    );
    root.append_child(&intro).unwrap();

    let filters = document.create_element("div").unwrap();
    style(
        &filters,
        "display:grid;grid-template-columns:1fr 1fr auto;gap:6px;",
    );
    filters
        .append_child(&input(document, "thread", "Thread / channel (general)"))
        .unwrap();
    filters
        .append_child(&input(document, "from", "Your DID"))
        .unwrap();
    let refresh = button(document, "Refresh");
    filters.append_child(&refresh).unwrap();
    root.append_child(&filters).unwrap();

    let messages = document.create_element("div").unwrap();
    messages.set_attribute("data-social-messages", "").ok();
    style(
        &messages,
        "display:flex;flex-direction:column;gap:6px;min-height:150px;max-height:360px;overflow:auto;border:1px solid var(--border-medium);border-radius:6px;padding:8px;",
    );
    root.append_child(&messages).unwrap();

    let composer = document.create_element("div").unwrap();
    composer
        .append_child(&input(
            document,
            "reply-to",
            "Reply to message id (optional)",
        ))
        .unwrap();
    composer
        .append_child(&input(
            document,
            "mentions",
            "Mention DIDs, comma-separated (optional)",
        ))
        .unwrap();
    let body = document.create_element("textarea").unwrap();
    body.set_attribute("data-social-body", "").ok();
    body.set_attribute("placeholder", "Write a message…").ok();
    style(&body, "width:100%;min-height:72px;");
    composer.append_child(&body).unwrap();
    let send = button(document, "Send to thread");
    composer.append_child(&send).unwrap();
    root.append_child(&composer).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "font:10px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let refresh_root = root.clone();
    let refresh_status = status.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_messages(&refresh_root, &refresh_status);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
        .unwrap();
    refresh_closure.forget();

    let send_root = root.clone();
    let send_status = status.clone();
    let send_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !is_daemon_connected() {
            send_status.set_text_content(Some("Unavailable: connect the local daemon."));
            return;
        }
        let thread = defaulted(&field(&send_root, "thread"), "general");
        let from = field(&send_root, "from");
        let reply_to = field(&send_root, "reply-to");
        let mentions = match parse_mentions(&field(&send_root, "mentions")) {
            Ok(mentions) => mentions,
            Err(error) => {
                send_status.set_text_content(Some(&error));
                return;
            }
        };
        let body = send_root
            .query_selector("[data-social-body]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            .map(|input| input.value())
            .unwrap_or_default();
        if from.trim().is_empty() || !from.trim().starts_with("did:") || body.trim().is_empty() {
            send_status.set_text_content(Some("Enter your DID and a message."));
            return;
        }
        send_status.set_text_content(Some("Saving message and publishing thread notice…"));
        let root = send_root.clone();
        let status = send_status.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut fields = serde_json::Map::new();
            fields.insert("thread".into(), serde_json::json!(thread));
            fields.insert("from".into(), serde_json::json!(from.trim()));
            fields.insert("body".into(), serde_json::json!(body.trim()));
            fields.insert("sensitivity".into(), serde_json::json!("restricted"));
            if !reply_to.trim().is_empty() {
                fields.insert("reply_to".into(), serde_json::json!(reply_to.trim()));
            }
            if !mentions.is_empty() {
                fields.insert("mentions".into(), serde_json::json!(mentions.join(",")));
            }
            let stored = daemon_records_upsert(NativeRecordUpsertRequest {
                family: "social_message".into(),
                title: format!("{} · {}", thread, truncate(body.trim(), 80)),
                id: None,
                fields,
            })
            .await;
            match stored {
                Ok(response) if response.ok => {
                    let message_id = response
                        .data
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let mut notification_failures = 0usize;
                    for recipient in mentions
                        .iter()
                        .filter(|recipient| *recipient != from.trim())
                    {
                        let notification = daemon_records_upsert(NativeRecordUpsertRequest {
                            family: "social_notification".into(),
                            title: format!("Mention in #{thread}"),
                            id: None,
                            fields: serde_json::Map::from_iter([
                                ("recipient".into(), serde_json::json!(recipient)),
                                ("from".into(), serde_json::json!(from.trim())),
                                ("source_message_id".into(), serde_json::json!(message_id)),
                                ("thread".into(), serde_json::json!(thread)),
                                ("status".into(), serde_json::json!("unread")),
                            ]),
                        })
                        .await;
                        if !notification.is_ok_and(|response| response.ok) {
                            notification_failures += 1;
                        }
                    }
                    let pulse = daemon_invoke(
                        "Pulse.publish_agent_message",
                        serde_json::json!({ "channel": format!("poet/social/{thread}") }),
                    )
                    .await;
                    if let Some(input) = root
                        .query_selector("[data-social-body]")
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
                    {
                        input.set_value("");
                    }
                    clear_input(&root, "reply-to");
                    clear_input(&root, "mentions");
                    let pulse_live = pulse.is_ok_and(|receipt| receipt.ok);
                    status.set_text_content(Some(&match (pulse_live, notification_failures) {
                        (true, 0) => "Message persisted; local mention receipts and live Pulse notice published.".into(),
                        (false, 0) => "Message and local mention receipts persisted; live delivery notice was unavailable.".into(),
                        (_, count) => format!("Message persisted, but {count} local mention notification(s) could not be recorded."),
                    }));
                    refresh_messages(&root, &status);
                    super::social_notifications::refresh_all_notifications();
                }
                Ok(response) => status.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Message persistence failed."),
                )),
                Err(error) => status.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    send.add_event_listener_with_callback("click", send_closure.as_ref().unchecked_ref())
        .unwrap();
    send_closure.forget();

    root.append_child(&super::social_lifecycle::build_social_lifecycle(
        document, &root,
    ))
    .unwrap();
    root.append_child(&super::social_inbox::build_social_inbox(document))
        .unwrap();
    root.append_child(&super::social_notifications::build_social_notifications(
        document, &root,
    ))
    .unwrap();
    root.append_child(&super::social_moderation::build_social_moderation(document))
        .unwrap();
    root.append_child(&super::social_presence::build_social_presence(document))
        .unwrap();

    refresh_messages(&root, &status);
    root
}

fn refresh_messages(root: &Element, status: &Element) {
    if !is_daemon_connected() {
        status.set_text_content(Some("Unavailable: connect the local daemon."));
        return;
    }
    let thread = field(root, "thread");
    let root = root.clone();
    let status = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "social_message".into(),
            query: String::new(),
            kind: String::new(),
        })
        .await
        {
            Ok(response) if response.ok => {
                let moderation = daemon_records_query(NativeRecordQueryRequest {
                    family: "social_moderation".into(),
                    ..Default::default()
                })
                .await
                .ok()
                .filter(|response| response.ok)
                .map(|response| response.data)
                .unwrap_or(serde_json::Value::Null);
                render_messages(&root, &response.data, &moderation, thread.trim());
                status.set_text_content(Some("Conversation loaded from the persistent ledger."));
            }
            Ok(response) => status.set_text_content(Some(
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Conversation query failed."),
            )),
            Err(error) => status.set_text_content(Some(&error)),
        }
    });
}

fn render_messages(
    root: &Element,
    data: &serde_json::Value,
    moderation: &serde_json::Value,
    thread_filter: &str,
) {
    let Some(container) = root.query_selector("[data-social-messages]").ok().flatten() else {
        return;
    };
    container.set_inner_html("");
    let document = root.owner_document().unwrap();
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut shown = 0usize;
    for record in records {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let thread = fields
            .and_then(|fields| fields.get("thread"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("general");
        if !thread_filter.is_empty() && thread != thread_filter {
            continue;
        }
        let from = fields
            .and_then(|fields| fields.get("from"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown DID");
        let body = fields
            .and_then(|fields| fields.get("body"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let message = document.create_element("article").unwrap();
        style(
            &message,
            "border-left:2px solid var(--accent-cyan);padding:5px 8px;background:var(--surface-panel);",
        );
        let meta = document.create_element("small").unwrap();
        let record_id = record
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        meta.set_text_content(Some(&format!("#{thread} · {from} · {record_id}")));
        message.append_child(&meta).unwrap();
        if let Some(reply_to) = fields
            .and_then(|fields| fields.get("reply_to"))
            .and_then(serde_json::Value::as_str)
        {
            let reply = document.create_element("div").unwrap();
            let context = records
                .iter()
                .find(|candidate| {
                    candidate.get("id").and_then(serde_json::Value::as_str) == Some(reply_to)
                })
                .and_then(|candidate| candidate.get("fields"))
                .and_then(serde_json::Value::as_object);
            let replied_body =
                if super::social_moderation::hidden_notice(moderation, reply_to).is_some() {
                    "[hidden by channel moderation]"
                } else {
                    context
                        .and_then(|fields| fields.get("body"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unavailable message")
                };
            reply.set_text_content(Some(&format!(
                "Reply to {}: {}",
                context
                    .and_then(|fields| fields.get("from"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                truncate(replied_body, 120)
            )));
            style(&reply, "font-size:9px;color:var(--text-muted);border-left:2px solid var(--border-medium);padding-left:5px;");
            message.append_child(&reply).unwrap();
        }
        let text = document.create_element("div").unwrap();
        let rendered_body = super::social_moderation::hidden_notice(moderation, record_id)
            .unwrap_or_else(|| body.to_string());
        text.set_text_content(Some(&rendered_body));
        message.append_child(&text).unwrap();
        container.append_child(&message).unwrap();
        shown += 1;
    }
    if shown == 0 {
        container.set_text_content(Some("No messages in this thread yet."));
    }
}

fn input(document: &Document, key: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_attribute("data-social-field", key).ok();
    input.set_attribute("placeholder", placeholder).ok();
    input
}

fn field(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-social-field=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn clear_input(root: &Element, key: &str) {
    if let Some(input) = root
        .query_selector(&format!("[data-social-field=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        input.set_value("");
    }
}

fn parse_mentions(raw: &str) -> Result<Vec<String>, String> {
    let mentions = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if mentions.len() > 16 {
        return Err("A message may mention at most 16 DIDs.".into());
    }
    if mentions
        .iter()
        .any(|did| !did.starts_with("did:") || did.bytes().any(|byte| byte.is_ascii_whitespace()))
    {
        return Err("Mentions must be comma-separated DIDs.".into());
    }
    Ok(mentions.into_iter().collect())
}

fn defaulted(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.trim().into()
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.into();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
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
