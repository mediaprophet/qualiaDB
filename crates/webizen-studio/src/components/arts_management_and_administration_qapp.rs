use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ArtsManagementAndAdministrationQapp` component.
pub fn ArtsManagementAndAdministrationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:artsmanagementandadministration".to_string(),
            title: "Arts Management And Administration Explorer".to_string()
        }
    }
}
