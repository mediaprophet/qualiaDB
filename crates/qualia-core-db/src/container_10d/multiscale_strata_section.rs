//! Multi-Scale Nested Computational Strata (`MSTR`, `0x0304`) and In-Envelope
//! Symbolic-AI / RDF 1.2 / Multilingual CBOR-LD (`CBLD`, `0x0305`) Codecs.
//!
//! In QualiaDB, the visual (3D/4D mesh) or auditory (`.qac`) representation is
//! a secondary sensory projection of a primary multi-scale neuro-symbolic
//! computational model. While a macro visual asset may initially depict only
//! the outer surface of a leaf, skin, or structural assembly, the underlying
//! `.10d` computational manifold encodes the nested strata of that entity across
//! seven scale tiers (`L0`..=`L6`):
//!
//! - **`L0` (`MacroHolobiontOrAssembly`):** Complete organism, fungal network, or made-world assembly.
//! - **`L1` (`OrganOrSubsystem`):** Leaf lamina, vascular cambium, heart, root system; engine block, PCB.
//! - **`L2` (`TissueOrFollicle`):** Hair follicles, trichomes, stomatal complexes, epidermis, mesophyll, trabeculae.
//! - **`L3` (`HarbouredConsortia`):** Co-habiting mycorrhizal/endophytic fungi, bacterial biofilms, microbiome, and parasites.
//! - **`L4` (`CellularOrOrganellar`):** Somatic/meristem cells, chloroplasts, mitochondria, cell membranes, turgor & voltage.
//! - **`L5` (`MolecularOrGenomicPathway`):** DNA/RNA/proteins, enzymatic pathways, biopolymers, crystal lattices.
//! - **`L6` (`L6QuantumPhysics`):** Foundational non-classical **Quantum Physics**—encompassing all quantum-mechanical
//!   considerations that cannot be reduced to general/classical physics or continuum mathematics (with Quantum Dynamics
//!   and Quantum Chemistry as sub-domains):
//!   - **Quantum Coherence, Superposition & Entanglement:** Non-commutative observables ($[\hat{A}, \hat{B}] \neq 0$),
//!     density matrices ($\hat{\rho}$), von Neumann entanglement entropy ($S_{\text{vN}}$), and non-local correlations.
//!   - **Open-System Quantum Dynamics, Tunneling & Spin Physics:** GKSL Lindblad / Redfield / HEOM decoherence ($T_1, T_2^*$),
//!     WKB/instanton proton & electron barrier tunneling, vibronic exciton coherence, and Zeeman/hyperfine radical-pair spin dynamics.
//!   - **Quantum Electrodynamics (QED) & Condensed/Topological Phases:** Cavity QED / polaritonic light-matter coupling,
//!     Casimir/van der Waals dispersion, Berry curvature, Chern invariants, and superconducting/many-body condensates.
//!   - **Quantum Chemistry Subset (Electronic Structure):** GTO basis sets (STO-3G, def2-SVP, ECPs), OS/HGP/Rys integrals,
//!     RHF/UHF DIIS/Broyden SCF ($10^{-8}$ Ha), Rust-native autodiff LDA/GGA DFT, and Q-Forge VQE/QUBO StateVector `NQuin` pointers.
//!
//! **Strict No-ASCII Policy:** All human-readable terms, multilingual taxonomic labels,
//! RDF IRIs, and literals use **UTF-8** (with deterministic lossless **UTF-16** bridge
//! helpers for UTF-16 platform/ABI consumers) inside the CBOR-LD (`CBLD`, `0x0305`)
//! envelope and `.q42` `Q42LEX` tables. Fixed 32-bit binary header tags are numeric
//! format discriminants, never ASCII text strings.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};
use half::f16;
use serde::{Deserialize, Serialize};

use crate::container_10d::crc32c::crc32c;
use crate::topology::header::{CellAnchor, TopologyInvariants};
use crate::NQuin;

/// 32-bit binary format discriminant for `MSTR` multi-scale strata payloads (`0x5254_534D` LE).
pub const MULTISCALE_STRATA_TAG: [u8; 4] = *b"MSTR";
pub const MULTISCALE_STRATA_VERSION: u16 = 1;
pub const MULTISCALE_STRATA_HEADER_SIZE: usize = 32;
pub const NESTED_STRATUM_NODE_SIZE: usize = 64;
pub const MAX_NESTED_STRATUM_NODES: usize = 65_535;
pub const STRATUM_NO_PARENT: u16 = 0xFFFF;

/// 32-bit binary format discriminant for `CBLD` Symbolic-AI / RDF / CBOR-LD payloads (`0x444C_4243` LE).
pub const SYMBOLIC_CBOR_LD_TAG: [u8; 4] = *b"CBLD";
pub const SYMBOLIC_CBOR_LD_VERSION: u16 = 1;
pub const SYMBOLIC_CBOR_LD_HEADER_SIZE: usize = 16;
pub const MAX_SYMBOLIC_CBOR_LD_BYTES: usize = 16 * 1024 * 1024;

