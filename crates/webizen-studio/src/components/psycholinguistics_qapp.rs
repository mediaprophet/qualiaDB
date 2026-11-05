use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PsycholinguisticsQapp` component.
pub fn PsycholinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:psycholinguistics".to_string(),
            title: "Psycholinguistics Explorer".to_string()
        }
    }
}
