use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EcoQueerTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecoqueertheory".to_string(),
            title: "Eco Queer Theory Explorer".to_string()
        }
    }
}
