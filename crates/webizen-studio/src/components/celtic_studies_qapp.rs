use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `CelticStudiesQapp` component.
pub fn CelticStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:celticstudies".to_string(),
            title: "Celtic Studies Explorer".to_string()
        }
    }
}
