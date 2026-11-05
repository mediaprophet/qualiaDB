use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `VitalMaterialismQapp` component.
pub fn VitalMaterialismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:vitalmaterialism".to_string(),
            title: "Vital Materialism Explorer".to_string()
        }
    }
}
