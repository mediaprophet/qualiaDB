use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SocialPsychologyQapp` component.
pub fn SocialPsychologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialpsychology".to_string(),
            title: "Social Psychology Explorer".to_string()
        }
    }
}
