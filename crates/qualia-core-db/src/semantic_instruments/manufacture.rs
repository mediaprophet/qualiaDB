//! Poet-facing manufacture checks (SI-09). Invalid or unlabelled packs
//! cannot publish. Source, graph and executable remain distinct layers.

use super::category::ContentCategory;
use super::errors::InstrumentError;
use super::package::open_collectable;

/// Authoring layers. Chrome must not collapse these into one blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoringLayer {
    Source,
    Graph,
    Executable,
}

/// Result of a pre-publish inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishInspection {
    pub release_id: String,
    pub category: ContentCategory,
    pub labelled: bool,
    pub has_source_n3: bool,
    pub has_graph_q42: bool,
    pub has_visual_10d: bool,
    pub round_trip_loss: bool,
}

/// WASM / lite hosts cannot mmap native Q42 v3 volumes. Inspect N3/HCF; hold the volume.
pub const WASM_Q42_HELD: &str =
    "held / not yet — native Q42 v3 volumes are not loadable on this WASM profile; N3/HCF remain inspectable";

pub fn inspect_for_publish(bytes: &[u8]) -> Result<PublishInspection, InstrumentError> {
    let opened = open_collectable(bytes)?;
    let labelled = opened.manifest.content_category.requires_seed_label();
    let has_n3 = !opened.definition_n3.is_empty();
    let has_q42 = opened.small_q42.as_ref().is_some_and(|b| !b.is_empty());
    Ok(PublishInspection {
        release_id: opened.manifest.release_id,
        category: opened.manifest.content_category,
        labelled,
        has_source_n3: has_n3,
        has_graph_q42: has_q42,
        has_visual_10d: !opened.visual_10d.is_empty(),
        // N3 present without compiled q42 is a declared loss, not a silent drop.
        round_trip_loss: has_n3 && !has_q42,
    })
}

pub fn assert_can_publish(inspection: &PublishInspection) -> Result<(), InstrumentError> {
    if !inspection.has_source_n3 {
        return Err(InstrumentError::MissingField("graphs/instrument-definition.n3"));
    }
    if inspection.category.requires_seed_label() && !inspection.labelled {
        return Err(InstrumentError::DemoUnlabelled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::demo::build_demo;

    #[test]
    fn demo_pack_inspects_and_may_publish() {
        let bytes = build_demo("unit-convert").unwrap();
        let insp = inspect_for_publish(&bytes).unwrap();
        assert_eq!(insp.category, ContentCategory::Demo);
        assert!(insp.labelled);
        assert!(insp.has_source_n3);
        assert!(insp.has_visual_10d);
        assert_can_publish(&insp).unwrap();
    }

    #[test]
    fn empty_bytes_cannot_publish() {
        assert!(inspect_for_publish(&[]).is_err());
    }
}
