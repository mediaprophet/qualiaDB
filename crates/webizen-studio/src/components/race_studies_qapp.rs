use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RaceStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:racestudies".to_string(),
            title: "Race Studies Explorer".to_string()
        }
    }
}
