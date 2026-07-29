use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn LatinAmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:latinamericanstudies".to_string(),
            title: "Latin American Studies Explorer".to_string()
        }
    }
}
