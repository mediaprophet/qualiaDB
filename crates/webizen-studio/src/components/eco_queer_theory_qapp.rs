use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EcoQueerTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecoqueertheory".to_string(),
            title: "Eco Queer Theory Explorer".to_string()
        }
    }
}
