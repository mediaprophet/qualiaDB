use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BotanyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:botany".to_string(),
            title: "Botany Explorer".to_string()
        }
    }
}
