use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn TheaterAndDramaQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:theateranddrama".to_string(),
            title: "Theater And Drama Explorer".to_string()
        }
    }
}
