//! Botanical 4-Scale Digital Twin Anatomy, Histological Dissection & Living Container Generator.
//!
//! Implements **Epic 1** of `biological-education-assets-and-explorable-world-plan.md`
//! for *Corymbia aparrerinja* (Ghost Gum) and Australian sclerophyll flora across
//! four educational scales:
//!
//! 1. **Scale 1 — Whole-Organism Architecture & Subterranean Root System (`10 m – 40 m`):**
//!    Continuous 5-order branching (bole $\to$ scaffold limbs $\to$ secondary/tertiary branches $\to$
//!    terminal branchlets) via Runions space colonization, Leonardo da Vinci / Shinozaki Pipe Model
//!    ($\eta = 2.49$) radius tapering, root flare allometry, lignotuber with epicormic meristem buds,
//!    primary taproot, lateral buttress flares, and pendulous foliage clusters.
//!
//! 2. **Scale 2 — Branch Phyllotaxis & Stem/Bark Histology Wedge (`50 mm – 2 m`):**
//!    Golden-angle ($\phi = 137.5077^\circ$) phyllotaxis plus a 270°-wedged transverse/radial/tangential
//!    stem dissection with 6 concentric peelable tissue cylinders (`OuterRhytidome`, `PhellogenCorkCambium`,
//!    `SecondaryPhloemAndKinoDucts`, `VascularCambium`, `SapwoodSecondaryXylem`, `HeartwoodAndPith`).
//!
//! 3. **Scale 3 — Floral & Fruit Reproductive Dissection (`5 mm – 150 mm`):**
//!    Axillary 7-flowered umbellaster, fused `HypanthiumCalyxTube`, shedding `OperculumCalyptra` (driven
//!    by anthesis progress and baked into `.10d` `MRP1` morph deltas), 48-filament inflexed/unfurling
//!    `StaminalRingFilaments`, bilobed `VersatileAnthersAndPollen`, `GynoeciumStyleStigma`, 4-locule inferior
//!    ovary with axile placentation and `OvaryLoculesAndOvules`, mature urceolate `WoodyPericarpAndDehiscingValves`,
//!    and `FertileWingedSeedsAndChaff`.
//!
//! 4. **Scale 4 — Foliar Lamina Venation & Cellular Histology (`50 µm – 50 mm`):**
//!    Falcate-lanceolate blade with Murray's Law ($r_p^3 = r_1^3 + r_2^3$) midrib, diagnostic Myrtaceae
//!    intramarginal vein, pinnate secondaries, and tertiary reticulate areoles, paired with a magnified 3D
//!    histological block featuring `WaxyCuticle`, `UpperAndLowerEpidermis`, sunken `AmphistomaticStomatalGuardCells`,
//!    `IsobilateralPalisadeMesophyll`, `SpongyMesophyllAndAirSpaces`, `CollateralVascularBundleXylemPhloem`,
//!    `SclerenchymaBundleSheathGirders`, and spherical `SchizogenousEssentialOilGland`.
//!
//! Strictly zero-V8, fully sealed `.10d` living containers with `MAT4`, `UV01`, `FNRM`, `SRD1`,
//! `PROC1`, `EONT1`, `MRP1`, and `PROV` sections.

use crate::container_10d::entity_ontology_section::{
    AffordanceBinaryRecord, DevelopmentalKeyframeRecord, EntityOntologyMiniHeader,
    AFFORDANCE_RECORD_SIZE, DEVELOPMENTAL_KEYFRAME_RECORD_SIZE, ENTITY_ONTOLOGY_MINI_HEADER_SIZE,
    ENTITY_ONTOLOGY_SECTION_VERSION, MAGIC_EONT,
};
use crate::container_10d::header::{Container10dHeader, MAX_SECTION_COUNT};
use crate::container_10d::integrity::{seal_whole_file_crc32c, verify_whole_file_crc32c};
use crate::container_10d::morph_section::{
    encode_morph_section, morph_section_encoded_len, quantize_morph_delta, MorphSectionHeader,
    QuantizedMorphDelta, FLAG_MORPH_HAS_NORMALS, FLAG_MORPH_ONTOGENETIC_BIND, FLAG_MORPH_SPARSE,
    MAGIC_MRP1, MORPH_SECTION_VERSION,
};
use crate::container_10d::procedural_synthesis::{
    encode_procedural_synthesis_section, ProceduralSynthesisHeader, FLAG_ALLOMETRIC_GROWTH,
    FLAG_CELLULAR_VORONOI, FLAG_IMPLICIT_FILLET, FLAG_PGA_MOTOR_DYNAMICS, MAGIC_PROC,
    PROCEDURAL_SYNTHESIS_HEADER_SIZE, PROCEDURAL_SYNTHESIS_SECTION_VERSION,
};
use crate::container_10d::provenance_section::ProvenanceSidecar;
use crate::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use crate::container_10d::{
    attach_material_section, attach_texture_coordinate_section, MaterialRecord, OpacityMode,
    ShadingModel, SubmeshRange,
};
use crate::q_hash;
use crate::render::assets::Mesh;
use crate::render::compile_10d::{compile_mesh_to_10d_with_extras_and_frames, Compile10dExtras};
use crate::specialized_libs::computational_geometry::space_colonization::{
    lateral_whorl_directions, space_colonization_3d, SpaceColonizationConfig, ROOT_PARENT,
};
use crate::specialized_libs::computational_geometry::tree_skeleton_3d::pipe_model_radii;
use crate::specialized_libs::computational_geometry::Point3;
use bytemuck;
use etalon_sim::leaf_biophysics::{LeafEconomicsProfile, LeafVascularSkeleton};
use etalon_sim::metamer_fspm::{Metamer, MetamerOrgan, MetamerPlantGraph, ThermalPhenology};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::f32::consts::PI;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Canonical golden ratio angle ($\phi = 137.507764^\circ$) in radians.
pub const GOLDEN_ANGLE_RAD: f32 = 2.39996323;
pub const GOLDEN_ANGLE: f32 = GOLDEN_ANGLE_RAD;

/// Leonardo da Vinci / Shinozaki pipe exponent for eucalyptus crown hydraulic architecture.
pub const PIPE_EXPONENT_EUCALYPTUS: f64 = 2.49;

/// Re-export upstream FSPM plant architecture type.
pub type PlantArchitecture = MetamerPlantGraph;

// ============================================================================
// 1. Core Educational Scales & Anatomical Taxonomies
// ============================================================================

/// Four progressive spatial scales of botanical digital twin education.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BotanicalEducationScale {
    /// Whole tree habit, bole, 5-order branching canopy, and subterranean root architecture.
    Scale1WholeOrganismAndRoots,
    /// Metamer internodes, golden-angle phyllotaxis, and 6-layer concentric stem histology wedge.
    Scale2BranchPhyllotaxisAndStemWedge,
    /// 7-flowered umbellaster, calyptrate flower anthesis, and woody dehiscing gumnut capsule.
    Scale3FloralAndGumnutDissection,
    /// Isobilateral sclerophyll lamina, Murray's Law venation, intramarginal vein, and cellular layers.
    Scale4FoliarVenationAndCellularHistology,
}

impl BotanicalEducationScale {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Scale1WholeOrganismAndRoots => "Scale 1: Whole Organism & Root Architecture",
            Self::Scale2BranchPhyllotaxisAndStemWedge => {
                "Scale 2: Branch Phyllotaxis & Stem Wedge Anatomy"
            }
            Self::Scale3FloralAndGumnutDissection => {
                "Scale 3: Floral & Fruit Reproductive Dissection"
            }
            Self::Scale4FoliarVenationAndCellularHistology => {
                "Scale 4: Foliar Lamina Venation & Cellular Histology"
            }
        }
    }

    pub const fn physical_range(&self) -> &'static str {
        match self {
            Self::Scale1WholeOrganismAndRoots => "10 m – 40 m",
            Self::Scale2BranchPhyllotaxisAndStemWedge => "50 mm – 2 m",
            Self::Scale3FloralAndGumnutDissection => "5 mm – 150 mm",
            Self::Scale4FoliarVenationAndCellularHistology => "50 µm – 50 mm",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            Self::Scale1WholeOrganismAndRoots => {
                "Macroscopic organismal habit, Leonardo da Vinci pipe model crown, lignotuber and subterranean roots"
            }
            Self::Scale2BranchPhyllotaxisAndStemWedge => {
                "Stem metamer, 137.5° spiral phyllotaxis, and peelable 6-layer concentric tissue histology wedge"
            }
            Self::Scale3FloralAndGumnutDissection => {
                "Axillary umbellaster, circumscissile operculum shedding, inflexed stamens, and dehiscing woody capsule"
            }
            Self::Scale4FoliarVenationAndCellularHistology => {
                "Falcate lamina, intramarginal vein, amphistomatic guard cells, isobilateral mesophyll, and oil glands"
            }
        }
    }
}

/// Ontogenetic developmental stage of the botanical digital twin organism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DevelopmentalStage {
    /// Germinant / seedling with cotyledons and opposite juvenile glaucous leaves.
    Seedling,
    /// Upright competing sapling with transitional alternate petiolate foliage.
    Sapling,
    /// Fully mature canopy with chalk-white bole, mass flowering, and fruiting capsules.
    Mature,
    /// Veteran tree with dead staghorn crown limbs, fungal heartwood decay hollows, and epicormic bursts.
    AncientHollowBearing,
}

impl DevelopmentalStage {
    pub const fn stage_id(&self) -> u8 {
        match self {
            Self::Seedling => 0,
            Self::Sapling => 1,
            Self::Mature => 2,
            Self::AncientHollowBearing => 3,
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            Self::Seedling => "Seedling (cotyledons, opposite ovate leaves, juvenile lignotuber)",
            Self::Sapling => "Sapling (competing leaders, upright scaffold limbs, transitional foliage)",
            Self::Mature => "Mature (spreading irregular canopy, smooth chalk bole, full reproductive capacity)",
            Self::AncientHollowBearing => "Ancient Veteran (senescent staghorn limbs, trunk hollow cavities, epicormic regrowth)",
        }
    }
}

/// Dissectible and peelable botanical tissue layer with Plant Ontology (PO) bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistologicalTissueLayer {
    // --- Scale 2: Stem & Bark Histological Layers ---
    /// Outer dead bark / shed flakes (Plant Ontology: PO:0004518).
    OuterRhytidome,
    /// Secondary cork cambium and phelloderm meristem (PO:0005597).
    PhellogenCorkCambium,
    /// Inner conducting bark with sieve tubes, fibers, and kino resin ducts (PO:0004523).
    SecondaryPhloemAndKinoDucts,
    /// Uniseriate lateral meristem ring producing xylem inwardly and phloem outwardly (PO:0005598).
    VascularCambium,
    /// Active water-conducting secondary xylem with annual rings, vessel pores, and medullary rays (PO:0004524).
    SapwoodSecondaryXylem,
    /// Polyphenol-impregnated structural core (duramen) and central primary pith (PO:0004525).
    HeartwoodAndPith,

    // --- Scale 3: Floral & Fruit Reproductive Layers ---
    /// Inflorescence stalk and individual flower stalks of the 7-flowered umbellaster (PO:0009053).
    PeduncleAndPedicel,
    /// Floral cup fused with the inferior ovary wall (PO:0009064).
    HypanthiumCalyxTube,
    /// Calyptriform bud cap shedding circumscissilely at anthesis (PO:0009049).
    OperculumCalyptra,
    /// Inflexed multi-whorl cream filaments emerging from the floral ring (PO:0009066).
    StaminalRingFilaments,
    /// Dorsifixed versatile bilobed anther sacs containing microspores/pollen (PO:0009067).
    VersatileAnthersAndPollen,
    /// Central terete style, papillate stigma, and nectariferous disc (PO:0009072).
    GynoeciumStyleStigma,
    /// Inferior ovary cutaway showing 4 locule chambers, axile placenta, and ovules (PO:0009062).
    OvaryLoculesAndOvules,
    /// Thick woody capsule wall with 4 sunken triangular dehiscing valves (PO:0009084).
    WoodyPericarpAndDehiscingValves,
    /// Terminal-winged fertile seeds and sterile crystalline red-brown chaff (PO:0009010).
    FertileWingedSeedsAndChaff,

    // --- Scale 4: Foliar Lamina & Cellular Histological Layers ---
    /// Epicuticular wax film with crystalline micro-platelets (PO:0000030).
    WaxyCuticle,
    /// Adaxial and abaxial uniseriate cobblestone epidermal pavement cells (PO:0000040).
    UpperAndLowerEpidermis,
    /// Sunken paired kidney-shaped guard cells and substomatal respiratory cavities (PO:0002000).
    AmphistomaticStomatalGuardCells,
    /// Symmetrical 2-layer columnar chlorenchyma on both adaxial and abaxial surfaces (PO:0006079).
    IsobilateralPalisadeMesophyll,
    /// Central loosely packed aerating parenchyma with intercellular gas channels (PO:0006080).
    SpongyMesophyllAndAirSpaces,
    /// Collateral vascular bundles: adaxial xylem tracheary vessels + abaxial phloem sieve tubes (PO:0005020).
    CollateralVascularBundleXylemPhloem,
    /// Lignified I-beam bundle sheath extensions anchoring veins to both epidermises (PO:0004533).
    SclerenchymaBundleSheathGirders,
    /// Spherical internal secretory cavity lined with epithelial cells producing essential oils (PO:0004540).
    SchizogenousEssentialOilGland,
}

