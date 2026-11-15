use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SpatialDataScienceQapp` component.
pub fn SpatialDataScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:spatialdatascience".to_string(),
            title: "Spatial Data Science Explorer".to_string()
        }
    }
}
