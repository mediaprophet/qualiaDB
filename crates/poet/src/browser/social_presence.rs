//! Voluntary, scoped and expiring presence publication.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::native_daemon::{
    daemon_invoke, daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

pub fn build_social_presence(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-social-presence", "").ok();
    style(
        &root,
        "border:1px solid var(--border-medium);border-radius:6px;padding:8px;display:flex;flex-direction:column;gap:6px;",
    );
    let title = document.create_element("strong").unwrap();
    title.set_text_content(Some("Voluntary presence"));
    root.append_child(&title).unwrap();
    let explanation = document.create_element("small").unwrap();
    explanation.set_text_content(Some(
        "Presence is scoped and expires automatically. Absence and expiry are not interpreted as social meaning.",
    ));
    root.append_child(&explanation).unwrap();

    let form = document.create_element("div").unwrap();
    style(
        &form,
        "display:grid;grid-template-columns:2fr 1fr 1fr 1fr auto;gap:5px;",
    );
    for (key, placeholder, default) in [
        ("did", "Your DID", ""),
        ("scope", "Scope", "general"),
        ("status", "here | away | unavailable", "here"),
        ("minutes", "Minutes", "30"),
    ] {
        let input = input(document, key, placeholder);
        input
            .clone()
            .dyn_into::<HtmlInputElement>()
            .unwrap()
            .set_value(default);
        form.append_child(&input).unwrap();
    }
    let publish = button(document, "Publish presence");
    form.append_child(&publish).unwrap();
    root.append_child(&form).unwrap();

    let roster = document.create_element("div").unwrap();
    roster.set_attribute("data-presence-roster", "").ok();
    roster.set_text_content(Some("No presence records loaded."));
    root.append_child(&roster).unwrap();
    let refresh = button(document, "Refresh roster");
    root.append_child(&refresh).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "font:9px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let local_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        publish_presence(&local_root);
    }) as Box<dyn FnMut(_)>);
    publish
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let local_root = root.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_roster(&local_root);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    refresh_roster(&root);
    root
}

fn publish_presence(root: &Element) {
    let did = value(root, "did");
    let scope = value(root, "scope");
    let presence_status = value(root, "status").to_ascii_lowercase();
    let minutes = value(root, "minutes").parse::<i64>().unwrap_or(0);
    if !did.starts_with("did:")
        || scope.trim().is_empty()
        || !matches!(presence_status.as_str(), "here" | "away" | "unavailable")
        || !(1..=1_440).contains(&minutes)
    {
        set_status(
            root,
            "Enter a DID, scope, valid status, and an expiry between 1 and 1440 minutes.",
        );
        return;
    }
    let root = root.clone();
    set_status(&root, "Publishing bounded presence…");
    wasm_bindgen_futures::spawn_local(async move {
        let existing = daemon_records_query(NativeRecordQueryRequest {
            family: "presence".into(),
            ..Default::default()
        })
        .await;
        let id = existing.ok().and_then(|response| {
            response
                .data
                .get("records")
                .and_then(serde_json::Value::as_array)
                .and_then(|records| {
                    records.iter().find(|record| {
                        let fields = record.get("fields").and_then(serde_json::Value::as_object);
                        field(fields, "did") == did && field(fields, "scope") == scope
                    })
                })
                .and_then(|record| record.get("id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
        let expires = ((js_sys::Date::now() / 1000.0).floor() as i64 + minutes * 60).to_string();
        let fields = serde_json::Map::from_iter([
            ("did".into(), serde_json::json!(did)),
            ("scope".into(), serde_json::json!(scope)),
            ("status".into(), serde_json::json!(presence_status)),
            ("expires_at".into(), serde_json::json!(expires)),
        ]);
        match daemon_records_upsert(NativeRecordUpsertRequest {
            family: "presence".into(),
            title: format!(
                "{} · {}",
                field(Some(&fields), "scope"),
                field(Some(&fields), "did")
            ),
            id,
            fields,
        })
        .await
        {
            Ok(response) if response.ok => {
                let scope = response
                    .data
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .map(|fields| field(Some(fields), "scope"))
                    .unwrap_or("general");
                let notice = daemon_invoke(
                    "Pulse.publish_presence",
                    serde_json::json!({"channel": format!("poet/presence/{scope}")}),
                )
                .await;
                set_status(
                    &root,
                    if notice.is_ok_and(|receipt| receipt.ok) {
                        "Presence persisted and a live notice was published."
                    } else {
                        "Presence persisted; the live notice was unavailable."
                    },
                );
                refresh_roster(&root);
            }
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Presence was rejected."),
            ),
            Err(error) => set_status(&root, &error),
        }
    });
}

fn refresh_roster(root: &Element) {
    let root = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "presence".into(),
            ..Default::default()
        })
        .await
        {
            Ok(response) if response.ok => render_roster(&root, &response.data),
            Ok(response) => set_status(
                &root,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Roster query failed."),
            ),
            Err(error) => set_status(&root, &error),
        }
    });
}

fn render_roster(root: &Element, data: &serde_json::Value) {
    let Some(roster) = root.query_selector("[data-presence-roster]").ok().flatten() else {
        return;
    };
    roster.set_inner_html("");
    let document = root.owner_document().unwrap();
    let now = (js_sys::Date::now() / 1000.0).floor() as i64;
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for record in records {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let expires = field(fields, "expires_at").parse::<i64>().unwrap_or(0);
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} · {} · {} · {}",
            field(fields, "scope"),
            field(fields, "did"),
            if expires > now {
                field(fields, "status")
            } else {
                "expired"
            },
            field(fields, "expires_at")
        )));
        roster.append_child(&row).unwrap();
    }
    if records.is_empty() {
        roster.set_text_content(Some("No voluntary presence has been published."));
    }
}

fn field<'a>(fields: Option<&'a serde_json::Map<String, serde_json::Value>>, key: &str) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}
fn value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-presence-field=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}
fn input(document: &Document, key: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_attribute("data-presence-field", key).ok();
    input.set_attribute("placeholder", placeholder).ok();
    input
}
fn set_status(root: &Element, message: &str) {
    if let Some(status) = root.query_selector("[role=\"status\"]").ok().flatten() {
        status.set_text_content(Some(message));
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
