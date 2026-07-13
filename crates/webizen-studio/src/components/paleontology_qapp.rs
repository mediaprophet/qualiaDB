use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PaleontologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:paleontology".to_string(),
            title: "Paleontology Explorer".to_string()
        }
    }
}
