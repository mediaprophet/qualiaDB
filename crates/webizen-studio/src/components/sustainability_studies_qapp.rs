use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SustainabilityStudiesQapp` component.
pub fn SustainabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sustainabilitystudies".to_string(),
            title: "Sustainability Studies Explorer".to_string()
        }
    }
}
