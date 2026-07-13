use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EnvironmentalChemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentalchemistry".to_string(),
            title: "Environmental Chemistry Explorer".to_string()
        }
    }
}
