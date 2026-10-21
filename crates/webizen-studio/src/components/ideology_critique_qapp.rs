use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `IdeologyCritiqueQapp` component.
pub fn IdeologyCritiqueQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ideologycritique".to_string(),
            title: "Ideology Critique Explorer".to_string()
        }
    }
}
