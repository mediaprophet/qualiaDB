use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn UrbanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanstudies".to_string(),
            title: "Urban Studies Explorer".to_string()
        }
    }
}
