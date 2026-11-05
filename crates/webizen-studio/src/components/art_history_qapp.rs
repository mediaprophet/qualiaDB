use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ArtHistoryQapp` component.
pub fn ArtHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:arthistory".to_string(),
            title: "Art History Explorer".to_string()
        }
    }
}
