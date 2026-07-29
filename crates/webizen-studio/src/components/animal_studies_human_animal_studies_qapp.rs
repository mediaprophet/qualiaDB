use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AnimalStudiesHumanAnimalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:animalstudieshumananimalstudies".to_string(),
            title: "Animal Studies Human Animal Studies Explorer".to_string()
        }
    }
}
