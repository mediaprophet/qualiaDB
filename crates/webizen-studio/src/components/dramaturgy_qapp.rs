use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `DramaturgyQapp` component.
pub fn DramaturgyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:dramaturgy".to_string(),
            title: "Dramaturgy Explorer".to_string()
        }
    }
}
