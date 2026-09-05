//! Construct shelf and nested-manifold / construct portals.
//!
//! Library Software is the index. This shelf is the desk: openable compositions.

use base64::Engine;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlAnchorElement, HtmlElement};

use crate::tool_chest::constructs::{all_constructs, construct_by_id};
use crate::tool_chest::core::construct::ConstructSource;
use crate::tool_chest::core::registry::SeedContainer;

use super::live_invoke;

fn card_style() -> &'static str {
    "border: 1px solid var(--border-subtle); border-radius: 6px; padding: 8px; \
     display: flex; flex-direction: column; gap: 4px; background: var(--surface-panel);"
}

/// Installed / bundled constructs. Stubs are listed but cannot be opened.
pub fn build_construct_shelf_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("construct-shelf");
    super::surface_aspects::mark(&root, "entrance");
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );

    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Construct shelf — observer-scopes (worlds for someone looking). Manifolds are lenses inside a construct. Anatomy is a manifold, not a construct. Library Software is discovery; this is the desk.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    root.append_child(&note).unwrap();

    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px;",
    );

    for seed in all_constructs() {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(card_style());
        card.set_attribute("data-construct-id", &seed.id).ok();
        card.set_attribute("data-honesty", &seed.honesty).ok();

        let title = document.create_element("div").unwrap();
        title.set_text_content(Some(&format!("{}  {}", seed.label, seed.honesty)));
        let title_el: HtmlElement = title.clone().dyn_into().unwrap();
        title_el
            .style()
            .set_css_text("font-size: 12px; font-weight: 700; color: var(--text-primary);");
        card.append_child(&title).unwrap();

        let desc = document.create_element("div").unwrap();
        desc.set_text_content(Some(&seed.description));
        let desc_el: HtmlElement = desc.clone().dyn_into().unwrap();
        desc_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&desc).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "{} · {} · {}",
            seed.source.as_str(),
            seed.library_uri,
            seed.manifold_ids.join(", ")
        )));
        let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
        meta_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        let open = document.create_element("button").unwrap();
        open.set_attribute("type", "button").ok();
        open.set_attribute("data-construct-open", &seed.id).ok();
        if seed.source == ConstructSource::Stub || seed.default_manifold.is_empty() {
            open.set_text_content(Some("Unavailable: no manifold seed"));
            open.set_attribute("disabled", "").ok();
            open.set_attribute("aria-disabled", "true").ok();
            open.set_attribute(
                "title",
                "Library Software stub. Not a pager tab until a construct seed exists.",
            )
            .ok();
        } else {
            open.set_text_content(Some("Open construct"));
            let id = seed.id.clone();
            let default = seed.default_manifold.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                super::open_construct(&id, Some(&default));
            }) as Box<dyn FnMut(_)>);
            open.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
        card.append_child(&open).unwrap();

        if seed.source != ConstructSource::Stub && !seed.default_manifold.is_empty() {
            for (label, archive) in [
                ("Export construct .hcf", false),
                ("Archive construct .hmc", true),
            ] {
                let export = document.create_element("button").unwrap();
                export.set_attribute("type", "button").ok();
                export.set_text_content(Some(label));
                let construct_id = seed.id.clone();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                        export_construct(&document, &construct_id, archive);
                    }
                }) as Box<dyn FnMut(_)>);
                export
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
                card.append_child(&export).unwrap();
            }
        }
        grid.append_child(&card).unwrap();
    }
    root.append_child(&grid).unwrap();
    root.append_child(&live_invoke::action_bar(
        document,
        &[(
            "CapabilityDiscovery.list",
            "CapabilityDiscovery.list",
            serde_json::json!({}),
        )],
    ))
    .unwrap();
    root
}

pub(crate) fn export_construct(document: &Document, construct_id: &str, archive: bool) {
    let Some(mut construct) = construct_by_id(construct_id) else {
        super::interactions::show_tool_status(
            document,
            "Construct export",
            "Unknown construct.",
            "error",
        );
        return;
    };
    let manifolds = if super::current_construct_id() == construct_id {
        super::history::sync_persistence_state();
        super::visible_seeds()
    } else if construct.id == "poet" {
        super::get_current_seeds()
    } else {
        super::get_current_seeds()
            .into_iter()
            .filter(|seed| construct.contains_manifold(&seed.id))
            .collect()
    };
    construct.manifold_ids = manifolds.iter().map(|seed| seed.id.clone()).collect();
    let observer = super::current_observer_did();
    let author = if observer.is_empty() {
        super::manifest::DEFAULT_ACTOR_DID
    } else {
        observer.as_str()
    };
    let result = if archive {
        super::manifest::export_construct_hmc(&construct, &manifolds, author)
    } else {
        super::manifest::export_construct_hcf(&construct, &manifolds, author)
    };
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => {
            super::interactions::show_tool_status(document, "Construct export", &error, "error");
            return;
        }
    };
    let extension = if archive { "hmc" } else { "hcf" };
    let mime = if archive {
        "application/vnd.qualia.hmc"
    } else {
        "application/vnd.qualia.hcf"
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let anchor: HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
    anchor.set_href(&format!("data:{mime};base64,{encoded}"));
    anchor.set_download(&format!("{}.{}", construct.id, extension));
    anchor.click();
    super::interactions::show_tool_status(
        document,
        "Construct export",
        &format!(
            "Exported `{}` with {} manifold(s) as checksummed .{}.",
            construct.id,
            manifolds.len(),
            extension
        ),
        "success",
    );
}

