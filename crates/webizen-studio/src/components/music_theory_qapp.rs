use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MusicTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musictheory".to_string(),
            title: "Music Theory Explorer".to_string()
        }
    }
}
