use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GrassrootsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:grassrootsstudies".to_string(),
            title: "Grassroots Studies Explorer".to_string()
        }
    }
}
