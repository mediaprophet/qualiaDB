use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `EnvironmentalScienceQapp` component.
pub fn EnvironmentalScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentalscience".to_string(),
            title: "Environmental Science Explorer".to_string()
        }
    }
}
