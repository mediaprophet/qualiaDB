use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MusicPerformanceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musicperformance".to_string(),
            title: "Music Performance Explorer".to_string()
        }
    }
}
