//! Taxonomic Semantic Spine, 3D Dissection Callout Pins & Specimen Loupe Dissection State (`biological_ontology.rs`).
//!
//! Integrates standard biological ontologies:
//! - **Plant Ontology (`PO:`)**: Plant anatomical structures, tissues, and developmental stages.
//! - **Uberon (`UBERON:`)**: Comparative vertebrate anatomical structures and organs.
//! - **Gene Ontology (`GO:`)**: Cellular components and molecular functions.
//! - **Australian Plant Census (`APC`)**: Authoritative Australian taxonomic nomenclature.
//!
//! Features:
//! - Canonical catalog of >= 24 rigorous educational pins spanning all 5 scales.
//! - Zero-heap ray-sphere picking (`query_closest_anatomical_pin_zero_heap`) for 3D Specimen Loupe interaction.
//! - Interactive 3D Specimen Loupe dissection state (`LoupeDissectionState`).
//! - Streaming `.q42` semantic graph encoding (`emit_anatomical_ontology_nquins_to_writer`).

use std::collections::HashMap;
use std::io;

use serde::{Deserialize, Serialize};

use crate::q_hash;
use crate::NQuin;
use crate::QUINS_PER_BLOCK;

#[cfg(not(target_arch = "wasm32"))]
use crate::q42::q42_volume::StreamingQ42VolumeWriter;

// ── Biological Ontology Standards ───────────────────────────────────────────

/// Standardized biological and anatomical ontology namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnatomicalOntologyStandard {
    /// Plant Ontology (`PO:`) for botanical organs, tissues, and cellular structures.
    PlantOntologyPO,
    /// Uberon multi-species anatomy ontology (`UBERON:`) for vertebrate zoology.
    UberonAnatomy,
    /// Gene Ontology (`GO:`) for subcellular components and biological processes.
    GeneOntologyGO,
    /// Australian Plant Census (`APC`) for verified Australian plant taxonomy.
    AustralianPlantCensusAPC,
}

impl AnatomicalOntologyStandard {
    /// Returns the standard CURIE prefix (e.g. `"PO:"` or `"UBERON:"`).
    pub const fn prefix(&self) -> &'static str {
        match self {
            Self::PlantOntologyPO => "PO:",
            Self::UberonAnatomy => "UBERON:",
            Self::GeneOntologyGO => "GO:",
            Self::AustralianPlantCensusAPC => "APC:",
        }
    }

    /// Returns the canonical base IRI for this ontology standard.
    pub const fn base_iri(&self) -> &'static str {
        match self {
            Self::PlantOntologyPO => "http://purl.obolibrary.org/obo/PO_",
            Self::UberonAnatomy => "http://purl.obolibrary.org/obo/UBERON_",
            Self::GeneOntologyGO => "http://purl.obolibrary.org/obo/GO_",
            Self::AustralianPlantCensusAPC => "https://biodiversity.org.au/voc/apc/",
        }
    }

    /// Resolves the standard from a CURIE string.
    pub fn from_curie(curie: &str) -> Option<Self> {
        if curie.starts_with("PO:") {
            Some(Self::PlantOntologyPO)
        } else if curie.starts_with("UBERON:") {
            Some(Self::UberonAnatomy)
        } else if curie.starts_with("GO:") {
            Some(Self::GeneOntologyGO)
        } else if curie.starts_with("APC:") {
            Some(Self::AustralianPlantCensusAPC)
        } else {
            None
        }
    }
}

// ── Anatomical Callout Pin ───────────────────────────────────────────────────

/// 3D Educational Callout Pin anchored to a specific anatomical or histological structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnatomicalCalloutPin {
    /// Unique identifier for this pin (e.g. `"pin_s4_stoma"`).
    pub pin_id: &'static str,
    /// Biological scale ladder tier (`1..=5`).
    pub scale: u8,
    /// Standardized ontology CURIE (e.g. `"PO:0000032"` or `"UBERON:0003661"`).
    pub ontology_curie: &'static str,
    /// Authoritative scientific term in biological nomenclature.
    pub scientific_term: &'static str,
    /// 3D local anchor coordinate in specimen object-space [x, y, z].
    pub anchor_local_xyz: [f32; 3],
    /// Outward surface normal vector [nx, ny, nz].
    pub normal_xyz: [f32; 3],
    /// Recommended camera offset for focusing on this structure [dx, dy, dz].
    pub camera_focus_offset: [f32; 3],
    /// Mathematical/biophysical equation governing this structure's function.
    pub biophysical_formula: &'static str,
    /// Rigorous educational and histological summary text.
    pub educational_summary: &'static str,
    /// Indigenous Australian custodial, ethnobotanical, and ecological context.
    pub indigenous_custodial_context: &'static str,
}

impl AnatomicalCalloutPin {
    /// Returns the ontology standard associated with this pin's CURIE.
    pub fn standard(&self) -> Option<AnatomicalOntologyStandard> {
        AnatomicalOntologyStandard::from_curie(self.ontology_curie)
    }

    /// Returns the full OBO / RDF IRI corresponding to this pin's CURIE.
    pub fn full_ontology_iri(&self) -> String {
        if let Some(id_part) = self.ontology_curie.strip_prefix("PO:") {
            format!("http://purl.obolibrary.org/obo/PO_{id_part}")
        } else if let Some(id_part) = self.ontology_curie.strip_prefix("UBERON:") {
            format!("http://purl.obolibrary.org/obo/UBERON_{id_part}")
        } else if let Some(id_part) = self.ontology_curie.strip_prefix("GO:") {
            format!("http://purl.obolibrary.org/obo/GO_{id_part}")
        } else if let Some(id_part) = self.ontology_curie.strip_prefix("APC:") {
            format!("https://biodiversity.org.au/voc/apc/{id_part}")
        } else {
            format!("urn:qualia:ontology:{}", self.ontology_curie)
        }
    }
}

// ── Specimen Loupe Dissection State ──────────────────────────────────────────

/// Dynamic anatomical dissection, tissue peel, and phenological state in the 3D Specimen Loupe.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoupeDissectionState {
    /// Active biological scale (1..=5):
    /// - 1: Whole Organism Macroform (Canopy / Habit)
    /// - 2: Stem, Bark Wedge & Secondary Xylem Radial Cross-Section
    /// - 3: Reproductive Organ Dissection (Flower / Operculum / Woody Gumnut)
    /// - 4: Foliar Lamina Venation & Cellular Microscopic Histology
    /// - 5: Comparative Vertebrate 3-Layer Anatomy (Osteology / Myology / Integument)
    pub active_scale: u8,
    /// Exploded view layer separation ratio in `[0.0, 1.0]`.
    pub explode_layers_01: f32,
    /// Anatomical cutaway slicing plane progress in `[0.0, 1.0]`.
    pub cutaway_plane_01: f32,
    /// Seasonal phenology month (`1..=12`, 1 = January, 12 = December).
    pub phenology_month: u8,
    /// Ontogenetic developmental progress in `[0.0, 1.0]` (Seedling -> Ancient).
    pub ontogenetic_progress_01: f32,
    /// Skeletal system (osteology) layer visibility in `[0.0, 1.0]`.
    pub osteology_vis_01: f32,
    /// Muscular system (myology) layer visibility in `[0.0, 1.0]`.
    pub myology_vis_01: f32,
    /// External skin/pelage/plumage (integument) layer visibility in `[0.0, 1.0]`.
    pub integument_vis_01: f32,
    /// Active biological ontology CURIE (e.g. `po:0009010`, `po:0000032`, `uberon:0002415`).
    pub selected_ontology_curie: &'static str,
}

