use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn VitalMaterialismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:vitalmaterialism".to_string(),
            title: "Vital Materialism Explorer".to_string()
        }
    }
}
