use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn TheaterAndDramaQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:theateranddrama".to_string(),
            title: "Theater And Drama Explorer".to_string()
        }
    }
}
