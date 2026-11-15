use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MetamodernismQapp` component.
pub fn MetamodernismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:metamodernism".to_string(),
            title: "Metamodernism Explorer".to_string()
        }
    }
}
