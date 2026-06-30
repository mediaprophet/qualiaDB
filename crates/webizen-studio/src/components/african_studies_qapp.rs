use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AfricanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:africanstudies".to_string(),
            title: "African Studies Explorer".to_string()
        }
    }
}
