use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MaterialistAestheticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:materialistaesthetics".to_string(),
            title: "Materialist Aesthetics Explorer".to_string()
        }
    }
}
