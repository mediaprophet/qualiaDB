//! Authoritative canvas identity and DOM snapshot helpers.
//!
//! Browser DOM is the active editing surface. This module converts that surface
//! back into the serialisable manifold model so history and exports do not lose
//! dynamically created containers, deleted nodes, or semantic wire metadata.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};

use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

use crate::tool_chest::core::registry::{
    ContainerKind, ManifoldSeed, SeedConnection, SeedContainer,
};

static NEXT_DYNAMIC_ID: AtomicU32 = AtomicU32::new(1);

pub fn normalise_seed_ids(seeds: &mut [ManifoldSeed]) {
    for seed in seeds {
        let mut container_ids = HashSet::new();
        for (index, container) in seed.containers.iter_mut().enumerate() {
            let original = container.id.trim();
            let mut candidate = if original.is_empty() || container_ids.contains(original) {
                seeded_container_id(&seed.id, index, &container.container_type)
            } else {
                original.to_string()
            };
            let mut suffix = 2usize;
            while container_ids.contains(&candidate) {
                candidate = format!(
                    "{}-{}",
                    seeded_container_id(&seed.id, index, &container.container_type),
                    suffix
                );
                suffix += 1;
            }
            container_ids.insert(candidate.clone());
            container.id = candidate;
        }

        let mut wire_ids = HashSet::new();
        for (index, connection) in seed.connections.iter_mut().enumerate() {
            let original = connection.id.trim();
            let base = format!("wire-{}-{}", slug(&seed.id), index);
            let mut candidate = if original.is_empty() || wire_ids.contains(original) {
                base.clone()
            } else {
                original.to_string()
            };
            let mut suffix = 2usize;
            while wire_ids.contains(&candidate) {
                candidate = format!("{}-{}", base, suffix);
                suffix += 1;
            }
            wire_ids.insert(candidate.clone());
            connection.id = candidate;
        }
    }
}

pub fn next_container_id(container_type: &str) -> String {
    let serial = NEXT_DYNAMIC_ID.fetch_add(1, Ordering::SeqCst);
    format!(
        "container-{}-{}-{}",
        slug(container_type),
        js_sys::Date::now() as u64,
        serial
    )
}

pub fn next_wire_id() -> String {
    let serial = NEXT_DYNAMIC_ID.fetch_add(1, Ordering::SeqCst);
    format!("wire-{}-{}", js_sys::Date::now() as u64, serial)
}

pub fn snapshot_seed_from_dom(document: &Document, base: &ManifoldSeed) -> ManifoldSeed {
    let mut seed = base.clone();
    if let Some(label) = document
        .query_selector(".canvas-title-input")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .filter(|label| !label.trim().is_empty())
    {
        seed.label = label;
    }
    seed.containers.clear();
    seed.connections.clear();

    let mut index_by_id = HashMap::new();
    if let Ok(nodes) = document.query_selector_all(".canvas-content-layer > .canvas-container-node")
    {
        for i in 0..nodes.length() {
            let Some(node) = nodes.get(i) else { continue };
            let Ok(element) = node.dyn_into::<Element>() else {
                continue;
            };
            let container = container_from_element(&element);
            index_by_id.insert(container.id.clone(), seed.containers.len());
            seed.containers.push(container);
        }
    }

    if let Ok(paths) =
        document.query_selector_all(".canvas-content-layer .wire-overlay path[data-id]")
    {
        for i in 0..paths.length() {
            let Some(node) = paths.get(i) else { continue };
            let Ok(path) = node.dyn_into::<Element>() else {
                continue;
            };
            let source_id = path.get_attribute("data-source-id").unwrap_or_default();
            let target_id = path.get_attribute("data-target-id").unwrap_or_default();
            let (Some(&from), Some(&to)) =
                (index_by_id.get(&source_id), index_by_id.get(&target_id))
            else {
                continue;
            };
            seed.connections.push(SeedConnection {
                id: path.get_attribute("data-id").unwrap_or_else(next_wire_id),
                from,
                to,
                wire_type: path
                    .get_attribute("data-modality")
                    .unwrap_or_else(|| "active".into()),
                label: path
                    .get_attribute("data-predicate")
                    .unwrap_or_else(|| "doc:references".into()),
            });
        }
    }

    seed
}