impl Default for LoupeDissectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl LoupeDissectionState {
    /// Constructs a new default `LoupeDissectionState` at Scale 1.
    pub const fn new() -> Self {
        Self {
            active_scale: 1,
            explode_layers_01: 0.0,
            cutaway_plane_01: 0.0,
            phenology_month: 1,
            ontogenetic_progress_01: 0.65,
            osteology_vis_01: 1.0,
            myology_vis_01: 1.0,
            integument_vis_01: 1.0,
            selected_ontology_curie: "po:0009010",
        }
    }

    /// Selects the active biological scale (`1..=5`) and updates the default CURIE for that scale.
    pub fn select_scale(&mut self, scale: u8) {
        self.active_scale = scale.clamp(1, 5);
        self.selected_ontology_curie = match self.active_scale {
            1 => "po:0025531",
            2 => "po:0004708",
            3 => "po:0009084",
            4 => "po:0000032",
            5 => "uberon:0002415",
            _ => "po:0009010",
        };
    }

    /// Sets the anatomical cutaway slicing plane progress (`[0.0, 1.0]`).
    pub fn set_layer_peel(&mut self, peel_01: f32) {
        self.cutaway_plane_01 = peel_01.clamp(0.0, 1.0);
    }

    /// Sets the ontogenetic / floral anthesis progress (`[0.0, 1.0]`).
    pub fn set_anthesis_progress(&mut self, progress_01: f32) {
        self.ontogenetic_progress_01 = progress_01.clamp(0.0, 1.0);
    }

    /// Sets the exploded-view layer separation ratio (`[0.0, 1.0]`).
    pub fn set_explode_layer(&mut self, explode_01: f32) {
        self.explode_layers_01 = explode_01.clamp(0.0, 1.0);
    }

    /// Sets the 3-layer comparative vertebrate visibility (`osteology`, `myology`, `integument` in `[0.0, 1.0]`).
    pub fn set_vertebrate_layer_visibility(
        &mut self,
        osteology_vis_01: f32,
        myology_vis_01: f32,
        integument_vis_01: f32,
    ) {
        self.osteology_vis_01 = osteology_vis_01.clamp(0.0, 1.0);
        self.myology_vis_01 = myology_vis_01.clamp(0.0, 1.0);
        self.integument_vis_01 = integument_vis_01.clamp(0.0, 1.0);
    }

    /// Queries the closest canonical callout pin along `ray_origin` + `t * ray_dir` for the active scale
    /// and updates `selected_ontology_curie` when a hit occurs within `max_dist`.
    pub fn pick_callout_pin(
        &mut self,
        ray_origin: [f32; 3],
        ray_dir: [f32; 3],
        max_dist: f32,
    ) -> Option<&'static AnatomicalCalloutPin> {
        let pin = query_closest_anatomical_pin_zero_heap(
            self.active_scale,
            ray_origin,
            ray_dir,
            max_dist,
        )?;
        self.selected_ontology_curie = pin.ontology_curie;
        Some(pin)
    }
}

// ── Canonical Educational Catalog (32 Pins Spanning Scales 1..=5) ────────────

