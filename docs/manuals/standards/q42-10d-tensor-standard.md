# Q42 10D Computational & Epistemic Tensor Standard (`.10d` / `Tensor10D` v2.0)

**Version:** 2.0 (Unified Computational Primacy, Multi-Scale `L0`–`L6` Strata, Symbolic-AI/RDF 1.2 & Container Specification)  
**Target Architecture:** QualiaDB (`crates/qualia-core-db/src/tensor/` & `crates/qualia-core-db/src/container_10d/`)  
**Companion Specifications:** [`10d-container-standard.md`](10d-container-standard.md), [`10d-container-format.md`](10d-container-format.md), [`10d-typed-field-payloads.md`](10d-typed-field-payloads.md), [`10d-skeletal-animation.md`](10d-skeletal-animation.md), [`10d-opaque-preservation.md`](10d-opaque-preservation.md), [`yaml-ld-q42-specification.md`](yaml-ld-q42-specification.md)

---

## 0. Foundational Principles: Computational Primacy, Multi-Scale Reality & Symbolic-AI Grounding

### 0.1 Computational Primacy Over Visual and Auditory Embodiment
In the QualiaDB and Webizen architecture, **the 3D/4D visual model (`QuantizedMesh`, `CDAG`) or auditory stream (`.qac`) is NEVER the primary entity.**
1. **The Primary Entity is the Neuro-Symbolic Computational & Epistemic Model:**
   * Every `.10d` manifold represents **observed empirical reality**, **scientific hypothesis synthesis** (`q` superposition), or **creative/intellectual work in a mixed zone** (where empirical facts, counterfactual simulations, civic policy scenarios, and creative works coexist with explicit epistemic truth and provenance weights).
   * A pure hypothesis, an unembodied creative or legislative construct, or a multi-scale biophysical field is already a complete, valid `.10d` entity (`Profile 0: ComputationalOnly`) even when no 3D triangle mesh is attached.
2. **Sensory & Spatial Representations are Bidirectionally Coupled Secondary Embodiments:**
   * When an entity is embodied in 3D/4D space, its visual geometry (`.10d` Mode-A inline constituency or Mode-B `AREF` archetype reference), physical matter (`.m64`), electromagnetic/radiance fields (`.emf`), and spatial acoustics (`.qac`) act as **coupled projections** driven by—and feeding back into—the primary computational model.

