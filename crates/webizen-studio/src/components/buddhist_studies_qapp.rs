use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BuddhistStudiesQapp` component.
pub fn BuddhistStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:buddhiststudies".to_string(),
            title: "Buddhist Studies Explorer".to_string()
        }
    }
}
