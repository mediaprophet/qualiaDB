use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `ArtsManagementAndAdministrationQapp` component.
pub fn ArtsManagementAndAdministrationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:artsmanagementandadministration".to_string(),
            title: "Arts Management And Administration Explorer".to_string()
        }
    }
}
