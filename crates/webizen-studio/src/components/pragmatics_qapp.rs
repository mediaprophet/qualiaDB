use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PragmaticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:pragmatics".to_string(),
            title: "Pragmatics Explorer".to_string()
        }
    }
}
