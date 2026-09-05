//! Context, perspective, and Bayesian epistemic mutations on container surfaces.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:set-spatio-temporal-context" => Some(set_spatio_temporal_context(container)),
        "epistemic:set-social-context" => Some(set_social_context(container)),
        "epistemic:register-perspective" => Some(register_perspective(container)),
        "epistemic:add-bias" => Some(add_bias(container)),
        "epistemic:set-access-level" => Some(set_access_level(container)),
        "epistemic:set-horizon" => Some(set_horizon(container)),
        "epistemic:query-perspectives" => Some(query_perspectives(document, container)),
        "epistemic:add-prior" => Some(add_prior(container)),
        "epistemic:update-posterior" => Some(update_posterior(container)),
        "epistemic:set-threshold" => Some(set_threshold(container)),
        _ => None,
    }
}

pub(crate) fn next_access_level(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("public") => "commons",
        Some("commons") => "bilateral",
        Some("bilateral") => "restricted",
        Some("restricted") => "classified",
        _ => "public",
    }
}

fn set_spatio_temporal_context(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-temporal", "observed:current_epoch")
        .map_err(|_| "Failed to set spatio-temporal context.".to_string())?;
    let _ = container.set_attribute("data-epistemic-spatial", "geo:unspecified");
    Ok(())
}

fn set_social_context(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-social", "peer_reviewed;institutional")
        .map_err(|_| "Failed to set social context.".to_string())
}

fn register_perspective(container: &Element) -> Result<(), String> {
    let current_perspectives = container
        .get_attribute("data-perspective-dids")
        .unwrap_or_default();
    let new_entry = "did:qualia:observer";
    let updated = if current_perspectives.is_empty() {
        new_entry.to_string()
    } else if current_perspectives.contains(new_entry) {
        current_perspectives
    } else {
        format!("{current_perspectives},{new_entry}")
    };
    container
        .set_attribute("data-perspective-dids", &updated)
        .map_err(|_| "Failed to register perspective observer DID.".to_string())
}

fn add_bias(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-epistemic-bias")
        .unwrap_or_default();
    let bias_marker = "confirmation_bias_risk";
    let updated = if current.is_empty() {
        bias_marker.to_string()
    } else if current.contains(bias_marker) {
        current
    } else {
        format!("{current};{bias_marker}")
    };
    container
        .set_attribute("data-epistemic-bias", &updated)
        .map_err(|_| "Failed to note cognitive bias on claim.".to_string())
}

fn set_access_level(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-epistemic-access");
    let next = next_access_level(current.as_deref());
    container
        .set_attribute("data-epistemic-access", next)
        .map_err(|_| "Failed to update epistemic access level.".to_string())
}

fn set_horizon(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-horizon", "epoch:active")
        .map_err(|_| "Failed to set observation horizon.".to_string())
}

fn query_perspectives(document: &Document, container: &Element) -> Result<(), String> {
    let elements = document
        .query_selector_all("[data-perspective-dids]")
        .map_err(|_| "Failed to query perspective elements.".to_string())?;
    container
        .set_attribute("data-perspective-count", &elements.length().to_string())
        .map_err(|_| "Failed to record perspective count.".to_string())
}

fn add_prior(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-prior", "0.50")
        .map_err(|_| "Failed to set Bayesian prior probability.".to_string())
}

fn update_posterior(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-epistemic-posterior", "0.85")
        .map_err(|_| "Failed to update Bayesian posterior probability.".to_string())
}

fn set_threshold(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-confidence-threshold", "0.90")
        .map_err(|_| "Failed to set confidence threshold.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_levels_cycle_properly() {
        assert_eq!(next_access_level(None), "public");
        assert_eq!(next_access_level(Some("public")), "commons");
        assert_eq!(next_access_level(Some("commons")), "bilateral");
        assert_eq!(next_access_level(Some("bilateral")), "restricted");
        assert_eq!(next_access_level(Some("restricted")), "classified");
        assert_eq!(next_access_level(Some("classified")), "public");
    }
}