### 0.2 Multi-Scale Nested Computational Depth (`L0` Macro Down to `L6` Quantum Physics)
While a visual representation of a species (`intenteco`: flora, fauna, funga) or a made-world thing (`qualia-poiesis`: vehicles, buildings, tools, creative works) may initially depict only a macro outer surface (such as the shape of a leaf, dermis, or structural beam), **the underlying `.10d` computational data structure (`FIELD_KIND_MULTISCALE_STRATA = 0x0304`, `MSTR`) encodes the nested multi-scale reality of that entity across seven strata (`L0`–`L6`)**:
* **`L0` ($10^2\text{ m} \dots 10^{-1}\text{ m}$) — Macro-Entity / Holobiont / System Assembly:** The complete tree, animal, fungal network, building, or vehicle.
* **`L1` ($10^{-1}\text{ m} \dots 10^{-2}\text{ m}$) — Organ / Anatomical Compartment / Structural Subsystem:** Leaf lamina, vascular cambium, heart, root system; engine block, PCB, structural frame.
* **`L2` ($10^{-2}\text{ m} \dots 10^{-4}\text{ m}$) — Tissue, Micro-Appendage & Histological Architecture:** **Hair follicles, trichomes, stomatal guard complexes**, epidermis, palisade/spongy mesophyll, xylem tracheids, osteons/trabeculae (`.m64` `M64BioTissueRecord`); weld zones, coatings.
* **`L3` ($10^{-3}\text{ m} \dots 10^{-6}\text{ m}$) — Harboured Consortia (Symbiotic Fungi, Bacteria, Microbiome & Parasites):** Endophytic and mycorrhizal fungi, phyllosphere/rhizosphere/follicular/gut bacterial biofilms, mites, nematodes, galls, mistletoe, viral/cellular parasites (and timber-rot fungi or corrosion biofilms on made-world things). Each co-habiting organism is anchored via [`CellAnchor`](../../../crates/qualia-core-db/src/topology/header.rs) and references its own canonical `intenteco` taxon and `.10d`/`.q42` model.
* **`L4` ($10^{-4}\text{ m} \dots 10^{-7}\text{ m}$) — Cellular & Organellar Stratum:** Somatic and meristem/stem cells, chloroplasts, thylakoid membranes, mitochondria, cell walls, turgor pressure, and membrane potential.
* **`L5` ($10^{-7}\text{ m} \dots 10^{-9}\text{ m}$) — Molecular, Genomic & Biochemical Pathway Stratum:** DNA/RNA/protein sequences (`bioinformatics_align`), enzymatic reaction centers (RuBisCO, ATP synthase, cytochrome P450), structural biopolymers (cellulose, lignin, keratin, collagen), and crystal lattices.
* **`L6` ($10^{-9}\text{ m} \dots 10^{-15}\text{ m}$) — Quantum Physics Stratum (`L6_QuantumPhysics`):** All **non-classical quantum-mechanical considerations that cannot be reduced to general/classical physics or continuum mathematics** (with Quantum Dynamics and Quantum Chemistry as sub-domains):
  * **Quantum Coherence, Superposition & Entanglement:** Non-commutative observables ($[\hat{A}, \hat{B}] \neq 0$), density matrices ($\hat{\rho}$), von Neumann entanglement entropy ($S_{\text{vN}}$), and non-local correlations (`QuantumCoherenceAndEntanglement`).
  * **Open-System Quantum Dynamics, Tunneling & Spin Physics:** GKSL Lindblad / Redfield / HEOM decoherence ($T_1, T_2^*$, `OpenQuantumLindbladDynamics`), WKB/instanton proton and electron barrier tunneling and vibronic exciton coherence (`QuantumTunnelingAndVibronicTransport`), and Zeeman/hyperfine radical-pair spin dynamics such as biological magnetoreception and spintronics (`SpinDynamicsAndRadicalPair`).
  * **Quantum Electrodynamics (QED) & Topological/Many-Body Quantum Phases:** Cavity QED / polaritonic light-matter coupling and Casimir dispersion (`CavityQedAndPhotonicCoupling`), Berry curvature, Chern invariants, and superconducting/BEC condensates (`TopologicalAndManyBodyQuantumPhase`).
  * **Quantum Chemistry Subset (Electronic Structure):** Contracted Gaussian-Type Orbitals (`basis_set.rs`: STO-3G, def2-SVP, ECPs), Obara-Saika / Head-Gordon-Pople / Rys molecular integrals (`integrals.rs`), RHF/UHF Self-Consistent Field with DIIS/Broyden convergence to $10^{-8}\text{ Ha}$ (`scf.rs`), Rust-native autodiff LDA/GGA Density Functional Theory (`dft.rs`), and `qualia-q-forge` VQE/QUBO StateVector `NQuin` topological pointers (`qpu_extension.rs`).

Each stratum node carries an explicit `EmbodimentResolutionState` (`ComputationalOnly`, `ProceduralSynthesized`, `ArchetypeReferenced`, `InlineEmbodied`) so deeper strata (`L2`–`L6`) are active in the computational model immediately and can be progressively given 3D/4D visual embodiments over time without changing the container schema.

### 0.3 Symbolic-AI, RDF 1.2, Logic Infrastructure & Strict No-ASCII Policy (UTF-8 / UTF-16 & CBOR / CBOR-LD)
1. **Strict No-ASCII Policy:**
   * ASCII cannot represent the world's languages (including Indigenous languages such as Arrernte, non-Latin scripts, and scientific notation).
   * **All human-readable text, multilingual labels, RDF IRIs, and literals MUST use UTF-8 (or lossless UTF-16 at platform/ABI boundaries), encoded in binary envelopes via CBOR / CBOR-LD (`FIELD_KIND_SYMBOLIC_RDF_CBOR_LD = 0x0305`, `CBLD`) or `.q42` `Q42LEX` tables with BCP-47 `@lang` tags.**
   * Fixed 4-byte tags in binary headers are 32-bit little-endian numeric format discriminants (`u32`), never ASCII strings.