pub fn container_from_element(element: &Element) -> SeedContainer {
    let style = element.get_attribute("style").unwrap_or_default();
    let container_type = element
        .get_attribute("data-container-type")
        .unwrap_or_else(|| "doc".into());
    SeedContainer {
        id: element
            .get_attribute("data-id")
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| next_container_id(&container_type)),
        kind: ContainerKind::from_type(&container_type),
        container_type,
        title: element
            .query_selector(".container-title")
            .ok()
            .flatten()
            .and_then(|title| title.text_content())
            .unwrap_or_else(|| "Untitled".into()),
        x: style_number(&style, "left", 0.0),
        y: style_number(&style, "top", 0.0),
        width: style_number(&style, "width", 400.0),
        height: style_number(&style, "height", 300.0),
        z: style_number(&style, "z-index", 100.0),
        honesty: element
            .query_selector(".honesty-badge")
            .ok()
            .flatten()
            .and_then(|badge| badge.text_content())
            .unwrap_or_else(|| "missing".into()),
        semantic_type: element
            .get_attribute("data-semantic-type")
            .unwrap_or_default(),
        semantic_uri: element
            .get_attribute("data-semantic-uri")
            .unwrap_or_default(),
        content_html: element
            .query_selector(".doc-editor")
            .ok()
            .flatten()
            .map(|editor| editor.inner_html())
            .unwrap_or_default(),
        tool_settings: element
            .get_attribute("data-tool-settings")
            .and_then(|settings| serde_json::from_str::<BTreeMap<String, String>>(&settings).ok())
            .unwrap_or_default(),
        view_state: super::view_state::capture(element),
        target_manifold: element
            .get_attribute("data-target-manifold")
            .unwrap_or_default(),
        target_construct: element
            .get_attribute("data-target-construct")
            .unwrap_or_default(),
    }
}

fn seeded_container_id(manifold: &str, index: usize, container_type: &str) -> String {
    format!(
        "container-{}-{}-{}",
        slug(manifold),
        index,
        slug(container_type)
    )
}

fn slug(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_dash = false;
    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if !last_dash {
            last_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(ch) = mapped {
            out.push(ch);
        }
    }
    out.trim_matches('-').to_string()
}

fn style_number(style: &str, property: &str, default: f32) -> f32 {
    style
        .split(';')
        .filter_map(|part| part.split_once(':'))
        .find_map(|(name, value)| {
            (name.trim() == property)
                .then(|| value.trim().trim_end_matches("px").parse::<f32>().ok())
                .flatten()
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_is_stable_and_unique_within_a_manifold() {
        let mut seeds = vec![ManifoldSeed {
            id: "research".into(),
            label: "Research".into(),
            icon: String::new(),
            ontology_prefix: String::new(),
            description: String::new(),
            containers: vec![
                SeedContainer::new("doc", "One", 0.0, 0.0, 1.0, 1.0),
                SeedContainer::new("doc", "Two", 0.0, 0.0, 1.0, 1.0),
            ],
            connections: vec![],
            panels: vec![],
            ..Default::default()
        }];
        normalise_seed_ids(&mut seeds);
        assert_eq!(seeds[0].containers[0].id, "container-research-0-doc");
        assert_eq!(seeds[0].containers[1].id, "container-research-1-doc");
        let first = seeds.clone();
        normalise_seed_ids(&mut seeds);
        assert_eq!(seeds, first);
    }

    #[test]
    fn style_parser_accepts_compact_and_spaced_css() {
        assert_eq!(style_number("left: 24px; top:32px", "left", 0.0), 24.0);
        assert_eq!(style_number("left: 24px; top:32px", "top", 0.0), 32.0);
        assert_eq!(style_number("left: -40px; top: -8px", "left", 0.0), -40.0);
        assert_eq!(style_number("left: -40px; top: -8px", "top", 0.0), -8.0);
    }

    #[test]
    fn normalisation_repairs_duplicate_container_and_wire_ids() {
        let mut seeds = vec![ManifoldSeed {
            id: "legacy".into(),
            containers: vec![
                SeedContainer {
                    id: "container-doc".into(),
                    ..SeedContainer::new("doc", "One", 0.0, 0.0, 1.0, 1.0)
                },
                SeedContainer {
                    id: "container-doc".into(),
                    ..SeedContainer::new("doc", "Two", 0.0, 0.0, 1.0, 1.0)
                },
            ],
            connections: vec![
                SeedConnection {
                    id: "wire-old".into(),
                    from: 0,
                    to: 1,
                    wire_type: "active".into(),
                    label: "a".into(),
                },
                SeedConnection {
                    id: "wire-old".into(),
                    from: 1,
                    to: 0,
                    wire_type: "active".into(),
                    label: "b".into(),
                },
            ],
            ..Default::default()
        }];

        normalise_seed_ids(&mut seeds);

        assert_eq!(seeds[0].containers[0].id, "container-doc");
        assert_eq!(seeds[0].containers[1].id, "container-legacy-1-doc");
        assert_eq!(seeds[0].connections[0].id, "wire-old");
        assert_eq!(seeds[0].connections[1].id, "wire-legacy-1");
    }
}