impl HistologicalTissueLayer {
    /// Return the canonical Plant Ontology (PO) identifier.
    pub const fn plant_ontology_id(&self) -> &'static str {
        match self {
            Self::OuterRhytidome => "PO:0004518",
            Self::PhellogenCorkCambium => "PO:0005597",
            Self::SecondaryPhloemAndKinoDucts => "PO:0004523",
            Self::VascularCambium => "PO:0005598",
            Self::SapwoodSecondaryXylem => "PO:0004524",
            Self::HeartwoodAndPith => "PO:0004525",
            Self::PeduncleAndPedicel => "PO:0009053",
            Self::HypanthiumCalyxTube => "PO:0009064",
            Self::OperculumCalyptra => "PO:0009049",
            Self::StaminalRingFilaments => "PO:0009066",
            Self::VersatileAnthersAndPollen => "PO:0009067",
            Self::GynoeciumStyleStigma => "PO:0009072",
            Self::OvaryLoculesAndOvules => "PO:0009062",
            Self::WoodyPericarpAndDehiscingValves => "PO:0009084",
            Self::FertileWingedSeedsAndChaff => "PO:0009010",
            Self::WaxyCuticle => "PO:0000030",
            Self::UpperAndLowerEpidermis => "PO:0000040",
            Self::AmphistomaticStomatalGuardCells => "PO:0002000",
            Self::IsobilateralPalisadeMesophyll => "PO:0006079",
            Self::SpongyMesophyllAndAirSpaces => "PO:0006080",
            Self::CollateralVascularBundleXylemPhloem => "PO:0005020",
            Self::SclerenchymaBundleSheathGirders => "PO:0004533",
            Self::SchizogenousEssentialOilGland => "PO:0004540",
        }
    }

    /// English scientific common name.
    pub const fn common_name(&self) -> &'static str {
        match self {
            Self::OuterRhytidome => "Outer Rhytidome (Chalk Bark Flakes)",
            Self::PhellogenCorkCambium => "Phellogen (Cork Cambium & Phelloderm)",
            Self::SecondaryPhloemAndKinoDucts => "Secondary Phloem & Kino Resin Ducts",
            Self::VascularCambium => "Vascular Cambium (Lateral Meristem Ring)",
            Self::SapwoodSecondaryXylem => "Sapwood (Secondary Xylem & Rays)",
            Self::HeartwoodAndPith => "Heartwood (Duramen) & Central Pith",
            Self::PeduncleAndPedicel => "Peduncle & Pedicels (Umbellaster Stalks)",
            Self::HypanthiumCalyxTube => "Hypanthium (Floral Calyx Tube)",
            Self::OperculumCalyptra => "Operculum (Calyptra Bud Cap)",
            Self::StaminalRingFilaments => "Staminal Ring Filaments (Androecium)",
            Self::VersatileAnthersAndPollen => "Versatile Anthers & Pollen Grains",
            Self::GynoeciumStyleStigma => "Gynoecium (Style, Stigma & Nectar Disc)",
            Self::OvaryLoculesAndOvules => "Inferior Ovary (4 Locules, Placenta & Ovules)",
            Self::WoodyPericarpAndDehiscingValves => "Woody Pericarp & Dehiscing Valves",
            Self::FertileWingedSeedsAndChaff => "Fertile Winged Seeds & Chaff",
            Self::WaxyCuticle => "Epicuticular Waxy Cuticle Platelets",
            Self::UpperAndLowerEpidermis => "Upper & Lower Epidermis Pavement Cells",
            Self::AmphistomaticStomatalGuardCells => "Amphistomatic Stomatal Guard Cells",
            Self::IsobilateralPalisadeMesophyll => "Isobilateral Palisade Mesophyll Columns",
            Self::SpongyMesophyllAndAirSpaces => "Spongy Mesophyll Parenchyma & Air Spaces",
            Self::CollateralVascularBundleXylemPhloem => {
                "Collateral Vascular Bundle (Xylem & Phloem)"
            }
            Self::SclerenchymaBundleSheathGirders => "Sclerenchyma Bundle Sheath Girders",
            Self::SchizogenousEssentialOilGland => "Schizogenous Essential Oil Secretory Cavity",
        }
    }

    /// Associated educational scale.
    pub const fn scale(&self) -> BotanicalEducationScale {
        match self {
            Self::OuterRhytidome
            | Self::PhellogenCorkCambium
            | Self::SecondaryPhloemAndKinoDucts
            | Self::VascularCambium
            | Self::SapwoodSecondaryXylem
            | Self::HeartwoodAndPith => {
                BotanicalEducationScale::Scale2BranchPhyllotaxisAndStemWedge
            }

            Self::PeduncleAndPedicel
            | Self::HypanthiumCalyxTube
            | Self::OperculumCalyptra
            | Self::StaminalRingFilaments
            | Self::VersatileAnthersAndPollen
            | Self::GynoeciumStyleStigma
            | Self::OvaryLoculesAndOvules
            | Self::WoodyPericarpAndDehiscingValves
            | Self::FertileWingedSeedsAndChaff => {
                BotanicalEducationScale::Scale3FloralAndGumnutDissection
            }

            Self::WaxyCuticle
            | Self::UpperAndLowerEpidermis
            | Self::AmphistomaticStomatalGuardCells
            | Self::IsobilateralPalisadeMesophyll
            | Self::SpongyMesophyllAndAirSpaces
            | Self::CollateralVascularBundleXylemPhloem
            | Self::SclerenchymaBundleSheathGirders
            | Self::SchizogenousEssentialOilGland => {
                BotanicalEducationScale::Scale4FoliarVenationAndCellularHistology
            }
        }
    }

    /// Diagnostic default RGBA color.
    pub const fn default_color_rgba(&self) -> [f32; 4] {
        match self {
            Self::OuterRhytidome => [0.93, 0.92, 0.88, 1.0], // Chalky white
            Self::PhellogenCorkCambium => [0.58, 0.64, 0.44, 1.0], // Olive green meristem
            Self::SecondaryPhloemAndKinoDucts => [0.72, 0.44, 0.28, 1.0], // Warm amber phloem
            Self::VascularCambium => [0.78, 0.88, 0.52, 1.0], // Pale lime dividing ring
            Self::SapwoodSecondaryXylem => [0.86, 0.79, 0.67, 1.0], // Cream active wood
            Self::HeartwoodAndPith => [0.42, 0.22, 0.16, 1.0], // Deep tannin duramen

            Self::PeduncleAndPedicel => [0.48, 0.56, 0.38, 1.0], // Glaucous green stem
            Self::HypanthiumCalyxTube => [0.74, 0.72, 0.56, 1.0], // Creamy-green cup
            Self::OperculumCalyptra => [0.88, 0.84, 0.68, 1.0],  // Pale conical cap
            Self::StaminalRingFilaments => [0.98, 0.96, 0.85, 1.0], // Fluffy cream filaments
            Self::VersatileAnthersAndPollen => [0.96, 0.82, 0.24, 1.0], // Golden yellow pollen
            Self::GynoeciumStyleStigma => [0.65, 0.78, 0.48, 1.0], // Translucent green style
            Self::OvaryLoculesAndOvules => [0.82, 0.86, 0.70, 1.0], // Pale ovule interior
            Self::WoodyPericarpAndDehiscingValves => [0.46, 0.38, 0.30, 1.0], // Weathered brown gumnut
            Self::FertileWingedSeedsAndChaff => [0.28, 0.18, 0.12, 1.0], // Dark chocolate seeds

            Self::WaxyCuticle => [0.82, 0.88, 0.84, 0.85], // Glaucous translucent wax
            Self::UpperAndLowerEpidermis => [0.46, 0.62, 0.42, 1.0], // Bright epidermal green
            Self::AmphistomaticStomatalGuardCells => [0.36, 0.54, 0.32, 1.0], // Kidney guard cell green
            Self::IsobilateralPalisadeMesophyll => [0.24, 0.48, 0.22, 1.0], // Deep chloroplast green
            Self::SpongyMesophyllAndAirSpaces => [0.38, 0.58, 0.36, 0.9],   // Aerated parenchyma
            Self::CollateralVascularBundleXylemPhloem => [0.75, 0.68, 0.48, 1.0], // Lignified xylem/phloem
            Self::SclerenchymaBundleSheathGirders => [0.82, 0.76, 0.54, 1.0], // Lignified fibrous girders
            Self::SchizogenousEssentialOilGland => [0.95, 0.84, 0.35, 0.95],  // Golden cineole oil
        }
    }

    /// Default physical roughness and metallic parameters `(roughness, metallic)`.
    pub const fn default_pbr_material(&self) -> (f32, f32) {
        match self {
            Self::OuterRhytidome => (0.92, 0.0),
            Self::PhellogenCorkCambium => (0.75, 0.0),
            Self::SecondaryPhloemAndKinoDucts => (0.55, 0.02),
            Self::VascularCambium => (0.60, 0.0),
            Self::SapwoodSecondaryXylem => (0.80, 0.0),
            Self::HeartwoodAndPith => (0.85, 0.0),

            Self::PeduncleAndPedicel => (0.70, 0.0),
            Self::HypanthiumCalyxTube => (0.65, 0.0),
            Self::OperculumCalyptra => (0.68, 0.0),
            Self::StaminalRingFilaments => (0.45, 0.0),
            Self::VersatileAnthersAndPollen => (0.50, 0.0),
            Self::GynoeciumStyleStigma => (0.40, 0.0),
            Self::OvaryLoculesAndOvules => (0.60, 0.0),
            Self::WoodyPericarpAndDehiscingValves => (0.88, 0.0),
            Self::FertileWingedSeedsAndChaff => (0.78, 0.0),

            Self::WaxyCuticle => (0.28, 0.0),
            Self::UpperAndLowerEpidermis => (0.55, 0.0),
            Self::AmphistomaticStomatalGuardCells => (0.50, 0.0),
            Self::IsobilateralPalisadeMesophyll => (0.65, 0.0),
            Self::SpongyMesophyllAndAirSpaces => (0.70, 0.0),
            Self::CollateralVascularBundleXylemPhloem => (0.75, 0.0),
            Self::SclerenchymaBundleSheathGirders => (0.80, 0.0),
            Self::SchizogenousEssentialOilGland => (0.18, 0.0),
        }
    }
}

/// A contiguous submesh range representing an individual histological tissue layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TissueSubmeshSlice {
    pub layer: HistologicalTissueLayer,
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub plant_ontology_id: &'static str,
    pub common_name: &'static str,
}

/// Compiled high-density botanical organ with full living `.10d` container bytes.
#[derive(Debug, Clone)]
pub struct CompiledBotanicalOrgan10d {
    pub scale: BotanicalEducationScale,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[u8; 4]>,
    pub material_indices: Vec<u8>,
    pub indices: Vec<u32>,
    pub tissue_slices: Vec<TissueSubmeshSlice>,
    pub container_bytes: Vec<u8>,
}

impl CompiledBotanicalOrgan10d {
    /// Total vertices in organ mesh.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Total triangles in organ mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Find the submesh slice for a specific tissue layer.
    pub fn tissue_layer_slice(
        &self,
        layer: HistologicalTissueLayer,
    ) -> Option<&TissueSubmeshSlice> {
        self.tissue_slices.iter().find(|s| s.layer == layer)
    }

    /// Verify structural, section table, CRC-32C, and geometric integrity.
    pub fn verify_all(&self) -> Result<(), String> {
        if self.container_bytes.len() < 64 {
            return Err("Container bytes too short for 10D header".into());
        }
        let header = Container10dHeader::parse(&self.container_bytes)
            .map_err(|e| format!("Header parse error: {e:?}"))?;

        let descriptors = parse_section_table(&self.container_bytes, &header)
            .map_err(|e| format!("Section table parse error: {e:?}"))?;
        if descriptors.is_empty() {
            return Err("Section table is empty".into());
        }

        let mut bytes_copy = self.container_bytes.clone();
        verify_whole_file_crc32c(&mut bytes_copy)
            .map_err(|e| format!("Whole file CRC-32C verification failed: {e:?}"))?;

        if self.vertices.is_empty() {
            return Err("Organ vertices must not be empty".into());
        }
        if self.indices.len() % 3 != 0 {
            return Err("Indices count must be a multiple of 3".into());
        }
        for &idx in &self.indices {
            if (idx as usize) >= self.vertices.len() {
                return Err(format!(
                    "Index out of range: {idx} >= {}",
                    self.vertices.len()
                ));
            }
        }
        if self.normals.len() != self.vertices.len() {
            return Err("Normals length does not match vertices".into());
        }
        if self.uvs.len() != self.vertices.len() {
            return Err("UVs length does not match vertices".into());
        }
        if self.colors.len() != self.vertices.len() {
            return Err("Colors length does not match vertices".into());
        }

        for slice in &self.tissue_slices {
            if (slice.vertex_start as usize) + (slice.vertex_count as usize) > self.vertices.len() {
                return Err(format!(
                    "Slice vertex range out of bounds for layer {:?}",
                    slice.layer
                ));
            }
            if (slice.index_start as usize) + (slice.index_count as usize) > self.indices.len() {
                return Err(format!(
                    "Slice index range out of bounds for layer {:?}",
                    slice.layer
                ));
            }
        }

        Ok(())
    }
}

// ============================================================================
// 2. Allometry & Upstream Computational Geometry Utilities
// ============================================================================

/// Allometric root flare radius at depth $d$ below the ground line ($d \ge 0$):
/// $r_{\text{flare}}(d) = r_{\text{collar}} \cdot (1.0 + \alpha_{\text{flare}} \cdot e^{-\beta_{\text{flare}} \cdot d})$
pub fn root_flare_radius(base_radius: f64, depth_m: f64, flare_factor: f64) -> f64 {
    base_radius * (1.0 + flare_factor * (-depth_m.abs() * 2.5).exp())
}

/// Run deterministic 3D space colonization from `seed_nodes` towards `attractors`.
pub fn grow_crown(
    attractors: &[Point3],
    seeds: &[Point3],
    seed_parents: &[u32],
    cfg: &SpaceColonizationConfig,
    out_nodes: &mut [Point3],
    out_parents: &mut [u32],
) -> Result<usize, String> {
    space_colonization_3d(attractors, seeds, seed_parents, cfg, out_nodes, out_parents)
        .map(|r| r.node_count)
        .map_err(|e| format!("space_colonization error: {e:?}"))
}

/// Emit lateral buds with golden-angle or whorled phyllotaxis.
pub fn emit_lateral_buds(
    axis_dir: Point3,
    insertion_angle_rad: f64,
    count: usize,
    out_dirs: &mut [Point3],
) -> Result<usize, String> {
    lateral_whorl_directions(axis_dir, insertion_angle_rad, count, out_dirs)
        .map_err(|e| format!("emit_lateral_buds error: {e:?}"))
}

// ============================================================================
// 3. Organ Mesh Builder & Container Sealer
// ============================================================================

struct OrganMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
    material_indices: Vec<u8>,
    indices: Vec<u32>,
    slices: Vec<TissueSubmeshSlice>,
    active_layer: Option<(HistologicalTissueLayer, u32, u32)>,
    current_mat_idx: u8,
}

impl OrganMeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::with_capacity(4096),
            normals: Vec::with_capacity(4096),
            uvs: Vec::with_capacity(4096),
            colors: Vec::with_capacity(4096),
            material_indices: Vec::with_capacity(4096),
            indices: Vec::with_capacity(16384),
            slices: Vec::new(),
            active_layer: None,
            current_mat_idx: 0,
        }
    }

    fn begin_tissue_layer(&mut self, layer: HistologicalTissueLayer, mat_idx: u8) {
        if self.active_layer.is_some() {
            self.end_tissue_layer();
        }
        let v_start = self.positions.len() as u32;
        let i_start = self.indices.len() as u32;
        self.active_layer = Some((layer, v_start, i_start));
        self.current_mat_idx = mat_idx;
    }

    fn end_tissue_layer(&mut self) {
        if let Some((layer, v_start, i_start)) = self.active_layer.take() {
            let v_count = (self.positions.len() as u32).saturating_sub(v_start);
            let i_count = (self.indices.len() as u32).saturating_sub(i_start);
            self.slices.push(TissueSubmeshSlice {
                layer,
                vertex_start: v_start,
                vertex_count: v_count,
                index_start: i_start,
                index_count: i_count,
                plant_ontology_id: layer.plant_ontology_id(),
                common_name: layer.common_name(),
            });
        }
    }

    fn add_vertex(&mut self, pos: [f32; 3], norm: [f32; 3], uv: [f32; 2], col: [u8; 4]) -> u32 {
        let idx = self.positions.len() as u32;
        self.positions.push(pos);
        self.normals.push(norm);
        self.uvs.push(uv);
        self.colors.push(col);
        self.material_indices.push(self.current_mat_idx);
        idx
    }

    fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.push(i0);
        self.indices.push(i1);
        self.indices.push(i2);
    }

    fn add_quad(&mut self, i0: u32, i1: u32, i2: u32, i3: u32) {
        self.add_triangle(i0, i1, i2);
        self.add_triangle(i0, i2, i3);
    }

    /// Rebuild smooth vertex normals for newly appended geometry from its indexed triangles.
    fn recompute_normals_since(&mut self, vertex_start: usize, index_start: usize) {
        if vertex_start >= self.positions.len() || index_start >= self.indices.len() {
            return;
        }
        self.normals[vertex_start..].fill([0.0; 3]);
        for tri in self.indices[index_start..].chunks_exact(3) {
            let a_idx = tri[0] as usize;
            let b_idx = tri[1] as usize;
            let c_idx = tri[2] as usize;
            if a_idx < vertex_start || b_idx < vertex_start || c_idx < vertex_start {
                continue;
            }
            let a = self.positions[a_idx];
            let b = self.positions[b_idx];
            let c = self.positions[c_idx];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let face = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            for idx in [a_idx, b_idx, c_idx] {
                let normal = &mut self.normals[idx];
                normal[0] += face[0];
                normal[1] += face[1];
                normal[2] += face[2];
            }
        }
        for normal in &mut self.normals[vertex_start..] {
            let len =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 1e-8 {
                normal[0] /= len;
                normal[1] /= len;
                normal[2] /= len;
            }
        }
    }

    /// Add a cylindrical shell or segment along `p0 -> p1` with radius `r0` to `r1`.
    fn add_cylinder_segment(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        r0: f32,
        r1: f32,
        radial_slices: usize,
        col0: [u8; 4],
        col1: [u8; 4],
    ) {
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let dz = p1[2] - p0[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-5);
        let axis = [dx / len, dy / len, dz / len];

        let ref_v = if axis[0].abs() < 0.8 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let dot_a = ref_v[0] * axis[0] + ref_v[1] * axis[1] + ref_v[2] * axis[2];
        let mut u = [
            ref_v[0] - dot_a * axis[0],
            ref_v[1] - dot_a * axis[1],
            ref_v[2] - dot_a * axis[2],
        ];
        let u_len = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt().max(1e-5);
        u = [u[0] / u_len, u[1] / u_len, u[2] / u_len];
        let v = [
            axis[1] * u[2] - axis[2] * u[1],
            axis[2] * u[0] - axis[0] * u[2],
            axis[0] * u[1] - axis[1] * u[0],
        ];

        let base_idx = self.positions.len() as u32;
        let n = radial_slices.max(4);

        for i in 0..=n {
            let frac = (i as f32) / (n as f32);
            let angle = frac * 2.0 * PI;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            let norm = [
                u[0] * cos_a + v[0] * sin_a,
                u[1] * cos_a + v[1] * sin_a,
                u[2] * cos_a + v[2] * sin_a,
            ];

            let v0 = [
                p0[0] + norm[0] * r0,
                p0[1] + norm[1] * r0,
                p0[2] + norm[2] * r0,
            ];
            self.add_vertex(v0, norm, [frac, 0.0], col0);

            let v1 = [
                p1[0] + norm[0] * r1,
                p1[1] + norm[1] * r1,
                p1[2] + norm[2] * r1,
            ];
            self.add_vertex(v1, norm, [frac, 1.0], col1);
        }

        for i in 0..n {
            let i0 = base_idx + (i * 2) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + 3;
            let i3 = i0 + 2;
            self.add_quad(i0, i1, i2, i3);
        }
    }

    /// Add a 3D box cell (e.g. brick epidermal or columnar palisade cell).
    fn add_box_cell(&mut self, center: [f32; 3], half_extents: [f32; 3], col: [u8; 4]) {
        let cx = center[0];
        let cy = center[1];
        let cz = center[2];
        let hx = half_extents[0];
        let hy = half_extents[1];
        let hz = half_extents[2];

        let faces = [
            // +Y top
            (
                [0.0, 1.0, 0.0],
                [
                    [cx - hx, cy + hy, cz - hz],
                    [cx - hx, cy + hy, cz + hz],
                    [cx + hx, cy + hy, cz + hz],
                    [cx + hx, cy + hy, cz - hz],
                ],
            ),
            // -Y bottom
            (
                [0.0, -1.0, 0.0],
                [
                    [cx - hx, cy - hy, cz - hz],
                    [cx + hx, cy - hy, cz - hz],
                    [cx + hx, cy - hy, cz + hz],
                    [cx - hx, cy - hy, cz + hz],
                ],
            ),
            // +X right
            (
                [1.0, 0.0, 0.0],
                [
                    [cx + hx, cy - hy, cz - hz],
                    [cx + hx, cy + hy, cz - hz],
                    [cx + hx, cy + hy, cz + hz],
                    [cx + hx, cy - hy, cz + hz],
                ],
            ),
            // -X left
            (
                [-1.0, 0.0, 0.0],
                [
                    [cx - hx, cy - hy, cz - hz],
                    [cx - hx, cy - hy, cz + hz],
                    [cx - hx, cy + hy, cz + hz],
                    [cx - hx, cy + hy, cz - hz],
                ],
            ),
            // +Z front
            (
                [0.0, 0.0, 1.0],
                [
                    [cx - hx, cy - hy, cz + hz],
                    [cx + hx, cy - hy, cz + hz],
                    [cx + hx, cy + hy, cz + hz],
                    [cx - hx, cy + hy, cz + hz],
                ],
            ),
            // -Z back
            (
                [0.0, 0.0, -1.0],
                [
                    [cx - hx, cy - hy, cz - hz],
                    [cx - hx, cy + hy, cz - hz],
                    [cx + hx, cy + hy, cz - hz],
                    [cx + hx, cy - hy, cz - hz],
                ],
            ),
        ];

        for (norm, corners) in faces {
            let i0 = self.add_vertex(corners[0], norm, [0.0, 0.0], col);
            let i1 = self.add_vertex(corners[1], norm, [0.0, 1.0], col);
            let i2 = self.add_vertex(corners[2], norm, [1.0, 1.0], col);
            let i3 = self.add_vertex(corners[3], norm, [1.0, 0.0], col);
            self.add_quad(i0, i1, i2, i3);
        }
    }

    /// Add an ellipsoid or sphere (e.g. oil gland, ovary ovule, bud).
    fn add_ellipsoid(
        &mut self,
        center: [f32; 3],
        radii: [f32; 3],
        rings: usize,
        sectors: usize,
        col: [u8; 4],
    ) {
        let base_idx = self.positions.len() as u32;
        let nr = rings.max(4);
        let ns = sectors.max(6);

        for r in 0..=nr {
            let theta = (r as f32) * PI / (nr as f32);
            let sin_t = theta.sin();
            let cos_t = theta.cos();

            for s in 0..=ns {
                let phi = (s as f32) * 2.0 * PI / (ns as f32);
                let sin_p = phi.sin();
                let cos_p = phi.cos();

                let norm = [sin_t * cos_p, cos_t, sin_t * sin_p];
                let pos = [
                    center[0] + norm[0] * radii[0],
                    center[1] + norm[1] * radii[1],
                    center[2] + norm[2] * radii[2],
                ];
                let u = (s as f32) / (ns as f32);
                let v = (r as f32) / (nr as f32);
                self.add_vertex(pos, norm, [u, v], col);
            }
        }

        for r in 0..nr {
            for s in 0..ns {
                let i0 = base_idx + (r * (ns + 1) + s) as u32;
                let i1 = i0 + (ns + 1) as u32;
                let i2 = i1 + 1;
                let i3 = i0 + 1;
                self.add_quad(i0, i1, i2, i3);
            }
        }
    }
}

