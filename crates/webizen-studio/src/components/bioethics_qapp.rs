use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BioethicsQapp` component.
pub fn BioethicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bioethics".to_string(),
            title: "Bioethics Explorer".to_string()
        }
    }
}
