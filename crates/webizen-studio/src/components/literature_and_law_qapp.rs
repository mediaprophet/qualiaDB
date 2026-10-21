use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `LiteratureAndLawQapp` component.
pub fn LiteratureAndLawQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:literatureandlaw".to_string(),
            title: "Literature And Law Explorer".to_string()
        }
    }
}
