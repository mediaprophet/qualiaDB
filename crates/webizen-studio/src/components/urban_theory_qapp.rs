use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn UrbanTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbantheory".to_string(),
            title: "Urban Theory Explorer".to_string()
        }
    }
}
