use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RadicalMediaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:radicalmediastudies".to_string(),
            title: "Radical Media Studies Explorer".to_string()
        }
    }
}