/// Scale tier (`L0`..=`L6`) in the 7-layer multi-scale computational hierarchy.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputationalStratumLevel {
    /// `L0` ($10^2\text{ m} \dots 10^{-1}\text{ m}$): Complete macro organism, holobiont, or assembly.
    L0MacroHolobiontOrAssembly = 0,
    /// `L1` ($10^{-1}\text{ m} \dots 10^{-2}\text{ m}$): Organ, anatomical compartment, or structural subsystem.
    L1OrganOrSubsystem = 1,
    /// `L2` ($10^{-2}\text{ m} \dots 10^{-4}\text{ m}$): Tissue architecture, hair follicles, trichomes, stomata.
    L2TissueOrFollicle = 2,
    /// `L3` ($10^{-3}\text{ m} \dots 10^{-6}\text{ m}$): Harboured symbiotic fungi, bacteria, microbiome, and parasites.
    L3HarbouredConsortia = 3,
    /// `L4` ($10^{-4}\text{ m} \dots 10^{-7}\text{ m}$): Individual cell populations and sub-cellular organelles.
    L4CellularOrOrganellar = 4,
    /// `L5` ($10^{-7}\text{ m} \dots 10^{-9}\text{ m}$): Genomic, proteomic, metabolic, and biopolymer/lattice pathways.
    L5MolecularOrGenomicPathway = 5,
    /// `L6` ($10^{-9}\text{ m} \dots 10^{-15}\text{ m}$): Non-classical Quantum Physics (open-system quantum dynamics,
    /// coherence, entanglement, tunneling, spin/radical-pair physics, cavity QED, topological phases, and the quantum chemistry subset).
    L6QuantumPhysics = 6,
}

impl ComputationalStratumLevel {
    /// Backwards-compatible alias for [`Self::L6QuantumPhysics`].
    #[allow(non_upper_case_globals)]
    pub const L6QuantumChemistry: Self = Self::L6QuantumPhysics;

    /// Backwards-compatible alias for [`Self::L6QuantumPhysics`].
    #[allow(non_upper_case_globals)]
    pub const L6QuantumPhysicsDynamicsAndChemistry: Self = Self::L6QuantumPhysics;

    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::L0MacroHolobiontOrAssembly),
            1 => Some(Self::L1OrganOrSubsystem),
            2 => Some(Self::L2TissueOrFollicle),
            3 => Some(Self::L3HarbouredConsortia),
            4 => Some(Self::L4CellularOrOrganellar),
            5 => Some(Self::L5MolecularOrGenomicPathway),
            6 => Some(Self::L6QuantumPhysics),
            _ => None,
        }
    }
}

/// Specific constituent classification within a [`ComputationalStratumLevel`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StratumConstituentKind {
    // L0..=L2 Anatomical / Structural
    HolobiontOrSystemRoot = 0x00,
    OrganOrSubAssembly = 0x01,
    TissueLayer = 0x02,
    FollicleOrTrichome = 0x03,
    StomatalOrPoreComplex = 0x04,
    VascularOrFluidConduit = 0x05,
    // L3 Harboured Consortia (Funga, Bacteriome, Parasitome)
    MycorrhizalFungus = 0x10,
    EndophyticOrDermatophyticFungus = 0x11,
    BacterialBiofilmOrMicrobiome = 0x12,
    Ectoparasite = 0x13,
    EndoparasiteOrPathogen = 0x14,
    ViralOrSubcellularAgent = 0x15,
    // L4 Cellular & Organellar
    SomaticOrParenchymaCell = 0x20,
    StemOrMeristemCell = 0x21,
    ChloroplastOrPlastid = 0x22,
    Mitochondrion = 0x23,
    CellWallOrMembrane = 0x24,
    // L5 Molecular & Genomic
    GenomicOrProteomicSequence = 0x30,
    EnzymaticReactionCenter = 0x31,
    BiopolymerOrCrystalLattice = 0x32,
    // L6 Quantum Physics, Quantum Dynamics & Quantum Chemistry (Non-Classical Considerations)
    QuantumGtoHamiltonian = 0x40,
    DftExchangeCorrelationGrid = 0x41,
    QForgeVqeStateVector = 0x42,
    /// Open-system quantum dynamics: GKSL Lindblad / Redfield / HEOM density matrix $\hat{\rho}$ evolution & decoherence ($T_1, T_2^*$).
    OpenQuantumLindbladDynamics = 0x43,
    /// Quantum barrier tunneling (WKB / instanton proton & electron tunneling) and vibronic exciton-phonon coherence.
    QuantumTunnelingAndVibronicTransport = 0x44,
    /// Non-classical spin Hamiltonian, Zeeman/hyperfine radical-pair singlet-triplet dynamics, magnetoreception, and spintronics.
    SpinDynamicsAndRadicalPair = 0x45,
    /// Cavity QED, Jaynes-Cummings polaritonic light-matter entanglement, Casimir/van der Waals dispersion, and Fock-state optics.
    CavityQedAndPhotonicCoupling = 0x46,
    /// Topological & many-body quantum phases: Berry curvature, Chern invariant, superconducting/BEC condensates, and lattice Hamiltonians.
    TopologicalAndManyBodyQuantumPhase = 0x47,
}

impl StratumConstituentKind {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0x00 => Some(Self::HolobiontOrSystemRoot),
            0x01 => Some(Self::OrganOrSubAssembly),
            0x02 => Some(Self::TissueLayer),
            0x03 => Some(Self::FollicleOrTrichome),
            0x04 => Some(Self::StomatalOrPoreComplex),
            0x05 => Some(Self::VascularOrFluidConduit),
            0x10 => Some(Self::MycorrhizalFungus),
            0x11 => Some(Self::EndophyticOrDermatophyticFungus),
            0x12 => Some(Self::BacterialBiofilmOrMicrobiome),
            0x13 => Some(Self::Ectoparasite),
            0x14 => Some(Self::EndoparasiteOrPathogen),
            0x15 => Some(Self::ViralOrSubcellularAgent),
            0x20 => Some(Self::SomaticOrParenchymaCell),
            0x21 => Some(Self::StemOrMeristemCell),
            0x22 => Some(Self::ChloroplastOrPlastid),
            0x23 => Some(Self::Mitochondrion),
            0x24 => Some(Self::CellWallOrMembrane),
            0x30 => Some(Self::GenomicOrProteomicSequence),
            0x31 => Some(Self::EnzymaticReactionCenter),
            0x32 => Some(Self::BiopolymerOrCrystalLattice),
            0x40 => Some(Self::QuantumGtoHamiltonian),
            0x41 => Some(Self::DftExchangeCorrelationGrid),
            0x42 => Some(Self::QForgeVqeStateVector),
            0x43 => Some(Self::OpenQuantumLindbladDynamics),
            0x44 => Some(Self::QuantumTunnelingAndVibronicTransport),
            0x45 => Some(Self::SpinDynamicsAndRadicalPair),
            0x46 => Some(Self::CavityQedAndPhotonicCoupling),
            0x47 => Some(Self::TopologicalAndManyBodyQuantumPhase),
            _ => None,
        }
    }
}

