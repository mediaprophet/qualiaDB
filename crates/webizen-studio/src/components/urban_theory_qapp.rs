use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `UrbanTheoryQapp` component.
pub fn UrbanTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbantheory".to_string(),
            title: "Urban Theory Explorer".to_string()
        }
    }
}
