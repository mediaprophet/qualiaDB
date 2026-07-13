use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LiteratureAndLawQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:literatureandlaw".to_string(),
            title: "Literature And Law Explorer".to_string()
        }
    }
}
