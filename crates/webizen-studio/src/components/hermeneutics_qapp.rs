use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HermeneuticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hermeneutics".to_string(),
            title: "Hermeneutics Explorer".to_string()
        }
    }
}
