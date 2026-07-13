use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ComparativeLiteratureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:comparativeliterature".to_string(),
            title: "Comparative Literature Explorer".to_string()
        }
    }
}
