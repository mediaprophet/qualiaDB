use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ScreenwritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenwriting".to_string(),
            title: "Screenwriting Explorer".to_string()
        }
    }
}