/// Subject card — the thing under consideration on this lens.
pub fn build_subject_view(document: &Document, container: &SeedContainer) -> Element {
    let subject_id = container
        .view_state
        .get("subject_id")
        .cloned()
        .unwrap_or_default();
    let seed = super::declared_subjects()
        .into_iter()
        .find(|candidate| candidate.id == subject_id);
    let label = seed
        .as_ref()
        .map(|s| s.label.clone())
        .or_else(|| container.view_state.get("label").cloned())
        .unwrap_or_else(|| container.title.clone());
    let description = seed
        .as_ref()
        .map(|s| s.description.clone())
        .or_else(|| container.view_state.get("description").cloned())
        .unwrap_or_default();
    let construct_id = seed
        .as_ref()
        .map(|s| s.construct_id.clone())
        .unwrap_or_else(super::current_construct_id);
    let manifold_id = seed
        .as_ref()
        .map(|s| s.manifold_id.clone())
        .unwrap_or_default();
    let uri = seed
        .as_ref()
        .map(|s| s.library_uri())
        .unwrap_or_else(|| format!("urn:poet:subject:{subject_id}"));

    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px; padding: 8px;");
    let heading = document.create_element("div").unwrap();
    heading.set_text_content(Some(&format!("Subject · {label}")));
    let heading_el: HtmlElement = heading.clone().dyn_into().unwrap();
    heading_el
        .style()
        .set_css_text("font-size: 12px; font-weight: 700;");
    root.append_child(&heading).unwrap();
    let body = document.create_element("div").unwrap();
    body.set_text_content(Some(if description.is_empty() {
        "Authored focus in this construct. Not a canned world and not a construct."
    } else {
        &description
    }));
    let body_el: HtmlElement = body.clone().dyn_into().unwrap();
    body_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    root.append_child(&body).unwrap();
    let meta = document.create_element("div").unwrap();
    meta.set_text_content(Some(&format!(
        "{uri} · construct:{construct_id} · lens:{manifold_id}"
    )));
    let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
    meta_el
        .style()
        .set_css_text("font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);");
    root.append_child(&meta).unwrap();
    root
}

/// Nested-manifold portal: open the target manifold on this construct.
pub fn build_nested_manifold_view(document: &Document, target_manifold: &str) -> Element {
    portal_card(
        document,
        "Nested manifold",
        &format!(
            "This container is a manifold, not a nested app. Open `{target_manifold}` on the pager."
        ),
        "Open nested manifold",
        "",
        target_manifold,
    )
}

/// Construct portal: open another construct (optional inner manifold).
pub fn build_construct_portal_view(
    document: &Document,
    target_construct: &str,
    target_manifold: &str,
) -> Element {
    let label = construct_by_id(target_construct)
        .map(|c| c.label)
        .unwrap_or_else(|| target_construct.to_string());
    portal_card(
        document,
        &format!("Construct · {label}"),
        "Opens a packaged composition. Not a QApp runtime.",
        "Open construct",
        target_construct,
        target_manifold,
    )
}

fn portal_card(
    document: &Document,
    heading: &str,
    body: &str,
    button_label: &str,
    target_construct: &str,
    target_manifold: &str,
) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; padding: 8px;");
    let title = document.create_element("div").unwrap();
    title.set_text_content(Some(heading));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-size: 12px; font-weight: 700;");
    root.append_child(&title).unwrap();
    let text = document.create_element("div").unwrap();
    text.set_text_content(Some(body));
    let text_el: HtmlElement = text.clone().dyn_into().unwrap();
    text_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    root.append_child(&text).unwrap();

    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(button_label));
    let construct = target_construct.to_string();
    let manifold = target_manifold.to_string();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !construct.is_empty() {
            super::open_construct(
                &construct,
                if manifold.is_empty() {
                    None
                } else {
                    Some(&manifold)
                },
            );
        } else if !manifold.is_empty() {
            super::dive_nested_manifold(&manifold);
        }
    }) as Box<dyn FnMut(_)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    root.append_child(&button).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use crate::tool_chest::constructs::construct_by_id;

    #[test]
    fn anatomy_is_a_manifold_on_health_not_a_construct() {
        assert!(construct_by_id("anatomy").is_none());
        let health = construct_by_id("health").unwrap();
        assert_eq!(health.default_manifold, "health");
        assert!(health.contains_manifold("anatomy"));
    }
}
