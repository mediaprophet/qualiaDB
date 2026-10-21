use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `IntermediaStudiesQapp` component.
pub fn IntermediaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:intermediastudies".to_string(),
            title: "Intermedia Studies Explorer".to_string()
        }
    }
}
