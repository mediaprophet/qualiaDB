//! Means to author manifolds, containers, nested links, and subjects.
//!
//! VibeScript is the language (`Poet.*`); the HyperCanvas applies layout.
//! Host receipts do not mutate the DOM. We do not ship canned worlds.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, HtmlElement, HtmlInputElement, HtmlSelectElement};

use crate::tool_chest::core::registry::{ManifoldSeed, SeedContainer};
use crate::tool_chest::core::{ManifoldSociality, SubjectSeed};

use super::native_daemon::{daemon_records_upsert, is_daemon_connected, NativeRecordUpsertRequest};

/// One authoring operation parsed from Vibe (or the + dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoringOp {
    ManifoldCreate(ManifoldCreate),
    ContainerPlace(ContainerPlace),
    NestedLink(NestedLink),
    SubjectDeclare(SubjectDeclare),
    ParticipantInvite(ParticipantInvite),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifoldCreate {
    pub label: String,
    pub description: String,
    pub nest: bool,
    pub social: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerPlace {
    pub container_type: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedLink {
    pub to: String,
    pub title: String,
    pub target_construct: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDeclare {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantInvite {
    pub did: String,
    pub role: String,
    pub label: String,
}

/// Parse `Poet.manifold_create` calls. Kept for existing tests.
pub fn parse_manifold_creates(source: &str) -> Vec<ManifoldCreate> {
    parse_authoring_ops(source)
        .into_iter()
        .filter_map(|op| match op {
            AuthoringOp::ManifoldCreate(spec) => Some(spec),
            _ => None,
        })
        .collect()
}

/// Parse all four Poet.* shell ops from Vibe source, in source order.
pub fn parse_authoring_ops(source: &str) -> Vec<AuthoringOp> {
    let mut out = Vec::new();
    let mut rest = source;
    loop {
        let create = rest.find("Poet.manifold_create");
        let place = rest.find("Poet.container_place");
        let link = rest.find("Poet.nested_link");
        let subject = rest.find("Poet.subject_declare");
        let invite = rest.find("Poet.participant_invite");
        let next = [create, place, link, subject, invite]
            .into_iter()
            .flatten()
            .min();
        let Some(idx) = next else {
            break;
        };
        let slice = &rest[idx..];
        let end = slice.find(')').unwrap_or(slice.len());
        let body = &slice[..end];
        if create == Some(idx) {
            let label = quoted_field(body, "label")
                .or_else(|| quoted_field(body, "name"))
                .unwrap_or_else(|| "Untitled lens".into());
            let description = quoted_field(body, "description").unwrap_or_default();
            let nest = body.contains("nest: true")
                || body.contains("nest:true")
                || body.contains("\"nest\": true");
            let social = body.contains("social: true")
                || body.contains("social:true")
                || body.contains("\"social\": true");
            out.push(AuthoringOp::ManifoldCreate(ManifoldCreate {
                label,
                description,
                nest,
                social,
            }));
            rest = &rest[idx + "Poet.manifold_create".len()..];
        } else if place == Some(idx) {
            let container_type = quoted_field(body, "container_type")
                .or_else(|| quoted_field(body, "type"))
                .unwrap_or_else(|| "doc".into());
            let title = quoted_field(body, "title")
                .or_else(|| quoted_field(body, "label"))
                .unwrap_or_else(|| container_type.clone());
            out.push(AuthoringOp::ContainerPlace(ContainerPlace {
                container_type,
                title,
            }));
            rest = &rest[idx + "Poet.container_place".len()..];
        } else if link == Some(idx) {
            let to = quoted_field(body, "to")
                .or_else(|| quoted_field(body, "target_manifold"))
                .unwrap_or_default();
            if !to.is_empty() {
                let title = quoted_field(body, "title").unwrap_or_else(|| to.clone());
                let target_construct = quoted_field(body, "target_construct").unwrap_or_default();
                out.push(AuthoringOp::NestedLink(NestedLink {
                    to,
                    title,
                    target_construct,
                }));
            }
            rest = &rest[idx + "Poet.nested_link".len()..];
        } else if subject == Some(idx) {
            let label = quoted_field(body, "label")
                .or_else(|| quoted_field(body, "name"))
                .unwrap_or_else(|| "Untitled subject".into());
            let description = quoted_field(body, "description").unwrap_or_default();
            out.push(AuthoringOp::SubjectDeclare(SubjectDeclare {
                label,
                description,
            }));
            rest = &rest[idx + "Poet.subject_declare".len()..];
        } else {
            let did = quoted_field(body, "did")
                .or_else(|| quoted_field(body, "participant"))
                .unwrap_or_default();
            if !did.is_empty() {
                let role = quoted_field(body, "role").unwrap_or_else(|| "member".into());
                let label = quoted_field(body, "label").unwrap_or_else(|| did.clone());
                out.push(AuthoringOp::ParticipantInvite(ParticipantInvite {
                    did,
                    role,
                    label,
                }));
            }
            rest = &rest[idx + "Poet.participant_invite".len()..];
        }
    }
    out
}

fn quoted_field(body: &str, key: &str) -> Option<String> {
    for pattern in [
        format!("{key}: \""),
        format!("{key}:\""),
        format!("\"{key}\": \""),
        format!("\"{key}\":\""),
    ] {
        if let Some(start) = body.find(&pattern) {
            let after = &body[start + pattern.len()..];
            if let Some(end) = after.find('"') {
                let value = after[..end].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Apply parsed Poet.* ops in the open construct. Returns log lines.
pub fn apply_authoring_ops(ops: &[AuthoringOp]) -> Vec<String> {
    let mut log = Vec::new();
    for op in ops {
        match op {
            AuthoringOp::ManifoldCreate(spec) => {
                let id = create_manifold(spec);
                log.push(format!(
                    "created lens `{}` as `{id}` nest={}",
                    spec.label, spec.nest
                ));
            }
            AuthoringOp::ContainerPlace(spec) => {
                let id = place_container(spec);
                log.push(format!(
                    "placed `{}` (`{}`) as `{id}`",
                    spec.title, spec.container_type
                ));
            }
            AuthoringOp::NestedLink(spec) => {
                let id = place_nested_link(spec);
                log.push(format!("linked `{}` → `{}` as `{id}`", spec.title, spec.to));
            }
            AuthoringOp::SubjectDeclare(spec) => {
                let id = declare_subject(spec);
                log.push(format!("declared subject `{}` as `{id}`", spec.label));
            }
            AuthoringOp::ParticipantInvite(spec) => {
                invite_participant(spec);
                log.push(format!(
                    "invited `{}` as {} on this lens",
                    spec.did, spec.role
                ));
            }
        }
    }
    persist_authored();
    log
}

fn persist_authored() {
    let _ = super::manifest::save_all_manifolds();
    super::persist_construct_extras();
    super::persist_subjects();
}

/// Create a lens in the open construct. Returns the new manifold id.
pub fn create_manifold(spec: &ManifoldCreate) -> String {
    let manifold_id = format!(
        "lens-{}",
        (js_sys::Date::now() as u64).saturating_add(spec.label.len() as u64)
    );
    let seed = ManifoldSeed {
        id: manifold_id.clone(),
        label: spec.label.clone(),
        icon: "\u{1F50D}".into(),
        description: if spec.description.is_empty() {
            format!(
                "Authored lens in construct `{}`.",
                super::current_construct_id()
            )
        } else {
            spec.description.clone()
        },
        containers: Vec::<SeedContainer>::new(),
        connections: Vec::new(),
        sociality: if spec.social {
            ManifoldSociality::Social
        } else {
            ManifoldSociality::Personal
        },
        ..Default::default()
    };
    super::replace_current_seed(&seed);
    super::register_construct_manifold(&manifold_id);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if spec.nest {
            place_nested_portal(&document, &manifold_id, &spec.label);
        }
        let visible = super::visible_seeds();
        super::topbar::rebuild_pager(&document, &visible, &manifold_id);
        if spec.nest {
            super::dive_nested_manifold(&manifold_id);
        } else {
            super::switch_to_sibling_manifold(&manifold_id);
        }
        super::interactions::show_tool_status(
            &document,
            "Manifold authoring",
            &format!("Created lens `{}` ({manifold_id}).", spec.label),
            "success",
        );
    }
    if spec.social {
        if let Some(did) = Some(super::current_observer_did()).filter(|d| !d.is_empty()) {
            invite_participant(&ParticipantInvite {
                did,
                role: "steward".into(),
                label: String::new(),
            });
        }
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            place_container(&ContainerPlace {
                container_type: "participants".into(),
                title: "People on this lens".into(),
            });
            super::manifold_social::refresh_people_chrome(&document);
        }
    }
    persist_authored();
    manifold_id
}

pub fn invite_participant(spec: &ParticipantInvite) {
    let manifold_id = super::current_manifold_id();
    let person = crate::tool_chest::core::ManifoldParticipant {
        did: spec.did.clone(),
        label: spec.label.clone(),
        role: if spec.role.is_empty() {
            "member".into()
        } else {
            spec.role.clone()
        },
    };
    super::add_participant_to_current(person.clone());
    super::manifold_social::persist_participant_cop(&person, &manifold_id);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        super::manifold_social::refresh_people_chrome(&document);
        super::interactions::show_tool_status(
            &document,
            "Social manifold",
            &format!("Invited `{}` as {}.", person.did, person.role),
            "success",
        );
    }
    persist_authored();
}

pub fn place_container(spec: &ContainerPlace) -> String {
    let container = SeedContainer {
        id: super::canvas_state::next_container_id(&spec.container_type),
        container_type: spec.container_type.clone(),
        title: spec.title.clone(),
        x: 80.0,
        y: 80.0,
        width: 400.0,
        height: 300.0,
        z: 120.0,
        honesty: "live".into(),
        ..Default::default()
    };
    let id = container.id.clone();
    append_seed_container(container);
    id
}

pub fn place_nested_link(spec: &NestedLink) -> String {
    let is_construct = !spec.target_construct.is_empty();
    let container_type = if is_construct {
        "construct_portal"
    } else {
        "nested_manifold"
    };
    let container = SeedContainer {
        id: super::canvas_state::next_container_id(container_type),
        container_type: container_type.into(),
        title: spec.title.clone(),
        x: 80.0,
        y: 220.0,
        width: 280.0,
        height: 140.0,
        z: 120.0,
        honesty: "live".into(),
        target_manifold: spec.to.clone(),
        target_construct: spec.target_construct.clone(),
        ..Default::default()
    };
    let id = container.id.clone();
    append_seed_container(container);
    id
}

pub fn declare_subject(spec: &SubjectDeclare) -> String {
    let id = format!(
        "subject-{}",
        (js_sys::Date::now() as u64).saturating_add(spec.label.len() as u64)
    );
    let construct_id = super::current_construct_id();
    let manifold_id = super::current_manifold_id();
    let seed = SubjectSeed {
        id: id.clone(),
        label: spec.label.clone(),
        description: spec.description.clone(),
        construct_id: construct_id.clone(),
        manifold_id: manifold_id.clone(),
        observer: super::current_observer_did(),
    };
    super::register_subject(seed.clone());
    persist_subject_cop(&seed);
    let mut view_state = std::collections::BTreeMap::new();
    view_state.insert("subject_id".into(), id.clone());
    view_state.insert("label".into(), spec.label.clone());
    view_state.insert("description".into(), spec.description.clone());
    let container = SeedContainer {
        id: super::canvas_state::next_container_id("subject"),
        container_type: "subject".into(),
        title: format!("Subject · {}", spec.label),
        x: 380.0,
        y: 80.0,
        width: 320.0,
        height: 180.0,
        z: 120.0,
        honesty: "live".into(),
        view_state,
        ..Default::default()
    };
    append_seed_container(container);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        super::interactions::show_tool_status(
            &document,
            "Subject",
            &format!("Declared `{}` ({id}) on {manifold_id}.", spec.label),
            "success",
        );
    }
    id
}

fn persist_subject_cop(subject: &SubjectSeed) {
    if !is_daemon_connected() {
        return;
    }
    let subject = subject.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut fields = serde_json::Map::new();
        fields.insert("subject_id".into(), subject.id.clone().into());
        fields.insert("description".into(), subject.description.clone().into());
        fields.insert("construct".into(), subject.construct_id.clone().into());
        fields.insert("manifold".into(), subject.manifold_id.clone().into());
        fields.insert("observer".into(), subject.observer.clone().into());
        fields.insert("library_uri".into(), subject.library_uri().into());
        let _ = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "poet_subject".into(),
            title: subject.label,
            id: None,
            fields,
        })
        .await;
    });
}

