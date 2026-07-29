use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn VitalMaterialismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:vitalmaterialism".to_string(),
            title: "Vital Materialism Explorer".to_string()
        }
    }
}
