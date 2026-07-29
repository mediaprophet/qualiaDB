use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RadicalMediaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:radicalmediastudies".to_string(),
            title: "Radical Media Studies Explorer".to_string()
        }
    }
}
