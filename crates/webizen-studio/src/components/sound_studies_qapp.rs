use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SoundStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:soundstudies".to_string(),
            title: "Sound Studies Explorer".to_string()
        }
    }
}
