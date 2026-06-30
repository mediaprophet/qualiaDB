use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PoststructuralismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:poststructuralism".to_string(),
            title: "Poststructuralism Explorer".to_string()
        }
    }
}
