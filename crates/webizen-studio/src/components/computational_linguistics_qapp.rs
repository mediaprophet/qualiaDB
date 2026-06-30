use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ComputationalLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:computationallinguistics".to_string(),
            title: "Computational Linguistics Explorer".to_string()
        }
    }
}
