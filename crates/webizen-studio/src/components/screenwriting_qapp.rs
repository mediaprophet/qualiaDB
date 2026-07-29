use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ScreenwritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenwriting".to_string(),
            title: "Screenwriting Explorer".to_string()
        }
    }
}