2. **Innate Symbolic-AI, RDF 1.2 & Logic Infrastructure:**
   * Every `.10d` entity and `L0`–`L6` stratum is anchored to a 64-bit RDF `NQuin` semantic identity (`q_hash` of a full UTF-8 IRI, `did:q42:`, or RDF 1.2 / RDF-Star `TAG_EMBEDDED` reified triple ID).
   * The computational manifold integrates directly with QualiaDB's **N3Logic, SHACL, zero-heap `SLG-VM` tabling solver, Belnap 4-valued paraconsistent logic (`Unknown`, `True`, `False`, `Both`), and Deontic governance (`OP_PERMIT`, `OP_OBLIGE`, `OP_FORBID` + `DEFEATER_BIT`)**.

---

## 1. Canonical 10D Coordinate Basis (`Tensor10D`) & T14 / T67 Reconciliation

### 1.1 The 40-Byte `Tensor10D` Atom (`[q, v, w, x, y, z, t, α, μ, σ]`)
A `Tensor10D` coordinate point (`crates/qualia-core-db/src/tensor/mod.rs` and `container_10d/axis_role.rs`) is an exact 40-byte ($10 \times \text{f32}$) vector:

$$\mathbf{X}_{10\text{D}} = \begin{bmatrix} q & v & w & x & y & z & t & \alpha & \mu & \sigma \end{bmatrix} \in \mathbb{R}^{10}$$

| Index | Symbol | Field | Canonical `AxisRole` (`axis_role.rs`) | Normative Mathematical & Epistemic Meaning (T67 Reconciled) |
| :---: | :---: | :--- | :--- | :--- |
| **0** | $q$ | `q` | `Selector` (`2`) | **Quantum / Epistemic Superposition & Hypothesis Axis:** Selects or weights empirical reality ($q = 0.0$ ground truth or collapsed eigenstate), active scientific hypotheses, and mixed-zone creative counterfactual branches. |
| **1** | $v$ | `v` | `Selector` (`2`) | **Topological & Physical Metric Regime Selector:** Selects the active metric law (`MetricKind`: Euclidean, Cyclic/Toroidal, Hyperbolic taxonomic tree, Boundary Clique, Spherical HEALPix, Lorentzian Spacetime, Multiply-Connected Portal). |
| **2** | $w$ | `w` | `Selector` (`2`) | **Domain & Multi-Head Bifurcation Selector:** Routes across domain sub-manifolds (biosphere, made-world, civic/jural, quantum chemistry, clinical). |
| **3** | $x$ | `x` | `Coordinate` (`1`) | **Spatial Coordinate $X$:** Metric position in SI meters (or local chart frame declared in `MID2`). |
| **4** | $y$ | `y` | `Coordinate` (`1`) | **Spatial Coordinate $Y$:** Metric position in SI meters. |
| **5** | $z$ | `z` | `Coordinate` (`1`) | **Spatial Coordinate $Z$:** Metric position / elevation in SI meters. |
| **6** | $t$ | `t` | `Coordinate` (`1`) | **Coordinate Time $t$:** 4th spacetime dimension (physical, ontogenetic, or simulation coordinate time in SI seconds or days). *Note (T67): `t` is strictly Coordinate Time, never a provenance hash.* |
| **7** | $\alpha$ | `alpha` | `Coordinate` (`1`) | **Semantic Pull / Conceptual Gravity $\alpha$:** Epistemic salience, structural mass/rigidity coupling, or attentional attractor weight. |
| **8** | $\mu$ | `mu` | `CoordinateCarrier` (`4`) | **Provenance Weight & Governance Carrier $\mu$:** Dual-role metric coordinate and in-band cryptographic provenance, DID authority, consent, and rights/obligation carrier. |
| **9** | $\sigma$ | `sigma` | `Coordinate` (`1`) | **Spectral & Logical Class Signature $\sigma$:** Continuous physical/logical signature bridging intrinsic material and quantum properties (`L6` bandgap / reflectance) into downstream sensory projections (**U2** visual / **U3** auditory). |

