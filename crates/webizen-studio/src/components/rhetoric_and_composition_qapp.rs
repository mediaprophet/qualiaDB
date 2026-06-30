use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RhetoricAndCompositionQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:rhetoricandcomposition".to_string(),
            title: "Rhetoric And Composition Explorer".to_string()
        }
    }
}
