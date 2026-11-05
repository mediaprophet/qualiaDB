use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `RevisionistCriticalTheoryQapp` component.
pub fn RevisionistCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:revisionistcriticaltheory".to_string(),
            title: "Revisionist Critical Theory Explorer".to_string()
        }
    }
}
