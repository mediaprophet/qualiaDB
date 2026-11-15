use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `LeadershipStudiesQapp` component.
pub fn LeadershipStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:leadershipstudies".to_string(),
            title: "Leadership Studies Explorer".to_string()
        }
    }
}
