use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AfricanAmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:africanamericanstudies".to_string(),
            title: "African American Studies Explorer".to_string()
        }
    }
}
