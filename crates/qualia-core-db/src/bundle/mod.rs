//! `.hmc` — the **hyper-media-container**: a **transparent container-of-files**
//! bundle for shipping a set of sealed Qualia assets (`.emf` spectral media,
//! `.10d` meshes, `.q42` graph volumes, `.p64` weights, …)
//! as one attestable unit (a release artefact, a downloadable pack, a signed
//! asset set).
//!
//! # Why a bundle, and why *transparent*
//! The Qualia container family (`.10d` / `.q42` / `.p64`) are all internally
//! navigable — you can pull a *segment* (a mesh section, a subgraph, a weight
//! shard) without reading the whole file, ideally `mmap` zero-copy. A bundle
//! must not destroy that. So a `.hmc` bundle only **concatenates intact files,
//! page-aligned, with an index of absolute offsets** — it never compresses,
//! re-chunks, or reframes an entry. Consequences:
//!
//! - `reader.get(key)` returns a zero-copy slice that **is** a byte-identical
//!   standalone file — feed it straight to the existing `.q42`/`.10d`/`.p64`
//!   reader; its interior offsets resolve unchanged.
//! - `reader.segment(key, off, len)` reaches an interior segment of an entry
//!   directly — the bundle does not interfere with segment-level access.
//! - HTTP range-fetching one interior segment works: `entry.offset + seg.off`.
//!
//! It lives in `qualia-core-db` so it is **one reader for both channels** —
//! native (`BundleMmap`, zero-copy) and WASM (parse fetched/ranged bytes).
//!
//! See [`format`] for the on-disk layout.

pub mod cbor_ld_manifest;
pub mod entity_compiler;
pub mod entity_verifier;
mod format;
mod reader;
pub use entity_compiler as twin_compiler;
pub use entity_verifier as twin_verifier;
pub mod variant_matrix;
mod writer;

pub use entity_compiler::{
    compile_digital_twin, compile_entity, EntityCompileInput, EntityCompileOutput,
    TwinCompileInput, TwinCompileOutput,
};

pub use entity_verifier::{
    verify_digital_twin_10d, verify_digital_twin_bundle, verify_entity_10d, verify_entity_bundle,
    AffordanceVerificationSummary, CommonsObligationSummary, DigitalTwin10dVerification,
    DigitalTwinBundleVerification, Entity10dVerification, EntityBundleVerification,
    ImpostorVerificationSummary, SurfelVerificationSummary,
};

pub use cbor_ld_manifest::{
    AcousticAnchors, CborLdManifest, CommonsObligation, CredentialProof, CredentialSubject,
    GeometryLayout, SectionLayoutEntry, SubjectAffordances, CBOR_LD_CONTEXT_CREDENTIALS_V1,
    CBOR_LD_CONTEXT_WEBCIVICS_V1, CBOR_LD_MANIFEST_ENTRY_KEY, CBOR_LD_MANIFEST_KIND,
};
pub use format::{
    BundleEntry, BundleError, BUNDLE_ENTRY_ALIGN, BUNDLE_HEADER_SIZE, BUNDLE_MAGIC, BUNDLE_VERSION,
    SHA256_LEN,
};
pub use reader::BundleReader;
pub use variant_matrix::{
    BiologicalPhenotype, EntityFamilyManifest, LodTier, ModelVariantDescriptor, StructuralState,
    TwinFamilyManifest, FAMILY_MANIFEST_ENTRY_KEY, FAMILY_MANIFEST_KIND,
};
pub use writer::BundleWriter;

#[cfg(not(target_arch = "wasm32"))]
pub use reader::BundleMmap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_ontologies::LifecycleStage;

    #[test]
    fn heterogeneous_pack_of_three_kinds_roundtrips() {
        // A bundle can carry different sealed formats side by side.
        let ten_d: Vec<u8> = (0..300u32).map(|i| (i * 7) as u8).collect();
        let q42 = b"q42 graph volume bytes".to_vec();
        let p64: Vec<u8> = vec![0x40; 129];

        let mut w = BundleWriter::new();
        w.add_file("body.10d", "10d", ten_d.clone(), None).unwrap();
        w.add_file("wordnet.q42", "q42", q42.clone(), None).unwrap();
        w.add_file("model.p64", "p64", p64.clone(), None).unwrap();
        let bytes = w.build().unwrap();

        let r = BundleReader::parse(&bytes).unwrap();
        assert_eq!(r.entries().len(), 3);
        assert_eq!(r.get("body.10d").unwrap(), ten_d.as_slice());
        assert_eq!(r.get("wordnet.q42").unwrap(), q42.as_slice());
        assert_eq!(r.get("model.p64").unwrap(), p64.as_slice());
        assert_eq!(r.entry("model.p64").unwrap().kind, "p64");
        // Every entry is page-aligned so its interior alignment is preserved.
        for e in r.entries() {
            assert_eq!(e.offset % BUNDLE_ENTRY_ALIGN as u64, 0);
        }
        // Every entry verifies against its recorded hash.
        assert!(r.entries().iter().all(|e| r.verify_entry(&e.key)));
    }

    #[test]
    fn empty_bundle_is_valid() {
        let bytes = BundleWriter::new().build().unwrap();
        let r = BundleReader::parse(&bytes).unwrap();
        assert!(r.entries().is_empty());
    }

    #[test]
    fn tampered_bundle_fails_verification() {
        let mut w = BundleWriter::new();
        w.add_file("asset.10d", "10d", b"valid asset bytes".to_vec(), None)
            .unwrap();
        let mut bytes = w.build().unwrap();

        // Tamper with one byte in the payload
        let r = BundleReader::parse(&bytes).unwrap();
        let offset = r.entry("asset.10d").unwrap().offset as usize;
        bytes[offset] ^= 0xFF;

        // Bundle parse must reject with CRC mismatch
        let res = BundleReader::parse(&bytes);
        assert!(matches!(res, Err(BundleError::CrcMismatch { .. })));
    }

    #[test]
    fn family_manifest_bundle_roundtrip() {
        let mut manifest = TwinFamilyManifest::new(
            "urn:taxon:water_pump_v3",
            "Water Pump V3",
            "machinery",
            "operational_lod0",
        );
        manifest.add_variant(ModelVariantDescriptor {
            variant_id: "operational_lod0".to_string(),
            phenotype: BiologicalPhenotype::Unspecified,
            stage: LifecycleStage::Operational,
            lod: LodTier::Lod0,
            state: StructuralState::Pristine,
            asset_key: "pump_hero.10d".to_string(),
            entity_ontology_key: Some("pump_ontology.q42".to_string()),
            procedural_seed: Some(101),
        });

        let mut w = BundleWriter::new();
        w.add_family_manifest(&manifest).unwrap();
        w.add_file("pump_hero.10d", "10d", vec![0x10; 64], None)
            .unwrap();
        w.add_file("pump_ontology.q42", "q42", vec![0x42; 64], None)
            .unwrap();
        let bytes = w.build().unwrap();

        let r = BundleReader::parse(&bytes).unwrap();
        let decoded_manifest = r.get_family_manifest().unwrap().expect("manifest exists");
        assert_eq!(decoded_manifest.family_id, "urn:taxon:water_pump_v3");
        let resolved = decoded_manifest
            .resolve_variant(
                BiologicalPhenotype::Unspecified,
                LifecycleStage::Operational,
                LodTier::Lod0,
                StructuralState::Pristine,
            )
            .unwrap();
        assert_eq!(resolved.variant_id, "operational_lod0");
        assert_eq!(resolved.asset_key, "pump_hero.10d");
    }
}
