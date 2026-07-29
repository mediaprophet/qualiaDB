use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PosthumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:posthumanities".to_string(),
            title: "Posthumanities Explorer".to_string()
        }
    }
}
