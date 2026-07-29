use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HistoriographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historiography".to_string(),
            title: "Historiography Explorer".to_string()
        }
    }
}
