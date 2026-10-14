//! Multi-Model Variant Matrix & Digital Twin Family Manifest for `.hmc` Bundles.
//!
//! Organizes model families (species with sexual dimorphism, ontogenetic life stages,
//! functional trims, damage states, and LOD hierarchies) into a single cohesive,
//! attestable `.hmc` bundle.
//!
//! Provides deterministic query resolution ($O(1)$) with principled fallback:
//! `(phenotype, stage, lod, state) -> &ModelVariantDescriptor`.

use serde::{Deserialize, Serialize};

use super::format::BundleError;
use crate::entity_view::entity_ontologies::LifecycleStage;

/// Biological sexual dimorphism / reproductive phenotype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BiologicalPhenotype {
    /// Distinct male morphology.
    Male,
    /// Distinct female morphology.
    Female,
    /// Sexually monomorphic taxon (males and females share indistinguishable morphology).
    #[default]
    Monomorphic,
    /// Non-reproductive worker / sterile caste.
    Neuter,
    /// Simultaneous or sequential hermaphroditic morphology.
    Hermaphroditic,
    /// Not applicable (constructed inanimate object, vehicle, building) or unrecorded.
    Unspecified,
}

/// Level of Detail (LOD) tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LodTier {
    /// Hero presentation asset (highest vertex density, full skeletal rigs).
    #[default]
    Lod0 = 0,
    /// High-detail close-range asset.
    Lod1 = 1,
    /// Medium-detail mid-range asset.
    Lod2 = 2,
    /// Low-detail distance asset.
    Lod3 = 3,
    /// Billboard / octahedral impostor representation.
    Impostor = 4,
}

/// Physical wear, weathering, or structural damage condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StructuralState {
    /// Intact, pristine condition.
    #[default]
    Pristine,
    /// Surface weathering, lichen, oxidation, patina, or wear.
    Weathered,
    /// Functional or localized structural damage (cracked, bent, punctured).
    Damaged,
    /// Terminal mechanical or physical fracture (split into sub-components).
    Fractured,
}

/// Descriptor of a single model variant stored within the `.hmc` bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVariantDescriptor {
    /// Unique identifier within the family (e.g. `"male_adult_lod0"`).
    pub variant_id: String,
    /// Biological sexual dimorphism form.
    pub phenotype: BiologicalPhenotype,
    /// Ontogenetic or mechanical lifecycle stage.
    pub stage: LifecycleStage,
    /// Level of detail tier.
    pub lod: LodTier,
    /// Structural wear or damage condition.
    pub state: StructuralState,
    /// Key of the embedded 3D geometry file in the bundle (e.g. `"models/male_adult.10d"`).
    pub asset_key: String,
    /// Optional key of the associated entity ontology volume (e.g. `"ontology/male_adult.q42"`).
    #[serde(default, alias = "personal_ontology_key", skip_serializing_if = "Option::is_none")]
    pub entity_ontology_key: Option<String>,
    /// Optional seed for procedural micro-surface/allometry synthesis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_seed: Option<u64>,
}

/// Family manifest describing a biological taxon or constructed asset family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TwinFamilyManifest {
    /// Stable URN or IRI (e.g. `"urn:taxon:phascolarctos_cinereus"`).
    pub family_id: String,
    /// Common or trade name (e.g. `"Koala"` or `"Centrifugal Pump V2"`).
    pub common_name: String,
    /// Taxonomic or asset domain classification (`"fauna"`, `"flora"`, `"funga"`, `"machinery"`, `"vehicle"`).
    pub classification: String,
    /// Set of declared model variants.
    pub variants: Vec<ModelVariantDescriptor>,
    /// Variant ID to select when no specific criteria are specified.
    pub default_variant: String,
}

impl TwinFamilyManifest {
    pub fn new(
        family_id: impl Into<String>,
        common_name: impl Into<String>,
        classification: impl Into<String>,
        default_variant: impl Into<String>,
    ) -> Self {
        Self {
            family_id: family_id.into(),
            common_name: common_name.into(),
            classification: classification.into(),
            variants: Vec::new(),
            default_variant: default_variant.into(),
        }
    }

    /// Add a model variant to the family manifest.
    pub fn add_variant(&mut self, variant: ModelVariantDescriptor) {
        if let Some(pos) = self.variants.iter().position(|v| v.variant_id == variant.variant_id) {
            self.variants[pos] = variant;
        } else {
            self.variants.push(variant);
        }
    }