### 1.2 T14 Disambiguation: `.10d` `Tensor10D` vs. `.p64` `Epistemic10D`
QualiaDB defines two distinct 10-float structures that MUST NEVER be conflated:
1. **`Tensor10D` (`crates/qualia-core-db/src/tensor/mod.rs`):** The `[q, v, w, x, y, z, t, α, μ, σ]` spatio-temporal and neuro-symbolic manifold coordinate used in `.10d` containers (`SectionType::Tensor10DNodes = 2` and `MID2 = 15`).
2. **`Epistemic10D` / `ManifoldCoordinate10D` (`crates/qualia-core-db/src/modalities/manifold.rs`):** The internal cognitive/neural weight modulation vector `[scale, attention_depth, epistemic_weight, topological_spin, temporal_decay, entropy_bias, spatial_phase, recurrence_frequency, density_threshold, manifold_curvature]` used exclusively inside `.p64` cognitive weight containers.

---

## 2. Bridging 48-Byte `NQuin`s and `Tensor10D`

Every 48-byte `NQuin` (`[subject, predicate, object, context, metadata, parity]`, $6 \times \text{u64}$) in `.q42` projects deterministically into `Tensor10D` via `Tensor10D::from_nquin(&quin)`:
* `subject`, `predicate`, `object` map to normalized coordinates in $[0.0, 1.0]$ across $(x, y, z)$.
* `context` (bits `0..56` graph namespace, bits `56..63` `SensitivityClass`) maps to $\alpha$ and deontic access gates.
* `metadata` (Lamport clock, Belnap 4-valued truth status, deontic opcodes) maps to coordinate time $t$ and provenance carrier $\mu$.
* `parity` ($S \oplus P \oplus O \oplus C$) initializes $\sigma$.

Conversely, inside `.10d` v2 containers, **`CellAnchor`** (`16B` in `GTOP` `0x0300` and `MSTR` `0x0304`) and **`SymbolicRdfCborLdPayload`** (`CBLD` `0x0305`) bind geometric k-cells and multi-scale strata directly back to 64-bit `NQuin` entity hashes and full 48-byte `NQuin` statements.

---

## 3. Topological Metric Regimes ($v$-Axis) & Distance Laws

### 3.1 Registered `MetricKind` Regimes (`metric_check.rs`)
1. **`Euclidean = 1` ($v = 0$):** Standard 7-axis Euclidean metric over $(x, y, z, t, \alpha, \mu, \sigma)$:
   $$d_{\text{Euc}}(\mathbf{a}, \mathbf{b}) = \sqrt{\sum_{k \in \{x,y,z,t,\alpha,\mu,\sigma\}} (a_k - b_k)^2}$$
2. **`CyclicToroidal = 2` ($v = 1$):** Periodic boundary metric on $\mathbb{T}^3$ with period $L = 1.0$:
   $$\Delta_k = \min\left(|a_k - b_k| \bmod 1.0,\; 1.0 - (|a_k - b_k| \bmod 1.0)\right), \quad k \in \{x, y, z\}$$
3. **`Hyperbolic = 3` ($v = 2$):** Poincaré ball hyperbolic metric for deep taxonomic trees (`intenteco` Linnaean/phylogenetic hierarchies and `qualia-poiesis` BOM ontologies):
   $$d_{\text{Hyp}}(\mathbf{u}, \mathbf{v}) = \operatorname{arcosh}\!\left(1 + \frac{2\|\mathbf{u} - \mathbf{v}\|^2}{(1 - \|\mathbf{u}\|^2)(1 - \|\mathbf{v}\|^2)}\right)$$
4. **`BoundaryClique = 4` ($v \ge 3$):** Topological regime separator $|v_a - v_b|$.
5. **`SphericalHealpix = 5`:** Equal-area planetary sphere metric ($S^2$) for WGS84 / GIS geodetic manifolds.
6. **`LorentzianSpacetime = 6`:** Relativistic causal spacetime interval $ds^2 = -c^2 dt^2 + dx^2 + dy^2 + dz^2$.
7. **`MultiplyConnectedPortal = 7`:** Non-trivial fundamental group $\pi_1(M) \neq 0$ bridged via portal transition motors.