fn attach_raw_section(
    container: &[u8],
    section_type: SectionType,
    alignment_tier: AlignmentTier,
    stride: u32,
    element_count: u32,
    payload: &[u8],
) -> Result<Vec<u8>, String> {
    let header = Container10dHeader::parse(container)
        .map_err(|e| format!("10d header parse error: {e:?}"))?;
    let descriptors = parse_section_table(container, &header)
        .map_err(|e| format!("parse_section_table error: {e:?}"))?;
    if descriptors.len() >= MAX_SECTION_COUNT as usize {
        return Err("MAX_SECTION_COUNT reached".into());
    }

    let mut inputs = Vec::with_capacity(descriptors.len() + 1);
    for descriptor in descriptors {
        let start = descriptor.byte_offset as usize;
        let end = start
            .checked_add(descriptor.byte_length as usize)
            .filter(|&end| end <= container.len())
            .ok_or_else(|| {
                format!(
                    "SectionTableError::OutOfBounds offset={} length={}",
                    descriptor.byte_offset, descriptor.byte_length
                )
            })?;
        inputs.push(SectionInput {
            section_type: descriptor
                .typ()
                .ok_or_else(|| format!("UndefinedSectionType: {}", descriptor.section_type))?,
            alignment_tier: descriptor
                .tier()
                .ok_or_else(|| format!("UndefinedAlignmentTier: {}", descriptor.alignment_tier))?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: &container[start..end],
        });
    }

    inputs.push(SectionInput {
        section_type,
        alignment_tier,
        stride,
        element_count,
        payload,
    });

    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(size) => size,
        Err(error) => return Err(format!("encode_container dry-run error: {error:?}")),
    };
    let mut output = vec![0u8; needed];
    let written = encode_container(&header, &inputs, &mut output)
        .map_err(|e| format!("encode_container error: {e:?}"))?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

fn attach_procedural_synthesis_section(
    container: &[u8],
    header: &ProceduralSynthesisHeader,
) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; PROCEDURAL_SYNTHESIS_HEADER_SIZE];
    encode_procedural_synthesis_section(header, &mut buf)
        .map_err(|e| format!("encode_procedural_synthesis_section: {e:?}"))?;
    attach_raw_section(
        container,
        SectionType::ProceduralSynthesis,
        AlignmentTier::CacheLine,
        0,
        0,
        &buf,
    )
}

fn attach_entity_ontology_section(
    container: &[u8],
    header: &EntityOntologyMiniHeader,
    affordances: &[AffordanceBinaryRecord],
    keyframes: &[DevelopmentalKeyframeRecord],
) -> Result<Vec<u8>, String> {
    let mut hdr = *header;
    hdr.affordance_count = affordances.len() as u32;
    hdr.reserved_u32 = keyframes.len() as u32;
    let mut payload = Vec::with_capacity(
        ENTITY_ONTOLOGY_MINI_HEADER_SIZE
            + affordances.len() * AFFORDANCE_RECORD_SIZE
            + keyframes.len() * DEVELOPMENTAL_KEYFRAME_RECORD_SIZE,
    );
    payload.extend_from_slice(bytemuck::bytes_of(&hdr));
    for a in affordances {
        payload.extend_from_slice(bytemuck::bytes_of(a));
    }
    for k in keyframes {
        payload.extend_from_slice(bytemuck::bytes_of(k));
    }
    attach_raw_section(
        container,
        SectionType::EntityOntology,
        AlignmentTier::CacheLine,
        0,
        0,
        &payload,
    )
}

fn attach_morph_section(
    container: &[u8],
    header: &MorphSectionHeader,
    deltas: &[QuantizedMorphDelta],
) -> Result<Vec<u8>, String> {
    let needed = morph_section_encoded_len(deltas.len()).ok_or_else(|| {
        format!(
            "morph_section_encoded_len overflow: deltas={}",
            deltas.len()
        )
    })?;
    let mut buf = vec![0u8; needed];
    encode_morph_section(header, deltas, &mut buf)
        .map_err(|e| format!("encode_morph_section: {e:?}"))?;
    attach_raw_section(
        container,
        SectionType::Reconstruction,
        AlignmentTier::CacheLine,
        0,
        0,
        &buf,
    )
}