/// Canonical catalog of 32 educational callout pins spanning all 5 scales of biological organization.
pub const CANONICAL_ANATOMICAL_PINS: &[AnatomicalCalloutPin] = &[
    // ═════════════════════════════════════════════════════════════════════════
    // SCALE 1: WHOLE ORGANISM ARCHITECTURE & ROOT SYSTEM (10 m – 40 m)
    // ═════════════════════════════════════════════════════════════════════════
    AnatomicalCalloutPin {
        pin_id: "pin_s1_lignotuber",
        scale: 1,
        ontology_curie: "PO:0025531",
        scientific_term: "Lignotuber Epicormic Meristem",
        anchor_local_xyz: [0.0, -0.25, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.35, 1.20],
        biophysical_formula: "Epicormic Bud Density: D_bud = rho_m * (1.0 - exp(-t / tau_b)); Fire Survival P_surv = 1.0 / (1.0 + exp(-alpha_f * (B_bark - B_crit)))",
        educational_summary: "A woody basal swelling of the root crown containing a dense reserve of dormant epicormic vegetative buds and starch reserves. When wild crown fires destroy the entire eucalypt foliage, the subterranean lignotuber activates hormone-mediated bud release, producing vigorous epicormic shoots within weeks.",
        indigenous_custodial_context: "Known in Wiradjuri and Walgalu custodial botany as 'Gunhan' (resilient root base). Traditional custodial fire practitioners utilized frequent cool patch burns ('badhang') so fire intensity remained below the lethal cambium threshold, stimulating vegetative renewal without fracturing the lignotuber's starch core.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s1_taproot",
        scale: 1,
        ontology_curie: "PO:0000038",
        scientific_term: "Primary Geotropic Taproot System",
        anchor_local_xyz: [0.0, -1.80, 0.0],
        normal_xyz: [0.0, -1.0, 0.0],
        camera_focus_offset: [0.50, -1.20, 2.20],
        biophysical_formula: "Darcy-Richards Unsaturated Soil Flux: q = -K_unsat(theta) * grad(psi_m + z); Hydraulic Lift Flux J_lift = sum(K_root * (psi_deep - psi_shallow))",
        educational_summary: "The primary vertical axis penetrating several meters through weathered saprolite and bedrock fractures down to deep water tables. During drought, eucalypts perform hydraulic lift—passively drawing water from deep moist strata at night and exuding it into shallow roots, hydrating the rhizosphere.",
        indigenous_custodial_context: "Traditional owners identified mature taproot surface expressions above dry watercourses as reliable indicators of subterranean soakages ('gnamma' or water trees), tapping roots sustainably for emergency drinking water during prolonged dry seasons.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s1_buttress_flare",
        scale: 1,
        ontology_curie: "PO:0000042",
        scientific_term: "Lateral Buttress Flare & Root Plate",
        anchor_local_xyz: [0.85, -0.10, 0.45],
        normal_xyz: [0.88, 0.22, 0.42],
        camera_focus_offset: [1.20, 0.40, 1.50],
        biophysical_formula: "Root Plate Overturning Moment: M_crit = C_shear * A_plate * D_root + W_soil * r_hinge; Mattheck Axiom of Uniform Stress: sigma_surf = const",
        educational_summary: "Flaring asymmetrical root adaptations that distribute wind drag forces acting on the crown into the shallow soil horizon. As the tree sways in alpine gales, windward roots experience severe tension while leeward roots absorb compression, governed by the biological axiom of uniform stress.",
        indigenous_custodial_context: "The sheltered hollows between buttress roots served as temporary windbreaks and cool resting places along high-country trading tracks. Resilient fibrous bark from buttress margins was peeled in narrow strips without harming root stability.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s1_mycorrhizal_rhizosphere",
        scale: 1,
        ontology_curie: "PO:0005707",
        scientific_term: "Ectomycorrhizal Absorptive Feeder Root Network",
        anchor_local_xyz: [-0.65, -0.40, -0.50],
        normal_xyz: [-0.70, 0.40, -0.58],
        camera_focus_offset: [-0.80, -0.10, 1.40],
        biophysical_formula: "Michaelis-Menten Phosphate Uptake: V_P = (V_max * [P_soil]) / (K_m + [P_soil]); Hyphal Surface Area Multiplier: A_eff = A_root * (1.0 + eta_hyphae)",
        educational_summary: "Fine cluster roots enveloped in fungal mantles of Basidiomycetes (e.g. Pisolithus, Cortinarius). Australian soils are famously depauperate in available phosphorus; fungal hyphae secrete phosphatase enzymes and extend the effective root surface area by two orders of magnitude in exchange for host photosynthates.",
        indigenous_custodial_context: "Ectomycorrhizal fruiting bodies (native truffles and mushrooms) associate with eucalypt roots and are prized seasonal foods ('mungar') tracked by bandicoots, potoroos, and Indigenous foragers who read soil crack patterns.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s1_da_vinci_pipe_bole",
        scale: 1,
        ontology_curie: "PO:0009005",
        scientific_term: "Primary Bole Da Vinci / Shinozaki Pipe Model",
        anchor_local_xyz: [0.0, 1.85, 0.0],
        normal_xyz: [0.0, 0.0, 1.0],
        camera_focus_offset: [0.0, 2.10, 3.50],
        biophysical_formula: "Leonardo da Vinci / Shinozaki Pipe Model: r_bole^eta = sum(r_branch_i^eta) (eta = 2.49); Allometric Biomass M = a * (D^2 * H)^b",
        educational_summary: "The main structural trunk maintains cross-sectional vessel area proportional to the total leaf area it supports above. Da Vinci observed that all branches at every height equal the trunk in combined thickness, a rule formalized as the Pipe Model of plant allometry with exponent eta ~ 2.49.",
        indigenous_custodial_context: "Major trunks of Snow Gums and Ghost Gums carried custodial markers, boundary engravings, and scar markings where bark was respectfully harvested for coolamons, leaving the underlying sapwood intact to callus over.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s1_crown_space_colonization",
        scale: 1,
        ontology_curie: "PO:0020100",
        scientific_term: "Scaffold Limb Architecture & Crown Space Colonization",
        anchor_local_xyz: [1.20, 4.50, -0.80],
        normal_xyz: [0.72, 0.60, -0.35],
        camera_focus_offset: [1.80, 5.00, 2.80],
        biophysical_formula: "Runions Space Colonization: v_grow = sum((p_attr - v_node) / ||p_attr - v_node||); Cantilever Bending Moment M_b = rho_wood * g * int(x * A(x) dx)",
        educational_summary: "The upper scaffold branches deploy via competitive space colonization toward canopy gaps. Snow Gums (*Eucalyptus pauciflora*) exhibit thick, gnarled, twisted branches that tolerate extreme snowpack loading and rime ice buildup without brittle trunk failure.",
        indigenous_custodial_context: "High-country elders read the angle of scaffold growth to predict alpine storm vectors and snowfall weight, guiding seasonal movements between mountain peaks and valley river camps.",
    },

    // ═════════════════════════════════════════════════════════════════════════
    // SCALE 2: BRANCH, PHYLLOTAXIS & BARK HISTOLOGY (0.5 m – 2 m)
    // ═════════════════════════════════════════════════════════════════════════
    AnatomicalCalloutPin {
        pin_id: "pin_s2_rhytidome_cork",
        scale: 2,
        ontology_curie: "PO:0005513",
        scientific_term: "Decorticating Periderm & Outer Rhytidome",
        anchor_local_xyz: [0.18, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.45, 0.70, 0.90],
        biophysical_formula: "Fourier Thermal Insulation: q_heat = -k_bark * (T_flame - T_cambium) / delta_x; Thermal Diffusivity alpha_t = k / (rho * c_p) = 1.3e-7 m^2/s",
        educational_summary: "The outermost dead tissue layers composed of alternated phellem and collapsed phloem. Smooth-barked gums periodically decorticate (shed) outer strips in ribbons, shedding combustible epiphyte loads and reducing fuel accumulation on the trunk while reflecting solar radiation.",
        indigenous_custodial_context: "Shed bark strips ('durra') served as instant kindling for fire drills, torches for night fishing, and insulating floor mats inside shelter huts. Paperbark and stringybark varieties provided twine, bandages, and cooking wrappers.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_phellogen",
        scale: 2,
        ontology_curie: "PO:0005509",
        scientific_term: "Phellogen (Cork Cambium) Meristematic Ring",
        anchor_local_xyz: [0.165, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.35, 0.68, 0.75],
        biophysical_formula: "Periclinal Cell Division Rate: dn/dt = mu_phellogen * (1.0 - exp(-T_eff / T_base)); Suberin Deposition: lambda_sub = f(w_phenol, w_aliphatic)",
        educational_summary: "A lateral meristem that produces suberized, impermeable cork (phellem) toward the exterior and living phelloderm parenchyma inward. Suberin lamellae deposited in cell walls prevent desiccating moisture loss and microbial pathogen invasion.",
        indigenous_custodial_context: "Knowledge of bark cambium depth was essential when carving shields or harvesting bark; harvesters scored shallowly above the phellogen during wet spring sap flows so the tree would rapidly form wound periderm and heal.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_secondary_phloem_kino",
        scale: 2,
        ontology_curie: "PO:0005417",
        scientific_term: "Secondary Phloem & Polyphenolic Kino Ducts",
        anchor_local_xyz: [0.15, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.32, 0.68, 0.70],
        biophysical_formula: "Münch Pressure-Flow Phloem Flux: J_s = (k_phloem / eta_sap) * grad(P_turgor); Kino Exudation Pressure P_kino = P_osmo - P_tissue",
        educational_summary: "The inner living bark containing sieve tube elements and companion cells conducting sucrose downward from leaves. Embedded within are traumatic resin/kino ducts that synthesize dense polyphenolic tannins that polymerize on exposure to air, sealing mechanical insect borings.",
        indigenous_custodial_context: "Red gum resin ('kino' or 'yandhirra') is a cornerstone bush medicine. Dissolved in water or applied as dried powder, its powerful astringent tannins arrest dysentery, stop bleeding, soothe burns, and seal woodworking joints.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_vascular_cambium",
        scale: 2,
        ontology_curie: "PO:0005511",
        scientific_term: "Vascular Cambium (Bifacial Lateral Meristem)",
        anchor_local_xyz: [0.138, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.30, 0.68, 0.65],
        biophysical_formula: "Auxin-Driven Radial Growth: dR/dt = k_camb * [IAA] * (psi_turgor - psi_yield); Ray Cell Ratio: f_ray = N_ray / (N_fusiform + N_ray)",
        educational_summary: "The microscopic single-cell layer of undifferentiated meristematic cells responsible for all radial growth in tree diameter. It divides bifacially, producing secondary xylem (wood) toward the interior and secondary phloem (inner bark) toward the exterior.",
        indigenous_custodial_context: "Custodial seasonal calendars linked the swelling of the vascular cambium in late spring (marked by the rising of the Pleiades and blooming of silver wattles) to the prime season for stripping bark without tearing wood fibres.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_sapwood_xylem",
        scale: 2,
        ontology_curie: "PO:0005352",
        scientific_term: "Active Secondary Xylem (Sapwood) & Vessel Elements",
        anchor_local_xyz: [0.10, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.28, 0.68, 0.60],
        biophysical_formula: "Hagen-Poiseuille Axial Conductance: K_ax = (pi * sum(r_vessel_i^4)) / (8.0 * eta_water); Cavitation Vulnerability P50: PLC = 100.0 / (1.0 + exp(s * (psi - psi_50)))",
        educational_summary: "The active sapwood conducts tens of liters of water per day upward from roots to the canopy under negative hydrostatic tension (-1.5 to -3.0 MPa). Wide vessel elements maximize hydraulic efficiency while bordered pits limit the spread of cavitation embolisms.",
        indigenous_custodial_context: "Sapwood from young branches was split to make flexible tool handles, fishing spears, and digging sticks ('kutti'), heat-tempered over coals to harden outer fibres without charring the inner core.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_heartwood_tyloses",
        scale: 2,
        ontology_curie: "PO:0005353",
        scientific_term: "Heartwood (Duramen) & Polyphenolic Tyloses Matrix",
        anchor_local_xyz: [0.03, 0.65, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.22, 0.68, 0.55],
        biophysical_formula: "Tyloses Vessel Occlusion Ratio: phi_occ = V_balloon / V_lumen; Phenolic Extractives Concentration: C_extract = C_0 * exp(-r / r_hw)",
        educational_summary: "The non-conducting central core of the tree where xylem parenchyma cells balloon into vessel lumens through pit membranes (tyloses), completely sealing the tubes. Heavy impregnation with tannins and polyphenols gives heartwood exceptional durability against termites and fungal rot.",
        indigenous_custodial_context: "Hard, dense heartwood from dead standing trees was chosen for crafting boomerangs ('kali'), woomeras, and clapsticks, valued for its resonant acoustic tone and centuries-long resistance to ground termites.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s2_pendulous_falcate_leaf",
        scale: 2,
        ontology_curie: "PO:0025034",
        scientific_term: "Pendulous Isobilateral Falcate Leaf Lamina",
        anchor_local_xyz: [0.32, 0.20, 0.15],
        normal_xyz: [0.0, -1.0, 0.0],
        camera_focus_offset: [0.35, 0.25, 0.45],
        biophysical_formula: "Cos-Zenith Light Interception: I_eff = I_0 * sin(theta_sun) * cos(theta_leaf); Photoinhibition Mitigation: F_v / F_m >= 0.80 at solar noon",
        educational_summary: "Adult eucalypt foliage hangs vertically on flexible petioles, an adaptation called pendulous falcate phyllotaxis. By presenting leaf edges to the scorching overhead sun at solar noon and leaf faces to gentle morning/afternoon light, the tree minimizes photoinhibition and heat load while maintaining carbon fixation.",
        indigenous_custodial_context: "Fresh falcate leaves placed on hot coals created aromatic healing steam ('marridha') inhaled for coughs and colds. Leaves scattered in waterways released cineole and saponins to stun fish for harvest without poisoning the creek.",
    },

    // ═════════════════════════════════════════════════════════════════════════
    // SCALE 3: REPRODUCTIVE ORGANS & FLORAL DISSECTION (5 mm – 150 mm)
    // ═════════════════════════════════════════════════════════════════════════
    AnatomicalCalloutPin {
        pin_id: "pin_s3_umbellaster",
        scale: 3,
        ontology_curie: "PO:0006322",
        scientific_term: "Compound Corymbose Panicle of Umbellasters",
        anchor_local_xyz: [0.0, 0.08, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.10, 0.22],
        biophysical_formula: "Floral Aggregation Index: N_buds = 3..7 per umbel; Total Sugar Flux: Q_nectar = sum(V_nectar_i * C_sugar_i * rho_sol)",
        educational_summary: "An inflorescence consisting of clustered umbels (umbellasters) borne on flattened peduncles. In *Corymbia aparrerinja* and related bloodwoods, terminal corymbose panicles produce dense masses of blossom outside the foliage canopy, maximizing visibility to nectar-feeding birds and bats.",
        indigenous_custodial_context: "Prolific flowering of corymbose blossoms signaled the arrival of nectar-feeding lorikeets ('bilany') and flying-foxes, marking gatherings where clans shared blossom-sweetened water and harvested fresh wild honey ('mayi').",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_operculum_cap",
        scale: 3,
        ontology_curie: "PO:0009084",
        scientific_term: "Operculum (Calyptriform Cap) Abscission Zone",
        anchor_local_xyz: [0.0, 0.035, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.05, 0.12],
        biophysical_formula: "Turgor Abscission Threshold: sigma_abs = E_hyd * epsilon_cell >= sigma_crit; Operculum Separation Force F_sep = 2.0 * pi * r_rim * gamma_wall",
        educational_summary: "The hallmark organ of the genus *Eucalyptus* (from Greek 'eu-kalyptos', well-covered). Fused sepals and petals form a conical or hemispherical cap enclosing stamens. As stamens hydrate and expand at anthesis, enzymatic dissolution of middle lamellae along the circumscissile rim pops the operculum off.",
        indigenous_custodial_context: "Children collected freshly shed opercula to use as miniature play cups, whistles, and pipe beads strung on kangaroo sinew. The pop of shedding opercula in late spring signified that native bees were filling tree-hollow hives with honey.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_hypanthium_cup",
        scale: 3,
        ontology_curie: "PO:0009064",
        scientific_term: "Hypanthium (Fused Floral Cup & Nectariferous Disc)",
        anchor_local_xyz: [0.0, 0.012, 0.0],
        normal_xyz: [0.0, 0.0, 1.0],
        camera_focus_offset: [0.0, 0.02, 0.10],
        biophysical_formula: "Nectary Secretion Rate: dV_nectar/dt = k_sec * A_disc * (psi_osm - psi_ext); Sugar Concentration: Bx = [Sucrose] + [Glucose] + [Fructose] (35-65%)",
        educational_summary: "A cup-like structure formed by the fusion of the basal parts of the sepals, petals, and stamens with the inferior ovary. The upper rim bears a prominent ring of nectariferous tissue that secretes concentrated sucrose, glucose, and fructose solutions.",
        indigenous_custodial_context: "Blossom heads were plucked and steeped in cool bark vessels of mountain water to create a delicate, sweet nectar cordial ('ngarru'). Elders taught children to shake morning blossoms over their palms to taste the first nectar drops.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_staminal_ring_anthers",
        scale: 3,
        ontology_curie: "PO:0009066",
        scientific_term: "Staminal Ring (Androecium) & Versatile Anthers",
        anchor_local_xyz: [0.018, 0.028, 0.0],
        normal_xyz: [0.85, 0.52, 0.0],
        camera_focus_offset: [0.03, 0.035, 0.08],
        biophysical_formula: "Filament Elastic Snap: F_spring = -k_fil * delta_x; Pollen Dehiscence: RH_crit <= 45% triggers endothecium shrinkage and slit rupture",
        educational_summary: "Hundreds of slender cream-white filaments bent inward in bud that spring outward upon operculum detachment. Each filament terminates in a versatile, dorsifixed anther opening via longitudinal slits, dusted with tricolporate pollen grains adapted for bird, mammal, and insect vectors.",
        indigenous_custodial_context: "Pollen dusting on foraging honeyeaters and sugar gliders was observed by custodial naturalists as part of 'Kurrumbene' (ecological balance)—recognizing the reciprocal relationship between bird pollination and seed set.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_style_stigma",
        scale: 3,
        ontology_curie: "PO:0009072",
        scientific_term: "Gynoecium Central Terete Style & Glandular Stigma",
        anchor_local_xyz: [0.0, 0.032, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.04, 0.08],
        biophysical_formula: "Pollen Tube Growth Rate: v_tube = k_chem * [Ca2+] * exp(-E_act / (R * T)); Gametophytic Self-Incompatibility: S_allele match arrests tube",
        educational_summary: "A single central, terete style rising above the staminal ring topped with a glandular, papillar stigma. The stigma secretes a sticky exudate when receptive, capturing pollen and guiding tube growth down the transmitting tissue to ovules in the ovary.",
        indigenous_custodial_context: "Women elders held custodial knowledge of plant reproductive cycles, identifying the precise post-anthesis stage when style receptivity ended and medicinal alkaloids concentrated in developing ovary walls.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_ovary_locules",
        scale: 3,
        ontology_curie: "PO:0009063",
        scientific_term: "Inferior Ovary with 3-4 Locules & Axile Placentation",
        anchor_local_xyz: [0.0, 0.005, 0.0],
        normal_xyz: [0.0, 0.0, 1.0],
        camera_focus_offset: [0.0, 0.015, 0.08],
        biophysical_formula: "Ovule Packaging: N_ovules = 15..45 per locule; Placentation Efficiency: eta_fert = N_viable / (N_viable + N_chaff)",
        educational_summary: "The ovary is completely inferior, embedded within and fused to the thick woody hypanthium. A transverse cutaway reveals 3 to 4 distinct locular chambers containing two rows of anatropous ovules attached to central axile placentas.",
        indigenous_custodial_context: "The seed chambers of green gumnuts were ground with river stone pestles into a cooling poultice applied to insect bites, stings, and skin sores to reduce swelling.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_gumnut_valves",
        scale: 3,
        ontology_curie: "PO:0009084",
        scientific_term: "Woody Capsule (Gumnut) Enclosed Dehiscent Valves",
        anchor_local_xyz: [0.0, -0.02, 0.0],
        normal_xyz: [0.0, -1.0, 0.0],
        camera_focus_offset: [0.0, -0.01, 0.12],
        biophysical_formula: "Hydro-Elastic Bilayer Curvature: kappa = (6.0 * (alpha_out - alpha_in) * delta_RH) / (h * (5.0 + (E_out / E_in))); Valve Opening T_crit >= 60 C",
        educational_summary: "A hard woody capsule (gumnut) protecting fertile seeds. The internal valves remain sunken and enclosed under ambient conditions, but desiccation following fire heat causes differential shrinkage between inner and outer sclerenchyma layers, curling the valves outward to release seeds into ash beds.",
        indigenous_custodial_context: "Dry gumnuts ('wirra') were used by children for spinning tops, musical rattles tied around ankles during dance, and whistles hollowed out to mimic parrot warning calls.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s3_seed_chaff",
        scale: 3,
        ontology_curie: "PO:0009010",
        scientific_term: "Saucer-Shaped Fertile Seeds & Sterile Chaff",
        anchor_local_xyz: [0.004, -0.018, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.01, -0.01, 0.06],
        biophysical_formula: "Seed Dispersal Terminal Velocity: v_t = sqrt((2.0 * m_seed * g) / (rho_air * C_D * A_wing)); Smoke Karrikin Germination: [KAR1] >= 1e-9 M",
        educational_summary: "Each capsule contains a few fertile, saucer-shaped or narrowly winged seeds surrounded by numerous reddish-brown sterile chaff particles. Seed germination is stimulated by butenolide karrikins present in bushfire smoke, timing seedling emergence to nutrient-rich post-fire ash beds.",
        indigenous_custodial_context: "Women harvested fertile eucalypt seeds by shaking branches over bark dishes ('coolamons'), winnowing chaff in the wind, and grinding the nutritious seeds into flour to bake damper on hot river stones.",
    },

    // ═════════════════════════════════════════════════════════════════════════
    // SCALE 4: FOLIAR LAMINA & HISTOLOGY CROSS-SECTION (50 µm – 10 mm)
    // ═════════════════════════════════════════════════════════════════════════
    AnatomicalCalloutPin {
        pin_id: "pin_s4_cuticle_wax",
        scale: 4,
        ontology_curie: "PO:0000014",
        scientific_term: "Epicuticular Waxy Cuticle & Crystalline Platelets",
        anchor_local_xyz: [0.0, 0.004, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.006, 0.025],
        biophysical_formula: "Cuticular Transpiration Resistance: r_c = delta_x / (D_wax * S_sol) >= 15000 s/m; Contact Angle: theta_contact >= 115 deg (Superhydrophobic)",
        educational_summary: "The outermost protective layer consisting of a cutin polymer matrix embedded with crystalline triterpenoid and diketone wax platelets. Gives glaucous Snow Gum and Ghost Gum leaves their chalky-white hue, reflecting high UV-B radiation and preventing non-stomatal water loss.",
        indigenous_custodial_context: "The thick glaucous wax was scraped from leaves with bone scrapers to mix with ochre pigments, producing water-resistant ceremonial body paints and rock shelter art that lasted through torrential storms.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s4_amphistomatic_stomata",
        scale: 4,
        ontology_curie: "PO:0000032",
        scientific_term: "Amphistomatic Stomatal Complex & Guard Cell Crypt",
        anchor_local_xyz: [0.002, 0.0035, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.003, 0.005, 0.015],
        biophysical_formula: "Fick's Stomatal Diffusion: J_v = g_s * (e_i - e_a) / P; Stomatal Conductance g_s = (d_stoma * A_pore) / (l_channel + 0.5 * pi * r_pore)",
        educational_summary: "Unlike most Northern Hemisphere broadleaf trees with stomata only beneath the leaf (hypostomatic), eucalypts possess stomata on both upper and lower leaf faces (amphistomatic). Paired guard cells modulate pore diameter based on potassium and ABA gradients, operating within sunken epistomatal crypts to reduce boundary layer wind stripping.",
        indigenous_custodial_context: "Custodial observers noted that leaf cooling via transpiration occurred symmetrically on both sides of vertically hanging leaves, correlating leaf wilting angles with river drought severity.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s4_isobilateral_palisade",
        scale: 4,
        ontology_curie: "PO:0005679",
        scientific_term: "Isobilateral Palisade Mesophyll (Dual Columnar Layers)",
        anchor_local_xyz: [0.0, 0.002, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.004, 0.018],
        biophysical_formula: "Farquhar-von Caemmerer C3 Photosynthesis: A = min(W_c, W_j, W_p) - R_d; Quantum Yield Phi = (1.0 - 0.5 * O / (S_c/o * C)) / (4.0 + 8.0 * O / (S_c/o * C))",
        educational_summary: "A diagnostic anatomical adaptation to vertical leaf orientation: dense, columnar palisade chlorenchyma cells packed with chloroplasts flank both adaxial and abaxial leaf surfaces, enclosing only a thin central band of spongy parenchyma. This doubles photosynthetic light harvesting efficiency under changing sun angles.",
        indigenous_custodial_context: "The dense green flesh of crushed leaves released high concentrations of active chlorophyll and cineole, utilized as an antiseptic compress for skin infections and abrasions.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s4_intramarginal_vein",
        scale: 4,
        ontology_curie: "PO:0005023",
        scientific_term: "Intramarginal Peripheral Vein & Murray's Law Anastomosis",
        anchor_local_xyz: [0.008, 0.0015, 0.0],
        normal_xyz: [1.0, 0.0, 0.0],
        camera_focus_offset: [0.012, 0.003, 0.020],
        biophysical_formula: "Murray's Law Vein Branching: r_parent^3 = r_1^3 + r_2^3; Intramarginal Loop Pressure: P_margin = P_midrib - (8.0 * eta * L * Q) / (pi * r_intra^4)",
        educational_summary: "The continuous intramarginal vein running just inside the blade margin is the hallmark taxonomic signature of Myrtaceae foliar venation. Secondary lateral veins merge into this loop, providing redundant hydraulic bypass circuits that keep leaf edges hydrated even if insect herbivores chew through primary lateral veins.",
        indigenous_custodial_context: "The intramarginal vein's clear outline was used by trackers as a diagnostic key to distinguish medicinal cineole-rich eucalypts from toxic or bitter lookalike species.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s4_schizogenous_oil_gland",
        scale: 4,
        ontology_curie: "PO:0005018",
        scientific_term: "Schizogenous Essential Oil Cavity (1,8-Cineole / Pinene)",
        anchor_local_xyz: [0.004, 0.0018, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.006, 0.003, 0.012],
        biophysical_formula: "Terpenoid Synthesis Flux: J_terp = k_DXP * [IPP] * [DMAPP]; Cavity Expansion Energy: delta_G_schizo = gamma_surf * delta_A_cavity",
        educational_summary: "Spherical sub-epidermal cavities lined with secretory epithelial cells that synthesize and store volatile monoterpenes and sesquiterpenes (1,8-cineole, alpha-pinene, p-cymene). The glands appear as translucent dots when held to the sun, providing potent defense against phytophagous insects and fungal hyphae.",
        indigenous_custodial_context: "Smoking ceremonies ('dhaagun') burned oil-rich green leaves over glowing coals, releasing cineole vapors that purified meeting grounds, cleansed spiritual pathways, and acted as an effective airborne insect repellent.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s4_sclerenchyma_bundle_sheath",
        scale: 4,
        ontology_curie: "PO:0005020",
        scientific_term: "Collateral Vascular Bundle & Sclerenchyma Sheath Girders",
        anchor_local_xyz: [0.0, 0.001, 0.0],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [0.0, 0.002, 0.015],
        biophysical_formula: "Leaf Elastic Modulus: E_blade = f_scler * E_scler + (1.0 - f_scler) * E_paren; Lignin Cross-Linking Stress: sigma_yield >= 22.5 MPa",
        educational_summary: "Each foliar vein is wrapped in a robust sheath of thick-walled sclerenchyma fibres that extend as vertical I-beam girders reaching all the way to both upper and lower epidermises. This architecture endows the leaf with extreme sclerophylly—resisting mechanical wilting collapse under negative turgor pressure.",
        indigenous_custodial_context: "Tough sclerophyllous leaf skeletons that persisted in creek beds after soft tissues decayed were gathered and tied into delicate filter strainers for sifting river gravels and panning native seed.",
    },

    // ═════════════════════════════════════════════════════════════════════════
    // SCALE 5: COMPARATIVE VERTEBRATE ANATOMY
    // ═════════════════════════════════════════════════════════════════════════
    AnatomicalCalloutPin {
        pin_id: "pin_s5_epipubic_bone",
        scale: 5,
        ontology_curie: "UBERON:0003661",
        scientific_term: "Epipubic Bone (Os Epipubicum) & Pouch Support",
        anchor_local_xyz: [-0.08, 0.72, 0.12],
        normal_xyz: [-0.45, 0.72, 0.52],
        camera_focus_offset: [-0.15, 0.85, 0.65],
        biophysical_formula: "Cantilever Bending Moment: M_b = F_pouch_young * L_epipubic; Muscle Anchor Strain: sigma_rectus = E_tendon * (delta_L / L_0)",
        educational_summary: "A pair of forward-projecting pelvic bones articulating with the pubis, unique to Marsupialia and Monotremata. Contrary to early hypotheses of passive pouch support, epipubic bones function as dynamic muscular levers that anchor the pectineus, pyramidalis, and rectus abdominis muscles, stiffening the torso during saltatorial locomotion.",
        indigenous_custodial_context: "Aboriginal hunters and butchers possessed deep anatomical familiarity with epipubic bones ('mimi'), which were carefully separated during preparation of kangaroo carcases ('marloo') and carved into fine awls for piercing possum skin cloaks.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s5_achilles_tendon",
        scale: 5,
        ontology_curie: "UBERON:0001434",
        scientific_term: "Elastic Achilles Tendon Energy-Recovery Spring",
        anchor_local_xyz: [-0.14, 0.28, -0.08],
        normal_xyz: [-0.55, 0.0, -0.83],
        camera_focus_offset: [-0.25, 0.35, 0.55],
        biophysical_formula: "Hookean Tendon Strain Energy: U = 0.5 * k_t * delta_L^2; Energy Recovery: eta_rebound = 0.65 (recovers 65% of mechanical hopping work per stride)",
        educational_summary: "The hypertrophied calcaneal tendon of macropods acts as an biological pogo stick. At high speeds (above 15 km/h), the tendon stretches upon impact and recoils elastically, dramatically reducing the metabolic cost of transport so that kangaroos expend less oxygen at high hopping speeds than at slow speeds.",
        indigenous_custodial_context: "Kangaroo leg sinew ('ngulu' or 'moeyan') was extracted intact, dried in the sun, and chewed until supple. It was the premier binding material across Australia for securing spear points to shafts, hafting stone axes, and repairing wooden tools.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s5_diprotodont_incisors",
        scale: 5,
        ontology_curie: "UBERON:0001759",
        scientific_term: "Diprotodont Dentition & Procumbent Lower Incisors",
        anchor_local_xyz: [0.0, 0.95, 0.42],
        normal_xyz: [0.0, 0.25, 0.97],
        camera_focus_offset: [0.0, 1.05, 0.55],
        biophysical_formula: "Masticatory Lever Law: F_bite = F_masseter * (L_in / L_out); Shearing Mechanical Advantage MA = 0.62",
        educational_summary: "The diagnostic trait of Order Diprotodontia (kangaroos, wombats, koalas, possums): a single pair of elongated, forward-pointing (procumbent) lower incisors that scissor against upper incisors across a wide diastema. In kangaroos, horizontal molar progression continuously replaces worn teeth.",
        indigenous_custodial_context: "The razor-sharp, enamel-coated lower incisors of kangaroos were embedded in grass-tree resin on hardwood handles to create precision carving chisels ('koondi') used to carve fluting on boomerangs and shields.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s5_syndactyl_grooming_pes",
        scale: 5,
        ontology_curie: "UBERON:0001452",
        scientific_term: "Syndactyl Grooming Digits II & III and Hopping Digit IV",
        anchor_local_xyz: [-0.12, 0.02, 0.16],
        normal_xyz: [0.0, 1.0, 0.0],
        camera_focus_offset: [-0.18, 0.15, 0.40],
        biophysical_formula: "Digit IV Ground Reaction Force: F_GRF = m_body * (g + a_hop); Syndactyly Fusion Index: W_skin_sheath = 2 * r_digit",
        educational_summary: "The macropod hindfoot exhibits extreme morphological specialization: digits II and III are fused inside a single skin sheath (syndactyly) with dual twin claws used exclusively for combing fur and grooming ectoparasites, while digit IV is hyper-elongated into the primary hopping propulsion column.",
        indigenous_custodial_context: "Traditional trackers read the unique footprint of the single prominent 4th toe claw print to identify species, age, speed, carrying weight, and whether a kangaroo was foraging, resting, or fleeing.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s5_keeled_carina",
        scale: 5,
        ontology_curie: "UBERON:0001006",
        scientific_term: "Avian Keeled Carina Sternum & Furcula Wishbone",
        anchor_local_xyz: [0.0, 0.55, 0.22],
        normal_xyz: [0.0, -1.0, 0.0],
        camera_focus_offset: [0.0, 0.45, 0.50],
        biophysical_formula: "Flight Aerodynamic Power: P_aero = 0.5 * rho_air * V^3 * C_D * S + (2.0 * (m * g)^2) / (pi * rho_air * V * b^2); Carina Area A_carina = f(m_pectoralis)",
        educational_summary: "In soaring raptors (*Aquila audax* Wedge-Tailed Eagle) and flying birds, the sternum projects a deep, blade-like keel (carina) providing expansive surface area for the massive pectoralis major (downstroke power) and supracoracoideus (upstroke pulley). In ratites (emus/cassowaries), the carina is lost.",
        indigenous_custodial_context: "The Wedge-Tailed Eagle ('Maliyan' or 'Bunjil') holds supreme ancestral significance across southeastern Australia as an eagle-hawk creator entity; eagle breastbones and flight feathers are sacred ceremonial regalia.",
    },
    AnatomicalCalloutPin {
        pin_id: "pin_s5_caudal_counterbalance",
        scale: 5,
        ontology_curie: "UBERON:0002415",
        scientific_term: "Caudal Vertebrae & Ventral Chevron Bones (Pentapedal Limb)",
        anchor_local_xyz: [0.0, 0.45, -0.65],
        normal_xyz: [0.0, 0.35, -0.94],
        camera_focus_offset: [0.0, 0.55, -0.30],
        biophysical_formula: "Caudal Angular Momentum Balance: I_tail * d_omega_tail/dt = -I_body * d_omega_body/dt; Pentapedal Ground Reaction: F_tail >= 0.15 * m_body * g",
        educational_summary: "In macropods, the tail is not merely a passive counterbalance; robust articulated caudal vertebrae with heavy ventral chevron bones (haemal arches) provide muscular lever arms that generate vertical ground reaction forces exceeding 15% of body weight, acting as a genuine fifth limb during pentapedal crawling.",
        indigenous_custodial_context: "Tail meat ('marloo wipu') with its rich gelatinous connective tissue was slow-roasted in earth pits ('murnong' ovens) over fragrant eucalypt coals, producing a highly prized tender feast shared communally.",
    },
];

