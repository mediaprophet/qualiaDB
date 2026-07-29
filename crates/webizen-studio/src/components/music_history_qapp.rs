use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MusicHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musichistory".to_string(),
            title: "Music History Explorer".to_string()
        }
    }
}
