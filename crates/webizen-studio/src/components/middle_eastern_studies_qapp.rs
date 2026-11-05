use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MiddleEasternStudiesQapp` component.
pub fn MiddleEasternStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:middleeasternstudies".to_string(),
            title: "Middle Eastern Studies Explorer".to_string()
        }
    }
}
