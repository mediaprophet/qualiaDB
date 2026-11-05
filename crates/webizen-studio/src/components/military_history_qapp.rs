use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MilitaryHistoryQapp` component.
pub fn MilitaryHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:militaryhistory".to_string(),
            title: "Military History Explorer".to_string()
        }
    }
}
