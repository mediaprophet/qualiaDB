use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ScreenPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenphilosophy".to_string(),
            title: "Screen Philosophy Explorer".to_string()
        }
    }
}
