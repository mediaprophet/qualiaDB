use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `CapitalStudiesQapp` component.
pub fn CapitalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:capitalstudies".to_string(),
            title: "Capital Studies Explorer".to_string()
        }
    }
}