// ── Pin Lookup & Query Helpers ───────────────────────────────────────────────

/// Finds a canonical pin by its static string identifier.
pub fn find_pin_by_id(pin_id: &str) -> Option<&'static AnatomicalCalloutPin> {
    CANONICAL_ANATOMICAL_PINS.iter().find(|p| p.pin_id == pin_id)
}

/// Returns an iterator over all canonical pins belonging to a given biological scale (`1..=5`).
pub fn find_pins_by_scale(scale: u8) -> impl Iterator<Item = &'static AnatomicalCalloutPin> {
    CANONICAL_ANATOMICAL_PINS.iter().filter(move |p| p.scale == scale)
}

// ── Zero-Heap Ray-Sphere Picking ─────────────────────────────────────────────

/// Queries the closest anatomical pin to a picking ray in zero heap allocations.
///
/// If `scale` is `1..=5`, only pins matching that scale are considered.
/// If `scale == 0`, all scales are matched.
///
/// Uses geometric ray-sphere proximity test. Returns `Some(&'static AnatomicalCalloutPin)`
/// if an anchor is within `max_dist` of the ray.
pub fn query_closest_anatomical_pin_zero_heap(
    scale: u8,
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    max_dist: f32,
) -> Option<&'static AnatomicalCalloutPin> {
    let dir_len_sq = ray_dir[0] * ray_dir[0] + ray_dir[1] * ray_dir[1] + ray_dir[2] * ray_dir[2];
    if dir_len_sq < 1e-12 {
        return None;
    }
    let inv_len = 1.0 / dir_len_sq.sqrt();
    let d = [ray_dir[0] * inv_len, ray_dir[1] * inv_len, ray_dir[2] * inv_len];

    let mut best_pin: Option<&'static AnatomicalCalloutPin> = None;
    let mut min_t = f32::INFINITY;
    let max_dist_sq = max_dist * max_dist;

    for pin in CANONICAL_ANATOMICAL_PINS {
        if scale != 0 && pin.scale != scale {
            continue;
        }

        let p = pin.anchor_local_xyz;
        let v = [p[0] - ray_origin[0], p[1] - ray_origin[1], p[2] - ray_origin[2]];
        let t = v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
        if t < 0.0 {
            // Anchor is behind the ray origin
            continue;
        }

        // Closest point on ray: c = ray_origin + t * d
        let c = [
            ray_origin[0] + t * d[0],
            ray_origin[1] + t * d[1],
            ray_origin[2] + t * d[2],
        ];

        let dist_sq = (p[0] - c[0]) * (p[0] - c[0])
            + (p[1] - c[1]) * (p[1] - c[1])
            + (p[2] - c[2]) * (p[2] - c[2]);

        if dist_sq <= max_dist_sq && t < min_t {
            min_t = t;
            best_pin = Some(pin);
        }
    }

    best_pin
}

