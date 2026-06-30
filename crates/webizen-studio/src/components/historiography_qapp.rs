use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HistoriographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historiography".to_string(),
            title: "Historiography Explorer".to_string()
        }
    }
}
