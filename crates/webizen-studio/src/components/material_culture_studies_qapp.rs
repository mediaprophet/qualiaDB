use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `MaterialCultureStudiesQapp` component.
pub fn MaterialCultureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:materialculturestudies".to_string(),
            title: "Material Culture Studies Explorer".to_string()
        }
    }
}
