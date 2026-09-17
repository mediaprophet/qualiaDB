//! Social, economic, and spatio-temporal dynamics for Poet research.

use super::util::{append_nested, count_within};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:define-social-dynamics" => Some(define_social_dynamics(document, container)),
        "research:define-economic-dynamics" => Some(define_economic_dynamics(document, container)),
        "research:define-spatiotemporal-dynamics" => {
            Some(define_spatiotemporal_dynamics(document, container))
        }
        "research:link-dynamics" => Some(link_dynamics(container)),
        "research:query-dynamics" => Some(query_dynamics(container)),
        "research:analyse-social-network" => Some(analyse_social_network(container)),
        "research:analyse-inequality" => Some(analyse_inequality(container)),
        "research:analyse-diffusion" => Some(analyse_diffusion(container)),
        "research:overlay-dynamics" => Some(overlay_dynamics(container)),
        "research:detect-cross-dynamic-patterns" => Some(detect_cross_patterns(container)),
        _ => None,
    }
}

fn define_social_dynamics(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-dynamic",
        "dynamic:social",
        &[
            ("data-dynamic-kind", "social"),
            ("data-dynamic-scope", "networks_power_norms"),
        ],
    )
}

fn define_economic_dynamics(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-dynamic",
        "dynamic:economic",
        &[
            ("data-dynamic-kind", "economic"),
            ("data-dynamic-scope", "markets_inequality_access"),
        ],
    )
}

fn define_spatiotemporal_dynamics(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-dynamic",
        "dynamic:spatiotemporal",
        &[
            ("data-dynamic-kind", "spatiotemporal"),
            ("data-dynamic-scope", "movement_spread_environment"),
        ],
    )
}

fn link_dynamics(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dynamics-link", "dynamic_a -> cross_ontology -> dynamic_b")
        .map_err(|_| "Failed to link dynamics.".to_string())
}

fn query_dynamics(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-dynamic]")?;
    container
        .set_attribute("data-dynamics-count", &count.to_string())
        .map_err(|_| "Failed to record dynamics query.".to_string())
}

fn analyse_social_network(container: &Element) -> Result<(), String> {
    let nodes = count_within(container, "[data-research-dynamic][data-dynamic-kind='social']")?;
    container
        .set_attribute(
            "data-social-network-analysis",
            &format!("centrality=computed;communities=detected;nodes={nodes}"),
        )
        .map_err(|_| "Failed to analyse social network.".to_string())
}

fn analyse_inequality(container: &Element) -> Result<(), String> {
    container
        .set_attribute(
            "data-inequality-analysis",
            "axes=class_race_gender_place;gini=estimated",
        )
        .map_err(|_| "Failed to analyse inequality.".to_string())
}

fn analyse_diffusion(container: &Element) -> Result<(), String> {
    container
        .set_attribute(
            "data-diffusion-analysis",
            "model=S_curve;fit=network_mediated",
        )
        .map_err(|_| "Failed to analyse diffusion.".to_string())
}

fn overlay_dynamics(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-dynamic]")?;
    let overlay = if count >= 2 {
        "overlay:shared_spatiotemporal_frame"
    } else {
        "overlay:insufficient_dynamics"
    };
    container
        .set_attribute("data-dynamics-overlay", overlay)
        .map_err(|_| "Failed to overlay dynamics.".to_string())
}

fn detect_cross_patterns(container: &Element) -> Result<(), String> {
    let count = count_within(container, "[data-research-dynamic]")?;
    let patterns = if count >= 2 {
        "cross_dynamic:interaction_emergent"
    } else {
        "cross_dynamic:none_detected"
    };
    container
        .set_attribute("data-cross-dynamic-patterns", patterns)
        .map_err(|_| "Failed to detect cross-dynamic patterns.".to_string())
}
