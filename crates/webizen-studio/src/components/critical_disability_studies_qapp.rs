use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CriticalDisabilityStudiesQapp` component.
pub fn CriticalDisabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticaldisabilitystudies".to_string(),
            title: "Critical Disability Studies Explorer".to_string()
        }
    }
}
