//! Creator-controlled roles and non-destructive channel moderation receipts.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement};

use super::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_social_moderation(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-moderation", "").ok();
    root.set_inner_html(
        "<strong>Channel roles and moderation</strong>\
         <p style=\"margin:2px 0;color:var(--text-muted);font-size:10px\">\
         Role changes are creator-only. Hiding creates an attributed receipt and preserves the original message.</p>",
    );
    for (key, placeholder) in [
        ("actor", "Acting DID"),
        ("channel", "Channel id"),
        ("participant", "Participant DID"),
        ("role", "moderator | member | guest"),
        ("message", "Message id to hide"),
        ("reason", "Moderation reason"),
    ] {
        let input = document.create_element("input").unwrap();
        input
            .set_attribute("data-social-moderation-field", key)
            .ok();
        input.set_attribute("placeholder", placeholder).ok();
        root.append_child(&input).unwrap();
    }
    let role = button(document, "Update participant role");
    let hide = button(document, "Hide message with receipt");
    root.append_child(&role).unwrap();
    root.append_child(&hide).unwrap();
    let status = document.create_element("div").unwrap();
    status
        .set_attribute("data-social-moderation-status", "")
        .ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();

    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        update_role(&click_root);
    }) as Box<dyn FnMut(_)>);
    role.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    let click_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        hide_message(&click_root);
    }) as Box<dyn FnMut(_)>);
    hide.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    root
}

pub fn hidden_notice(data: &serde_json::Value, message_id: &str) -> Option<String> {
    data.get("records")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .rev()
        .find_map(|record| {
            let fields = record.get("fields")?.as_object()?;
            (field(fields, "message_id") == message_id && field(fields, "action") == "hide").then(
                || {
                    format!(
                        "[hidden by {}: {}]",
                        field(fields, "actor_did"),
                        field(fields, "reason")
                    )
                },
            )
        })
}

fn update_role(root: &Element) {
    let actor = value(root, "actor");
    let channel = value(root, "channel");
    let participant = value(root, "participant");
    let role = value(root, "role").to_ascii_lowercase();
    if !valid_did(&actor)
        || !valid_did(&participant)
        || channel.trim().is_empty()
        || !matches!(role.as_str(), "moderator" | "member" | "guest")
    {
        set_status(
            root,
            " Enter actor/channel/participant and a supported role.",
        );
        return;
    }
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let response = daemon_records_query(NativeRecordQueryRequest {
            family: "manifold_participant".into(),
            ..Default::default()
        })
        .await;
        let record = response.ok().and_then(|response| {
            response
                .data
                .get("records")
                .and_then(serde_json::Value::as_array)
                .and_then(|records| {
                    records.iter().find(|record| {
                        let fields = record.get("fields").and_then(serde_json::Value::as_object);
                        field_opt(fields, "manifold") == format!("channel:{channel}")
                            && field_opt(fields, "participant") == participant
                    })
                })
                .cloned()
        });
        let Some(record) = record else {
            set_status(&root, " Participant membership record was not found.");
            return;
        };
        let mut fields = record
            .get("fields")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();
        fields.insert("role".into(), serde_json::json!(role));
        fields.insert("acted_by".into(), serde_json::json!(actor));
        fields.insert("acted_at".into(), serde_json::json!(unix_now().to_string()));
        let result = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "manifold_participant".into(),
            title: record
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Channel participant")
                .into(),
            id: record
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            fields,
        })
        .await;
        match result {
            Ok(response) if response.ok => {
                set_status(&root, " Participant role updated with creator attribution.")
            }
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Role update rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn hide_message(root: &Element) {
    let actor = value(root, "actor");
    let thread = value(root, "channel");
    let message_id = value(root, "message");
    let reason = value(root, "reason");
    if !valid_did(&actor)
        || thread.trim().is_empty()
        || message_id.trim().is_empty()
        || reason.trim().is_empty()
    {
        set_status(
            root,
            " Enter actor, channel, message id, and moderation reason.",
        );
        return;
    }
    let request = NativeRecordUpsertRequest {
        family: "social_moderation".into(),
        title: format!("Hide {message_id} in #{thread}"),
        id: None,
        fields: serde_json::Map::from_iter([
            ("actor_did".into(), serde_json::json!(actor.trim())),
            ("thread".into(), serde_json::json!(thread.trim())),
            ("message_id".into(), serde_json::json!(message_id.trim())),
            ("reason".into(), serde_json::json!(reason.trim())),
            ("action".into(), serde_json::json!("hide")),
            ("acted_at".into(), serde_json::json!(unix_now().to_string())),
        ]),
    };
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(request).await {
            Ok(response) if response.ok => set_status(
                &root,
                " Message hidden by attributed receipt; original retained.",
            ),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or(" Moderation rejected."),
            ),
            Err(error) => set_status(&root, &format!(" {error}")),
        }
    });
}

fn unix_now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}
fn value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-social-moderation-field=\"{key}\"]"))
        .ok()
        .flatten()
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
fn field_opt<'a>(
    fields: Option<&'a serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn valid_did(value: &str) -> bool {
    value.trim().starts_with("did:") && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}
fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}
fn set_status(root: &Element, text: &str) {
    if let Ok(Some(status)) = root.query_selector("[data-social-moderation-status]") {
        status.set_text_content(Some(text));
    }
}
