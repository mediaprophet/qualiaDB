use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BiogeochemistryQapp` component.
pub fn BiogeochemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biogeochemistry".to_string(),
            title: "Biogeochemistry Explorer".to_string()
        }
    }
}
