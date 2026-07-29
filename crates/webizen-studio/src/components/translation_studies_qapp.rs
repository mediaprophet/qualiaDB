use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn TranslationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:translationstudies".to_string(),
            title: "Translation Studies Explorer".to_string()
        }
    }
}
