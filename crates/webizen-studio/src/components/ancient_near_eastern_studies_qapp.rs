use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AncientNearEasternStudiesQapp` component.
pub fn AncientNearEasternStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ancientneareasternstudies".to_string(),
            title: "Ancient Near Eastern Studies Explorer".to_string()
        }
    }
}
