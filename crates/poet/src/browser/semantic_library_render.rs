//! Safe DOM rendering for live Semantic Library query responses.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element};

use super::semantic_library_view::spawn_refresh;

fn render_construct_index(document: &Document, results: &Element, section: &str) {
    if section != "software" && section != "all" {
        return;
    }
    let heading = document.create_element("div").unwrap();
    heading.set_text_content(Some(
        "Constructs (your mindware environments). Infosphere/noosphere are broader layers these join.",
    ));
    results.append_child(&heading).unwrap();
    for seed in crate::tool_chest::constructs::all_constructs() {
        if seed.source == crate::tool_chest::core::construct::ConstructSource::Stub {
            continue;
        }
        let card = document.create_element("article").unwrap();
        card.set_class_name("poet-library-entry");
        let title = document.create_element("strong").unwrap();
        title.set_text_content(Some(&format!("{} · {}", seed.label, seed.library_uri)));
        card.append_child(&title).unwrap();
        let meta = document.create_element("div").unwrap();
        meta.set_class_name("cr-meta");
        meta.set_text_content(Some(&format!(
            "construct · {} · lenses: {}",
            seed.honesty,
            seed.manifold_ids.join(", ")
        )));
        card.append_child(&meta).unwrap();
        let open = document.create_element("button").unwrap();
        open.set_attribute("type", "button").ok();
        open.set_text_content(Some("Open construct"));
        let id = seed.id.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            crate::browser::open_construct(&id, None);
        }) as Box<dyn FnMut(_)>);
        open.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        card.append_child(&open).unwrap();
        results.append_child(&card).unwrap();
    }
    let subjects = crate::browser::declared_subjects();
    if subjects.is_empty() {
        return;
    }
    let heading = document.create_element("div").unwrap();
    heading.set_text_content(Some(
        "Subjects (things under consideration). Not constructs; aspects become nested manifolds.",
    ));
    results.append_child(&heading).unwrap();
    for seed in subjects {
        let card = document.create_element("article").unwrap();
        card.set_class_name("poet-library-entry");
        let title = document.create_element("strong").unwrap();
        title.set_text_content(Some(&format!("{} · {}", seed.label, seed.library_uri())));
        card.append_child(&title).unwrap();
        let meta = document.create_element("div").unwrap();
        meta.set_class_name("cr-meta");
        meta.set_text_content(Some(&format!(
            "subject · construct:{} · lens:{}",
            seed.construct_id, seed.manifold_id
        )));
        card.append_child(&meta).unwrap();
        results.append_child(&card).unwrap();
    }
}

pub(super) fn render_results(root: &Element, data: &serde_json::Value) {
    let Ok(Some(results)) = root.query_selector(".poet-library-results") else {
        return;
    };
    results.set_inner_html("");
    let document = results.owner_document().unwrap();
    let section = root
        .get_attribute("data-library-section")
        .unwrap_or_else(|| "all".into());
    render_construct_index(&document, &results, &section);
    let entries = data
        .get("entries")
        .and_then(|value| value.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if entries.is_empty() {
        let empty = document.create_element("div").unwrap();
        empty.set_text_content(Some(
            "No persistent library entries match this query. This is an empty result, not a claim that the library is unavailable.",
        ));
        results.append_child(&empty).unwrap();
        return;
    }
    for entry in entries {
        let card = document.create_element("article").unwrap();
        card.set_class_name("poet-library-entry");
        let heading = document.create_element("strong").unwrap();
        heading.set_text_content(Some(str_field(
            entry,
            "asset_uri",
            "Unnamed semantic asset",
        )));
        card.append_child(&heading).unwrap();
        let meta = document.create_element("div").unwrap();
        meta.set_class_name("cr-meta");
        meta.set_text_content(Some(&format!(
            "{} · {} · {} Quins",
            str_field(entry, "section", "personal"),
            str_field(entry, "media_type", "unknown media"),
            entry
                .get("quin_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        )));
        card.append_child(&meta).unwrap();
        let excerpt = document.create_element("p").unwrap();
        excerpt.set_text_content(Some(str_field(entry, "excerpt", "No excerpt stored.")));
        card.append_child(&excerpt).unwrap();
        let topics = entry
            .get("topics")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| value.as_str())
                    .take(8)
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .unwrap_or_default();
        if !topics.is_empty() {
            let topic_line = document.create_element("small").unwrap();
            topic_line.set_text_content(Some(&format!("Topics: {topics}")));
            card.append_child(&topic_line).unwrap();
        }
        results.append_child(&card).unwrap();
    }
}

pub(super) fn render_facets(root: &Element, status: &Element, data: &serde_json::Value) {
    let Ok(Some(panel)) = root.query_selector(".poet-library-facets") else {
        return;
    };
    panel.set_inner_html("");
    let Some(facets) = data.get("facets") else {
        return;
    };
    let document = panel.owner_document().unwrap();
    for (label, field, attr) in [
        ("Topics", "topics", "data-library-topic"),
        ("Categories", "categories", "data-library-category"),
        ("Media", "media_types", "data-library-media-type"),
    ] {
        let group = document.create_element("div").unwrap();
        let title = document.create_element("strong").unwrap();
        title.set_text_content(Some(label));
        group.append_child(&title).unwrap();
        if let Some(values) = facets.get(field).and_then(|value| value.as_object()) {
            let mut ranked: Vec<_> = values.iter().collect();
            ranked.sort_by_key(|(_, count)| std::cmp::Reverse(count.as_u64().unwrap_or(0)));
            for (value, count) in ranked.into_iter().take(10) {
                let chip = button(
                    &document,
                    &format!("{value} ({})", count.as_u64().unwrap_or(0)),
                );
                let active = root.get_attribute(attr).as_deref() == Some(value.as_str());
                chip.set_attribute("aria-pressed", if active { "true" } else { "false" })
                    .ok();
                let root_clone = root.clone();
                let status_clone = status.clone();
                let attr = attr.to_string();
                let value = value.clone();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                    if root_clone.get_attribute(&attr).as_deref() == Some(value.as_str()) {
                        root_clone.remove_attribute(&attr).ok();
                    } else {
                        root_clone.set_attribute(&attr, &value).ok();
                    }
                    spawn_refresh(root_clone.clone(), status_clone.clone());
                }) as Box<dyn FnMut(_)>);
                chip.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
                group.append_child(&chip).unwrap();
            }
        }
        panel.append_child(&group).unwrap();
    }
}

pub(super) fn show_error(root: &Element, status: &Element, message: &str) {
    status.set_text_content(Some(&format!("Semantic Library unavailable: {message}")));
    if let Ok(Some(results)) = root.query_selector(".poet-library-results") {
        results.set_text_content(Some(
            "Start or reconnect the local QualiaDB daemon; no sample entries are substituted.",
        ));
    }
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_class_name("vibe-run-btn");
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn str_field<'a>(value: &'a serde_json::Value, field: &str, fallback: &'a str) -> &'a str {
    value
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or(fallback)
}