fn place_nested_portal(document: &Document, target_manifold: &str, label: &str) {
    let container = SeedContainer {
        id: super::canvas_state::next_container_id("nested_manifold"),
        container_type: "nested_manifold".into(),
        title: format!("Lens · {label}"),
        x: 80.0,
        y: 80.0,
        width: 280.0,
        height: 140.0,
        z: 120.0,
        honesty: "live".into(),
        target_manifold: target_manifold.into(),
        ..Default::default()
    };
    append_seed_container_to(document, container);
}

fn append_seed_container(container: SeedContainer) {
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        append_seed_container_to(&document, container);
        super::history::push_current_frame("author container");
    }
}

fn append_seed_container_to(document: &Document, container: SeedContainer) {
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::containers::build_container(document, &container);
        if let Some(content) = canvas
            .query_selector(".canvas-content-layer")
            .ok()
            .flatten()
        {
            content.append_child(&el).ok();
        } else {
            canvas.append_child(&el).ok();
        }
        super::interactions::wire_container_selection(document);
        super::interactions::wire_container_dragging(document);
        super::interactions::wire_container_resize(document);
        super::interactions::wire_container_deletion(document);
        super::interactions::wire_port_dragging(document);
        super::canvas_extent::ensure_manifold_extent(document);
        super::canvas_extent::pan_to_show(
            document,
            container.x,
            container.y,
            container.width,
            container.height,
        );
    }
}

