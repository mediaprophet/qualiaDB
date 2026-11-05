use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `ScreenPhilosophyQapp` component.
pub fn ScreenPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenphilosophy".to_string(),
            title: "Screen Philosophy Explorer".to_string()
        }
    }
}
