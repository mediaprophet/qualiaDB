use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ScreenPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenphilosophy".to_string(),
            title: "Screen Philosophy Explorer".to_string()
        }
    }
}
