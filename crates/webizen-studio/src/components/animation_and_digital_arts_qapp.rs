use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AnimationAndDigitalArtsQapp` component.
pub fn AnimationAndDigitalArtsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:animationanddigitalarts".to_string(),
            title: "Animation And Digital Arts Explorer".to_string()
        }
    }
}