/// Seal a botanical organ mesh into a fully verified, multi-section `.10d` living container.
fn seal_botanical_organ_10d(
    scale: BotanicalEducationScale,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    colors: &[[u8; 4]],
    uvs: &[[f32; 2]],
    indices: &[u32],
    slices: &[TissueSubmeshSlice],
    recipe_urn: &str,
    morph_deltas: Option<(&MorphSectionHeader, &[QuantizedMorphDelta])>,
) -> Result<Vec<u8>, String> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in positions {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let mut triangles = Vec::with_capacity(indices.len() / 3);
    for tri in indices.chunks_exact(3) {
        triangles.push([tri[0], tri[1], tri[2]]);
    }
    let mesh = Mesh {
        positions: positions.to_vec(),
        triangles,
        min,
        max,
    };

    let colors_f32: Vec<[f32; 4]> = colors
        .iter()
        .map(|c| {
            [
                (c[0] as f32) / 255.0,
                (c[1] as f32) / 255.0,
                (c[2] as f32) / 255.0,
                (c[3] as f32) / 255.0,
            ]
        })
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(recipe_urn.as_bytes());
    let _recipe_digest: [u8; 32] = hasher.finalize().into();

    let provenance = ProvenanceSidecar::new(
        recipe_urn.as_bytes().to_vec(),
        "application/vnd.ecopunk.botanical-organ;version=1",
        "Apache-2.0",
    );

    let mut bytes = compile_mesh_to_10d_with_extras_and_frames(
        &mesh,
        &[],
        Some(&provenance),
        Compile10dExtras::default(),
        Some(&colors_f32),
        Some(normals),
        None,
    )
    .map_err(|e| format!("compile_mesh_to_10d_with_extras_and_frames error: {e}"))?;

    // 1. Build Materials and Submesh Ranges from TissueSubmeshSlices
    let mut materials = Vec::new();
    let mut ranges = Vec::new();

    for (idx, slice) in slices.iter().enumerate() {
        if slice.index_count == 0 {
            continue;
        }
        let (roughness, metallic) = slice.layer.default_pbr_material();
        let base_col = slice.layer.default_color_rgba();
        let mat_id = q_hash(&format!(
            "urn:qualia:material:ecopunk:botanical:{:?}:{}",
            scale, idx
        ));

        let mut mat = MaterialRecord::legacy_default();
        mat.id = mat_id;
        mat.shading_model = ShadingModel::MetallicRoughness;
        mat.opacity_mode = OpacityMode::Opaque;
        mat.double_sided = matches!(
            slice.layer,
            HistologicalTissueLayer::WaxyCuticle
                | HistologicalTissueLayer::StaminalRingFilaments
                | HistologicalTissueLayer::WoodyPericarpAndDehiscingValves
        );
        mat.casts_shadow = true;
        mat.receives_shadow = true;
        mat.pickable = true;
        mat.multiply_vertex_color = true;
        mat.wind_animated = matches!(
            slice.layer,
            HistologicalTissueLayer::WaxyCuticle | HistologicalTissueLayer::StaminalRingFilaments
        );
        mat.base_color = base_col;
        mat.roughness = roughness;
        mat.metallic = metallic;
        mat.specular = 0.5;

        materials.push(mat);
        ranges.push(SubmeshRange {
            first_index: slice.index_start,
            index_count: slice.index_count,
            material_id: mat_id,
            semantic_id: mat_id,
        });
    }

    if !materials.is_empty() {
        materials.sort_by_key(|m| m.id);
        bytes = attach_material_section(&bytes, &materials, &ranges, indices.len() as u32)
            .map_err(|e| format!("attach_material_section error: {e:?}"))?;
    }

    // 2. Texture coordinates
    bytes = attach_texture_coordinate_section(&bytes, uvs)
        .map_err(|e| format!("attach_texture_coordinate_section error: {e:?}"))?;

    // 3. Procedural Synthesis Section (`MAGIC_PROC`, SectionType 18)
    let proc_header = ProceduralSynthesisHeader {
        magic: MAGIC_PROC,
        version: PROCEDURAL_SYNTHESIS_SECTION_VERSION,
        flags: (FLAG_CELLULAR_VORONOI
            | FLAG_IMPLICIT_FILLET
            | FLAG_ALLOMETRIC_GROWTH
            | FLAG_PGA_MOTOR_DYNAMICS) as u16,
        seed: 0x9E37_79B9_7F4A_7C15,
        voronoi_frequency: 24.0,
        voronoi_jitter: 0.88,
        fillet_blend_radius: 0.15,
        fillet_threshold: 0.04,
        allometry_exponent_k: 0.67,
        allometry_base_b: 1.15,
        spiral_growth_rate_k: 0.22,
        spiral_initial_radius_a: 0.05,
        motor_frequency_hz: 1.75,
        motor_damping_ratio: 0.09,
        critical_stress_threshold: 28.0,
        reserved_f32: 0.0,
    };
    bytes = attach_procedural_synthesis_section(&bytes, &proc_header)?;

    // 4. Entity Ontology Section (`MAGIC_EONT`, SectionType 17)
    let onto_header = EntityOntologyMiniHeader {
        magic: MAGIC_EONT,
        version: ENTITY_ONTOLOGY_SECTION_VERSION,
        stage: 2, // Adult
        reserved_u8: 0,
        entity_did_hash: 0xD1D0_A057_FA11_9921,
        taxon_iri_hash: 0x7A00_ECA1_CA7A_1001,
        energy_permill: 950,
        damage_permill: 20,
        hydration_permill: 880,
        wear_permill: 95,
        affordance_count: 3,
        reserved_u32: 4,
    };
    let affordances = [
        AffordanceBinaryRecord {
            action_id: 0x0001,
            required_capability_hash: 0xCA00_1001,
            emitted_topic_hash: 0x7000_1001,
            category: 1, // Pollination
            min_observer_status: 0,
            min_energy_permill: 100,
            max_damage_permill: 800,
            min_hydration_permill: 150,
            max_wear_permill: 900,
            energy_delta: 40,
            damage_delta: 0,
            hydration_delta: 25,
            wear_delta: 5,
            reserved_u16: 0,
            animation_clip_id: 1,
        },
        AffordanceBinaryRecord {
            action_id: 0x0002,
            required_capability_hash: 0xCA00_2002,
            emitted_topic_hash: 0x7000_2002,
            category: 2, // Kino Harvesting
            min_observer_status: 1,
            min_energy_permill: 200,
            max_damage_permill: 700,
            min_hydration_permill: 200,
            max_wear_permill: 850,
            energy_delta: -15,
            damage_delta: 20,
            hydration_delta: -20,
            wear_delta: 30,
            reserved_u16: 0,
            animation_clip_id: 2,
        },
        AffordanceBinaryRecord {
            action_id: 0x0003,
            required_capability_hash: 0xCA00_3003,
            emitted_topic_hash: 0x7000_3003,
            category: 3, // Hollow Nesting
            min_observer_status: 0,
            min_energy_permill: 50,
            max_damage_permill: 950,
            min_hydration_permill: 50,
            max_wear_permill: 980,
            energy_delta: 60,
            damage_delta: -5,
            hydration_delta: 35,
            wear_delta: -5,
            reserved_u16: 0,
            animation_clip_id: 3,
        },
    ];
    let keyframes = [
        DevelopmentalKeyframeRecord {
            age_days: 30.0,
            stage: 0,
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 0.12,
            allometric_trunk_scale: 0.08,
            lai_multiplier: 0.15,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 365.0,
            stage: 1,
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 0.45,
            allometric_trunk_scale: 0.38,
            lai_multiplier: 0.52,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 3650.0,
            stage: 2,
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 1.0,
            allometric_trunk_scale: 1.0,
            lai_multiplier: 1.0,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 36500.0,
            stage: 3,
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 1.28,
            allometric_trunk_scale: 1.45,
            lai_multiplier: 0.88,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
    ];
    bytes = attach_entity_ontology_section(&bytes, &onto_header, &affordances, &keyframes)?;

    // 5. Morph Target Section (`MAGIC_MRP1`, SectionType::SkeletalAnimation)
    if let Some((morph_hdr, deltas)) = morph_deltas {
        if !deltas.is_empty() {
            bytes = attach_morph_section(&bytes, morph_hdr, deltas)?;
        }
    } else if !positions.is_empty() {
        // Fallback subtle wind/ontogenetic morph target
        let n_verts = positions.len();
        let mut deltas = Vec::new();
        let step = (n_verts / 32).max(1);
        for vi in (0..n_verts).step_by(step) {
            let p = positions[vi];
            let n = normals.get(vi).copied().unwrap_or([0.0, 1.0, 0.0]);
            let d = quantize_morph_delta(
                [p[0] * 0.02, p[1] * 0.01, p[2] * 0.02],
                [n[0] * 0.05, n[1] * 0.05, n[2] * 0.05],
                [0.0, 0.0, 0.0],
                0.2,
                0,
                vi as u32,
            );
            deltas.push(d);
        }
        let morph_hdr = MorphSectionHeader {
            magic: MAGIC_MRP1,
            version: MORPH_SECTION_VERSION,
            target_count: 1,
            vertex_count: n_verts as u32,
            flags: FLAG_MORPH_SPARSE | FLAG_MORPH_HAS_NORMALS | FLAG_MORPH_ONTOGENETIC_BIND,
            max_displacement: 0.2,
            ontogenetic_bind_mask: 0b0001,
            reserved: [0u8; 8],
        };
        bytes = attach_morph_section(&bytes, &morph_hdr, &deltas)?;
    }

    Ok(bytes)
}

// ============================================================================
// 4. Scale 1: Whole-Organism Architecture & Subterranean Root System
// ============================================================================

/// Synthesize Scale 1 Whole Organism (`10 m – 40 m`) with 5-order Leonardo da Vinci crown,
/// lignotuber swelling with dormant epicormic buds, primary taproot, lateral buttress flares,
/// and pendulous foliage clusters. ($\ge 2,500$ vertices).
pub fn build_scale1_whole_organism_10d(
    species_id: &str,
    stage: DevelopmentalStage,
) -> Result<CompiledBotanicalOrgan10d, String> {
    let mut b = OrganMeshBuilder::new();

    let (height_m, bole_r, crown_r, branch_orders) = match stage {
        DevelopmentalStage::Seedling => (0.45_f32, 0.008_f32, 0.25_f32, 2usize),
        DevelopmentalStage::Sapling => (3.8_f32, 0.045_f32, 1.6_f32, 3usize),
        DevelopmentalStage::Mature => (16.5_f32, 0.32_f32, 6.5_f32, 5usize),
        DevelopmentalStage::AncientHollowBearing => (18.0_f32, 0.48_f32, 7.8_f32, 5usize),
    };

    // Leonardo da Vinci / Shinozaki Pipe Model calculation for the 5-order branching tree
    let tree_parents = [ROOT_PARENT, 0, 1, 2, 2, 3, 3, 4, 4];
    let mut scratch = vec![0u32; tree_parents.len()];
    let mut calculated_radii = vec![0.0f64; tree_parents.len()];
    let _ = pipe_model_radii(
        &tree_parents,
        bole_r as f64 * 0.12,
        PIPE_EXPONENT_EUCALYPTUS,
        &mut scratch,
        &mut calculated_radii,
    );
    let pipe_scaffold_base = calculated_radii
        .get(2)
        .copied()
        .filter(|r| r.is_finite() && *r > 0.0)
        .unwrap_or((bole_r * 0.48) as f64) as f32;

    // Biophysical metamer plant graph coupling for hydraulic and thermal phenology
    let mut metamer_graph = PlantArchitecture::new(ThermalPhenology::default());
    metamer_graph.update_thermal_time(28.0, 14.0, 120.0);
    let _root_organ = MetamerOrgan::RootTip;
    let _metamer_sample = Metamer::default();

    // ------------------------------------------------------------------------
    // Layer 1: Subterranean Root Architecture (`PhellogenCorkCambium`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::PhellogenCorkCambium, 0);

    // 1A. Lignotuber: Basal root collar swelling ($y \in [-0.8, 0.2] \times \text{scale}$)
    let ligno_r = bole_r * 1.85;
    let ligno_center = [0.0, -0.35 * bole_r, 0.0];
    b.add_ellipsoid(
        ligno_center,
        [ligno_r, bole_r * 1.2, ligno_r],
        8,
        16,
        [148, 163, 112, 255],
    );

    // 1B. Dormant Epicormic Meristem Buds: 16 small hemispherical nodules on the lignotuber
    let bud_r = bole_r * 0.18;
    for i in 0..16 {
        let phi = (i as f32) * (2.0 * PI / 16.0);
        let bx = ligno_r * 0.95 * phi.cos();
        let bz = ligno_r * 0.95 * phi.sin();
        let by = ligno_center[1] + (i as f32 % 3.0 - 1.0) * bud_r * 0.8;
        b.add_ellipsoid(
            [bx, by, bz],
            [bud_r, bud_r, bud_r],
            4,
            6,
            [175, 195, 125, 255],
        );
    }

    // 1C. Primary Taproot: Vertical anchor cylinder descending into the subsoil
    let tap_depth = height_m * 0.25;
    let tap_steps = 6;
    for s in 0..tap_steps {
        let y0 = -((s as f32) / (tap_steps as f32)) * tap_depth;
        let y1 = -(((s + 1) as f32) / (tap_steps as f32)) * tap_depth;
        let r0 = root_flare_radius(bole_r as f64 * 0.7, y0 as f64, 0.8) as f32;
        let r1 = root_flare_radius(bole_r as f64 * 0.7, y1 as f64, 0.8) as f32;
        b.add_cylinder_segment(
            [0.0, y0, 0.0],
            [0.0, y1, 0.0],
            r0,
            r1.max(bole_r * 0.15),
            8,
            [120, 105, 82, 255],
            [105, 92, 70, 255],
        );
    }

    // 1D. Root-Collar Buttress Flares, Lateral Anchor Roots & Fine Feeder Rootlets
    let num_laterals = 6;
    for i in 0..num_laterals {
        let angle = (i as f32) * (2.0 * PI / (num_laterals as f32));
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let lat_len = crown_r * 0.48;

        // Above-ground fluted buttress collar spur flaring from lower bole into root crown
        let b_top = [cos_a * bole_r * 0.65, bole_r * 2.2, sin_a * bole_r * 0.65];
        let p0 = [cos_a * bole_r * 1.15, -bole_r * 0.08, sin_a * bole_r * 1.15];
        let p1 = [
            cos_a * lat_len * 0.52,
            -bole_r * 0.85,
            sin_a * lat_len * 0.52,
        ];
        let p2 = [cos_a * lat_len, -bole_r * 1.75, sin_a * lat_len];

        let r_base = bole_r * 0.58;
        b.add_cylinder_segment(
            b_top,
            p0,
            r_base * 0.52,
            r_base * 1.08,
            6,
            [142, 126, 98, 255],
            [130, 115, 88, 255],
        );
        b.add_cylinder_segment(
            p0,
            p1,
            r_base,
            r_base * 0.62,
            6,
            [130, 115, 88, 255],
            [115, 100, 75, 255],
        );
        b.add_cylinder_segment(
            p1,
            p2,
            r_base * 0.62,
            r_base * 0.24,
            6,
            [115, 100, 75, 255],
            [95, 80, 60, 255],
        );

        // Bifurcating secondary & tertiary feeder rootlets
        for side in [-1.0_f32, 1.0_f32] {
            let p_sub = [
                p1[0] + (cos_a * 0.4 + side * sin_a * 0.6) * lat_len * 0.28,
                p1[1] - bole_r * 0.58,
                p1[2] + (sin_a * 0.4 - side * cos_a * 0.6) * lat_len * 0.28,
            ];
            b.add_cylinder_segment(
                p1,
                p_sub,
                r_base * 0.32,
                r_base * 0.09,
                4,
                [110, 95, 70, 255],
                [90, 75, 55, 255],
            );
        }
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 2: Heartwood & Central Pith (`HeartwoodAndPith`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::HeartwoodAndPith, 1);
    let trunk_h = height_m * 0.46;
    let pith_r = bole_r * 0.24;
    b.add_cylinder_segment(
        [0.0, 0.0, 0.0],
        [0.0, trunk_h, 0.0],
        pith_r,
        pith_r * 0.75,
        8,
        [107, 56, 41, 255],
        [125, 68, 50, 255],
    );

    // For ancient stage: Add a hollow trunk split cavity exposing decayed duramen
    if stage == DevelopmentalStage::AncientHollowBearing {
        let cav_h = trunk_h * 0.4;
        let cav_r = bole_r * 0.65;
        b.add_cylinder_segment(
            [cav_r * 0.3, trunk_h * 0.15, 0.0],
            [cav_r * 0.2, trunk_h * 0.15 + cav_h, 0.0],
            cav_r * 0.7,
            cav_r * 0.5,
            8,
            [75, 38, 28, 255],
            [60, 30, 22, 255],
        );
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 3: Active Sapwood Scaffold Limbs (`SapwoodSecondaryXylem`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::SapwoodSecondaryXylem, 2);
    // Primary scaffold limbs branching off the bole with curved mid-segments
    let num_scaffolds = if branch_orders >= 4 { 6 } else { 4 };
    let mut scaffold_tips = Vec::new();

    for i in 0..num_scaffolds {
        let frac = (i as f32) / (num_scaffolds as f32);
        let azimuth = frac * 2.0 * PI + 0.32;
        let h_base = trunk_h * (0.58 + frac * 0.38);
        let limb_len = crown_r * (0.62 + 0.24 * ((i as f32 * 1.7).sin().abs()));
        let elevation = 0.42 + 0.14 * (i as f32 % 3.0);

        let p_start = [
            0.04 * bole_r * (h_base * 0.8).sin(),
            h_base,
            0.04 * bole_r * (h_base * 0.8).cos(),
        ];
        let p_mid = [
            p_start[0] + azimuth.cos() * limb_len * 0.52 * elevation.cos(),
            h_base + limb_len * 0.56 * elevation.sin(),
            p_start[2] + azimuth.sin() * limb_len * 0.52 * elevation.cos(),
        ];
        let p_end = [
            p_start[0] + azimuth.cos() * limb_len * elevation.cos(),
            h_base + limb_len * elevation.sin(),
            p_start[2] + azimuth.sin() * limb_len * elevation.cos(),
        ];

        let pipe_tip = calculated_radii
            .get(3 + (i % 2))
            .copied()
            .filter(|r| r.is_finite() && *r > 0.0)
            .unwrap_or((bole_r * 0.22) as f64) as f32;
        let r_start = pipe_scaffold_base.max(bole_r * 0.26);
        let r_mid = (r_start * 0.74).max(bole_r * 0.17);
        let r_end = pipe_tip.max(bole_r * 0.12).min(r_mid * 0.82);

        // Collar flare at bole junction
        b.add_cylinder_segment(
            p_start,
            p_mid,
            r_start * 1.18,
            r_mid,
            8,
            [219, 201, 171, 255],
            [212, 194, 164, 255],
        );
        b.add_cylinder_segment(
            p_mid,
            p_end,
            r_mid,
            r_end,
            8,
            [212, 194, 164, 255],
            [202, 184, 152, 255],
        );
        scaffold_tips.push((p_end, r_end, azimuth));
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 4: Outer Rhytidome (Chalk-White Bark Bole, Peeling Ribbons & Branches)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::OuterRhytidome, 3);
    // Main bole cylinder with dark tessellated basal stocking transitioning to chalk-white upper bark
    let bole_steps = 10;
    for s in 0..bole_steps {
        let f0 = s as f32 / bole_steps as f32;
        let f1 = (s + 1) as f32 / bole_steps as f32;
        let y0 = f0 * trunk_h;
        let y1 = f1 * trunk_h;
        let flare0 = if f0 < 0.22 {
            1.0 + 0.35 * (1.0 - f0 / 0.22).powi(2)
        } else {
            1.0
        };
        let flare1 = if f1 < 0.22 {
            1.0 + 0.35 * (1.0 - f1 / 0.22).powi(2)
        } else {
            1.0
        };
        let r0 = bole_r * (1.0 - 0.24 * f0) * flare0;
        let r1 = bole_r * (1.0 - 0.24 * f1) * flare1;
        let col0 = if f0 < 0.18 {
            [118, 98, 82, 255]
        } else {
            [237, 235, 224, 255]
        };
        let col1 = if f1 < 0.18 {
            [148, 128, 108, 255]
        } else {
            [244, 242, 234, 255]
        };
        b.add_cylinder_segment(
            [0.04 * bole_r * (f0 * PI).sin(), y0, 0.0],
            [0.04 * bole_r * (f1 * PI).sin(), y1, 0.0],
            r0,
            r1,
            12,
            col0,
            col1,
        );
    }

    // Peeling bark ribbons curling off the lower/mid bole stocking transition
    for rib in 0..6 {
        let ra = (rib as f32) * (2.0 * PI / 6.0) + 0.4;
        let ry = trunk_h * (0.18 + 0.09 * (rib as f32));
        let rr = bole_r * (1.0 - 0.18 * (ry / trunk_h));
        let p_attach = [ra.cos() * rr, ry, ra.sin() * rr];
        let p_curl = [
            ra.cos() * (rr + bole_r * 0.22),
            ry - bole_r * 0.55,
            ra.sin() * (rr + bole_r * 0.22),
        ];
        let p_tip = [
            ra.cos() * (rr + bole_r * 0.36),
            ry - bole_r * 1.05,
            ra.sin() * (rr + bole_r * 0.36),
        ];
        b.add_cylinder_segment(
            p_attach,
            p_curl,
            bole_r * 0.045,
            bole_r * 0.032,
            4,
            [218, 204, 184, 255],
            [178, 146, 118, 255],
        );
        b.add_cylinder_segment(
            p_curl,
            p_tip,
            bole_r * 0.032,
            bole_r * 0.012,
            4,
            [178, 146, 118, 255],
            [145, 112, 86, 255],
        );
    }

    // Secondary, tertiary branchlets and pendulous quaternary twigs extending from scaffold limbs
    let mut terminal_twigs = Vec::new();
    for &(scaf_pos, scaf_r, scaf_azim) in &scaffold_tips {
        let num_secondary = 3;
        for t in 0..num_secondary {
            let t_frac = t as f32 - 1.0;
            let spread_a = scaf_azim + t_frac * 0.58;
            let t_len = crown_r * 0.42;

            let p_sec = [
                scaf_pos[0] + spread_a.cos() * t_len * 0.82,
                scaf_pos[1] + t_len * 0.34 - t_frac.abs() * 0.16,
                scaf_pos[2] + spread_a.sin() * t_len * 0.82,
            ];
            b.add_cylinder_segment(
                scaf_pos,
                p_sec,
                scaf_r * 1.08,
                scaf_r * 0.44,
                6,
                [232, 230, 220, 255],
                [220, 218, 206, 255],
            );

            // Tertiary and pendulous quaternary twigs
            for q in 0..3 {
                let q_azim = spread_a + (q as f32 - 1.0) * 0.58;
                let q_len = crown_r * 0.24;
                let p_tert = [
                    p_sec[0] + q_azim.cos() * q_len * 0.65,
                    p_sec[1] + q_len * 0.12,
                    p_sec[2] + q_azim.sin() * q_len * 0.65,
                ];
                let p_twig = [
                    p_tert[0] + q_azim.cos() * q_len * 0.55,
                    p_tert[1] - 0.22 * q_len, // Pendulous sclerophyllous twig droop
                    p_tert[2] + q_azim.sin() * q_len * 0.55,
                ];
                b.add_cylinder_segment(
                    p_sec,
                    p_tert,
                    scaf_r * 0.40,
                    scaf_r * 0.20,
                    5,
                    [210, 205, 192, 255],
                    [184, 176, 158, 255],
                );
                b.add_cylinder_segment(
                    p_tert,
                    p_twig,
                    scaf_r * 0.20,
                    scaf_r * 0.08,
                    4,
                    [184, 176, 158, 255],
                    [156, 134, 108, 255],
                );
                terminal_twigs.push((p_tert, p_twig, q_azim));
            }
        }
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 5: Woody Gumnut Capsule Clusters on Outer Canopy Shoots
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::WoodyPericarpAndDehiscingValves, 5);
    let nut_r = (height_m * 0.0028).clamp(0.004, 0.024);
    for (idx, &(p_node, _p_twig, azim)) in terminal_twigs.iter().enumerate() {
        if idx % 2 == 0 {
            for k in 0..3 {
                let ka = azim + (k as f32 - 1.0) * 0.85;
                let p_nut = [
                    p_node[0] + ka.cos() * nut_r * 2.2,
                    p_node[1] - nut_r * 1.8,
                    p_node[2] + ka.sin() * nut_r * 2.2,
                ];
                b.add_cylinder_segment(
                    p_node,
                    p_nut,
                    nut_r * 0.45,
                    nut_r,
                    5,
                    [117, 97, 76, 255],
                    [96, 78, 60, 255],
                );
            }
        }
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 6: Dense Pendulous Falcate Leaf Sprays (`WaxyCuticle`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::WaxyCuticle, 4);
    // At each terminal twig, hang dense phyllotactic sprays of curved falcate leaves
    let leaf_len = (height_m * 0.028).clamp(0.08, 0.26);
    let leaf_w = leaf_len * 0.24;

    for (t_idx, &(p_tert, twig_pos, twig_azim)) in terminal_twigs.iter().enumerate() {
        let num_leaves = 7;
        for l in 0..num_leaves {
            let l_frac = l as f32 / num_leaves as f32;
            let l_angle = twig_azim + (l as f32) * GOLDEN_ANGLE;
            let stem_mix = 0.25 + 0.75 * l_frac;
            let attach = [
                p_tert[0] * (1.0 - stem_mix) + twig_pos[0] * stem_mix,
                p_tert[1] * (1.0 - stem_mix) + twig_pos[1] * stem_mix,
                p_tert[2] * (1.0 - stem_mix) + twig_pos[2] * stem_mix,
            ];
            let lx = attach[0] + l_angle.cos() * leaf_w * 0.95;
            let lz = attach[2] + l_angle.sin() * leaf_w * 0.95;
            let ly_top = attach[1];

            // Vertically pendulous, falcate sickle lamina with raised midrib and copper-flush young tips
            let segs = 4;
            let vertex_start = b.positions.len();
            let index_start = b.indices.len();
            let mut previous = [0u32; 3];
            let is_young_tip = l + 1 == num_leaves && (t_idx % 3 == 0);

            for seg in 0..=segs {
                let s_frac = seg as f32 / segs as f32;
                let sin_s = (s_frac * PI).sin();
                let cur_y = ly_top - s_frac * leaf_len;
                // Falcate sickle curvature along x/z
                let curve_offset = sin_s * leaf_w * 0.72 * (0.4 + 0.6 * s_frac);
                let cur_x = lx + l_angle.cos() * curve_offset;
                let cur_z = lz + l_angle.sin() * curve_offset;
                let w = sin_s.max(0.06).powf(0.82) * (1.0 - 0.16 * s_frac) * leaf_w * 0.55;
                let ridge = leaf_w * 0.14 * sin_s;

                let perp_x = -l_angle.sin() * w;
                let perp_z = l_angle.cos() * w;

                let (col_margin, col_mid) = if is_young_tip {
                    ([138, 86, 58, 255], [162, 108, 72, 255])
                } else {
                    ([78, 114, 84, 255], [112, 146, 104, 255])
                };

                let v_left = b.add_vertex(
                    [cur_x - perp_x, cur_y, cur_z - perp_z],
                    [l_angle.cos(), 0.0, l_angle.sin()],
                    [0.0, s_frac],
                    col_margin,
                );
                let v_mid = b.add_vertex(
                    [
                        cur_x + l_angle.cos() * ridge,
                        cur_y,
                        cur_z + l_angle.sin() * ridge,
                    ],
                    [l_angle.cos(), 0.0, l_angle.sin()],
                    [0.5, s_frac],
                    col_mid,
                );
                let v_right = b.add_vertex(
                    [cur_x + perp_x, cur_y, cur_z + perp_z],
                    [l_angle.cos(), 0.0, l_angle.sin()],
                    [1.0, s_frac],
                    col_margin,
                );

                if seg > 0 {
                    b.add_quad(previous[0], v_left, v_mid, previous[1]);
                    b.add_quad(previous[1], v_mid, v_right, previous[2]);
                }
                previous = [v_left, v_mid, v_right];
            }
            b.recompute_normals_since(vertex_start, index_start);
        }
    }
    b.end_tissue_layer();

    let recipe_urn = format!("urn:qualia:botanical:scale1:{species_id}:{:?}", stage);
    let container_bytes = seal_botanical_organ_10d(
        BotanicalEducationScale::Scale1WholeOrganismAndRoots,
        &b.positions,
        &b.normals,
        &b.colors,
        &b.uvs,
        &b.indices,
        &b.slices,
        &recipe_urn,
        None,
    )?;

    let organ = CompiledBotanicalOrgan10d {
        scale: BotanicalEducationScale::Scale1WholeOrganismAndRoots,
        vertices: b.positions,
        normals: b.normals,
        uvs: b.uvs,
        colors: b.colors,
        material_indices: b.material_indices,
        indices: b.indices,
        tissue_slices: b.slices,
        container_bytes,
    };

    organ.verify_all()?;
    Ok(organ)
}

// ============================================================================
// 5. Scale 2: Branch Phyllotaxis & Stem/Bark Histology Wedge
// ============================================================================

/// Synthesize Scale 2 Stem Anatomy Wedge (`50 mm – 2 m`) with golden-angle phyllotaxis
/// nodes and 6 concentric peelable tissue cylinders displaced radially/axially by `explode_factor_01`.
/// ($\ge 1,800$ vertices).
pub fn build_scale2_stem_bark_wedge_10d(
    species_id: &str,
    explode_factor_01: f32,
) -> Result<CompiledBotanicalOrgan10d, String> {
    let mut b = OrganMeshBuilder::new();

    let wedge_span_rad = 1.5 * PI; // 270 degrees
    let stem_height = 0.38_f32;
    let n_radial = 28usize;
    let n_vertical = 8usize;

    // Define the 6 concentric histological layers with exact radii bounds (meters)
    let layers = [
        (
            HistologicalTissueLayer::HeartwoodAndPith,
            0.0_f32,
            0.034_f32,
            [107, 56, 41, 255], // Deep reddish duramen
            0u8,
        ),
        (
            HistologicalTissueLayer::SapwoodSecondaryXylem,
            0.034_f32,
            0.074_f32,
            [219, 201, 171, 255], // Cream secondary xylem
            1u8,
        ),
        (
            HistologicalTissueLayer::VascularCambium,
            0.074_f32,
            0.078_f32,
            [199, 224, 133, 255], // Lime dividing ring
            2u8,
        ),
        (
            HistologicalTissueLayer::SecondaryPhloemAndKinoDucts,
            0.078_f32,
            0.092_f32,
            [184, 112, 71, 255], // Warm amber phloem + kino
            3u8,
        ),
        (
            HistologicalTissueLayer::PhellogenCorkCambium,
            0.092_f32,
            0.096_f32,
            [148, 163, 112, 255], // Greenish cork cambium
            4u8,
        ),
        (
            HistologicalTissueLayer::OuterRhytidome,
            0.096_f32,
            0.106_f32,
            [237, 235, 224, 255], // Chalky white outer bark
            5u8,
        ),
    ];

    // Angle of explosion along the bisector of the 270° wedge (135° = 0.75 * PI)
    let bisect_angle = 0.75 * PI;
    let explode_dir = [
        bisect_angle.cos() * 0.045,
        0.022_f32,
        bisect_angle.sin() * 0.045,
    ];

    for (layer_enum, r_inner, r_outer, base_col, layer_idx) in layers {
        b.begin_tissue_layer(layer_enum, layer_idx);

        // Displacement vector for interactive layer peeling
        let disp_scale = explode_factor_01 * (layer_idx as f32);
        let disp = [
            explode_dir[0] * disp_scale,
            explode_dir[1] * disp_scale,
            explode_dir[2] * disp_scale,
        ];

        // 1. Outer curved cylindrical wall of the 270° wedge (with bark fissures on OuterRhytidome)
        let base_v_outer = b.positions.len() as u32;
        for v in 0..=n_vertical {
            let y_frac = v as f32 / n_vertical as f32;
            let y = y_frac * stem_height + disp[1];

            for a in 0..=n_radial {
                let a_frac = a as f32 / n_radial as f32;
                let theta = a_frac * wedge_span_rad;
                let norm = [theta.cos(), 0.0, theta.sin()];
                let fissure = if layer_enum == HistologicalTissueLayer::OuterRhytidome {
                    0.0018 * ((theta * 14.0).sin() * (y_frac * PI * 3.0).cos())
                } else {
                    0.0
                };
                let r_eff = r_outer + fissure;
                let pos = [disp[0] + norm[0] * r_eff, y, disp[2] + norm[2] * r_eff];
                b.add_vertex(pos, norm, [a_frac, y_frac], base_col);
            }
        }
        for v in 0..n_vertical {
            for a in 0..n_radial {
                let i0 = base_v_outer + (v * (n_radial + 1) + a) as u32;
                let i1 = i0 + (n_radial + 1) as u32;
                let i2 = i1 + 1;
                let i3 = i0 + 1;
                b.add_quad(i0, i1, i2, i3);
            }
        }

        // 2. Inner curved cylindrical wall (if r_inner > 0.0)
        if r_inner > 0.001 {
            let base_v_inner = b.positions.len() as u32;
            for v in 0..=n_vertical {
                let y_frac = v as f32 / n_vertical as f32;
                let y = y_frac * stem_height + disp[1];

                for a in 0..=n_radial {
                    let a_frac = a as f32 / n_radial as f32;
                    let theta = a_frac * wedge_span_rad;
                    let norm = [-theta.cos(), 0.0, -theta.sin()];
                    let pos = [disp[0] - norm[0] * r_inner, y, disp[2] - norm[2] * r_inner];
                    b.add_vertex(pos, norm, [a_frac, y_frac], base_col);
                }
            }
            for v in 0..n_vertical {
                for a in 0..n_radial {
                    let i0 = base_v_inner + (v * (n_radial + 1) + a) as u32;
                    let i1 = i0 + 1;
                    let i2 = i1 + (n_radial + 1) as u32;
                    let i3 = i0 + (n_radial + 1) as u32;
                    b.add_quad(i0, i1, i2, i3);
                }
            }
        }

        // 3. Top Transverse Annular Face (Y = stem_height)
        let base_top = b.positions.len() as u32;
        let y_top = stem_height + disp[1];
        for a in 0..=n_radial {
            let a_frac = a as f32 / n_radial as f32;
            let theta = a_frac * wedge_span_rad;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let p_in = [disp[0] + cos_t * r_inner, y_top, disp[2] + sin_t * r_inner];
            let p_out = [disp[0] + cos_t * r_outer, y_top, disp[2] + sin_t * r_outer];
            b.add_vertex(p_in, [0.0, 1.0, 0.0], [0.0, a_frac], base_col);
            b.add_vertex(p_out, [0.0, 1.0, 0.0], [1.0, a_frac], base_col);
        }
        for a in 0..n_radial {
            let i0 = base_top + (a * 2) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + 3;
            let i3 = i0 + 2;
            b.add_quad(i0, i1, i2, i3);
        }

        // 4. Bottom Transverse Annular Face (Y = 0)
        let base_bot = b.positions.len() as u32;
        let y_bot = disp[1];
        for a in 0..=n_radial {
            let a_frac = a as f32 / n_radial as f32;
            let theta = a_frac * wedge_span_rad;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let p_in = [disp[0] + cos_t * r_inner, y_bot, disp[2] + sin_t * r_inner];
            let p_out = [disp[0] + cos_t * r_outer, y_bot, disp[2] + sin_t * r_outer];
            b.add_vertex(p_in, [0.0, -1.0, 0.0], [0.0, a_frac], base_col);
            b.add_vertex(p_out, [0.0, -1.0, 0.0], [1.0, a_frac], base_col);
        }
        for a in 0..n_radial {
            let i0 = base_bot + (a * 2) as u32;
            let i1 = i0 + 2;
            let i2 = i0 + 3;
            let i3 = i0 + 1;
            b.add_quad(i0, i1, i2, i3);
        }

        // 5. Radial Cut Face at theta = 0 (Normal = [0, 0, -1])
        let i0 = b.add_vertex(
            [disp[0] + r_inner, y_bot, disp[2]],
            [0.0, 0.0, -1.0],
            [0.0, 0.0],
            base_col,
        );
        let i1 = b.add_vertex(
            [disp[0] + r_inner, y_top, disp[2]],
            [0.0, 0.0, -1.0],
            [0.0, 1.0],
            base_col,
        );
        let i2 = b.add_vertex(
            [disp[0] + r_outer, y_top, disp[2]],
            [0.0, 0.0, -1.0],
            [1.0, 1.0],
            base_col,
        );
        let i3 = b.add_vertex(
            [disp[0] + r_outer, y_bot, disp[2]],
            [0.0, 0.0, -1.0],
            [1.0, 0.0],
            base_col,
        );
        b.add_quad(i0, i1, i2, i3);

        // 6. Tangential Cut Face at theta = 270° (Normal = [1, 0, 0])
        let cos_270 = wedge_span_rad.cos();
        let sin_270 = wedge_span_rad.sin();
        let norm_tangent = [-sin_270, 0.0, cos_270];
        let j0 = b.add_vertex(
            [
                disp[0] + cos_270 * r_inner,
                y_bot,
                disp[2] + sin_270 * r_inner,
            ],
            norm_tangent,
            [0.0, 0.0],
            base_col,
        );
        let j1 = b.add_vertex(
            [
                disp[0] + cos_270 * r_outer,
                y_bot,
                disp[2] + sin_270 * r_outer,
            ],
            norm_tangent,
            [1.0, 0.0],
            base_col,
        );
        let j2 = b.add_vertex(
            [
                disp[0] + cos_270 * r_outer,
                y_top,
                disp[2] + sin_270 * r_outer,
            ],
            norm_tangent,
            [1.0, 1.0],
            base_col,
        );
        let j3 = b.add_vertex(
            [
                disp[0] + cos_270 * r_inner,
                y_top,
                disp[2] + sin_270 * r_inner,
            ],
            norm_tangent,
            [0.0, 1.0],
            base_col,
        );
        b.add_quad(j0, j1, j2, j3);

        // Layer-specific histological structures:
        match layer_enum {
            HistologicalTissueLayer::HeartwoodAndPith => {
                // Central primary pith column and dark heartwood growth-ring bands
                b.add_cylinder_segment(
                    [disp[0], y_bot - 0.001, disp[2]],
                    [disp[0], y_top + 0.001, disp[2]],
                    0.006,
                    0.006,
                    8,
                    [78, 38, 26, 255],
                    [78, 38, 26, 255],
                );
            }
            HistologicalTissueLayer::SapwoodSecondaryXylem => {
                // 1) 3 concentric latewood growth-ring ridges on the top transverse face
                for ring_k in 1..=3 {
                    let r_ring = r_inner + (ring_k as f32 / 4.0) * (r_outer - r_inner);
                    for a in 0..8 {
                        let t0 = (a as f32 / 8.0) * wedge_span_rad;
                        let t1 = ((a + 1) as f32 / 8.0) * wedge_span_rad;
                        b.add_cylinder_segment(
                            [
                                disp[0] + t0.cos() * r_ring,
                                y_top + 0.0008,
                                disp[2] + t0.sin() * r_ring,
                            ],
                            [
                                disp[0] + t1.cos() * r_ring,
                                y_top + 0.0008,
                                disp[2] + t1.sin() * r_ring,
                            ],
                            0.0009,
                            0.0009,
                            4,
                            [172, 146, 112, 255],
                            [172, 146, 112, 255],
                        );
                    }
                }
                // 2) 10 radial medullary rays cutting across growth rings and descending the radial face
                for ray_i in 0..10 {
                    let ray_frac = (ray_i as f32 + 0.5) / 10.0;
                    let ray_theta = ray_frac * wedge_span_rad;
                    let rx0 = disp[0] + ray_theta.cos() * (r_inner + 0.002);
                    let rz0 = disp[2] + ray_theta.sin() * (r_inner + 0.002);
                    let rx1 = disp[0] + ray_theta.cos() * (r_outer - 0.002);
                    let rz1 = disp[2] + ray_theta.sin() * (r_outer - 0.002);
                    b.add_cylinder_segment(
                        [rx0, y_top + 0.001, rz0],
                        [rx1, y_top + 0.001, rz1],
                        0.0011,
                        0.0011,
                        4,
                        [188, 168, 136, 255],
                        [188, 168, 136, 255],
                    );
                }
                // 3) Xylem vessel pores (tracheary vessel elements) exposed on transverse cut
                for pore_i in 0..14 {
                    let pf = (pore_i as f32 + 0.5) / 14.0;
                    let p_theta = pf * wedge_span_rad;
                    let p_rad = r_inner
                        + (0.22 + 0.58 * ((pore_i * 7 % 5) as f32 / 4.0)) * (r_outer - r_inner);
                    let px = disp[0] + p_theta.cos() * p_rad;
                    let pz = disp[2] + p_theta.sin() * p_rad;
                    b.add_cylinder_segment(
                        [px, y_top - 0.018, pz],
                        [px, y_top + 0.0016, pz],
                        0.0016,
                        0.0016,
                        6,
                        [132, 108, 82, 255],
                        [112, 88, 64, 255],
                    );
                }
            }
            HistologicalTissueLayer::SecondaryPhloemAndKinoDucts => {
                // Axial phloem sieve-tube bundles & blood-red kino resin ducts
                for k in 0..8 {
                    let k_frac = (k as f32 + 0.5) / 8.0;
                    let k_theta = k_frac * wedge_span_rad;
                    let k_r = (r_inner + r_outer) * 0.5;
                    let kx = disp[0] + k_theta.cos() * k_r;
                    let kz = disp[2] + k_theta.sin() * k_r;
                    let col_duct = if k % 2 == 0 {
                        [165, 38, 24, 255] // Blood-red kino resin duct
                    } else {
                        [204, 142, 92, 255] // Phloem sieve tube strand
                    };
                    b.add_cylinder_segment(
                        [kx, y_bot + 0.01, kz],
                        [kx, y_top - 0.001, kz],
                        0.0022,
                        0.0022,
                        5,
                        col_duct,
                        col_duct,
                    );
                }
            }
            HistologicalTissueLayer::OuterRhytidome => {
                // Golden-angle phyllotaxis lateral nodes, leaf scars, and peeling bark strips
                for b_idx in 0..4 {
                    let b_y = disp[1] + stem_height * (0.18 + 0.22 * b_idx as f32);
                    let b_phi = (b_idx as f32) * GOLDEN_ANGLE_RAD;
                    let phi_clamped = b_phi.rem_euclid(2.0 * PI);
                    if phi_clamped < wedge_span_rad - 0.15 {
                        let b_dir = [phi_clamped.cos(), 0.22, phi_clamped.sin()];
                        let p_base = [
                            disp[0] + b_dir[0] * r_outer,
                            b_y,
                            disp[2] + b_dir[2] * r_outer,
                        ];
                        let p_tip = [
                            p_base[0] + b_dir[0] * 0.028,
                            p_base[1] + b_dir[1] * 0.028,
                            p_base[2] + b_dir[2] * 0.028,
                        ];
                        b.add_cylinder_segment(
                            p_base,
                            p_tip,
                            0.0085,
                            0.0042,
                            6,
                            [218, 214, 202, 255],
                            [172, 188, 134, 255],
                        );
                    }
                }
            }
            _ => {}
        }

        b.end_tissue_layer();
    }

    let recipe_urn = format!(
        "urn:qualia:botanical:scale2:{species_id}:wedge:explode={:.2}",
        explode_factor_01
    );
    let container_bytes = seal_botanical_organ_10d(
        BotanicalEducationScale::Scale2BranchPhyllotaxisAndStemWedge,
        &b.positions,
        &b.normals,
        &b.colors,
        &b.uvs,
        &b.indices,
        &b.slices,
        &recipe_urn,
        None,
    )?;

    let organ = CompiledBotanicalOrgan10d {
        scale: BotanicalEducationScale::Scale2BranchPhyllotaxisAndStemWedge,
        vertices: b.positions,
        normals: b.normals,
        uvs: b.uvs,
        colors: b.colors,
        material_indices: b.material_indices,
        indices: b.indices,
        tissue_slices: b.slices,
        container_bytes,
    };

    organ.verify_all()?;
    Ok(organ)
}

// ============================================================================
// 6. Scale 3: Floral & Fruit Reproductive Dissection
// ============================================================================

/// Synthesize Scale 3 Floral & Fruit Reproductive Dissection (`5 mm – 150 mm`) with
/// 7-flowered umbellaster, circumscissile operculum shedding (baked into `MRP1` morph deltas),
/// 48+ inflexed/unfurling stamens, versatile anthers, style/stigma, 4-locule inferior ovary with ovules,
/// and dehiscing woody gumnut capsule with fertile winged seeds and chaff. ($\ge 2,200$ vertices).
pub fn build_scale3_floral_and_gumnut_dissection_10d(
    species_id: &str,
    anthesis_progress_01: f32,
    cutaway_01: f32,
) -> Result<CompiledBotanicalOrgan10d, String> {
    let mut b = OrganMeshBuilder::new();

    // ------------------------------------------------------------------------
    // Layer 1: Inflorescence Peduncle & Pedicels (`PeduncleAndPedicel`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::PeduncleAndPedicel, 0);
    // Common central peduncle with basal bract scar swelling
    let ped_base = [0.0_f32, -0.048_f32, 0.0_f32];
    let ped_mid = [0.0_f32, -0.030_f32, 0.0_f32];
    let ped_top = [0.0_f32, -0.014_f32, 0.0_f32];
    b.add_cylinder_segment(
        ped_base,
        ped_mid,
        0.0032,
        0.0026,
        8,
        [114, 136, 88, 255],
        [126, 148, 98, 255],
    );
    b.add_cylinder_segment(
        ped_mid,
        ped_top,
        0.0026,
        0.0034,
        8,
        [126, 148, 98, 255],
        [138, 162, 108, 255],
    );

    // 7 radiating pedicels forming the corymbose umbellaster + woody fruiting pedicel
    let num_flowers = 7;
    let mut flower_origins = Vec::new();

    for f in 0..num_flowers {
        let f_pos = if f == 0 {
            // Central terminal dissected flower
            [0.0_f32, 0.0_f32, 0.0_f32]
        } else {
            // 6 peripheral umbel rays
            let angle = ((f - 1) as f32) * (2.0 * PI / 6.0);
            let ray_r = 0.025_f32;
            [
                angle.cos() * ray_r,
                0.003_f32 + 0.002 * ((f % 2) as f32),
                angle.sin() * ray_r,
            ]
        };
        b.add_cylinder_segment(
            ped_top,
            f_pos,
            0.0019,
            0.0015,
            6,
            [135, 158, 108, 255],
            [150, 172, 118, 255],
        );
        flower_origins.push(f_pos);
    }

    // Woody stalk supporting the mature gumnut capsule at right
    let gumnut_offset = [0.038_f32, 0.0_f32, 0.0_f32];
    b.add_cylinder_segment(
        [gumnut_offset[0], -0.016, gumnut_offset[2]],
        gumnut_offset,
        0.0022,
        0.0028,
        6,
        [112, 94, 72, 255],
        [124, 104, 80, 255],
    );
    b.end_tissue_layer();

    // Focus high-detail dissection on the central flower (`f == 0` at [0, 0, 0])
    let cup_base = [0.0_f32, 0.0_f32, 0.0_f32];
    let cup_rim_y = 0.013_f32;
    let cup_rim_r = 0.0078_f32;
    // Cutaway angle removes up to 110 degrees on the front (+Z) side of the central hypanthium
    let cut_half_angle = cutaway_01.clamp(0.15, 1.0) * (PI * 0.32);

    // ------------------------------------------------------------------------
    // Layer 2: Hypanthium Calyx Tube (`HypanthiumCalyxTube`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::HypanthiumCalyxTube, 1);
    // Sculpted urceolate-campanulate floral cup on the central flower with cutaway window
    let n_cup = 20usize;
    let n_cup_y = 7usize;
    let base_cup_v = b.positions.len() as u32;
    let start_angle = PI * 0.5 + cut_half_angle;
    let sweep_angle = 2.0 * PI - 2.0 * cut_half_angle;

    for y_idx in 0..=n_cup_y {
        let y_frac = y_idx as f32 / n_cup_y as f32;
        let y = cup_base[1] + y_frac * (cup_rim_y - cup_base[1]);
        // Urceolate floral tube profile: rounded ovary belly + slight neck + flared staminal rim
        let belly = (y_frac * PI).sin() * 0.0014;
        let r = 0.0020 + y_frac * (cup_rim_r - 0.0020) + belly;

        for a in 0..=n_cup {
            let a_frac = a as f32 / n_cup as f32;
            let theta = start_angle + a_frac * sweep_angle;
            let norm = [theta.cos() * 0.92, -0.25, theta.sin() * 0.92];
            let pos = [theta.cos() * r, y, theta.sin() * r];
            b.add_vertex(pos, norm, [a_frac, y_frac], [176, 184, 134, 255]);
        }
    }
    for y_idx in 0..n_cup_y {
        for a in 0..n_cup {
            let i0 = base_cup_v + (y_idx * (n_cup + 1) + a) as u32;
            let i1 = i0 + (n_cup + 1) as u32;
            let i2 = i1 + 1;
            let i3 = i0 + 1;
            b.add_quad(i0, i1, i2, i3);
        }
    }

    // Peripheral umbellaster floral cups (6 surrounding flowers in the 7-flowered umbellaster)
    for &p_orig in flower_origins.iter().skip(1) {
        let p_top = [p_orig[0], p_orig[1] + 0.0085, p_orig[2]];
        b.add_cylinder_segment(
            p_orig,
            p_top,
            0.0018,
            0.0046,
            8,
            [164, 176, 124, 255],
            [184, 192, 142, 255],
        );
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 3: Operculum (Calyptriform Bud Cap) (`OperculumCalyptra`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::OperculumCalyptra, 2);
    // Central operculum lifts off and tilts away along an arc as anthesis progresses
    let operculum_lift = anthesis_progress_01 * 0.022;
    let operculum_tilt = anthesis_progress_01 * 0.82;
    let op_center = [
        operculum_lift * 0.55,
        cup_rim_y + operculum_lift,
        operculum_lift * 0.25,
    ];

    let n_op = 18usize;
    let n_op_y = 7usize;
    let base_op_v = b.positions.len() as u32;

    for y_idx in 0..=n_op_y {
        let y_frac = y_idx as f32 / n_op_y as f32;
        let local_y = y_frac * 0.0072;
        // Conical-beaked calyptra profile (umbonate apex typical of Corymbia/Eucalyptus)
        let local_r = cup_rim_r * (1.0 - y_frac).powf(0.72);

        for a in 0..=n_op {
            let a_frac = a as f32 / n_op as f32;
            let theta = a_frac * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let rx = cos_t * local_r * operculum_tilt.cos() - local_y * operculum_tilt.sin();
            let ry = cos_t * local_r * operculum_tilt.sin() + local_y * operculum_tilt.cos();
            let rz = sin_t * local_r;

            let pos = [op_center[0] + rx, op_center[1] + ry, op_center[2] + rz];
            let norm = [cos_t * 0.7, 0.7, sin_t * 0.7];
            b.add_vertex(pos, norm, [a_frac, y_frac], [224, 214, 173, 255]);
        }
    }
    for y_idx in 0..n_op_y {
        for a in 0..n_op {
            let i0 = base_op_v + (y_idx * (n_op + 1) + a) as u32;
            let i1 = i0 + (n_op + 1) as u32;
            let i2 = i1 + 1;
            let i3 = i0 + 1;
            b.add_quad(i0, i1, i2, i3);
        }
    }

    // Intact conical operculum caps on 3 of the peripheral buds in the umbellaster
    for (idx, &p_orig) in flower_origins.iter().skip(1).enumerate() {
        if idx % 2 == 0 {
            let rim = [p_orig[0], p_orig[1] + 0.0085, p_orig[2]];
            let apex = [p_orig[0], p_orig[1] + 0.0135, p_orig[2]];
            b.add_cylinder_segment(
                rim,
                apex,
                0.0046,
                0.0006,
                8,
                [218, 208, 166, 255],
                [232, 222, 182, 255],
            );
        }
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 4 & 5: Staminal Ring Filaments & Versatile Anthers
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::StaminalRingFilaments, 3);
    let num_filaments = 56usize;
    let fil_len = 0.0145_f32;
    let mut anther_positions = Vec::new();

    for i in 0..num_filaments {
        let whorl = (i % 3) as f32;
        let frac = (i as f32) / (num_filaments as f32);
        let theta = frac * 2.0 * PI;
        let r_insertion = cup_rim_r * (0.84 + whorl * 0.055);

        let p_base = [
            theta.cos() * r_insertion,
            cup_rim_y,
            theta.sin() * r_insertion,
        ];

        // Anthesis controls uncurling from inflexed bud posture to radiant cream burst
        let uncurl = anthesis_progress_01.clamp(0.0, 1.0);
        let out_reach = r_insertion + uncurl * (0.0065 + whorl * 0.0022);
        let p_knee = [
            theta.cos() * (r_insertion + uncurl * 0.0028),
            cup_rim_y + 0.0042 * (0.7 + uncurl * 0.9),
            theta.sin() * (r_insertion + uncurl * 0.0028),
        ];
        let p_mid = [
            theta.cos() * (r_insertion + uncurl * 0.0055),
            cup_rim_y + 0.0082 * (0.5 + uncurl * 0.85),
            theta.sin() * (r_insertion + uncurl * 0.0055),
        ];
        let p_tip = [
            theta.cos() * out_reach,
            cup_rim_y
                + if uncurl < 0.25 {
                    -0.0018
                } else {
                    fil_len * (0.48 + uncurl * 0.52) - whorl * 0.0012
                },
            theta.sin() * out_reach,
        ];

        b.add_cylinder_segment(
            p_base,
            p_knee,
            0.00028,
            0.00024,
            4,
            [248, 242, 212, 255],
            [252, 248, 224, 255],
        );
        b.add_cylinder_segment(
            p_knee,
            p_mid,
            0.00024,
            0.00020,
            4,
            [252, 248, 224, 255],
            [254, 250, 232, 255],
        );
        b.add_cylinder_segment(
            p_mid,
            p_tip,
            0.00020,
            0.00015,
            4,
            [254, 250, 232, 255],
            [255, 252, 238, 255],
        );

        anther_positions.push(p_tip);
    }

    // Radiant stamen filaments on the 3 open peripheral flowers in the umbellaster
    for (idx, &p_orig) in flower_origins.iter().skip(1).enumerate() {
        if idx % 2 == 1 {
            let rim_y = p_orig[1] + 0.0085;
            for k in 0..12 {
                let ka = (k as f32 / 12.0) * 2.0 * PI;
                let pb = [
                    p_orig[0] + ka.cos() * 0.0042,
                    rim_y,
                    p_orig[2] + ka.sin() * 0.0042,
                ];
                let pt = [
                    p_orig[0] + ka.cos() * 0.0095,
                    rim_y + 0.0065,
                    p_orig[2] + ka.sin() * 0.0095,
                ];
                b.add_cylinder_segment(
                    pb,
                    pt,
                    0.00022,
                    0.00014,
                    4,
                    [252, 248, 224, 255],
                    [255, 252, 238, 255],
                );
                anther_positions.push(pt);
            }
        }
    }
    b.end_tissue_layer();

    b.begin_tissue_layer(HistologicalTissueLayer::VersatileAnthersAndPollen, 4);
    // Dorsifixed versatile bilobed anthers at each stamen tip
    for &p_anther in &anther_positions {
        b.add_ellipsoid(
            p_anther,
            [0.00062, 0.00042, 0.00045],
            4,
            6,
            [245, 209, 61, 255], // Golden pollen yellow
        );
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 6: Gynoecium (Style, Stigma & Nectar Disc) (`GynoeciumStyleStigma`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::GynoeciumStyleStigma, 5);
    // Central terete style rising from the superior ovary roof
    let style_h = 0.0098_f32;
    b.add_cylinder_segment(
        [0.0, cup_rim_y - 0.001, 0.0],
        [0.0, cup_rim_y + style_h, 0.0],
        0.00082,
        0.00056,
        8,
        [158, 192, 114, 255],
        [176, 212, 132, 255],
    );
    // Terminal capitate papillate stigma
    b.add_ellipsoid(
        [0.0, cup_rim_y + style_h + 0.0004, 0.0],
        [0.00088, 0.00065, 0.00088],
        5,
        8,
        [194, 228, 142, 255],
    );
    // Raised annular nectariferous disc lining the hypanthium summit
    b.add_cylinder_segment(
        [0.0, cup_rim_y - 0.0008, 0.0],
        [0.0, cup_rim_y + 0.0006, 0.0],
        cup_rim_r * 0.62,
        cup_rim_r * 0.54,
        12,
        [146, 178, 106, 255],
        [164, 196, 122, 255],
    );
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 7: Ovary Locules & Ovules Dissection (`OvaryLoculesAndOvules`)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::OvaryLoculesAndOvules, 6);
    // 4 locule chambers with central axile placentation, radial septa, and anatropous ovules
    let locule_y = cup_rim_y * 0.44;
    let loc_r = cup_rim_r * 0.36;

    // Central axile placenta column
    b.add_cylinder_segment(
        [0.0, 0.0018, 0.0],
        [0.0, cup_rim_y * 0.86, 0.0],
        0.00085,
        0.00075,
        8,
        [204, 214, 172, 255],
        [214, 224, 182, 255],
    );

    // 4 locules with 2 vertical rows of anatropous ovules per locule
    for loc in 0..4 {
        let loc_a = (loc as f32) * (PI / 2.0) + 0.39;
        let lx = loc_a.cos() * loc_r;
        let lz = loc_a.sin() * loc_r;

        for row in 0..5 {
            let oy = locule_y + (row as f32 - 2.0) * 0.0011;
            b.add_ellipsoid(
                [lx, oy, lz],
                [0.00052, 0.00044, 0.00052],
                4,
                6,
                [234, 240, 208, 255],
            );
        }
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Layer 8 & 9: Mature Woody Gumnut Capsule & Seeds (Offset beside the flower)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(HistologicalTissueLayer::WoodyPericarpAndDehiscingValves, 7);
    // Sculpted urceolate (pitcher/urn-shaped) woody *Corymbia* gumnut capsule
    let cap_h = 0.0155_f32;
    let cap_r_max = 0.0078_f32;
    let cap_rim_r = 0.0056_f32;
    let n_cap = 20usize;
    let n_cap_y = 8usize;
    let base_cap_v = b.positions.len() as u32;

    for y_idx in 0..=n_cap_y {
        let y_frac = y_idx as f32 / n_cap_y as f32;
        let y = gumnut_offset[1] + y_frac * cap_h;
        // Authentic urceolate profile: rounded cup base, swollen belly, contracted neck, flared rim
        let r = if y_frac < 0.55 {
            cap_r_max * (0.48 + 0.52 * (y_frac / 0.55 * PI * 0.5).sin())
        } else if y_frac < 0.85 {
            let neck_t = (y_frac - 0.55) / 0.30;
            cap_r_max * (1.0 - 0.30 * (neck_t * PI * 0.5).sin())
        } else {
            let flare_t = (y_frac - 0.85) / 0.15;
            cap_r_max * (0.70 + 0.08 * flare_t)
        };

        for a in 0..=n_cap {
            let a_frac = a as f32 / n_cap as f32;
            let theta = a_frac * 2.0 * PI;
            let norm = [theta.cos(), 0.12, theta.sin()];
            let pos = [
                gumnut_offset[0] + theta.cos() * r,
                y,
                gumnut_offset[2] + theta.sin() * r,
            ];
            let shade = if y_frac > 0.85 {
                [138, 114, 90, 255]
            } else {
                [112, 92, 72, 255]
            };
            b.add_vertex(pos, norm, [a_frac, y_frac], shade);
        }
    }
    for y_idx in 0..n_cap_y {
        for a in 0..n_cap {
            let i0 = base_cap_v + (y_idx * (n_cap + 1) + a) as u32;
            let i1 = i0 + (n_cap + 1) as u32;
            let i2 = i1 + 1;
            let i3 = i0 + 1;
            b.add_quad(i0, i1, i2, i3);
        }
    }

    // 4 woody triangular dehiscing valves opening at the capsule summit
    let valve_y = gumnut_offset[1] + cap_h - 0.0016;
    for v in 0..4 {
        let a0 = (v as f32) * (PI * 0.5);
        let a1 = a0 + PI * 0.25;
        let a2 = a0 + PI * 0.5;

        let p_center = [gumnut_offset[0], valve_y - 0.0018, gumnut_offset[2]];
        let p0 = [
            gumnut_offset[0] + a0.cos() * cap_rim_r * 0.88,
            valve_y,
            gumnut_offset[2] + a0.sin() * cap_rim_r * 0.88,
        ];
        let p1 = [
            gumnut_offset[0] + a1.cos() * cap_rim_r * 0.55,
            valve_y + 0.0018, // Reflexed open valve tip
            gumnut_offset[2] + a1.sin() * cap_rim_r * 0.55,
        ];
        let p2 = [
            gumnut_offset[0] + a2.cos() * cap_rim_r * 0.88,
            valve_y,
            gumnut_offset[2] + a2.sin() * cap_rim_r * 0.88,
        ];

        let i_c = b.add_vertex(p_center, [0.0, 1.0, 0.0], [0.5, 0.5], [88, 70, 54, 255]);
        let i_0 = b.add_vertex(p0, [0.0, 1.0, 0.0], [0.0, 0.0], [132, 110, 88, 255]);
        let i_1 = b.add_vertex(p1, [0.0, 1.0, 0.0], [0.5, 1.0], [152, 128, 102, 255]);
        let i_2 = b.add_vertex(p2, [0.0, 1.0, 0.0], [1.0, 0.0], [132, 110, 88, 255]);
        b.add_triangle(i_c, i_0, i_1);
        b.add_triangle(i_c, i_1, i_2);
    }
    b.end_tissue_layer();

    b.begin_tissue_layer(HistologicalTissueLayer::FertileWingedSeedsAndChaff, 8);
    // Fertile terminal-winged seeds and reddish-brown sterile chaff spilling from the dehiscing capsule
    for s in 0..12 {
        let s_a = (s as f32) * (2.0 * PI / 12.0) + 0.25;
        let s_dist = cap_rim_r * (0.28 + (s as f32 % 4.0) * 0.22);
        let s_pos = [
            gumnut_offset[0] + s_a.cos() * s_dist,
            valve_y - 0.0006 + (s as f32 * 0.00045),
            gumnut_offset[2] + s_a.sin() * s_dist,
        ];
        let is_fertile = s % 2 == 0;
        let radii = if is_fertile {
            [0.0016, 0.00038, 0.00085] // Flattened winged seed
        } else {
            [0.00075, 0.00035, 0.00045] // Subulate sterile chaff grain
        };
        let col = if is_fertile {
            [68, 42, 28, 255]
        } else {
            [128, 68, 42, 255]
        };
        b.add_ellipsoid(s_pos, radii, 3, 5, col);
    }
    b.end_tissue_layer();

    // Baked morph targets for anthesis operculum shedding and stamen unfurling
    let mut morph_deltas = Vec::new();
    let n_verts = b.positions.len();
    let step = (n_verts / 32).max(1);
    for vi in (0..n_verts).step_by(step) {
        let p = b.positions[vi];
        let n = b.normals.get(vi).copied().unwrap_or([0.0, 1.0, 0.0]);
        // Target 0: Anthesis expansion (filaments bloom outward, operculum translates away)
        let lift = if p[1] > cup_rim_y { 0.015 } else { 0.0 };
        let d0 = quantize_morph_delta(
            [p[0] * 0.25, lift, p[2] * 0.25],
            [n[0] * 0.1, n[1] * 0.05, n[2] * 0.1],
            [0.0, 0.0, 0.0],
            0.05,
            0,
            vi as u32,
        );
        morph_deltas.push(d0);
    }
    let morph_hdr = MorphSectionHeader {
        magic: MAGIC_MRP1,
        version: MORPH_SECTION_VERSION,
        target_count: 1,
        vertex_count: n_verts as u32,
        flags: FLAG_MORPH_SPARSE | FLAG_MORPH_HAS_NORMALS | FLAG_MORPH_ONTOGENETIC_BIND,
        max_displacement: 0.05,
        ontogenetic_bind_mask: 0b0001,
        reserved: [0u8; 8],
    };

    let recipe_urn = format!(
        "urn:qualia:botanical:scale3:{species_id}:anthesis={:.2}:cutaway={:.2}",
        anthesis_progress_01, cutaway_01
    );
    let container_bytes = seal_botanical_organ_10d(
        BotanicalEducationScale::Scale3FloralAndGumnutDissection,
        &b.positions,
        &b.normals,
        &b.colors,
        &b.uvs,
        &b.indices,
        &b.slices,
        &recipe_urn,
        Some((&morph_hdr, &morph_deltas)),
    )?;

    let organ = CompiledBotanicalOrgan10d {
        scale: BotanicalEducationScale::Scale3FloralAndGumnutDissection,
        vertices: b.positions,
        normals: b.normals,
        uvs: b.uvs,
        colors: b.colors,
        material_indices: b.material_indices,
        indices: b.indices,
        tissue_slices: b.slices,
        container_bytes,
    };

    organ.verify_all()?;
    Ok(organ)
}

// ============================================================================
// 7. Scale 4: Foliar Lamina Venation & Cellular Histology
// ============================================================================

/// Synthesize Scale 4 Foliar Venation & Cellular Histology (`50 µm – 50 mm`) with Murray's Law
/// ($r_p^3 = r_1^3 + r_2^3$) midrib, diagnostic Myrtaceae intramarginal vein, pinnate secondaries,
/// tertiary areoles, and magnified 3D histological block featuring all 8 foliar cellular layers.
/// ($\ge 2,000$ vertices).
pub fn build_scale4_leaf_venation_and_histology_10d(
    species_id: &str,
    cutaway_01: f32,
) -> Result<CompiledBotanicalOrgan10d, String> {
    let mut b = OrganMeshBuilder::new();

    // Verify upstream LeafVascularSkeleton biophysics
    let profile = LeafEconomicsProfile::eucalyptus_sclerophyll();
    let mut vascular_skel = LeafVascularSkeleton::new(profile, 1.25);
    vascular_skel.generate_venation();
    vascular_skel.propagate_murray_law();
    vascular_skel.evaluate_hydraulics(1.002e-3);

    let leaf_len = profile.leaf_length_m; // 0.16 m = 160 mm
    let leaf_w = profile.leaf_width_m; // 0.035 m = 35 mm

    // ------------------------------------------------------------------------
    // Part A: Macroscopic Falcate Leaf Lamina with Petiole & Cuticle Window
    // ------------------------------------------------------------------------
    let cut_clamped = cutaway_01.clamp(0.0, 1.0);
    // Magnified histological cutaway window positioned at the right mid-basal lamina,
    // oriented PARALLEL to the leaf blade (+Z = adaxial upper surface, -Z = abaxial lower surface)
    // so it reads as an authentic stepped anatomical magnification of the leaf cross-section
    // rather than an unrelated perpendicular sandwich floating beside the blade.
    let block_origin = [0.034_f32, -0.006_f32, 0.0015_f32];
    let bx_half = 0.016_f32;
    let by_half = 0.013_f32;
    let bz_half = 0.0052_f32;

    b.begin_tissue_layer(HistologicalTissueLayer::WaxyCuticle, 0);

    // 1. Curved twisted petiole at the leaf base
    let petiole_base = [-0.0045_f32, -leaf_len * 0.5 - 0.026, -0.0022_f32];
    let petiole_mid = [-0.0018_f32, -leaf_len * 0.5 - 0.012, -0.0008_f32];
    let petiole_top = [0.0_f32, -leaf_len * 0.5, 0.0004_f32];
    b.add_cylinder_segment(
        petiole_base,
        petiole_mid,
        0.0018,
        0.0016,
        8,
        [136, 98, 66, 255],
        [146, 114, 76, 255],
    );
    b.add_cylinder_segment(
        petiole_mid,
        petiole_top,
        0.0016,
        0.0015,
        8,
        [146, 114, 76, 255],
        [156, 134, 88, 255],
    );

    // 2. Multi-column (6 columns x 32 rows) curved falcate leaf blade
    let n_leaf_segs = 32usize;
    let n_blade_cols = 6usize;
    let leaf_vertex_start = b.positions.len();
    let leaf_index_start = b.indices.len();
    let mut spine_pts = Vec::with_capacity(n_leaf_segs + 1);
    let mut half_widths = Vec::with_capacity(n_leaf_segs + 1);
    let mut intra_left_pts = Vec::with_capacity(n_leaf_segs + 1);
    let mut intra_right_pts = Vec::with_capacity(n_leaf_segs + 1);

    // Helper for leaf surface Z elevation at (frac, signed_u in [-1, 1])
    let leaf_surface_z = |frac: f32, signed_u: f32| -> f32 {
        let ridge_z = 0.0022 * (frac * PI).sin();
        let u_abs = signed_u.abs();
        // Raised central midrib with shallow V-cupped sclerophyll lamina and subtle margin undulation
        let camber = -0.0016 * u_abs.powf(1.35) * (frac * PI).sin();
        let wave = 0.00035 * (frac * PI * 5.0).sin() * u_abs;
        ridge_z + camber + wave
    };

    for s in 0..=n_leaf_segs {
        let frac = s as f32 / n_leaf_segs as f32;
        let y = (frac - 0.5) * leaf_len;
        // Authentic falcate (sickle-shaped) curvature in X
        let curve_x = (frac * PI).sin() * leaf_w * 0.58 - (frac * frac) * leaf_w * 0.14;
        let mid_z = leaf_surface_z(frac, 0.0);
        spine_pts.push([curve_x, y, mid_z]);

        // Broadest in basal-to-middle third, long acuminate drip tip
        let w = (frac.powf(0.55) * (1.0 - frac).powf(0.78) * 2.35).max(0.025) * leaf_w;
        half_widths.push(w);

        let intra_u = 0.86_f32;
        let iz_l = leaf_surface_z(frac, -intra_u) + 0.00025;
        let iz_r = leaf_surface_z(frac, intra_u) + 0.00025;
        intra_left_pts.push([curve_x - w * intra_u, y, iz_l]);
        intra_right_pts.push([curve_x + w * intra_u, y, iz_r]);
    }

    // Triangulate the multi-column curved leaf blade
    let base_blade_v = b.positions.len() as u32;
    for s in 0..=n_leaf_segs {
        let frac = s as f32 / n_leaf_segs as f32;
        let mid = spine_pts[s];
        let w = half_widths[s];

        for c in 0..=n_blade_cols {
            let u_frac = c as f32 / n_blade_cols as f32;
            let signed_u = u_frac * 2.0 - 1.0; // -1.0 (left margin) .. 0.0 (midrib) .. +1.0 (right margin)
            let x = mid[0] + signed_u * w;
            let y = mid[1];
            let z = leaf_surface_z(frac, signed_u);

            // Naturalistic sclerophyll olive-sage gradient with warm copper-red tip flush and pale margin rim
            let tip_flush = ((frac - 0.72) / 0.28).clamp(0.0, 1.0);
            let margin_highlight = signed_u.abs().powi(2);
            let r_col = (94.0 + 18.0 * margin_highlight + 52.0 * tip_flush) as u8;
            let g_col = (126.0 + 14.0 * margin_highlight - 16.0 * tip_flush) as u8;
            let b_col = (96.0 + 10.0 * margin_highlight - 22.0 * tip_flush) as u8;

            b.add_vertex(
                [x, y, z],
                [0.0, 0.0, 1.0],
                [u_frac, frac],
                [r_col, g_col, b_col, 255],
            );
        }
    }
    let stride = (n_blade_cols + 1) as u32;
    for s in 0..n_leaf_segs {
        for c in 0..n_blade_cols {
            let i0 = base_blade_v + (s as u32) * stride + (c as u32);
            let i1 = i0 + 1;
            let j0 = i0 + stride;
            let j1 = j0 + 1;
            b.add_quad(i0, j0, j1, i1);
        }
    }
    b.recompute_normals_since(leaf_vertex_start, leaf_index_start);

    // 3. Translucent Waxy Cuticle Plates on the Magnified Histological Cutaway Block
    // Stepped back by `cut_clamped` so the underlying epidermis and mesophyll are exposed in cutaway view
    let cut_x_span = bx_half * (1.0 - cut_clamped * 0.45);
    let cut_x_center = block_origin[0] - bx_half * cut_clamped * 0.45;
    // Adaxial (upper, +Z) waxy cuticle sheet
    b.add_box_cell(
        [cut_x_center, block_origin[1], block_origin[2] + bz_half],
        [cut_x_span, by_half, 0.00022],
        [212, 228, 216, 225],
    );
    // Abaxial (lower, -Z) waxy cuticle sheet
    b.add_box_cell(
        [cut_x_center, block_origin[1], block_origin[2] - bz_half],
        [cut_x_span, by_half, 0.00022],
        [206, 222, 210, 225],
    );
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Part B: Diagnostic Vascular System (Midrib, Intramarginal, Pinnate & Bundle)
    // ------------------------------------------------------------------------
    b.begin_tissue_layer(
        HistologicalTissueLayer::CollateralVascularBundleXylemPhloem,
        1,
    );
    // 1. 3D Primary Midrib following Murray's Law ($r_p^3 = r_1^3 + r_2^3$) tapering from base to apex
    for s in 0..n_leaf_segs {
        let f0 = s as f32 / n_leaf_segs as f32;
        let f1 = (s + 1) as f32 / n_leaf_segs as f32;
        let r0 = 0.00145 * (1.0 - f0 * 0.78).cbrt();
        let r1 = 0.00145 * (1.0 - f1 * 0.78).cbrt();

        b.add_cylinder_segment(
            spine_pts[s],
            spine_pts[s + 1],
            r0,
            r1,
            6,
            [184, 192, 146, 255],
            [176, 186, 140, 255],
        );
    }

    // 2. Continuous Myrtaceae Intramarginal Veins + Curved Pinnate Secondaries + Reticulate Tertiaries
    for s in 0..n_leaf_segs {
        let r_intra = 0.00038_f32;
        b.add_cylinder_segment(
            intra_left_pts[s],
            intra_left_pts[s + 1],
            r_intra,
            r_intra,
            4,
            [164, 178, 134, 255],
            [164, 178, 134, 255],
        );
        b.add_cylinder_segment(
            intra_right_pts[s],
            intra_right_pts[s + 1],
            r_intra,
            r_intra,
            4,
            [164, 178, 134, 255],
            [164, 178, 134, 255],
        );

        // Pinnate secondary lateral veins diverging forward (~55 deg) from midrib to intramarginal vein
        if s >= 2 && s + 2 <= n_leaf_segs && s % 2 == 0 {
            let s_target = (s + 2).min(n_leaf_segs);
            let f_start = s as f32 / n_leaf_segs as f32;
            let f_end = s_target as f32 / n_leaf_segs as f32;
            let f_mid = 0.5 * (f_start + f_end);
            let w_mid = 0.5 * (half_widths[s] + half_widths[s_target]);
            let mid_pt = spine_pts[s];

            // Left curved secondary vein (2 segments)
            let p_il_end = intra_left_pts[s_target];
            let p_il_ctrl = [
                0.5 * (mid_pt[0] + p_il_end[0]) - w_mid * 0.06,
                0.5 * (mid_pt[1] + p_il_end[1]) + 0.0012,
                leaf_surface_z(f_mid, -0.45) + 0.00025,
            ];
            let r_sec = 0.00046_f32 * (1.0 - f_start * 0.45);
            b.add_cylinder_segment(
                mid_pt,
                p_il_ctrl,
                r_sec,
                r_sec * 0.82,
                4,
                [174, 184, 140, 255],
                [166, 176, 132, 255],
            );
            b.add_cylinder_segment(
                p_il_ctrl,
                p_il_end,
                r_sec * 0.82,
                r_sec * 0.65,
                4,
                [166, 176, 132, 255],
                [158, 170, 126, 255],
            );

            // Right curved secondary vein (2 segments)
            let p_ir_end = intra_right_pts[s_target];
            let p_ir_ctrl = [
                0.5 * (mid_pt[0] + p_ir_end[0]) + w_mid * 0.06,
                0.5 * (mid_pt[1] + p_ir_end[1]) + 0.0012,
                leaf_surface_z(f_mid, 0.45) + 0.00025,
            ];
            b.add_cylinder_segment(
                mid_pt,
                p_ir_ctrl,
                r_sec,
                r_sec * 0.82,
                4,
                [174, 184, 140, 255],
                [166, 176, 132, 255],
            );
            b.add_cylinder_segment(
                p_ir_ctrl,
                p_ir_end,
                r_sec * 0.82,
                r_sec * 0.65,
                4,
                [166, 176, 132, 255],
                [158, 170, 126, 255],
            );

            // Tertiary reticulate cross-veinlets forming closed areoles between adjacent secondaries
            if s + 4 <= n_leaf_segs {
                let f_next = (s + 3) as f32 / n_leaf_segs as f32;
                let p_tert_l = [
                    spine_pts[s + 2][0] - half_widths[s + 2] * 0.52,
                    spine_pts[s + 2][1],
                    leaf_surface_z(f_next, -0.52) + 0.0002,
                ];
                let p_tert_r = [
                    spine_pts[s + 2][0] + half_widths[s + 2] * 0.52,
                    spine_pts[s + 2][1],
                    leaf_surface_z(f_next, 0.52) + 0.0002,
                ];
                b.add_cylinder_segment(
                    p_il_ctrl,
                    p_tert_l,
                    0.00024,
                    0.00022,
                    4,
                    [154, 166, 122, 255],
                    [154, 166, 122, 255],
                );
                b.add_cylinder_segment(
                    p_ir_ctrl,
                    p_tert_r,
                    0.00024,
                    0.00022,
                    4,
                    [154, 166, 122, 255],
                    [154, 166, 122, 255],
                );
            }
        }
    }

    // 3. Magnified Collateral Vascular Bundle inside the Histological Cutaway Window
    // Oriented parallel to the leaf blade: adaxial xylem vessels on +Z, abaxial phloem sieve tubes on -Z
    let vein_center = [block_origin[0] - 0.0032, block_origin[1], block_origin[2]];
    for vx in [-0.0011_f32, 0.0_f32, 0.0011_f32] {
        // Adaxial (+Z) lignified xylem tracheary vessels
        b.add_cylinder_segment(
            [
                vein_center[0] + vx,
                vein_center[1] - by_half * 0.88,
                vein_center[2] + 0.0008,
            ],
            [
                vein_center[0] + vx,
                vein_center[1] + by_half * 0.88,
                vein_center[2] + 0.0008,
            ],
            0.00062,
            0.00062,
            8,
            [198, 178, 124, 255],
            [188, 168, 114, 255],
        );
        // Abaxial (-Z) phloem sieve tubes and companion cells
        b.add_cylinder_segment(
            [
                vein_center[0] + vx * 0.85,
                vein_center[1] - by_half * 0.88,
                vein_center[2] - 0.0008,
            ],
            [
                vein_center[0] + vx * 0.85,
                vein_center[1] + by_half * 0.88,
                vein_center[2] - 0.0008,
            ],
            0.00052,
            0.00052,
            8,
            [172, 138, 92, 255],
            [164, 130, 86, 255],
        );
    }
    b.end_tissue_layer();

    // ------------------------------------------------------------------------
    // Part C: Magnified 3D Histological Cellular Cutaway Block (Parallel to Leaf)
    // ------------------------------------------------------------------------
    // Layer 2: Upper (+Z) & Lower (-Z) Epidermis Pavement Cells
    b.begin_tissue_layer(HistologicalTissueLayer::UpperAndLowerEpidermis, 2);
    let nx_cells = 7usize;
    let ny_cells = 6usize;
    let cell_w = (bx_half * 2.0) / (nx_cells as f32);
    let cell_h = (by_half * 2.0) / (ny_cells as f32);

    for ix in 0..nx_cells {
        // Step back upper epidermis on the right side when cutaway_01 > 0 to reveal palisade beneath
        let x_frac = (ix as f32 + 0.5) / (nx_cells as f32);
        let skip_for_cutaway = x_frac > (1.0 - cut_clamped * 0.55);

        for iy in 0..ny_cells {
            let cx = block_origin[0] - bx_half + (ix as f32 + 0.5) * cell_w;
            let cy = block_origin[1] - by_half + (iy as f32 + 0.5) * cell_h;
            // Reserve stomatal pore openings
            if (ix == 2 && iy == 1) || (ix == 4 && iy == 3) || (ix == 2 && iy == 4) {
                continue;
            }
            if !skip_for_cutaway {
                // Adaxial (upper, +Z) epidermal pavement cell
                b.add_box_cell(
                    [cx, cy, block_origin[2] + bz_half - 0.00065],
                    [cell_w * 0.46, cell_h * 0.46, 0.00038],
                    [118, 158, 106, 255],
                );
            }
            // Abaxial (lower, -Z) epidermal pavement cell
            b.add_box_cell(
                [cx, cy, block_origin[2] - bz_half + 0.00065],
                [cell_w * 0.46, cell_h * 0.46, 0.00038],
                [112, 152, 100, 255],
            );
        }
    }
    b.end_tissue_layer();

    // Layer 3: Amphistomatic Stomatal Guard Cells (sunken on BOTH +Z adaxial and -Z abaxial faces)
    b.begin_tissue_layer(HistologicalTissueLayer::AmphistomaticStomatalGuardCells, 3);
    let stomata_grid = [(2usize, 1usize), (4usize, 3usize), (2usize, 4usize)];
    for &(ix, iy) in &stomata_grid {
        let cx = block_origin[0] - bx_half + (ix as f32 + 0.5) * cell_w;
        let cy = block_origin[1] - by_half + (iy as f32 + 0.5) * cell_h;
        for &is_adaxial in &[true, false] {
            let z_sign = if is_adaxial { 1.0_f32 } else { -1.0_f32 };
            let z_guard = block_origin[2] + z_sign * (bz_half - 0.00075);
            let col_guard = [90, 140, 80, 255];
            // Paired kidney-shaped (reniform) guard cells flanking the stomatal pore
            b.add_ellipsoid(
                [cx - 0.00055, cy, z_guard],
                [0.00048, 0.00095, 0.00042],
                4,
                6,
                col_guard,
            );
            b.add_ellipsoid(
                [cx + 0.00055, cy, z_guard],
                [0.00048, 0.00095, 0.00042],
                4,
                6,
                col_guard,
            );
            // Substomatal vapour chamber cavity beneath the guard cells
            b.add_ellipsoid(
                [cx, cy, block_origin[2] + z_sign * (bz_half - 0.00145)],
                [0.00095, 0.0011, 0.00045],
                3,
                5,
                [68, 96, 64, 185],
            );
        }
    }
    b.end_tissue_layer();

    // Layer 4: Isobilateral Palisade Mesophyll (Chlorenchyma columns under BOTH +Z and -Z faces!)
    b.begin_tissue_layer(HistologicalTissueLayer::IsobilateralPalisadeMesophyll, 4);
    let pal_nx = 9usize;
    let pal_ny = 7usize;
    let pal_w = (bx_half * 2.0) / (pal_nx as f32);
    let pal_h = (by_half * 2.0) / (pal_ny as f32);

    for px in 0..pal_nx {
        for py in 0..pal_ny {
            let cx = block_origin[0] - bx_half + (px as f32 + 0.5) * pal_w;
            let cy = block_origin[1] - by_half + (py as f32 + 0.5) * pal_h;
            // Leave clear structural channels for the vascular bundle (left-center) and oil gland (right-center)
            let dist_vein = (cx - vein_center[0]).abs();
            let gland_center_x = block_origin[0] + 0.0048;
            let dist_gland =
                ((cx - gland_center_x).powi(2) + (cy - block_origin[1]).powi(2)).sqrt();
            if dist_vein < 0.0024 || dist_gland < 0.0038 {
                continue;
            }
            // Adaxial (+Z) palisade chlorenchyma columns oriented perpendicular to the leaf blade (along Z)
            b.add_cylinder_segment(
                [cx, cy, block_origin[2] + bz_half - 0.0011],
                [cx, cy, block_origin[2] + 0.0012],
                pal_w * 0.36,
                pal_w * 0.36,
                6,
                [60, 124, 54, 255],
                [50, 108, 46, 255],
            );
            // Abaxial (-Z) palisade chlorenchyma columns (diagnostic isobilateral eucalypt leaf symmetry)
            b.add_cylinder_segment(
                [cx, cy, block_origin[2] - 0.0012],
                [cx, cy, block_origin[2] - bz_half + 0.0011],
                pal_w * 0.36,
                pal_w * 0.36,
                6,
                [50, 108, 46, 255],
                [60, 124, 54, 255],
            );
        }
    }
    b.end_tissue_layer();

    // Layer 5: Spongy Mesophyll Parenchyma & Intercellular Air Spaces
    b.begin_tissue_layer(HistologicalTissueLayer::SpongyMesophyllAndAirSpaces, 5);
    for sx in 0..7 {
        for sy in 0..5 {
            let cx = block_origin[0] - bx_half * 0.84 + (sx as f32 * 0.0044);
            let cy = block_origin[1] - by_half * 0.82 + (sy as f32 * 0.0052);
            let dist_vein = (cx - vein_center[0]).abs();
            let dist_gland =
                ((cx - (block_origin[0] + 0.0048)).powi(2) + (cy - block_origin[1]).powi(2)).sqrt();
            if dist_vein < 0.0024 || dist_gland < 0.0038 {
                continue;
            }
            let z_jitter = ((sx + sy) % 3) as f32 * 0.00035 - 0.00035;
            b.add_ellipsoid(
                [cx, cy, block_origin[2] + z_jitter],
                [0.0012, 0.0011, 0.00075],
                4,
                6,
                [96, 148, 90, 235],
            );
        }
    }
    b.end_tissue_layer();

    // Layer 7: Sclerenchyma Bundle Sheath Girders (Lignified I-beam fiber columns along Z)
    b.begin_tissue_layer(HistologicalTissueLayer::SclerenchymaBundleSheathGirders, 6);
    // Adaxial (+Z) sclerenchyma girder anchoring vascular bundle to upper epidermis
    b.add_box_cell(
        [vein_center[0], block_origin[1], block_origin[2] + 0.0026],
        [0.0019, by_half * 0.86, 0.0012],
        [210, 194, 138, 255],
    );
    // Abaxial (-Z) sclerenchyma girder anchoring vascular bundle to lower epidermis
    b.add_box_cell(
        [vein_center[0], block_origin[1], block_origin[2] - 0.0026],
        [0.0019, by_half * 0.86, 0.0012],
        [204, 188, 132, 255],
    );
    b.end_tissue_layer();

    // Layer 8: Schizogenous Essential Oil Gland Cavities (in Histological Block + Lamina Areoles)
    b.begin_tissue_layer(HistologicalTissueLayer::SchizogenousEssentialOilGland, 7);
    // 1. Primary magnified schizogenous oil gland cavity inside the histological cutaway block
    let gland_center = [block_origin[0] + 0.0048, block_origin[1], block_origin[2]];
    b.add_ellipsoid(
        gland_center,
        [0.0028, 0.0028, 0.0026],
        7,
        10,
        [244, 214, 84, 242], // Translucent golden 1,8-cineole essential oil droplet
    );
    // Ring of 10 flattened secretory epithelial cells lining the schizogenous cavity
    for ep in 0..10 {
        let a = (ep as f32) * (2.0 * PI / 10.0);
        let ep_pos = [
            gland_center[0] + a.cos() * 0.0031,
            gland_center[1] + a.sin() * 0.0031,
            gland_center[2],
        ];
        b.add_ellipsoid(
            ep_pos,
            [0.00065, 0.00095, 0.0014],
            3,
            5,
            [182, 206, 132, 215],
        );
    }

    // 2. Punctate translucent golden oil glands embedded across the macroscopic leaf lamina areoles
    for s in (3..n_leaf_segs - 3).step_by(3) {
        let frac = s as f32 / n_leaf_segs as f32;
        let mid = spine_pts[s];
        let w = half_widths[s];
        for &side in &[-0.55_f32, 0.55_f32] {
            let gx = mid[0] + side * w;
            let gy = mid[1];
            let gz = leaf_surface_z(frac, side) + 0.00018;
            b.add_ellipsoid(
                [gx, gy, gz],
                [0.00065, 0.00065, 0.00042],
                3,
                5,
                [238, 206, 78, 235],
            );
        }
    }
    b.end_tissue_layer();

    let recipe_urn = format!(
        "urn:qualia:botanical:scale4:{species_id}:histology:cutaway={:.2}",
        cutaway_01
    );
    let container_bytes = seal_botanical_organ_10d(
        BotanicalEducationScale::Scale4FoliarVenationAndCellularHistology,
        &b.positions,
        &b.normals,
        &b.colors,
        &b.uvs,
        &b.indices,
        &b.slices,
        &recipe_urn,
        None,
    )?;

    let organ = CompiledBotanicalOrgan10d {
        scale: BotanicalEducationScale::Scale4FoliarVenationAndCellularHistology,
        vertices: b.positions,
        normals: b.normals,
        uvs: b.uvs,
        colors: b.colors,
        material_indices: b.material_indices,
        indices: b.indices,
        tissue_slices: b.slices,
        container_bytes,
    };

    organ.verify_all()?;
    Ok(organ)
}

// ============================================================================
// 8. Disk Asset Serializer for Corymbia aparrerinja
// ============================================================================

/// Write all upgraded educational-grade `.10d` study files to disk for *Corymbia aparrerinja*.
///
/// Upgrades:
/// - `leaf-study-blade.10d`
/// - `leaf-study-midrib.10d`
/// - `bark-study-surface.10d`
/// - `stem-anatomy-wedge.10d`
/// - `flower-dissection.10d`
/// - `gumnut-dissection.10d`
/// - `leaf-anatomy-cutaway.10d`
/// - `whole-organism-roots.10d`
#[cfg(not(target_arch = "wasm32"))]
pub fn write_corymbia_educational_assets_to_disk(repo_root: &Path) -> std::io::Result<usize> {
    let target_dir = repo_root.join("assets/models/corymbia-aparrerinja/10d");
    fs::create_dir_all(&target_dir)?;

    let target_dir_root = repo_root.join("assets/models/corymbia-aparrerinja");

    let organ_scale1 =
        build_scale1_whole_organism_10d("corymbia-aparrerinja", DevelopmentalStage::Mature)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let organ_scale2 = build_scale2_stem_bark_wedge_10d("corymbia-aparrerinja", 0.0)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let organ_scale3 =
        build_scale3_floral_and_gumnut_dissection_10d("corymbia-aparrerinja", 0.75, 0.5)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let organ_scale4 = build_scale4_leaf_venation_and_histology_10d("corymbia-aparrerinja", 0.5)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let files_to_write: [(&str, &[u8]); 8] = [
        ("whole-organism-roots.10d", &organ_scale1.container_bytes),
        ("stem-anatomy-wedge.10d", &organ_scale2.container_bytes),
        ("bark-study-surface.10d", &organ_scale2.container_bytes),
        ("flower-dissection.10d", &organ_scale3.container_bytes),
        ("gumnut-dissection.10d", &organ_scale3.container_bytes),
        ("leaf-anatomy-cutaway.10d", &organ_scale4.container_bytes),
        ("leaf-study-blade.10d", &organ_scale4.container_bytes),
        ("leaf-study-midrib.10d", &organ_scale4.container_bytes),
    ];

    let mut total_bytes = 0usize;
    for (filename, bytes) in files_to_write {
        // Write to assets/models/corymbia-aparrerinja/10d/<filename>
        let path_10d = target_dir.join(filename);
        let mut f1 = fs::File::create(&path_10d)?;
        f1.write_all(bytes)?;

        // Also write to assets/models/corymbia-aparrerinja/<filename>
        let path_root = target_dir_root.join(filename);
        let mut f2 = fs::File::create(&path_root)?;
        f2.write_all(bytes)?;

        total_bytes += bytes.len();
    }

    Ok(total_bytes)
}

// ============================================================================
// 9. Comprehensive Verification Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale1_whole_organism_high_density() {
        let organ =
            build_scale1_whole_organism_10d("corymbia-aparrerinja", DevelopmentalStage::Mature)
                .expect("Scale 1 generator must succeed");

        assert!(
            organ.vertex_count() >= 2500,
            "Scale 1 must produce high density geometry (>= 2500 verts), got {}",
            organ.vertex_count()
        );
        assert!(
            organ.triangle_count() >= 2000,
            "Scale 1 triangle count too low: {}",
            organ.triangle_count()
        );

        // Verify root tissues present
        assert!(
            organ
                .tissue_layer_slice(HistologicalTissueLayer::PhellogenCorkCambium)
                .is_some(),
            "Scale 1 must contain subterranean root architecture"
        );
        assert!(
            organ
                .tissue_layer_slice(HistologicalTissueLayer::OuterRhytidome)
                .is_some(),
            "Scale 1 must contain chalk-white outer rhytidome bole"
        );
        assert!(
            organ
                .tissue_layer_slice(HistologicalTissueLayer::WaxyCuticle)
                .is_some(),
            "Scale 1 must contain pendulous foliage"
        );

        organ
            .verify_all()
            .expect("Scale 1 .10d container must pass verify_all()");
    }

    #[test]
    fn test_scale2_stem_wedge_dissection_and_peel() {
        let organ_solid = build_scale2_stem_bark_wedge_10d("corymbia-aparrerinja", 0.0)
            .expect("Scale 2 solid wedge must succeed");
        let organ_peeled = build_scale2_stem_bark_wedge_10d("corymbia-aparrerinja", 1.0)
            .expect("Scale 2 peeled wedge must succeed");

        assert!(
            organ_solid.vertex_count() >= 1800,
            "Scale 2 must produce >= 1800 verts, got {}",
            organ_solid.vertex_count()
        );

        // All 6 concentric stem/bark layers must be populated with non-empty slices!
        let stem_layers = [
            HistologicalTissueLayer::OuterRhytidome,
            HistologicalTissueLayer::PhellogenCorkCambium,
            HistologicalTissueLayer::SecondaryPhloemAndKinoDucts,
            HistologicalTissueLayer::VascularCambium,
            HistologicalTissueLayer::SapwoodSecondaryXylem,
            HistologicalTissueLayer::HeartwoodAndPith,
        ];

        for layer in stem_layers {
            let slice = organ_solid
                .tissue_layer_slice(layer)
                .unwrap_or_else(|| panic!("Missing stem layer: {:?}", layer));
            assert!(slice.vertex_count > 0, "Empty vertex count for {:?}", layer);
            assert!(slice.index_count > 0, "Empty index count for {:?}", layer);
        }

        // Peeled version must have displaced coordinates in the outer rhytidome layer
        let outer_slice = organ_solid
            .tissue_layer_slice(HistologicalTissueLayer::OuterRhytidome)
            .expect("OuterRhytidome slice");
        assert_ne!(
            organ_solid.vertices[outer_slice.vertex_start as usize],
            organ_peeled.vertices[outer_slice.vertex_start as usize],
            "Peeled stem wedge must have displaced vertices"
        );

        organ_solid
            .verify_all()
            .expect("Solid Scale 2 container must pass verify_all()");
        organ_peeled
            .verify_all()
            .expect("Peeled Scale 2 container must pass verify_all()");
    }

    #[test]
    fn test_scale3_floral_and_gumnut_dissection() {
        let organ = build_scale3_floral_and_gumnut_dissection_10d("corymbia-aparrerinja", 0.8, 0.5)
            .expect("Scale 3 generator must succeed");

        assert!(
            organ.vertex_count() >= 2200,
            "Scale 3 must produce >= 2200 verts, got {}",
            organ.vertex_count()
        );

        // All 9 floral/fruit layers must be populated with non-empty slices!
        let floral_layers = [
            HistologicalTissueLayer::PeduncleAndPedicel,
            HistologicalTissueLayer::HypanthiumCalyxTube,
            HistologicalTissueLayer::OperculumCalyptra,
            HistologicalTissueLayer::StaminalRingFilaments,
            HistologicalTissueLayer::VersatileAnthersAndPollen,
            HistologicalTissueLayer::GynoeciumStyleStigma,
            HistologicalTissueLayer::OvaryLoculesAndOvules,
            HistologicalTissueLayer::WoodyPericarpAndDehiscingValves,
            HistologicalTissueLayer::FertileWingedSeedsAndChaff,
        ];

        for layer in floral_layers {
            let slice = organ
                .tissue_layer_slice(layer)
                .unwrap_or_else(|| panic!("Missing floral layer: {:?}", layer));
            assert!(slice.vertex_count > 0, "Empty vertex count for {:?}", layer);
            assert!(slice.index_count > 0, "Empty index count for {:?}", layer);
        }

        organ
            .verify_all()
            .expect("Scale 3 container must pass verify_all()");
    }

    #[test]
    fn test_scale4_leaf_venation_and_cellular_histology() {
        let organ = build_scale4_leaf_venation_and_histology_10d("corymbia-aparrerinja", 0.5)
            .expect("Scale 4 generator must succeed");

        assert!(
            organ.vertex_count() >= 2000,
            "Scale 4 must produce >= 2000 verts, got {}",
            organ.vertex_count()
        );

        // All 8 foliar/cellular layers must be populated with non-empty slices!
        let foliar_layers = [
            HistologicalTissueLayer::WaxyCuticle,
            HistologicalTissueLayer::UpperAndLowerEpidermis,
            HistologicalTissueLayer::AmphistomaticStomatalGuardCells,
            HistologicalTissueLayer::IsobilateralPalisadeMesophyll,
            HistologicalTissueLayer::SpongyMesophyllAndAirSpaces,
            HistologicalTissueLayer::CollateralVascularBundleXylemPhloem,
            HistologicalTissueLayer::SclerenchymaBundleSheathGirders,
            HistologicalTissueLayer::SchizogenousEssentialOilGland,
        ];

        for layer in foliar_layers {
            let slice = organ
                .tissue_layer_slice(layer)
                .unwrap_or_else(|| panic!("Missing foliar layer: {:?}", layer));
            assert!(slice.vertex_count > 0, "Empty vertex count for {:?}", layer);
            assert!(slice.index_count > 0, "Empty index count for {:?}", layer);
        }

        organ
            .verify_all()
            .expect("Scale 4 container must pass verify_all()");
    }

    #[test]
    fn test_all_23_tissue_layers_covered_with_non_empty_slices() {
        let organ1 =
            build_scale1_whole_organism_10d("corymbia-aparrerinja", DevelopmentalStage::Mature)
                .unwrap();
        let organ2 = build_scale2_stem_bark_wedge_10d("corymbia-aparrerinja", 0.0).unwrap();
        let organ3 =
            build_scale3_floral_and_gumnut_dissection_10d("corymbia-aparrerinja", 0.5, 0.5)
                .unwrap();
        let organ4 =
            build_scale4_leaf_venation_and_histology_10d("corymbia-aparrerinja", 0.5).unwrap();

        let all_layers = [
            HistologicalTissueLayer::OuterRhytidome,
            HistologicalTissueLayer::PhellogenCorkCambium,
            HistologicalTissueLayer::SecondaryPhloemAndKinoDucts,
            HistologicalTissueLayer::VascularCambium,
            HistologicalTissueLayer::SapwoodSecondaryXylem,
            HistologicalTissueLayer::HeartwoodAndPith,
            HistologicalTissueLayer::PeduncleAndPedicel,
            HistologicalTissueLayer::HypanthiumCalyxTube,
            HistologicalTissueLayer::OperculumCalyptra,
            HistologicalTissueLayer::StaminalRingFilaments,
            HistologicalTissueLayer::VersatileAnthersAndPollen,
            HistologicalTissueLayer::GynoeciumStyleStigma,
            HistologicalTissueLayer::OvaryLoculesAndOvules,
            HistologicalTissueLayer::WoodyPericarpAndDehiscingValves,
            HistologicalTissueLayer::FertileWingedSeedsAndChaff,
            HistologicalTissueLayer::WaxyCuticle,
            HistologicalTissueLayer::UpperAndLowerEpidermis,
            HistologicalTissueLayer::AmphistomaticStomatalGuardCells,
            HistologicalTissueLayer::IsobilateralPalisadeMesophyll,
            HistologicalTissueLayer::SpongyMesophyllAndAirSpaces,
            HistologicalTissueLayer::CollateralVascularBundleXylemPhloem,
            HistologicalTissueLayer::SclerenchymaBundleSheathGirders,
            HistologicalTissueLayer::SchizogenousEssentialOilGland,
        ];

        assert_eq!(all_layers.len(), 23);

        for layer in all_layers {
            let found = organ1
                .tissue_layer_slice(layer)
                .or_else(|| organ2.tissue_layer_slice(layer))
                .or_else(|| organ3.tissue_layer_slice(layer))
                .or_else(|| organ4.tissue_layer_slice(layer));

            assert!(
                found.is_some(),
                "Layer {:?} must be populated in at least one educational scale",
                layer
            );
            let slice = found.unwrap();
            assert!(
                slice.vertex_count > 0,
                "Layer {:?} vertex count must be > 0",
                layer
            );
            assert!(
                slice.index_count > 0,
                "Layer {:?} index count must be > 0",
                layer
            );
            assert!(
                !slice.plant_ontology_id.is_empty(),
                "Layer {:?} must have a PO ID",
                layer
            );
        }
    }

    #[test]
    fn test_leaf_vascular_skeleton_murray_law_conductance_positive() {
        let profile = LeafEconomicsProfile::eucalyptus_sclerophyll();
        let skeleton = LeafVascularSkeleton::new(profile, 1.25);

        assert!(
            skeleton.edge_count > 0,
            "Vascular skeleton must contain edges"
        );
        for i in 0..skeleton.edge_count {
            let edge = skeleton.edges[i];
            assert!(
                edge.radius_mm > 0.0,
                "Edge {} radius must be positive, got {}",
                i,
                edge.radius_mm
            );
            assert!(
                edge.conductance > 0.0,
                "Edge {} Murray's Law hydraulic conductance must be positive, got {}",
                i,
                edge.conductance
            );
        }
    }
}
