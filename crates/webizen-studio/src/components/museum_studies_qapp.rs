use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `MuseumStudiesQapp` component.
pub fn MuseumStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:museumstudies".to_string(),
            title: "Museum Studies Explorer".to_string()
        }
    }
}
