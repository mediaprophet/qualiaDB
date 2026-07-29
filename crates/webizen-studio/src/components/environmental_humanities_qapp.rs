use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EnvironmentalHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentalhumanities".to_string(),
            title: "Environmental Humanities Explorer".to_string()
        }
    }
}
