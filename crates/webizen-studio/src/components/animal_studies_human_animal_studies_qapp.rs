use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AnimalStudiesHumanAnimalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:animalstudieshumananimalstudies".to_string(),
            title: "Animal Studies Human Animal Studies Explorer".to_string()
        }
    }
}
