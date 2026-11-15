use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BiophysicsQapp` component.
pub fn BiophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biophysics".to_string(),
            title: "Biophysics Explorer".to_string()
        }
    }
}
