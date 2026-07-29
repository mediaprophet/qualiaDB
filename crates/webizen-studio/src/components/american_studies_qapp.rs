use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:americanstudies".to_string(),
            title: "American Studies Explorer".to_string()
        }
    }
}
