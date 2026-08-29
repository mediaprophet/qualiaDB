//! Social manifold roster — many people on one lens.
//!
//! The construct stays personal. A social manifold (especially a project)
//! is a shared lens: members, roles, presence. Natural persons are not owl:Thing.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use crate::tool_chest::core::ManifoldParticipant;

use super::cop_records;
use super::native_daemon::{daemon_records_upsert, is_daemon_connected, NativeRecordUpsertRequest};

/// Participants container on a social lens (projects, social, communications).
pub fn build_participants_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; padding: 8px; overflow: auto;",
    );

    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "This lens is social: many people participate. Your construct remains yours. Projects are the primary case.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    root.append_child(&note).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_attribute("data-participant-list", "").ok();
    fill_roster(document, &list);
    root.append_child(&list).unwrap();

    let invite = document.create_element("button").unwrap();
    invite.set_attribute("type", "button").ok();
    invite.set_text_content(Some("Invite a person"));
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            open_invite_dialog(&doc);
        }
    }) as Box<dyn FnMut(_)>);
    invite
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    root.append_child(&invite).unwrap();

    root.append_child(&cop_records::build_family_panel(
        document,
        "manifold_participant",
        "Participant records persist on the COP ledger when the daemon is up. Pulse.publish_presence is who is here now.",
        &[
            cop_records::CopField {
                key: "did",
                placeholder: "Member DID",
            },
            cop_records::CopField {
                key: "role",
                placeholder: "Role (member|steward|observer)",
            },
        ],
    ))
    .unwrap();
    root
}

fn fill_roster(document: &Document, list: &Element) {
    while let Some(child) = list.first_element_child() {
        child.remove();
    }
    let participants = super::current_participants();
    if participants.is_empty() {
        let empty = document.create_element("div").unwrap();
        empty.set_text_content(Some(
            "No participants yet. Invite a DID, or run Poet.participant_invite.",
        ));
        list.append_child(&empty).unwrap();
        return;
    }
    for person in participants {
        let row = document.create_element("div").unwrap();
        let label = if person.label.is_empty() {
            person.did.clone()
        } else {
            format!("{} · {}", person.label, person.did)
        };
        row.set_text_content(Some(&format!("{} ({})", label, person.role)));
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text(
            "font-size: 11px; font-family: var(--font-mono); padding: 4px 0; border-bottom: 1px solid var(--border-subtle);",
        );
        list.append_child(&row).unwrap();
    }
}

/// Compact strip in the canvas chrome. Hidden on personal lenses.
pub fn refresh_people_chrome(document: &Document) {
    let Some(host) = document.get_element_by_id("manifold-people") else {
        return;
    };
    while let Some(child) = host.first_element_child() {
        child.remove();
    }
    host.set_text_content(None);
    let social = super::current_manifold_is_social();
    host.set_attribute("data-social", if social { "true" } else { "false" })
        .ok();
    if !social {
        host.set_text_content(Some("personal lens"));
        return;
    }
    let mark = document.create_element("span").unwrap();
    mark.set_text_content(Some("social ·"));
    host.append_child(&mark).unwrap();
    let people = super::current_participants();
    let count = document.create_element("span").unwrap();
    count.set_text_content(Some(&format!("{} people", people.len())));
    host.append_child(&count).unwrap();
    let invite = document.create_element("button").unwrap();
    invite.set_attribute("type", "button").ok();
    invite.set_class_name("breadcrumb-pop");
    invite.set_text_content(Some("Invite"));
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            open_invite_dialog(&doc);
        }
    }) as Box<dyn FnMut(_)>);
    invite
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    host.append_child(&invite).unwrap();
}