*Note on the 64-Byte `.10d` Header Gate:* Bytes `20..52` of `Container10dHeader` store the 4-branch `CANONICAL_METRIC_TABLE` (`v = 0..=3`) verifying runtime parity with `Tensor10D::full_distance`. Higher metric regimes (`5..=7`) are declared via `MID2` (`coordinate_convention_id` / `domain_id`) and `GTOP` (`PoetGeometryHeader::metric_kind`).

---

## 4. Three `.10d` Embodiment Profiles & Two-Tier Geometry Constituency

Paralleling how `.emf` (`Q42EMF64`), `.qac` (`qac\0`), and `.m64` (`m64\0`) were factored into dedicated domain containers, `.10d` v2 supports three embodiment profiles:

### 4.1 Profile 0 — Pure Computational, Hypothesis & Mixed-Zone `.10d` (`computational_only`)
* **Sections Present:** `ManifoldIdentityV2` (`MID2`, Sec `15`), optional `Tensor10DNodes` (Sec `2`), optional `FieldSidecar` (Sec `8`: `FIEL`, `GTOP` `0x0300`, `MSTR` `0x0304`, `CBLD` `0x0305`), optional `EntityOntology` (`EONT`, Sec `17`), optional `ProceduralSynthesis` (`PROC`, Sec `18`), optional `ProvenanceSidecar` (Sec `7`).
* **Purpose:** Represents pure computational models, scientific hypotheses, ecological consortia, quantum chemistry systems, or creative works in a mixed zone without requiring a 3D triangle mesh.

### 4.2 Profile 1 (Geometry Mode A) — Archetype Embodiment `.10d` (`archetype_inline`)
* **Sections Present:** All Profile-0 computational sections **plus inline 3D/4D geometry and look-dev constituency**: `QuantizedMesh` (`1`), `CorrespondenceMap` (`9`, `FEMC`), `Topology` (`10`), `SpatialIndex` (`11`), `SurfaceReading` (`12`), `Materials` (`13`, `MAT1`..`MAT4`), `TextureCoordinates` (`14`, `UV01`), `SkeletalAnimation` (`16`, `SKL1`/`SKL2` + `SSM1` PCA shape basis), `ImpostorAtlas` (`19`, `IMPO`), `HierarchicalSurfels` (`20`, `SURF`), `HyperspectralCube` (`21`), `VoxelLattice` (`22`, `VOX0`), `ClusterDag` (`23`, `CDAG`).
* **Purpose:** The canonical master embodiment produced per species repository in `intenteco` or per thing repository in `qualia-poiesis`.

### 4.3 Profile 2 (Geometry Mode B) — Instance / Population `.10d` (`archetype_instance`)
* **Sections Present:** `ManifoldIdentityV2` (`MID2`, Sec `15`), `FieldSidecar` (Sec `8`) containing **`FIELD_KIND_ARCHETYPE_REF = 0x0301` (`AREF`)** + optional **`FIELD_KIND_INSTANCE_OVERRIDES = 0x0302` (`IOVR`)** + optional **`FIELD_KIND_INSTANCE_BATCH = 0x0303` (`INST`)** + optional **`FIELD_KIND_MULTISCALE_STRATA = 0x0304` (`MSTR`)** + optional **`FIELD_KIND_SYMBOLIC_RDF_CBOR_LD = 0x0305` (`CBLD`)** + optional **`FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE = 0x0306` (`HOLO`)**, plus optional `EntityOntology` (`EONT`, Sec `17`), `ProceduralSynthesis` (`PROC`, Sec `18`), and `Tensor10DNodes` (Sec `2`).
* **Purpose:** An individual asset in the world (e.g., a specific ghost gum tree at `[x, y, z, t]` with its own age, hydration, endophytic fungi, and 64-bit procedural seed, or a specific 4WD vehicle with its own wear, owner DID, and damage) or a 10,000-entity population cloud (`Tensor10DNodes` + `INST`). It **omits duplicate geometry sections** and references the Mode-A Archetype `.10d` by its 256-bit `BLAKE3`/`SHA-256` digest in `AREF`.

