//! Logical instrument manifest (cold metadata). Not a dataset volume.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::category::{seed_notice_is_labelled, ContentCategory};
use super::errors::InstrumentError;

/// Collectable format version for this SI-03 logical model. Not a wire freeze.
pub const COLLECTABLE_FORMAT_VERSION: &str = "0.1.0-demo";

/// In-memory manifest. Extra keys are preserved (non-critical unknown fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstrumentManifest {
    pub format_version: String,
    pub instrument_id: String,
    pub release_id: String,
    pub version: String,
    pub name: String,
    pub purpose: String,
    pub content_category: ContentCategory,
    pub prohibited_interpretation: String,
    pub domain: String,
    pub ontology_reference: String,
    pub entry_point: String,
    pub citation: String,
    pub honesty_notice: String,
    pub incomplete_input: String,
    pub result_kind: String,
    pub licence: String,
    pub authored_by: String,
    pub accessible_text: String,
    pub visual_media_type: String,
    /// Digest of the HMC bytes excluding this field's own value is computed
    /// after members are packed; stored here once known.
    #[serde(default)]
    pub content_digest: String,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl InstrumentManifest {
    pub fn validate(&self) -> Result<(), InstrumentError> {
        let required = [
            ("format_version", self.format_version.as_str()),
            ("instrument_id", self.instrument_id.as_str()),
            ("release_id", self.release_id.as_str()),
            ("version", self.version.as_str()),
            ("name", self.name.as_str()),
            ("purpose", self.purpose.as_str()),
            ("prohibited_interpretation", self.prohibited_interpretation.as_str()),
            ("domain", self.domain.as_str()),
            ("ontology_reference", self.ontology_reference.as_str()),
            ("entry_point", self.entry_point.as_str()),
            ("citation", self.citation.as_str()),
            ("honesty_notice", self.honesty_notice.as_str()),
            ("incomplete_input", self.incomplete_input.as_str()),
            ("result_kind", self.result_kind.as_str()),
            ("licence", self.licence.as_str()),
            ("authored_by", self.authored_by.as_str()),
            ("accessible_text", self.accessible_text.as_str()),
            ("visual_media_type", self.visual_media_type.as_str()),
        ];
        for (name, value) in required {
            if value.trim().is_empty() {
                return Err(InstrumentError::MissingField(name));
            }
        }
        if self.instrument_id == self.release_id {
            return Err(InstrumentError::Canonical(
                "instrument_id and release_id must differ".into(),
            ));
        }
        if self.content_category.requires_seed_label()
            && !seed_notice_is_labelled(&self.honesty_notice)
        {
            return Err(InstrumentError::DemoUnlabelled);
        }
        Ok(())
    }
}
