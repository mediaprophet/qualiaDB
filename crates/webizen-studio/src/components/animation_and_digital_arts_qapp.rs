use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AnimationAndDigitalArtsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:animationanddigitalarts".to_string(),
            title: "Animation And Digital Arts Explorer".to_string()
        }
    }
}
