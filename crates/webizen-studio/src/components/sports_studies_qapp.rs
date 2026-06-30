use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SportsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sportsstudies".to_string(),
            title: "Sports Studies Explorer".to_string()
        }
    }
}
