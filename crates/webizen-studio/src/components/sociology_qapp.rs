use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `SociologyQapp` component.
pub fn SociologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sociology".to_string(),
            title: "Sociology Explorer".to_string()
        }
    }
}
