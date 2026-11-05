use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MaterialCultureStudiesQapp` component.
pub fn MaterialCultureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:materialculturestudies".to_string(),
            title: "Material Culture Studies Explorer".to_string()
        }
    }
}
