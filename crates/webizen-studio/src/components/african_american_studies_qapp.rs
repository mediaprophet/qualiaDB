use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AfricanAmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:africanamericanstudies".to_string(),
            title: "African American Studies Explorer".to_string()
        }
    }
}
