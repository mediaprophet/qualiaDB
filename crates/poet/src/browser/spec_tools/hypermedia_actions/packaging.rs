//! Broadcast packaging, social moderation, and package export.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "hypermedia:social-moderation" => Some(tag_attr(container, "data-social-moderation", "rules_applied")),
        "hypermedia:hbbtv-package" => Some(tag_attr(container, "data-hbbtv-package", "app_manifest_built")),
        "hypermedia:ait-config" => Some(tag_attr(container, "data-ait-config", "application_table_filled")),
        "hypermedia:dvb-stream-bind" => Some(tag_attr(container, "data-dvb-stream", "service_bound")),
        "hypermedia:drm-config" => Some(tag_attr(container, "data-drm-config", "rights_lock_set")),
        "hypermedia:app-data-bundle" => Some(tag_attr(container, "data-app-data-bundle", "aux_files_packed")),
        "hypermedia:package-validate" => Some(tag_attr(container, "data-package-validate", "rules_passed")),
        "hypermedia:package-export" => Some(tag_attr(container, "data-package-export", "broadcaster_bundle_ready")),
        "hypermedia:open-graph-meta" => Some(tag_open_graph(container)),
        "hypermedia:activitypub-outbox" => Some(tag_activitypub(container)),
        _ => None,
    }
}

fn tag_open_graph(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-og-title", "Poet Document Canvas")
        .map_err(|_| "Failed to write OpenGraph title.".to_string())?;
    let _ = container.set_attribute("data-og-type", "article");
    Ok(())
}

fn tag_activitypub(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-activitypub-outbox", "https://qualiadb.org/outbox/user")
        .map_err(|_| "Failed to bind ActivityPub outbox.".to_string())
}

fn tag_attr(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to set {key}."))
}
