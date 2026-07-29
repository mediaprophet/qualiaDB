use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BehavioralEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:behavioralecology".to_string(),
            title: "Behavioral Ecology Explorer".to_string()
        }
    }
}