### 4.4 Holographic Phase-Space Primacy (`FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE = 0x0306`, `HOLO`)
Unlike static polygon formats (`.obj`, `.fbx`, `.gltf`), a `.10d` asset is fundamentally an **in-situ holographic phase-space & thermodynamic energy tensor**:
* **`HOLO` (`0x0306` in `FieldSidecar`):** Encodes a 64-byte [`HolographicPhaseHeader`](../../../crates/qualia-core-db/src/container_10d/holographic_phase_section.rs) (`u32` tag `HOLO`, `version=1`, `flags`, `max_raymarch_steps <= 32`, `active_harmonic_bands_k <= 12`, `holo_record_count`, `wavefield_record_count`, 12-triangle proxy lens AABB `proxy_aabb_min/max`, `sdf_smooth_union_radius`, `record_crc32c`), followed by `N × 64B` [`M64HoloRecord`](../../../crates/qualia-core-db/src/m64_material.rs) rows (Mode `0x01`: 12 `f16` Bessel/Spherical `sdf_harmonics`, `eigen_frequencies: [u16; 3]`, `damping_ratios: [u8; 3]`, 6-band 380–780 nm `spectral_spd_base: [u8; 6]`) and `W × 64B` [`EulerianWavefieldRecord`](../../../crates/qualia-core-db/src/container_10d/holographic_phase_section.rs) rows.
* **Hamiltonian Stationary-Action Splines ($\delta S = \delta \int (T - V)\,dt = 0$):** Time $t$ and environmental forces (wind/water drag $F_{\text{drag},k}$) evaluate continuous analytical Cubic Hermite Hamiltonian phase trajectories (`evaluate_hamiltonian_stationary_action_spline_zero_heap`) with zero keyframe tables.
* **Hyperbolic $v = 2$ Möbius Morphogenesis & Scale-Space Harmonic LODs ($k = 0..K$):** Botanical/biological growth and fractal vascular branching evaluate Poincaré ball Möbius transformations (`evaluate_hyperbolic_mobius_morphogenesis_zero_heap`) with golden-angle ($\Phi \approx 137.50776^\circ$) phyllotaxis and automatic scale-space harmonic truncation ($k = 0..K_{\text{active}}(d)$) that analytically eliminates Nyquist aliasing and polygon LOD pop-in.
* **12-Triangle Proxy Lens & `< 32` Step Raymarching:** When `SectionType::QuantizedMesh` (`1`) is omitted (or when rendering in holographic proxy mode), `decode_load_10d` automatically synthesizes the 8-vertex, 12-triangle bounding cube lens from `HolographicPhaseHeader` and raymarches the analytical SDF in `< 32` steps (TDR guardrail). Existing B-Rep `.10d` assets convert losslessly via `upgrade_legacy_10d_to_holographic_v2`.

---

## 5. Memory Budgets: 42 MiB Q42 Hot-Path Sentinel vs. Streaming Asset & Scene Containers (> 42 MiB)

1. **42 MiB Q42 Sentinel (`SlgArena` / Symbolic-AI & Prolog Hot Path):**
   * All Symbolic-AI, `SLG-VM` logical inference, `NQuin` indexing, and per-frame zero-heap evaluation kernels obey the strict **42 MiB Sentinel** zero-heap ceiling.
2. **Streaming / `mmap` Asset & Scene Containers (`> 42 MiB`, Analogous to `.p64` LLM Weight Files):**
   * Just as `.p64` LLM weight files exceed 42 MiB via zero-copy `mmap` paging, `.10d`, `.m64`, `.emf`, and `.hmc` asset and scene containers are explicitly permitted to exceed 42 MiB on disk/VRAM (e.g., **128 MiB** per packed `.10d` container and **2 GiB** aggregate in the `production16` quality profile), while all hot-path decoders (`parse_section_table`, `decode_manifold_identity_v2`, `decode_holographic_phase_section_into`, `decode_archetype_ref_record`, `decode_multiscale_strata_section_into`, `select_cut_clusters_zero_heap`) remain strictly zero-heap and caller-buffered.