pub fn open_invite_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("participant-invite-dialog") {
        existing.remove();
    }
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("participant-invite-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 10020; \
         display: flex; align-items: flex-start; justify-content: center; padding-top: 14vh;",
    );
    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 420px; background: var(--surface-glass-heavy); border: 1px solid var(--border-medium); \
         border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 8px;",
    );
    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Invite a person onto this lens"));
    panel.append_child(&title).unwrap();
    let hint = document.create_element("div").unwrap();
    hint.set_text_content(Some(
        "Social manifold: many people. Construct stays yours. Vibe: Poet.participant_invite({ did: \"did:…\", role: \"member\" })",
    ));
    let hint_el: HtmlElement = hint.clone().dyn_into().unwrap();
    hint_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    panel.append_child(&hint).unwrap();

    let did = document.create_element("input").unwrap();
    did.set_attribute("data-invite-did", "").ok();
    did.set_attribute("placeholder", "DID (did:qualia:… or did:web:…)")
        .ok();
    panel.append_child(&did).unwrap();
    let label = document.create_element("input").unwrap();
    label.set_attribute("data-invite-label", "").ok();
    label.set_attribute("placeholder", "Name (optional)").ok();
    panel.append_child(&label).unwrap();
    let role = document.create_element("input").unwrap();
    role.set_attribute("data-invite-role", "").ok();
    role.set_attribute("placeholder", "Role (member, steward, observer)")
        .ok();
    panel.append_child(&role).unwrap();

    let actions = document.create_element("div").unwrap();
    let add = document.create_element("button").unwrap();
    add.set_attribute("type", "button").ok();
    add.set_text_content(Some("Invite"));
    let cancel = document.create_element("button").unwrap();
    cancel.set_attribute("type", "button").ok();
    cancel.set_text_content(Some("Cancel"));
    actions.append_child(&add).unwrap();
    actions.append_child(&cancel).unwrap();
    panel.append_child(&actions).unwrap();
    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }

    let overlay_close = overlay.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        overlay_close.remove();
    }) as Box<dyn FnMut(_)>);
    cancel
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();

    let overlay_add = overlay.clone();
    let panel_add = panel.clone();
    let add_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let did = panel_add
            .query_selector("[data-invite-did]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let label = panel_add
            .query_selector("[data-invite-label]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let role = panel_add
            .query_selector("[data-invite-role]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let did = did.trim();
        if did.is_empty() {
            return;
        }
        let role = if role.trim().is_empty() {
            "member"
        } else {
            role.trim()
        };
        super::manifold_authoring::invite_participant(
            &super::manifold_authoring::ParticipantInvite {
                did: did.to_string(),
                role: role.to_string(),
                label: label.trim().to_string(),
            },
        );
        overlay_add.remove();
    }) as Box<dyn FnMut(_)>);
    add.add_event_listener_with_callback("click", add_closure.as_ref().unchecked_ref())
        .unwrap();
    add_closure.forget();
}

pub fn persist_participant_cop(person: &ManifoldParticipant, manifold_id: &str) {
    if !is_daemon_connected() {
        return;
    }
    let did = person.did.clone();
    let role = person.role.clone();
    let label = person.label.clone();
    let manifold = manifold_id.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let mut fields = serde_json::Map::new();
        fields.insert("did".into(), serde_json::Value::String(did.clone()));
        fields.insert("role".into(), serde_json::Value::String(role));
        fields.insert(
            "manifold".into(),
            serde_json::Value::String(manifold.clone()),
        );
        if !label.is_empty() {
            fields.insert("label".into(), serde_json::Value::String(label.clone()));
        }
        let title = if label.is_empty() {
            format!("{did} on {manifold}")
        } else {
            format!("{label} on {manifold}")
        };
        let _ = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "manifold_participant".into(),
            title,
            id: None,
            fields,
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::ManifoldSociality;

    #[test]
    fn sociality_is_not_a_construct() {
        assert_ne!(ManifoldSociality::Social.as_str(), "construct");
        let p = ManifoldParticipant::new("did:qualia:alice", "member");
        assert_eq!(p.role, "member");
    }
}
