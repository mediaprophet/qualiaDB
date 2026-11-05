use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CriminologyAndCriminalJusticeQapp` component.
pub fn CriminologyAndCriminalJusticeQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criminologyandcriminaljustice".to_string(),
            title: "Criminology And Criminal Justice Explorer".to_string()
        }
    }
}
