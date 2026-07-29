use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ComputationalLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:computationallinguistics".to_string(),
            title: "Computational Linguistics Explorer".to_string()
        }
    }
}
