//! Shared metadata schema for a `.qualia` **anatomy asset pack** — the per-organ
//! `meta` carried in each bundle entry.
//!
//! A packed anatomy body is a `.qualia` bundle (see [`crate::bundle`]) whose
//! entries are the per-organ sealed `.10d` meshes. Each entry's opaque `meta`
//! holds one CBOR-encoded [`AnatomyOrganMeta`]: which body **system** the organ
//! belongs to, an **approximate** anatomical position for assembling the whole
//! body, and a **neutral default colour** (the pack ships no personal data — a
//! person's real burden colouring, from `AnatomyViewReport::paint_organs`,
//! overrides this at runtime when their records are loaded).
//!
//! This lives in `qualia-core-db` so the producer (`qualia-client-core`, which
//! discovers/compiles organs) and the consumer (the browser `QualiaPortal`
//! renderer + the native read-through loader) share **one** typed schema.

use serde::{Deserialize, Serialize};

/// Per-organ render metadata carried in a `.qualia` anatomy-pack entry's `meta`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnatomyOrganMeta {
    /// The organ's **primary** body-system id (its default colour/placement),
    /// e.g. `"digestive"`. An organ is a building block shared across systems;
    /// see [`AnatomyOrganMeta::systems`] for the full set.
    pub system: String,
    /// Human-readable name of this part (e.g. `"diaphragm"`, `"liver"`), for a selectable parts list.
    /// Empty/absent in older packs → the consumer falls back to the entry key (`#[serde(default)]`).
    #[serde(default)]
    pub label: String,
    /// **All** body systems the organ participates in, primary first (the pancreas
    /// is `["digestive", "endocrine", "exocrine"]`). Lets the renderer colour by the
    /// primary system *or* blend across memberships, and lets a person's condition on
    /// any member system light the organ. Empty/absent in older packs → treat as
    /// `[system]` (back-compatible; `#[serde(default)]`).
    #[serde(default)]
    pub systems: Vec<String>,
    /// Approximate anatomical position offset in normalised body space
    /// `[x, y, z]` (0..1; x=right, y=up, z=front). Approximate placement — a
    /// future pass can substitute real CCF transforms.
    pub position: [f32; 3],
    /// Neutral default linear RGBA for the organ (overridden by the person's
    /// σ-derived burden colour at runtime when their data is present).
    pub rgba: [f32; 4],
}

impl AnatomyOrganMeta {
    /// Encode to CBOR for storage in a bundle entry's `meta`.
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // Infallible for this small POD struct into a Vec writer.
        ciborium::into_writer(self, &mut out).expect("cbor encode AnatomyOrganMeta");
        out
    }

    /// Decode from a bundle entry's `meta` bytes.
    pub fn from_cbor(bytes: &[u8]) -> Option<Self> {
        ciborium::from_reader(bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbor_round_trips() {
        let m = AnatomyOrganMeta {
            system: "digestive".to_string(),
            label: "pancreas".to_string(),
            systems: vec!["digestive".to_string(), "endocrine".to_string(), "exocrine".to_string()],
            position: [0.5, 0.6, 0.55],
            rgba: [0.8, 0.3, 0.3, 1.0],
        };
        let bytes = m.to_cbor();
        let back = AnatomyOrganMeta::from_cbor(&bytes).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.systems.len(), 3, "all memberships round-trip");
        assert!(AnatomyOrganMeta::from_cbor(b"not cbor").is_none());
    }

    /// A pack written before `systems` existed must still decode (the field defaults to empty), so an
    /// older `.qualia` on disk keeps working. The consumer treats an empty `systems` as `[system]`.
    #[test]
    fn old_meta_without_systems_field_still_decodes() {
        #[derive(serde::Serialize)]
        struct OldMeta {
            system: String,
            position: [f32; 3],
            rgba: [f32; 4],
        }
        let mut bytes = Vec::new();
        ciborium::into_writer(
            &OldMeta { system: "respiratory".to_string(), position: [0.4, 0.6, 0.5], rgba: [0.5, 0.7, 0.9, 1.0] },
            &mut bytes,
        )
        .unwrap();
        let m = AnatomyOrganMeta::from_cbor(&bytes).expect("old meta decodes");
        assert_eq!(m.system, "respiratory");
        assert!(m.systems.is_empty(), "absent field defaults to empty");
    }
}