/// Dialog for the pager `+` control — author a lens, container, link, or subject.
pub fn open_authoring_dialog(document: &Document) {
    open_authoring_dialog_kind(document, "lens");
}

pub fn open_authoring_dialog_kind(document: &Document, kind: &str) {
    if let Some(existing) = document.get_element_by_id("manifold-authoring-dialog") {
        existing.remove();
    }
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("manifold-authoring-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; inset: 0; background: rgba(0,0,0,0.55); z-index: 10020; \
         display: flex; align-items: flex-start; justify-content: center; padding-top: 12vh;",
    );
    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 440px; background: var(--surface-glass-heavy); border: 1px solid var(--border-medium); \
         border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 8px;",
    );
    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Author in this construct"));
    panel.append_child(&title).unwrap();
    let hint = document.create_element("div").unwrap();
    hint.set_text_content(Some(
        "Means to author — not a canned world. Vibe: Poet.manifold_create / container_place / nested_link / subject_declare",
    ));
    let hint_el: HtmlElement = hint.clone().dyn_into().unwrap();
    hint_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    panel.append_child(&hint).unwrap();

    let kind_select = document.create_element("select").unwrap();
    kind_select.set_attribute("data-author-kind", "").ok();
    for (value, label) in [
        ("lens", "Manifold (lens)"),
        ("container", "Container"),
        ("nested", "Nested link"),
        ("subject", "Subject"),
        ("participant", "Invite person (social lens)"),
    ] {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", value).ok();
        option.set_text_content(Some(label));
        if value == kind {
            option.set_attribute("selected", "").ok();
        }
        kind_select.append_child(&option).unwrap();
    }
    panel.append_child(&kind_select).unwrap();

    let label = document.create_element("input").unwrap();
    label.set_attribute("data-author-label", "").ok();
    label
        .set_attribute("placeholder", "Name (lens / container / subject)")
        .ok();
    panel.append_child(&label).unwrap();
    let extra = document.create_element("input").unwrap();
    extra.set_attribute("data-author-extra", "").ok();
    extra
        .set_attribute(
            "placeholder",
            "container_type, nested target, or leave blank",
        )
        .ok();
    panel.append_child(&extra).unwrap();
    let description = document.create_element("input").unwrap();
    description
        .set_attribute("data-author-description", "")
        .ok();
    description
        .set_attribute("placeholder", "Description (optional)")
        .ok();
    panel.append_child(&description).unwrap();

    let nest_row = document.create_element("label").unwrap();
    nest_row.set_text_content(Some(
        " Leave a nested-manifold portal on the current surface (lenses)",
    ));
    let nest = document.create_element("input").unwrap();
    nest.set_attribute("type", "checkbox").ok();
    nest.set_attribute("data-author-nest", "").ok();
    nest_row
        .insert_before(&nest, nest_row.first_child().as_ref())
        .ok();
    panel.append_child(&nest_row).unwrap();
    let social_row = document.create_element("label").unwrap();
    social_row.set_text_content(Some(" Social lens (many people — important for projects)"));
    let social = document.create_element("input").unwrap();
    social.set_attribute("type", "checkbox").ok();
    social.set_attribute("data-author-social", "").ok();
    if kind == "participant" {
        social.set_attribute("checked", "").ok();
    }
    social_row
        .insert_before(&social, social_row.first_child().as_ref())
        .ok();
    panel.append_child(&social_row).unwrap();

    let actions = document.create_element("div").unwrap();
    let create = document.create_element("button").unwrap();
    create.set_attribute("type", "button").ok();
    create.set_text_content(Some("Author"));
    let cancel = document.create_element("button").unwrap();
    cancel.set_attribute("type", "button").ok();
    cancel.set_text_content(Some("Cancel"));
    actions.append_child(&create).unwrap();
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

    let overlay_create = overlay.clone();
    let panel_create = panel.clone();
    let create_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let kind = panel_create
            .query_selector("[data-author-kind]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
            .map(|el| el.value())
            .unwrap_or_else(|| "lens".into());
        let label = panel_create
            .query_selector("[data-author-label]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let extra = panel_create
            .query_selector("[data-author-extra]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let description = panel_create
            .query_selector("[data-author-description]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let nest = panel_create
            .query_selector("[data-author-nest]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.checked())
            .unwrap_or(false);
        let social = panel_create
            .query_selector("[data-author-social]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.checked())
            .unwrap_or(false);
        let label = label.trim();
        let extra = extra.trim();
        let op = match kind.as_str() {
            "container" => {
                let container_type = if extra.is_empty() { "doc" } else { extra };
                let title = if label.is_empty() {
                    container_type
                } else {
                    label
                };
                AuthoringOp::ContainerPlace(ContainerPlace {
                    container_type: container_type.to_string(),
                    title: title.to_string(),
                })
            }
            "nested" => {
                if extra.is_empty() {
                    return;
                }
                AuthoringOp::NestedLink(NestedLink {
                    to: extra.to_string(),
                    title: if label.is_empty() {
                        extra.to_string()
                    } else {
                        label.to_string()
                    },
                    target_construct: String::new(),
                })
            }
            "subject" => {
                if label.is_empty() {
                    return;
                }
                AuthoringOp::SubjectDeclare(SubjectDeclare {
                    label: label.to_string(),
                    description,
                })
            }
            "participant" => {
                let did = if extra.is_empty() { label } else { extra };
                if did.is_empty() {
                    return;
                }
                AuthoringOp::ParticipantInvite(ParticipantInvite {
                    did: did.to_string(),
                    role: if description.trim().is_empty() {
                        "member".into()
                    } else {
                        description.trim().to_string()
                    },
                    label: if extra.is_empty() {
                        String::new()
                    } else {
                        label.to_string()
                    },
                })
            }
            _ => {
                if label.is_empty() {
                    return;
                }
                AuthoringOp::ManifoldCreate(ManifoldCreate {
                    label: label.to_string(),
                    description,
                    nest,
                    social,
                })
            }
        };
        apply_authoring_ops(&[op]);
        overlay_create.remove();
    }) as Box<dyn FnMut(_)>);
    create
        .add_event_listener_with_callback("click", create_closure.as_ref().unchecked_ref())
        .unwrap();
    create_closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vibe_manifold_create() {
        let source = r#"
            capability.invoke("Poet.manifold_create", { label: "Cellular structure", nest: true })
        "#;
        let ops = parse_manifold_creates(source);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].label, "Cellular structure");
        assert!(ops[0].nest);
        assert!(!ops[0].social);
    }

    #[test]
    fn parses_social_manifold_and_invite() {
        let source = r#"
            capability.invoke("Poet.manifold_create", { label: "Camping sites", social: true })
            capability.invoke("Poet.participant_invite", { did: "did:qualia:alice", role: "member" })
        "#;
        let ops = parse_authoring_ops(source);
        assert_eq!(ops.len(), 2);
        match &ops[0] {
            AuthoringOp::ManifoldCreate(s) => assert!(s.social),
            other => panic!("{other:?}"),
        }
        match &ops[1] {
            AuthoringOp::ParticipantInvite(p) => {
                assert_eq!(p.did, "did:qualia:alice");
                assert_eq!(p.role, "member");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_all_poet_ops_in_order() {
        let source = r#"
            capability.invoke("Poet.manifold_create", { label: "Cellular", nest: true })
            capability.invoke("Poet.container_place", { container_type: "doc", title: "Notes" })
            capability.invoke("Poet.nested_link", { to: "anatomy", title: "Anatomy lens" })
            capability.invoke("Poet.subject_declare", { label: "North Spring", description: "catchment" })
        "#;
        let ops = parse_authoring_ops(source);
        assert_eq!(ops.len(), 4);
        match &ops[1] {
            AuthoringOp::ContainerPlace(p) => {
                assert_eq!(p.container_type, "doc");
                assert_eq!(p.title, "Notes");
            }
            other => panic!("{other:?}"),
        }
        match &ops[2] {
            AuthoringOp::NestedLink(l) => assert_eq!(l.to, "anatomy"),
            other => panic!("{other:?}"),
        }
        match &ops[3] {
            AuthoringOp::SubjectDeclare(s) => assert_eq!(s.label, "North Spring"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ignores_unrelated_vibe() {
        let ops = parse_authoring_ops("capability.invoke(\"CapabilityDiscovery.list\", {})");
        assert!(ops.is_empty());
    }
}
