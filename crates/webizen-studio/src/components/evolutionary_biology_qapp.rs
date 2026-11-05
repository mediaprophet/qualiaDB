use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `EvolutionaryBiologyQapp` component.
pub fn EvolutionaryBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:evolutionarybiology".to_string(),
            title: "Evolutionary Biology Explorer".to_string()
        }
    }
}
