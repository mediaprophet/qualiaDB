use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ComputationalLinguisticsQapp` component.
pub fn ComputationalLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:computationallinguistics".to_string(),
            title: "Computational Linguistics Explorer".to_string()
        }
    }
}
