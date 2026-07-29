use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AfricanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:africanstudies".to_string(),
            title: "African Studies Explorer".to_string()
        }
    }
}