    /// Resolve the most appropriate model variant using a principled fallback policy:
    /// 1. Exact match on `(phenotype, stage, lod, state)`
    /// 2. Match on `(phenotype, stage, state)` with closest available LOD tier
    /// 3. Fallback to `Monomorphic` if sexual dimorphism variant is missing
    /// 4. Fallback to `default_variant`
    pub fn resolve_variant(
        &self,
        phenotype: BiologicalPhenotype,
        stage: LifecycleStage,
        lod: LodTier,
        state: StructuralState,
    ) -> Option<&ModelVariantDescriptor> {
        if self.variants.is_empty() {
            return None;
        }

        // Tier 1: Exact match
        if let Some(v) = self.variants.iter().find(|v| {
            v.phenotype == phenotype && v.stage == stage && v.lod == lod && v.state == state
        }) {
            return Some(v);
        }

        // Tier 2: Match (phenotype, stage, state) with closest LOD
        let candidate_lods: Vec<&ModelVariantDescriptor> = self
            .variants
            .iter()
            .filter(|v| v.phenotype == phenotype && v.stage == stage && v.state == state)
            .collect();

        if !candidate_lods.is_empty() {
            // Find variant with minimum LOD distance
            let closest = candidate_lods
                .into_iter()
                .min_by_key(|v| (v.lod as i32 - lod as i32).abs());
            if let Some(c) = closest {
                return Some(c);
            }
        }

        // Tier 3: Monomorphic fallback if looking for Male/Female
        if phenotype != BiologicalPhenotype::Monomorphic {
            if let Some(v) = self.variants.iter().find(|v| {
                v.phenotype == BiologicalPhenotype::Monomorphic
                    && v.stage == stage
                    && v.lod == lod
                    && v.state == state
            }) {
                return Some(v);
            }
        }

        // Tier 4: Default variant fallback
        self.variants
            .iter()
            .find(|v| v.variant_id == self.default_variant)
            .or_else(|| self.variants.first())
    }

    /// Serialise the family manifest to CBOR bytes.
    pub fn encode_to_cbor(&self) -> Result<Vec<u8>, BundleError> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| BundleError::Cbor(e.to_string()))?;
        Ok(buf)
    }

    /// Deserialise a family manifest from CBOR bytes.
    pub fn decode_from_cbor(bytes: &[u8]) -> Result<Self, BundleError> {
        ciborium::from_reader(bytes).map_err(|e| BundleError::Cbor(e.to_string()))
    }
}

pub const FAMILY_MANIFEST_ENTRY_KEY: &str = "manifest/entity_family.cbor";
pub const FAMILY_MANIFEST_LEGACY_KEY: &str = "manifest/twin_family.cbor";
pub const FAMILY_MANIFEST_KIND: &str = "family_manifest";

/// Semantic alias reflecting ontological entity framing over legacy industrial "twin".
pub type EntityFamilyManifest = TwinFamilyManifest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twin_family_manifest_roundtrips_and_resolves_variants() {
        let mut manifest = TwinFamilyManifest::new(
            "urn:taxon:phascolarctos_cinereus",
            "Koala",
            "fauna",
            "male_adult_lod0",
        );

        manifest.add_variant(ModelVariantDescriptor {
            variant_id: "male_adult_lod0".to_string(),
            phenotype: BiologicalPhenotype::Male,
            stage: LifecycleStage::Adult,
            lod: LodTier::Lod0,
            state: StructuralState::Pristine,
            asset_key: "models/male_adult_lod0.10d".to_string(),
            entity_ontology_key: Some("ontology/male_adult.q42".to_string()),
            procedural_seed: Some(42),
        });

        manifest.add_variant(ModelVariantDescriptor {
            variant_id: "female_adult_lod0".to_string(),
            phenotype: BiologicalPhenotype::Female,
            stage: LifecycleStage::Adult,
            lod: LodTier::Lod0,
            state: StructuralState::Pristine,
            asset_key: "models/female_adult_lod0.10d".to_string(),
            entity_ontology_key: Some("ontology/female_adult.q42".to_string()),
            procedural_seed: Some(43),
        });

        manifest.add_variant(ModelVariantDescriptor {
            variant_id: "monomorphic_juvenile_lod0".to_string(),
            phenotype: BiologicalPhenotype::Monomorphic,
            stage: LifecycleStage::Juvenile,
            lod: LodTier::Lod0,
            state: StructuralState::Pristine,
            asset_key: "models/juvenile_lod0.10d".to_string(),
            entity_ontology_key: None,
            procedural_seed: Some(44),
        });

        // Exact resolution: Female Adult Lod0
        let res1 = manifest
            .resolve_variant(
                BiologicalPhenotype::Female,
                LifecycleStage::Adult,
                LodTier::Lod0,
                StructuralState::Pristine,
            )
            .unwrap();
        assert_eq!(res1.variant_id, "female_adult_lod0");

        // Closest LOD fallback: Male Adult Lod2 -> resolves to Lod0
        let res2 = manifest
            .resolve_variant(
                BiologicalPhenotype::Male,
                LifecycleStage::Adult,
                LodTier::Lod2,
                StructuralState::Pristine,
            )
            .unwrap();
        assert_eq!(res2.variant_id, "male_adult_lod0");

        // Monomorphic fallback: Female Juvenile Lod0 -> resolves to Monomorphic Juvenile
        let res3 = manifest
            .resolve_variant(
                BiologicalPhenotype::Female,
                LifecycleStage::Juvenile,
                LodTier::Lod0,
                StructuralState::Pristine,
            )
            .unwrap();
        assert_eq!(res3.variant_id, "monomorphic_juvenile_lod0");

        // Roundtrip CBOR
        let cbor_bytes = manifest.encode_to_cbor().unwrap();
        let decoded = TwinFamilyManifest::decode_from_cbor(&cbor_bytes).unwrap();
        assert_eq!(decoded.family_id, manifest.family_id);
        assert_eq!(decoded.variants.len(), 3);
    }
}