/// Functional, ecological, or non-classical quantum interaction regime between a nested stratum node and its host.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StratumInteractionRegime {
    StructuralConstituent = 0,
    MutualistSymbiont = 1,
    Commensal = 2,
    Parasitic = 3,
    Pathogenic = 4,
    CatalyticOrMetabolic = 5,
    ElectronicExcitation = 6,
    /// Non-separable quantum coherence, superposition, and entanglement coupling ($\hat{\rho}$ off-diagonal coherence, von Neumann entropy).
    QuantumCoherenceAndEntanglement = 7,
    /// Barrier tunneling, non-adiabatic conical intersection, and spin-selective radical-pair coupling.
    QuantumTunnelingAndNonAdiabatic = 8,
}

impl StratumInteractionRegime {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::StructuralConstituent),
            1 => Some(Self::MutualistSymbiont),
            2 => Some(Self::Commensal),
            3 => Some(Self::Parasitic),
            4 => Some(Self::Pathogenic),
            5 => Some(Self::CatalyticOrMetabolic),
            6 => Some(Self::ElectronicExcitation),
            7 => Some(Self::QuantumCoherenceAndEntanglement),
            8 => Some(Self::QuantumTunnelingAndNonAdiabatic),
            _ => None,
        }
    }
}

/// Tracks whether a computational stratum is currently embodied visually/spatially
/// or exists purely in the underlying computational model (awaiting progressive
/// multi-scale embodiment over time).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmbodimentResolutionState {
    /// Active in the computational/symbolic/quantum model; not visually rendered at macro scale.
    ComputationalOnly = 0,
    /// Synthesized procedurally on demand (`PROC` / shader) when zoomed to this stratum's scale.
    ProceduralSynthesized = 1,
    /// References an external `.10d` / `.hmc` archetype for its visual/spatial geometry.
    ArchetypeReferenced = 2,
    /// Directly embodied inline in this container's `QuantizedMesh` / `CDAG` / `VOX0` sections.
    InlineEmbodied = 3,
}

impl EmbodimentResolutionState {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::ComputationalOnly),
            1 => Some(Self::ProceduralSynthesized),
            2 => Some(Self::ArchetypeReferenced),
            3 => Some(Self::InlineEmbodied),
            _ => None,
        }
    }
}

/// 32-byte fixed-layout POD header for a `FIELD_KIND_MULTISCALE_STRATA` (`0x0304`, `MSTR`) section slice.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct MultiscaleStrataHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub node_count: u32,
    /// Bitmask of populated strata levels (`bit k` set if `Lk` has $\ge 1$ node).
    pub populated_strata_mask: u16,
    pub max_stratum_level: u8,
    pub reserved_u8: u8,
    /// 64-bit `q_hash` of the root N3Logic / SLG-VM multi-scale coupling rule graph.
    pub coupling_rule_graph_hash: u64,
    pub payload_crc32c: u32,
    pub reserved_u32: u32,
}

const _: () = assert!(core::mem::size_of::<MultiscaleStrataHeader>() == MULTISCALE_STRATA_HEADER_SIZE);

