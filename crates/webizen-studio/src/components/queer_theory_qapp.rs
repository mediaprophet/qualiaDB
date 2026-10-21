use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `QueerTheoryQapp` component.
pub fn QueerTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:queertheory".to_string(),
            title: "Queer Theory Explorer".to_string()
        }
    }
}
