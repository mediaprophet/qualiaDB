use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AffectTheoryQapp` component.
pub fn AffectTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:affecttheory".to_string(),
            title: "Affect Theory Explorer".to_string()
        }
    }
}
