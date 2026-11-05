use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `DisabilityStudiesQapp` component.
pub fn DisabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:disabilitystudies".to_string(),
            title: "Disability Studies Explorer".to_string()
        }
    }
}