/// 64-byte cache-line-aligned POD record representing one node in the `L0`..=`L6`
/// multi-scale computational hierarchy (`FIELD_KIND_MULTISCALE_STRATA = 0x0304`).
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct NestedStratumNodeRecord {
    /// 16B: Geometric/topological anchor in the host complex + 64-bit RDF `NQuin` entity identity.
    pub host_anchor: CellAnchor,
    /// 2B: Index of the enclosing parent node in this `MSTR` table (`0xFFFF` for root).
    pub parent_node_index: u16,
    /// 1B: [`ComputationalStratumLevel`] (`0..=6` for `L0`..=`L6`).
    pub stratum_level: u8,
    /// 1B: [`StratumConstituentKind`] discriminant (organ, follicle, fungus, bacterium, parasite, cell, quantum state).
    pub constituent_kind: u8,
    /// 1B: [`StratumInteractionRegime`] (`MutualistSymbiont`, `Parasitic`, `ElectronicExcitation`, etc.).
    pub interaction_regime: u8,
    /// 1B: [`EmbodimentResolutionState`] (`ComputationalOnly` vs. `ProceduralSynthesized` / `ArchetypeReferenced` / `InlineEmbodied`).
    pub embodiment_state: u8,
    /// 1B: Packed `L6` quantum basis set (low nibble: `1=STO-3G, 2=def2-SVP, 3=def2-TZVP, 4=ECP`)
    /// and solver/functional (high nibble: `1=RHF-DIIS, 2=UHF-Broyden, 3=LDA-Autodiff, 4=GGA-PBE-Autodiff, 5=QForge-VQE`).
    pub quantum_basis_and_xc: u8,
    /// 1B: Characteristic spatial scale as $\text{round}(\log_{10}(\text{meters}) \times 12)$.
    pub characteristic_scale_log10_m_q12: i8,
    /// 8B: 64-bit `q_hash` of the full UTF-8 RDF taxon/class IRI (e.g., symbiont/parasite species in `intenteco` or chemical/cell IRI).
    pub taxon_or_ontology_iri_hash: u64,
    /// 8B: Child `.10d` archetype reference hash (`L1..=L4`) or deterministic Q-Forge VQE `NQuin` / N3Logic rule hash (`L5..=L6`).
    pub target_archetype_or_qpu_nquin_hash: u64,
    /// 4B: Population density (cells/m$^3$, bacterial CFU/m$^2$, fungal hyphae density, follicles/cm$^2$, or molarity).
    pub density_or_abundance_per_m3: f32,
    /// 4B: Metabolic flux rate (W or mol/s at `L1..=L5`) or converged SCF/DFT total energy in Hartrees ($E_{\text{h}}$ at `L6`).
    pub metabolic_flux_or_hartree_energy: f32,
    /// 2B: `f16` bits — at `L6`, HOMO-LUMO gap in eV; at `L2..=L4`, turgor / host-load ratio.
    pub coupled_metric_a_f16: u16,
    /// 2B: `f16` bits — at `L6`, dipole moment (Debye) or net charge; at `L2..=L4`, membrane potential (mV).
    pub coupled_metric_b_f16: u16,
    /// 2B: Symbolic-AI & homological [`TopologyInvariants`] (`REIFIED_EDGES`, `DEONTIC_GUARDED`, `PARACONSISTENT_ISOLATED`, etc.).
    pub logic_invariants: TopologyInvariants,
    /// 2B: Low byte = Belnap 4-valued truth status (`0=Unknown, 1=True, 2=False, 3=Both/Paraconsistent`);
    /// high byte = spin multiplicity $2S+1$ (`L6`) or ontogenetic stage mask (`L0..=L4`).
    pub belnap_and_spin_u16: u16,
    /// 4B: Active orbital / electron count (`L6`) or ontogenetic / pathology coupling flags (`L0..=L5`).
    pub orbital_or_coupling_flags: u32,
    /// 4B: Castagnoli CRC-32C over bytes `0..60`.
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<NestedStratumNodeRecord>() == NESTED_STRATUM_NODE_SIZE);
const _: () = assert!(core::mem::align_of::<NestedStratumNodeRecord>() == 64);

impl NestedStratumNodeRecord {
    /// Compute Castagnoli CRC-32C over bytes `0..60`.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; NESTED_STRATUM_NODE_SIZE] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `record_crc32c` in place.
    #[inline]
    pub fn seal_crc32c(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }

    /// Set `coupled_metric_a_f16` (e.g., HOMO-LUMO gap in eV or turgor ratio) from `f32`.
    #[inline]
    pub fn set_coupled_metric_a(&mut self, value: f32) {
        self.coupled_metric_a_f16 = f16::from_f32(value).to_bits();
    }

    /// Read `coupled_metric_a_f16` as `f32`.
    #[inline]
    pub fn coupled_metric_a(&self) -> f32 {
        f16::from_bits(self.coupled_metric_a_f16).to_f32()
    }

    /// Set `coupled_metric_b_f16` (e.g., dipole moment in Debye or membrane voltage in mV) from `f32`.
    #[inline]
    pub fn set_coupled_metric_b(&mut self, value: f32) {
        self.coupled_metric_b_f16 = f16::from_f32(value).to_bits();
    }