// ── Q42 Semantic Graph Lexicon & Quin Emission ───────────────────────────────

/// Builds an in-memory string lexicon for all canonical anatomical pins ready for Q42 volume authoring.
pub fn build_anatomical_ontology_lexicon() -> HashMap<u64, String> {
    let mut lex = HashMap::with_capacity(CANONICAL_ANATOMICAL_PINS.len() * 10 + 32);

    let base_strings = [
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        "urn:qualia:bio:AnatomicalCalloutPin",
        "urn:qualia:bio:hasScale",
        "urn:qualia:bio:hasOntologyCurie",
        "urn:qualia:bio:scientificTerm",
        "urn:qualia:bio:biophysicalFormula",
        "urn:qualia:bio:educationalSummary",
        "urn:qualia:bio:indigenousCustodialContext",
        "http://purl.obolibrary.org/obo/BFO_0000050", // part_of
        "urn:qualia:bio:anchorLocalX",
        "urn:qualia:bio:anchorLocalY",
        "urn:qualia:bio:anchorLocalZ",
    ];

    for s in &base_strings {
        lex.insert(q_hash(s), (*s).to_string());
    }

    for pin in CANONICAL_ANATOMICAL_PINS {
        let pin_uri = format!("urn:qualia:bio:pin:{}", pin.pin_id);
        lex.insert(q_hash(&pin_uri), pin_uri);

        let curie = pin.ontology_curie;
        lex.insert(q_hash(curie), curie.to_string());

        let iri = pin.full_ontology_iri();
        lex.insert(q_hash(&iri), iri);

        let term = pin.scientific_term;
        lex.insert(q_hash(term), term.to_string());

        let formula = pin.biophysical_formula;
        lex.insert(q_hash(formula), formula.to_string());

        let summary = pin.educational_summary;
        lex.insert(q_hash(summary), summary.to_string());

        let context = pin.indigenous_custodial_context;
        lex.insert(q_hash(context), context.to_string());
    }

    lex
}

