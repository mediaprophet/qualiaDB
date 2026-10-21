use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `PoststructuralismQapp` component.
pub fn PoststructuralismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:poststructuralism".to_string(),
            title: "Poststructuralism Explorer".to_string()
        }
    }
}
