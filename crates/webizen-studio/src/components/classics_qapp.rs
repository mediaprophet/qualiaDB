use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ClassicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:classics".to_string(),
            title: "Classics Explorer".to_string()
        }
    }
}
