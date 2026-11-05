use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `ClassicsQapp` component.
pub fn ClassicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:classics".to_string(),
            title: "Classics Explorer".to_string()
        }
    }
}