    /// Read `coupled_metric_b_f16` as `f32`.
    #[inline]
    pub fn coupled_metric_b(&self) -> f32 {
        f16::from_bits(self.coupled_metric_b_f16).to_f32()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiscaleStrataError {
    PayloadTooShort { need: usize, have: usize },
    OutputTooSmall { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    TooManyNodes { got: usize, max: usize },
    LengthMismatch { expected: usize, actual: usize },
    InvalidStratumLevel { index: usize, raw: u8 },
    InvalidConstituentKind { index: usize, raw: u8 },
    InvalidInteractionRegime { index: usize, raw: u8 },
    InvalidEmbodimentState { index: usize, raw: u8 },
    InvalidParentIndex { index: usize, parent: u16 },
    NonFiniteFloat { index: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
    CborEncoding(String),
}

impl core::fmt::Display for MultiscaleStrataError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MultiscaleStrataError {}

/// Compute exact encoded byte length for an `MSTR` section slice with `node_count` records.
#[inline]
pub const fn multiscale_strata_encoded_len(node_count: usize) -> Option<usize> {
    if node_count > MAX_NESTED_STRATUM_NODES {
        return None;
    }
    Some(MULTISCALE_STRATA_HEADER_SIZE + node_count * NESTED_STRATUM_NODE_SIZE)
}

/// Encode a slice of [`NestedStratumNodeRecord`] entries into a caller-supplied buffer. Zero heap.
pub fn encode_multiscale_strata_section(
    nodes: &[NestedStratumNodeRecord],
    coupling_rule_graph_hash: u64,
    out: &mut [u8],
) -> Result<usize, MultiscaleStrataError> {
    if nodes.len() > MAX_NESTED_STRATUM_NODES {
        return Err(MultiscaleStrataError::TooManyNodes {
            got: nodes.len(),
            max: MAX_NESTED_STRATUM_NODES,
        });
    }
    let needed = MULTISCALE_STRATA_HEADER_SIZE + nodes.len() * NESTED_STRATUM_NODE_SIZE;
    if out.len() < needed {
        return Err(MultiscaleStrataError::OutputTooSmall {
            need: needed,
            have: out.len(),
        });
    }

    let mut populated_mask = 0u16;
    let mut max_level = 0u8;
    let mut cursor = MULTISCALE_STRATA_HEADER_SIZE;

    for (index, node) in nodes.iter().enumerate() {
        if ComputationalStratumLevel::from_u8(node.stratum_level).is_none() {
            return Err(MultiscaleStrataError::InvalidStratumLevel {
                index,
                raw: node.stratum_level,
            });
        }
        if StratumConstituentKind::from_u8(node.constituent_kind).is_none() {
            return Err(MultiscaleStrataError::InvalidConstituentKind {
                index,
                raw: node.constituent_kind,
            });
        }
        if StratumInteractionRegime::from_u8(node.interaction_regime).is_none() {
            return Err(MultiscaleStrataError::InvalidInteractionRegime {
                index,
                raw: node.interaction_regime,
            });
        }
        if EmbodimentResolutionState::from_u8(node.embodiment_state).is_none() {
            return Err(MultiscaleStrataError::InvalidEmbodimentState {
                index,
                raw: node.embodiment_state,
            });
        }
        if node.parent_node_index != STRATUM_NO_PARENT
            && (node.parent_node_index as usize) >= nodes.len()
        {
            return Err(MultiscaleStrataError::InvalidParentIndex {
                index,
                parent: node.parent_node_index,
            });
        }
        if !node.density_or_abundance_per_m3.is_finite()
            || !node.metabolic_flux_or_hartree_energy.is_finite()
        {
            return Err(MultiscaleStrataError::NonFiniteFloat { index });
        }

        populated_mask |= 1u16 << (node.stratum_level as u16);
        if node.stratum_level > max_level {
            max_level = node.stratum_level;
        }

        let mut sealed = *node;
        sealed.seal_crc32c();
        out[cursor..cursor + NESTED_STRATUM_NODE_SIZE].copy_from_slice(bytes_of(&sealed));
        cursor += NESTED_STRATUM_NODE_SIZE;
    }

    let body_crc = crc32c(&out[MULTISCALE_STRATA_HEADER_SIZE..needed]);
    let header = MultiscaleStrataHeader {
        magic: MULTISCALE_STRATA_TAG,
        version: MULTISCALE_STRATA_VERSION,
        flags: 0,
        node_count: nodes.len() as u32,
        populated_strata_mask: populated_mask,
        max_stratum_level: max_level,
        reserved_u8: 0,
        coupling_rule_graph_hash,
        payload_crc32c: body_crc,
        reserved_u32: 0,
    };
    out[..MULTISCALE_STRATA_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    Ok(needed)
}

/// Decode and validate a `FIELD_KIND_MULTISCALE_STRATA` (`MSTR`) payload into caller storage. Zero heap.
pub fn decode_multiscale_strata_section_into(
    payload: &[u8],
    out_nodes: &mut [NestedStratumNodeRecord],
) -> Result<(MultiscaleStrataHeader, usize), MultiscaleStrataError> {
    if payload.len() < MULTISCALE_STRATA_HEADER_SIZE {
        return Err(MultiscaleStrataError::PayloadTooShort {
            need: MULTISCALE_STRATA_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: MultiscaleStrataHeader =
        pod_read_unaligned(&payload[..MULTISCALE_STRATA_HEADER_SIZE]);
    if header.magic != MULTISCALE_STRATA_TAG {
        return Err(MultiscaleStrataError::BadMagic(header.magic));
    }
    if header.version != MULTISCALE_STRATA_VERSION {
        return Err(MultiscaleStrataError::UnsupportedVersion(header.version));
    }
    let count = header.node_count as usize;
    if count > MAX_NESTED_STRATUM_NODES {
        return Err(MultiscaleStrataError::TooManyNodes {
            got: count,
            max: MAX_NESTED_STRATUM_NODES,
        });
    }
    let expected = MULTISCALE_STRATA_HEADER_SIZE + count * NESTED_STRATUM_NODE_SIZE;
    if payload.len() != expected {
        return Err(MultiscaleStrataError::LengthMismatch {
            expected,
            actual: payload.len(),
        });
    }
    if out_nodes.len() < count {
        return Err(MultiscaleStrataError::OutputTooSmall {
            need: count,
            have: out_nodes.len(),
        });
    }
    let body_crc = crc32c(&payload[MULTISCALE_STRATA_HEADER_SIZE..expected]);
    if header.payload_crc32c != body_crc {
        return Err(MultiscaleStrataError::ChecksumMismatch {
            expected: header.payload_crc32c,
            actual: body_crc,
        });
    }

    for index in 0..count {
        let start = MULTISCALE_STRATA_HEADER_SIZE + index * NESTED_STRATUM_NODE_SIZE;
        let node: NestedStratumNodeRecord =
            pod_read_unaligned(&payload[start..start + NESTED_STRATUM_NODE_SIZE]);
        let node_crc = node.compute_crc32c();
        if node.record_crc32c != node_crc {
            return Err(MultiscaleStrataError::ChecksumMismatch {
                expected: node.record_crc32c,
                actual: node_crc,
            });
        }
        if ComputationalStratumLevel::from_u8(node.stratum_level).is_none() {
            return Err(MultiscaleStrataError::InvalidStratumLevel {
                index,
                raw: node.stratum_level,
            });
        }
        if StratumConstituentKind::from_u8(node.constituent_kind).is_none() {
            return Err(MultiscaleStrataError::InvalidConstituentKind {
                index,
                raw: node.constituent_kind,
            });
        }
        if StratumInteractionRegime::from_u8(node.interaction_regime).is_none() {
            return Err(MultiscaleStrataError::InvalidInteractionRegime {
                index,
                raw: node.interaction_regime,
            });
        }
        if EmbodimentResolutionState::from_u8(node.embodiment_state).is_none() {
            return Err(MultiscaleStrataError::InvalidEmbodimentState {
                index,
                raw: node.embodiment_state,
            });
        }
        if node.parent_node_index != STRATUM_NO_PARENT
            && (node.parent_node_index as usize) >= count
        {
            return Err(MultiscaleStrataError::InvalidParentIndex {
                index,
                parent: node.parent_node_index,
            });
        }
        if !node.density_or_abundance_per_m3.is_finite()
            || !node.metabolic_flux_or_hartree_energy.is_finite()
        {
            return Err(MultiscaleStrataError::NonFiniteFloat { index });
        }
        out_nodes[index] = node;
    }

    Ok((header, count))
}

// ── In-Envelope Symbolic-AI, RDF 1.2 & Multilingual UTF-8 / UTF-16 CBOR-LD (`CBLD`, 0x0305) ──

/// Multilingual UTF-8 / UTF-16 lexical entry mapping a 64-bit `q_hash` identity inside `.10d`
/// to its full UTF-8 RDF IRI, optional BCP-47 language tag (e.g., `"en"`, `"rer"`, `"zh"`, `"ar"`),
/// and full Unicode label. Never restricted to ASCII.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultilingualLexicalTerm {
    /// 64-bit `q_hash` referenced by `MID2`, `MSTR`, `EONT`, `GTOP`, or `SKL2` records.
    pub term_hash: u64,
    /// Full Unicode RDF IRI in UTF-8 (e.g., `"https://intenteco.org/taxa/Corymbia_aparrerinja"`).
    pub iri_utf8: String,
    /// Optional BCP-47 language tag (`Some("rer")` for Arrernte, `Some("en")`, etc.).
    pub lang_bcp47: Option<String>,
    /// Human-readable Unicode label in UTF-8 (supports all scripts; convertible losslessly to UTF-16).
    pub label_utf8: String,
}

impl MultilingualLexicalTerm {
    /// Losslessly encode `label_utf8` as a UTF-16 code-unit vector for UTF-16 platform/ABI consumers.
    pub fn label_to_utf16(&self) -> Vec<u16> {
        self.label_utf8.encode_utf16().collect()
    }

    /// Construct a [`MultilingualLexicalTerm`] directly from UTF-16 code units, validating Unicode scalar values.
    pub fn from_utf16_label(
        term_hash: u64,
        iri_utf8: impl Into<String>,
        lang_bcp47: Option<String>,
        label_utf16: &[u16],
    ) -> Result<Self, MultiscaleStrataError> {
        let label_utf8 = String::from_utf16(label_utf16).map_err(|e| {
            MultiscaleStrataError::CborEncoding(format!("invalid UTF-16 lexical label: {e}"))
        })?;
        Ok(Self {
            term_hash,
            iri_utf8: iri_utf8.into(),
            lang_bcp47,
            label_utf8,
        })
    }
}

/// Self-describing CBOR-LD payload (`FIELD_KIND_SYMBOLIC_RDF_CBOR_LD = 0x0305`, `CBLD`)
/// embedding Symbolic-AI `NQuin` facts (including RDF 1.2 `TAG_EMBEDDED` reified triples),
/// N3Logic / SLG-VM / SHACL rule descriptors, and UTF-8 / UTF-16 multilingual lexical bindings
/// directly inside a `.10d` container.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SymbolicRdfCborLdPayload {
    /// CBOR-LD context IRI (e.g., `"https://qualiadb.org/contexts/10d-symbolic-v2.cbor-ld"`).
    pub context_iri: String,
    /// Multilingual UTF-8 / UTF-16 lexicon resolving 64-bit `q_hash` IDs in `.10d` binary tables.
    pub lexicon: Vec<MultilingualLexicalTerm>,
    /// Embedded 48-byte Symbolic-AI / RDF 1.2 `NQuin` statements `[s, p, o, c, m, parity]`.
    pub nquins: Vec<[u64; 6]>,
    /// Active N3Logic / SLG-VM rule graph IRIs or UTF-8 N3 rule expressions governing this manifold.
    pub n3_logic_rules_utf8: Vec<String>,
    /// Active SHACL shape IRIs enforcing structural and deontic constraints.
    pub shacl_shape_iris_utf8: Vec<String>,
}

impl SymbolicRdfCborLdPayload {
    /// Append a validated [`NQuin`] to the CBOR-LD payload.
    pub fn push_nquin(&mut self, quin: &NQuin) {
        self.nquins.push([
            quin.subject,
            quin.predicate,
            quin.object,
            quin.context,
            quin.metadata,
            quin.parity,
        ]);
    }

    /// Reconstruct validated [`NQuin`] structs from the CBOR-LD payload.
    pub fn decode_nquins(&self) -> Vec<NQuin> {
        self.nquins
            .iter()
            .map(|&[subject, predicate, object, context, metadata, parity]| NQuin {
                subject,
                predicate,
                object,
                context,
                metadata,
                parity,
            })
            .collect()
    }
}

/// Encode a [`SymbolicRdfCborLdPayload`] into a self-describing `CBLD` binary envelope
/// (`16B` header + deterministic CBOR body + Castagnoli CRC-32C).
pub fn encode_symbolic_rdf_cbor_ld(
    payload: &SymbolicRdfCborLdPayload,
) -> Result<Vec<u8>, MultiscaleStrataError> {
    let mut cbor_body = Vec::new();
    ciborium::into_writer(payload, &mut cbor_body)
        .map_err(|e| MultiscaleStrataError::CborEncoding(e.to_string()))?;
    if cbor_body.len() > MAX_SYMBOLIC_CBOR_LD_BYTES {
        return Err(MultiscaleStrataError::TooManyNodes {
            got: cbor_body.len(),
            max: MAX_SYMBOLIC_CBOR_LD_BYTES,
        });
    }
    let body_crc = crc32c(&cbor_body);
    let body_len = cbor_body.len() as u32;

    let mut out = Vec::with_capacity(SYMBOLIC_CBOR_LD_HEADER_SIZE + cbor_body.len());
    out.extend_from_slice(&SYMBOLIC_CBOR_LD_TAG);
    out.extend_from_slice(&SYMBOLIC_CBOR_LD_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&body_len.to_le_bytes());
    out.extend_from_slice(&body_crc.to_le_bytes());
    out.extend_from_slice(&cbor_body);
    Ok(out)
}

/// Decode and verify a `CBLD` (`FIELD_KIND_SYMBOLIC_RDF_CBOR_LD = 0x0305`) envelope.
pub fn decode_symbolic_rdf_cbor_ld(
    bytes: &[u8],
) -> Result<SymbolicRdfCborLdPayload, MultiscaleStrataError> {
    if bytes.len() < SYMBOLIC_CBOR_LD_HEADER_SIZE {
        return Err(MultiscaleStrataError::PayloadTooShort {
            need: SYMBOLIC_CBOR_LD_HEADER_SIZE,
            have: bytes.len(),
        });
    }
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != SYMBOLIC_CBOR_LD_TAG {
        return Err(MultiscaleStrataError::BadMagic(magic));
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != SYMBOLIC_CBOR_LD_VERSION {
        return Err(MultiscaleStrataError::UnsupportedVersion(version));
    }
    let declared_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let declared_crc = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    if declared_len > MAX_SYMBOLIC_CBOR_LD_BYTES {
        return Err(MultiscaleStrataError::TooManyNodes {
            got: declared_len,
            max: MAX_SYMBOLIC_CBOR_LD_BYTES,
        });
    }
    let expected_total = SYMBOLIC_CBOR_LD_HEADER_SIZE + declared_len;
    if bytes.len() != expected_total {
        return Err(MultiscaleStrataError::LengthMismatch {
            expected: expected_total,
            actual: bytes.len(),
        });
    }
    let cbor_body = &bytes[SYMBOLIC_CBOR_LD_HEADER_SIZE..expected_total];
    let actual_crc = crc32c(cbor_body);
    if declared_crc != actual_crc {
        return Err(MultiscaleStrataError::ChecksumMismatch {
            expected: declared_crc,
            actual: actual_crc,
        });
    }
    ciborium::from_reader(cbor_body)
        .map_err(|e| MultiscaleStrataError::CborEncoding(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::header::AnchorDomain;

    #[test]
    fn multiscale_strata_l0_to_l6_holobiont_and_quantum_chemistry_roundtrips() {
        let mut l0_tree = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: 0x1000_0001,
                cell_index: 0,
                anchor_domain: AnchorDomain::Cell3Volume as u8,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
            parent_node_index: STRATUM_NO_PARENT,
            stratum_level: ComputationalStratumLevel::L0MacroHolobiontOrAssembly as u8,
            constituent_kind: StratumConstituentKind::HolobiontOrSystemRoot as u8,
            interaction_regime: StratumInteractionRegime::StructuralConstituent as u8,
            embodiment_state: EmbodimentResolutionState::InlineEmbodied as u8,
            quantum_basis_and_xc: 0,
            characteristic_scale_log10_m_q12: 12, // 10^1 m
            taxon_or_ontology_iri_hash: 0xAAAA_0001,
            target_archetype_or_qpu_nquin_hash: 0,
            density_or_abundance_per_m3: 1.0,
            metabolic_flux_or_hartree_energy: 45.0,
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::REIFIED_EDGES
                | TopologyInvariants::DEONTIC_GUARDED,
            belnap_and_spin_u16: 1, // Belnap True
            orbital_or_coupling_flags: 0,
            record_crc32c: 0,
        };
        l0_tree.set_coupled_metric_a(0.95);

        let l2_trichome_follicle = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: 0x1000_0002,
                cell_index: 14,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 2,
            },
            parent_node_index: 0,
            stratum_level: ComputationalStratumLevel::L2TissueOrFollicle as u8,
            constituent_kind: StratumConstituentKind::FollicleOrTrichome as u8,
            interaction_regime: StratumInteractionRegime::StructuralConstituent as u8,
            embodiment_state: EmbodimentResolutionState::ProceduralSynthesized as u8,
            quantum_basis_and_xc: 0,
            characteristic_scale_log10_m_q12: -48, // 10^-4 m
            taxon_or_ontology_iri_hash: 0xAAAA_0002,
            target_archetype_or_qpu_nquin_hash: 0,
            density_or_abundance_per_m3: 2500.0,
            metabolic_flux_or_hartree_energy: 0.012,
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::REIFIED_EDGES,
            belnap_and_spin_u16: 1,
            orbital_or_coupling_flags: 0,
            record_crc32c: 0,
        };

        let l3_endophytic_fungus = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: 0x1000_0003,
                cell_index: 14,
                anchor_domain: AnchorDomain::Cell3Volume as u8,
                orientation_sign: 1,
                material_or_stratum: 3,
            },
            parent_node_index: 1,
            stratum_level: ComputationalStratumLevel::L3HarbouredConsortia as u8,
            constituent_kind: StratumConstituentKind::EndophyticOrDermatophyticFungus as u8,
            interaction_regime: StratumInteractionRegime::MutualistSymbiont as u8,
            embodiment_state: EmbodimentResolutionState::ComputationalOnly as u8,
            quantum_basis_and_xc: 0,
            characteristic_scale_log10_m_q12: -60, // 10^-5 m
            taxon_or_ontology_iri_hash: 0xBBBB_1111, // Reference to Funga species in intenteco
            target_archetype_or_qpu_nquin_hash: 0xCCCC_2222,
            density_or_abundance_per_m3: 1.5e6,
            metabolic_flux_or_hartree_energy: 0.004,
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::REIFIED_EDGES
                | TopologyInvariants::PARACONSISTENT_ISOLATED,
            belnap_and_spin_u16: 1,
            orbital_or_coupling_flags: 0,
            record_crc32c: 0,
        };

        let mut l6_chlorophyll_dft = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: 0x1000_0006,
                cell_index: 14,
                anchor_domain: AnchorDomain::SemanticQuin as u8,
                orientation_sign: 0,
                material_or_stratum: 6,
            },
            parent_node_index: 1,
            stratum_level: ComputationalStratumLevel::L6QuantumChemistry as u8,
            constituent_kind: StratumConstituentKind::DftExchangeCorrelationGrid as u8,
            interaction_regime: StratumInteractionRegime::ElectronicExcitation as u8,
            embodiment_state: EmbodimentResolutionState::ComputationalOnly as u8,
            quantum_basis_and_xc: (4 << 4) | 2, // GGA-PBE-Autodiff + def2-SVP
            characteristic_scale_log10_m_q12: -120, // 10^-10 m
            taxon_or_ontology_iri_hash: 0xDDDD_3333,
            target_archetype_or_qpu_nquin_hash: 0xEEEE_4444, // Q-Forge StateVector NQuin pointer
            density_or_abundance_per_m3: 6.022e23,
            metabolic_flux_or_hartree_energy: -2934.12841797, // Converged Hartree energy
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::REIFIED_EDGES,
            belnap_and_spin_u16: (1 << 8) | 1, // Singlet spin multiplicity (2S+1 = 1), Belnap True
            orbital_or_coupling_flags: 136,
            record_crc32c: 0,
        };
        l6_chlorophyll_dft.set_coupled_metric_a(1.86); // HOMO-LUMO gap 1.86 eV
        l6_chlorophyll_dft.set_coupled_metric_b(4.12); // Dipole moment 4.12 D

        let nodes = [
            l0_tree,
            l2_trichome_follicle,
            l3_endophytic_fungus,
            l6_chlorophyll_dft,
        ];
        let mut buf = [0u8; 512];
        let written = encode_multiscale_strata_section(&nodes, 0x7777_8888, &mut buf).unwrap();
        assert_eq!(
            written,
            MULTISCALE_STRATA_HEADER_SIZE + 4 * NESTED_STRATUM_NODE_SIZE
        );

        let mut decoded = [NestedStratumNodeRecord::zeroed(); 4];
        let (hdr, count) =
            decode_multiscale_strata_section_into(&buf[..written], &mut decoded).unwrap();
        assert_eq!(count, 4);
        assert_eq!(hdr.max_stratum_level, 6);
        assert_eq!(
            hdr.populated_strata_mask,
            (1 << 0) | (1 << 2) | (1 << 3) | (1 << 6)
        );
        assert_eq!(
            decoded[2].constituent_kind,
            StratumConstituentKind::EndophyticOrDermatophyticFungus as u8
        );
        assert!((decoded[3].coupled_metric_a() - 1.86).abs() < 0.01);
        assert_eq!(decoded[3].metabolic_flux_or_hartree_energy, -2934.12841797);
    }

    #[test]
    fn symbolic_rdf_cbor_ld_supports_multilingual_utf8_utf16_and_nquins() {
        let arrernte_utf16: Vec<u16> = "Ilwempe (Ghost Gum — 幽灵桉 — شجرة الصمغ)".encode_utf16().collect();
        let term = MultilingualLexicalTerm::from_utf16_label(
            0xAAAA_0001,
            "https://intenteco.org/taxa/Corymbia_aparrerinja",
            Some("rer".to_string()),
            &arrernte_utf16,
        )
        .unwrap();
        assert_eq!(term.label_to_utf16(), arrernte_utf16);

        let mut payload = SymbolicRdfCborLdPayload {
            context_iri: "https://qualiadb.org/contexts/10d-symbolic-v2.cbor-ld".to_string(),
            lexicon: vec![term],
            nquins: Vec::new(),
            n3_logic_rules_utf8: vec![
                "{ ?leaf :harboursSymbiont ?fungus } => { ?leaf :droughtTolerance :Elevated } ."
                    .to_string(),
            ],
            shacl_shape_iris_utf8: vec![
                "urn:qualia:shapes:HolobiontMultiScaleShape".to_string(),
            ],
        };
        let quin = NQuin {
            subject: 0xAAAA_0001,
            predicate: 0xBBBB_0002,
            object: 0xCCCC_0003,
            context: 0xDDDD_0004,
            metadata: 1u64 << 32,
            parity: 0xAAAA_0001 ^ 0xBBBB_0002 ^ 0xCCCC_0003 ^ 0xDDDD_0004,
        };
        payload.push_nquin(&quin);

        let bytes = encode_symbolic_rdf_cbor_ld(&payload).unwrap();
        let decoded = decode_symbolic_rdf_cbor_ld(&bytes).unwrap();
        assert_eq!(decoded, payload);
        assert_eq!(decoded.decode_nquins(), vec![quin]);
    }
}
