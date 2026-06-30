use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CanonLawQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:canonlaw".to_string(),
            title: "Canon Law Explorer".to_string()
        }
    }
}
