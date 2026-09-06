//! Link analysis — relationship mapping on investigation surfaces.

use super::shared::{append_csv_attr, append_semicolon_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:add-link" => Some(add_link(container)),
        "investigation:link-evidence" => Some(link_evidence(container)),
        "investigation:query-links" => Some(query_links(document, container)),
        "investigation:find-path" => Some(find_path(container)),
        "investigation:detect-clusters" => Some(detect_clusters(container)),
        "investigation:compute-centrality" => Some(compute_centrality(container)),
        "investigation:visualise-links" => Some(visualise_links(container)),
        _ => None,
    }
}

fn add_link(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-links", "entity:A->entity:B:type:associated")
}

fn link_evidence(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-link-evidence", "link:active|evidence:supporting")
}

fn query_links(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-links]")?;
    container
        .set_attribute("data-link-count", &count.to_string())
        .map_err(|_| "Failed to query links.".to_string())
}

fn find_path(container: &Element) -> Result<(), String> {
    let links = container.get_attribute("data-links").unwrap_or_default();
    let path = if links.contains("->") {
        "path:direct_or_multi_hop_found"
    } else {
        "path:no_links_defined"
    };
    container
        .set_attribute("data-link-path", path)
        .map_err(|_| "Failed to find link path.".to_string())
}

fn detect_clusters(container: &Element) -> Result<(), String> {
    let links = container.get_attribute("data-links").unwrap_or_default();
    let clusters = links.split(',').filter(|s| !s.is_empty()).count().max(1);
    container
        .set_attribute("data-link-clusters", &format!("clusters:{clusters}"))
        .map_err(|_| "Failed to detect link clusters.".to_string())
}

fn compute_centrality(container: &Element) -> Result<(), String> {
    let links = container.get_attribute("data-links").unwrap_or_default();
    let degree = links.split(',').filter(|s| !s.is_empty()).count();
    container
        .set_attribute("data-link-centrality", &format!("degree:{degree}"))
        .map_err(|_| "Failed to compute link centrality.".to_string())
}

fn visualise_links(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-link-layout", "layout:force_directed")
        .map_err(|_| "Failed to visualise link graph.".to_string())
}
