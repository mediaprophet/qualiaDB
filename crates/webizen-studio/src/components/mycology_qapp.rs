use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MycologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mycology".to_string(),
            title: "Mycology Explorer".to_string()
        }
    }
}
