use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `PlaywritingQapp` component.
pub fn PlaywritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:playwriting".to_string(),
            title: "Playwriting Explorer".to_string()
        }
    }
}
