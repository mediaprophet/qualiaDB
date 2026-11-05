use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `CanonLawQapp` component.
pub fn CanonLawQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:canonlaw".to_string(),
            title: "Canon Law Explorer".to_string()
        }
    }
}
