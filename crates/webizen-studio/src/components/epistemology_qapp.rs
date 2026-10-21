use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `EpistemologyQapp` component.
pub fn EpistemologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:epistemology".to_string(),
            title: "Epistemology Explorer".to_string()
        }
    }
}
