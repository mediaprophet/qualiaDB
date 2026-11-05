use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `MaritimeHistoryQapp` component.
pub fn MaritimeHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:maritimehistory".to_string(),
            title: "Maritime History Explorer".to_string()
        }
    }
}
