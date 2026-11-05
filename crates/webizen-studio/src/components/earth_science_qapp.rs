use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `EarthScienceQapp` component.
pub fn EarthScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:earthscience".to_string(),
            title: "Earth Science Explorer".to_string()
        }
    }
}
