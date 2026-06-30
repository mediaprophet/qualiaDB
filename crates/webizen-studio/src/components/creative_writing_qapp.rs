use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CreativeWritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:creativewriting".to_string(),
            title: "Creative Writing Explorer".to_string()
        }
    }
}
