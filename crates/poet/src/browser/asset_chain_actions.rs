//! Dual-path Tool Chest actions for curated Host-bound `Asset.*` ids (wave 25).

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_ID: &str = "asset-1";
const DEFAULT_KIND: &str = "document";
const DEFAULT_KIND_IRI: &str = "q42:Document";
const DEFAULT_ASPECT: &str = "production";
const DEFAULT_TOPIC: &str = "q42:topic";
const DEFAULT_ANCHOR: &str = "anchor-1";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn local_id(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn sketch_asset(asset_id: &str) -> serde_json::Value {
    json!({
        "asset_id": asset_id,
        "kind": DEFAULT_KIND_IRI,
        "temporal_aspects": [],
        "topics": []
    })
}

fn resolve_id(document: &Document) -> String {
    local_id(
        string_attr(selected_container(document).as_ref(), "data-asset-id").as_deref(),
        DEFAULT_ID,
    )
}

/// `Asset.create` — `{ asset_id, kind, owner_did? }`.
pub(super) fn run_create(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.create",
        format!("asset create sketch id={id} kind={DEFAULT_KIND}"),
        json!({ "asset_id": id, "kind": DEFAULT_KIND }),
    );
}

/// `Asset.add_temporal` — `{ asset, kind, secs, nanos? }`.
pub(super) fn run_add_temporal(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.add_temporal",
        format!("asset add_temporal sketch id={id} kind={DEFAULT_ASPECT} secs=0"),
        json!({ "asset": sketch_asset(&id), "kind": DEFAULT_ASPECT, "secs": 0, "nanos": 0 }),
    );
}

/// `Asset.add_topic` — `{ asset, topic, subject, relation }`.
pub(super) fn run_add_topic(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.add_topic",
        format!("asset add_topic sketch id={id} topic={DEFAULT_TOPIC}"),
        json!({
            "asset": sketch_asset(&id),
            "topic": DEFAULT_TOPIC,
            "subject": id,
            "relation": "q42:about"
        }),
    );
}

/// `Asset.set_spatial` — `{ asset, anchor_id, lat, lon, alt }`.
pub(super) fn run_set_spatial(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.set_spatial",
        format!("asset set_spatial sketch id={id} anchor={DEFAULT_ANCHOR}"),
        json!({
            "asset": sketch_asset(&id),
            "anchor_id": DEFAULT_ANCHOR,
            "lat": 0.0,
            "lon": 0.0,
            "alt": 0.0
        }),
    );
}

/// `Asset.compile` — `{ asset }`.
pub(super) fn run_compile(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.compile",
        format!("asset compile sketch id={id}"),
        json!({ "asset": sketch_asset(&id) }),
    );
}

/// `Asset.temporal_span` — `{ asset }`.
pub(super) fn run_temporal_span(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.temporal_span",
        format!("asset temporal_span sketch id={id} (needs ≥2 aspects)"),
        json!({ "asset": sketch_asset(&id) }),
    );
}

/// `Asset.query_aspects` — `{ asset, kind }`.
pub(super) fn run_query_aspects(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.query_aspects",
        format!("asset query_aspects sketch id={id} kind={DEFAULT_ASPECT}"),
        json!({ "asset": sketch_asset(&id), "kind": DEFAULT_ASPECT }),
    );
}

/// `Asset.persist` — `{ asset_id, kind }`.
pub(super) fn run_persist(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.persist",
        format!("asset persist sketch id={id} kind={DEFAULT_KIND}"),
        json!({ "asset_id": id, "kind": DEFAULT_KIND }),
    );
}

/// `Asset.resolve` — `{ asset_id }`.
pub(super) fn run_resolve(document: &Document, label: &str) {
    let id = resolve_id(document);
    invoke_dual(
        document,
        label,
        "Asset.resolve",
        format!("asset resolve sketch id={id}"),
        json!({ "asset_id": id }),
    );
}

/// `Asset.resolve_by_spatial` — `{ anchor_iri }`.
pub(super) fn run_resolve_by_spatial(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Asset.resolve_by_spatial",
        format!("asset resolve_by_spatial sketch anchor={DEFAULT_ANCHOR}"),
        json!({ "anchor_iri": DEFAULT_ANCHOR }),
    );
}

/// `Asset.resolve_by_topic` — `{ topic }`.
pub(super) fn run_resolve_by_topic(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Asset.resolve_by_topic",
        format!("asset resolve_by_topic sketch topic={DEFAULT_TOPIC}"),
        json!({ "topic": DEFAULT_TOPIC }),
    );
}

/// `Asset.resolve_by_temporal` — `{ kind_iri }`.
pub(super) fn run_resolve_by_temporal(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Asset.resolve_by_temporal",
        "asset resolve_by_temporal sketch kind_iri=q42:productionDate".into(),
        json!({ "kind_iri": "q42:productionDate" }),
    );
}

/// `Asset.list` — no required args.
pub(super) fn run_list(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Asset.list",
        "asset list sketch count=0".into(),
        json!({}),
    );
}

/// `Asset.count` — no required args.
pub(super) fn run_count(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Asset.count",
        "asset count sketch count=0".into(),
        json!({}),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_asset_wave25_defaults() {
        assert_eq!(local_id(None, DEFAULT_ID), DEFAULT_ID);
        let asset = sketch_asset(DEFAULT_ID);
        assert_eq!(asset["asset_id"], DEFAULT_ID);
        assert_eq!(asset["kind"], DEFAULT_KIND_IRI);
    }
}
