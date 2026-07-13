use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn InternationalRelationsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:internationalrelations".to_string(),
            title: "International Relations Explorer".to_string()
        }
    }
}
