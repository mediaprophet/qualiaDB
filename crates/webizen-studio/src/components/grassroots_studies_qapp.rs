use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GrassrootsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:grassrootsstudies".to_string(),
            title: "Grassroots Studies Explorer".to_string()
        }
    }
}
