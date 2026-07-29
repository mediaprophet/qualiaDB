use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CanonLawQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:canonlaw".to_string(),
            title: "Canon Law Explorer".to_string()
        }
    }
}
