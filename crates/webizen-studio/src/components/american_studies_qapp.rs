use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:americanstudies".to_string(),
            title: "American Studies Explorer".to_string()
        }
    }
}
