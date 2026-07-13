use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PlaywritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:playwriting".to_string(),
            title: "Playwriting Explorer".to_string()
        }
    }
}
