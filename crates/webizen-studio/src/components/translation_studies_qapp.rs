use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn TranslationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:translationstudies".to_string(),
            title: "Translation Studies Explorer".to_string()
        }
    }
}
