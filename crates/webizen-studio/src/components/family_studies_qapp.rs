use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `FamilyStudiesQapp` component.
pub fn FamilyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:familystudies".to_string(),
            title: "Family Studies Explorer".to_string()
        }
    }
}