/// Encodes all canonical anatomical callout pins and their biological ontology bindings into
/// a streaming Q42 volume writer, ensuring all Quin blocks are globally object-sorted.
///
/// Returns the total number of semantic Quins written into the Q42 volume.
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_anatomical_ontology_nquins_to_writer(
    writer: &mut StreamingQ42VolumeWriter,
) -> io::Result<usize> {
    let p_type = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let o_pin_type = q_hash("urn:qualia:bio:AnatomicalCalloutPin");
    let p_scale = q_hash("urn:qualia:bio:hasScale");
    let p_curie = q_hash("urn:qualia:bio:hasOntologyCurie");
    let p_term = q_hash("urn:qualia:bio:scientificTerm");
    let p_formula = q_hash("urn:qualia:bio:biophysicalFormula");
    let p_summary = q_hash("urn:qualia:bio:educationalSummary");
    let p_context = q_hash("urn:qualia:bio:indigenousCustodialContext");
    let p_part_of = q_hash("http://purl.obolibrary.org/obo/BFO_0000050");

    let mut quins: Vec<NQuin> = Vec::with_capacity(CANONICAL_ANATOMICAL_PINS.len() * 8);

    for pin in CANONICAL_ANATOMICAL_PINS {
        let pin_s = q_hash(&format!("urn:qualia:bio:pin:{}", pin.pin_id));

        // 1. rdf:type AnatomicalCalloutPin
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_type,
            object: o_pin_type,
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 2. hasScale (scale 1..=5)
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_scale,
            object: pin.scale as u64,
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 3. hasOntologyCurie
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_curie,
            object: q_hash(pin.ontology_curie),
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 4. part_of Full Ontology IRI
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_part_of,
            object: q_hash(&pin.full_ontology_iri()),
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 5. scientificTerm
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_term,
            object: q_hash(pin.scientific_term),
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 6. biophysicalFormula
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_formula,
            object: q_hash(pin.biophysical_formula),
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 7. educationalSummary
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_summary,
            object: q_hash(pin.educational_summary),
            context: 0,
            metadata: 0,
            parity: 0,
        });

        // 8. indigenousCustodialContext
        quins.push(NQuin {
            subject: pin_s,
            predicate: p_context,
            object: q_hash(pin.indigenous_custodial_context),
            context: 0,
            metadata: 0,
            parity: 0,
        });
    }

    // Sort strictly by object ascending to satisfy Q42 BIDX SuperBlock invariant
    quins.sort_by_key(|q| q.object);

    let total_quins = quins.len();
    let mut seq_id = 0u64;

    for chunk in quins.chunks(QUINS_PER_BLOCK) {
        writer.push_block(seq_id, chunk)?;
        seq_id += 1;
    }

    Ok(total_quins)
}

// ── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_pins_catalog() {
        // 1. Verify pin count (must be >= 24)
        assert!(
            CANONICAL_ANATOMICAL_PINS.len() >= 24,
            "Expected at least 24 pins, got {}",
            CANONICAL_ANATOMICAL_PINS.len()
        );

        // 2. Verify all scales 1..=5 are represented
        for scale in 1..=5 {
            let count = CANONICAL_ANATOMICAL_PINS.iter().filter(|p| p.scale == scale).count();
            assert!(count >= 3, "Scale {} has {} pins, expected at least 3", scale, count);
        }

        // 3. Verify non-empty CURIEs, terms, formulas, summaries, and indigenous contexts
        for pin in CANONICAL_ANATOMICAL_PINS {
            assert!(!pin.pin_id.is_empty());
            assert!(pin.scale >= 1 && pin.scale <= 5);
            assert!(pin.ontology_curie.starts_with("PO:") || pin.ontology_curie.starts_with("UBERON:"));
            assert!(!pin.scientific_term.is_empty());
            assert!(!pin.biophysical_formula.is_empty());
            assert!(!pin.educational_summary.is_empty());
            assert!(!pin.indigenous_custodial_context.is_empty());

            // Verify IRI generation
            let iri = pin.full_ontology_iri();
            assert!(iri.starts_with("http://purl.obolibrary.org/obo/"));

            // Verify normal is roughly unit length
            let n_len = (pin.normal_xyz[0].powi(2) + pin.normal_xyz[1].powi(2) + pin.normal_xyz[2].powi(2)).sqrt();
            assert!(n_len > 0.8 && n_len < 1.2, "Pin {} normal not unit length: {}", pin.pin_id, n_len);
        }
    }

    #[test]
    fn test_pin_lookups() {
        let pin_ligno = find_pin_by_id("pin_s1_lignotuber").expect("Must find lignotuber pin");
        assert_eq!(pin_ligno.ontology_curie, "PO:0025531");
        assert_eq!(pin_ligno.scale, 1);

        let pin_achilles = find_pin_by_id("pin_s5_achilles_tendon").expect("Must find Achilles tendon pin");
        assert_eq!(pin_achilles.ontology_curie, "UBERON:0001434");
        assert_eq!(pin_achilles.scale, 5);

        let pin_epipubic = find_pin_by_id("pin_s5_epipubic_bone").expect("Must find epipubic bone pin");
        assert_eq!(pin_epipubic.ontology_curie, "UBERON:0003661");
        assert_eq!(pin_epipubic.scale, 5);

        let scale4_count = find_pins_by_scale(4).count();
        assert!(scale4_count >= 5, "Expected at least 5 Scale 4 histology pins, got {}", scale4_count);
    }

    #[test]
    fn test_ray_picking_zero_heap() {
        // Direct hit against lignotuber pin at [0.0, -0.25, 0.0]
        let ray_origin = [0.0, 0.25, 0.0];
        let ray_dir = [0.0, -1.0, 0.0]; // Points straight down at the subterranean lignotuber

        let picked = query_closest_anatomical_pin_zero_heap(1, ray_origin, ray_dir, 0.5);
        assert!(picked.is_some(), "Ray directed at lignotuber must hit");
        assert_eq!(picked.unwrap().pin_id, "pin_s1_lignotuber");

        // Ray pointing away from all pins
        let away_ray_origin = [100.0, 100.0, 100.0];
        let away_ray_dir = [1.0, 0.0, 0.0];
        let no_hit = query_closest_anatomical_pin_zero_heap(0, away_ray_origin, away_ray_dir, 1.0);
        assert!(no_hit.is_none(), "Ray pointing into empty space must not hit");

        // Scale filtering: ray hits lignotuber (scale 1), but querying scale 2 must return None or scale 2 pin
        let scale2_query = query_closest_anatomical_pin_zero_heap(2, ray_origin, ray_dir, 0.1);
        assert!(scale2_query.is_none() || scale2_query.unwrap().scale == 2);
    }

    #[test]
    fn test_loupe_dissection_state_controls() {
        let mut loupe = LoupeDissectionState::new();
        assert_eq!(loupe.active_scale, 1);
        loupe.select_scale(4);
        assert_eq!(loupe.active_scale, 4);
        assert_eq!(loupe.selected_ontology_curie, "po:0000032");

        loupe.set_layer_peel(0.75);
        assert_eq!(loupe.cutaway_plane_01, 0.75);
        loupe.set_anthesis_progress(0.9);
        assert_eq!(loupe.ontogenetic_progress_01, 0.9);
        loupe.set_explode_layer(0.5);
        assert_eq!(loupe.explode_layers_01, 0.5);
        loupe.set_vertebrate_layer_visibility(1.0, 0.5, 0.0);
        assert_eq!(loupe.osteology_vis_01, 1.0);
        assert_eq!(loupe.myology_vis_01, 0.5);
        assert_eq!(loupe.integument_vis_01, 0.0);

        loupe.select_scale(1);
        let picked = loupe.pick_callout_pin([0.0, 0.25, 0.0], [0.0, -1.0, 0.0], 0.5);
        assert!(picked.is_some());
        assert_eq!(loupe.selected_ontology_curie, "PO:0025531");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_q42_emission() {
        let lex = build_anatomical_ontology_lexicon();
        assert!(lex.len() >= 100, "Lexicon should contain >= 100 entries, got {}", lex.len());

        let mut writer = StreamingQ42VolumeWriter::new(&lex).expect("Writer creation must succeed");
        let count = emit_anatomical_ontology_nquins_to_writer(&mut writer)
            .expect("Quin emission must succeed");

        assert!(count >= CANONICAL_ANATOMICAL_PINS.len() * 6);

        let temp_path = std::env::temp_dir().join(format!(
            "test_botanical_ontology_{}.q42",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        writer.finish(&temp_path).expect("Q42 volume finish must succeed");

        let volume = crate::q42::q42_volume::Q42Volume::open(&temp_path)
            .expect("Q42 volume open must succeed");
        assert!(volume.block_count() > 0);

        let _ = std::fs::remove_file(&temp_path);
    }
}
