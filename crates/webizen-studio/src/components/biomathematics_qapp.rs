use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BiomathematicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biomathematics".to_string(),
            title: "Biomathematics Explorer".to_string()
        }
    }
}
