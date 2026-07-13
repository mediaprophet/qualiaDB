# Computational 3D Assets and Digital Twins

**Status:** architecture and capability manual  
**Date:** 2026-07-03 (harmonized 2026-07-04)  
**Implementation plan:** [`../plans/native-visual-intelligence-and-generative-3d.md`](../plans/native-visual-intelligence-and-generative-3d.md)
**Computational-geometry substrate:** [`../plans/native-computational-geometry.md`](../plans/native-computational-geometry.md)
§12 — the `.10d` container is the compiled geometry/analysis-mesh/field sidecar this manual specifies in
§3.1; the F/A tier model (§4) applies to geometry outputs; the visual-plan Phase 9/10/11 dependencies on
computational geometry are mapped in §12.3.

This manual records what QualiaDB can actually do with 3D assets and computational models,
what the present GLB-to-Q42 path preserves, and how engineering or biological digital twins
should be added without confusing rendering, simulation fidelity, and safety assurance.

## 1. The short answer

Qualia already converts OBJ, STL, and GLB assets into `.q42` volumes, but the conversion is
currently a **semantic projection**, not a dense geometry conversion:

- `qualia-cli` memory-maps the source asset and calls
  `qualia_core_db::render::assets::import_asset`;
- the importer extracts vertex positions and triangle indices;
- `mesh_to_nquins` writes mesh identity, source format, counts, bounds, and centroid as NQuins;
- the Q42 writer stores those facts in a unified, indexed, compressed v3 volume; and
- the dense vertex/index data is not written into that Q42 volume.

This is a sound separation of concerns, but it is only the first half of the intended
acceleration. Q42 accelerates semantic discovery and policy evaluation. Fast rendering and
simulation also require a content-addressed, page-aligned geometry/field payload linked from
Q42. Reopening and reparsing the original GLB does not obtain the full speed benefit.

The correct target is therefore a compiled asset bundle:

```text
source GLB/OBJ/STL
    -> validated canonical scene
    -> Q42 semantic and governance volume
    -> compiled geometry/mesh/field sidecars
    -> renderer and solver views
    -> signed result/evidence sidecars linked back into Q42
```

Dense geometry should not be expanded into one NQuin per vertex. P64 also must not be reused:
P64 is the page-aligned model-weight container.

## 2. Verified current capability

### 2.1 Asset ingest and rendering

| Capability | Status | Boundary |
|---|---|---|
| OBJ/STL/GLB detection and CLI ingest | Implemented | Input is limited to 256 MiB by the CLI guard. |
| GLB parser | Implemented subset | Self-contained GLB, `POSITION` as f32 `VEC3`, scalar u8/u16/u32 indices, and triangle primitives. |
| Q42 mesh facts | Implemented | Type, source format, vertex/triangle counts, AABB, and centroid. |
| Unified Q42 v3 output | Implemented | Embedded lexicon/index, LZ4 SuperBlocks, mmap reader, graph-oriented lookup. |
| Native/WASM mesh renderer | Implemented subset | Positions, per-vertex colours, triangle indices, depth, bloom, picking, offscreen RGBA8. |
| Physics admission around rendered artefacts | Implemented | AABB transforms, bounds/material-floor refusal, mass/momentum, and PGA kinematic joints. |
| Full glTF scene preservation | Missing | Normals, tangents, UVs, materials, textures, node hierarchy/transforms, skins, animation, morphs, sparse accessors, and general primitive modes are not preserved. |
| Compiled geometry sidecar | Missing | There is no canonical mmap/GPU-ready mesh payload emitted by `ingest asset`. |
| Geometry-to-FEA linkage | Reserved but unwired | `QualiaSuperBlock::fea_mesh_index_id` exists, but current Q42 block encoding leaves it zero. |

`render/assets.rs` is the canonical parser to extend. The separate
`webizen-desktop/src/commands/glb_ingest.rs` Anatomy prototype should not become a second
asset compiler. It reads a file into a `Vec`, assumes positions begin at byte zero of the BIN
chunk, ignores accessor/buffer-view offsets and strides, and maps ontology identifiers into
unrelated Tensor10D axes. Those assumptions are not valid for general glTF.

One current correctness item also belongs in the implementation backlog:
`mesh_to_nquins` constructs mesh facts with zero parity rather than the normal
`subject ^ predicate ^ object ^ context` fold. The canonical compiler must emit valid parity
and tests must reject invalid records.

### 2.2 Numerical and engineering substrate

The following are useful, real building blocks:

- dense and fixed-size linear algebra: GEMM, LU, QR, Cholesky, SVD, symmetric eigensolvers,
  vector operations, and spectral routines;
- RK4 ODE integration, numerical integration, interpolation, optimization, statistics,
  transforms, vector calculus, exact rational/integer support, and dimensional units;
- a tested 1-D steady thermal-conduction finite-difference solver with Dirichlet and Neumann
  boundary conditions;
- a bounded 2-D laminar incompressible CFD implementation with explicit validation and
  convergence/error reporting;
- axial member stress, strain, displacement, and factor-of-safety calculation;
- Cauchy stress invariants, principal stress, von Mises/Tresca values, drag, Reynolds number,
  Basquin fatigue life, and Miner cumulative damage;
- one-degree-of-freedom transient dynamics and basic kinematics;
- Monte Carlo component/system reliability calculations;
- a 1-D Burgers-equation physics path and CFL/diffusion time-step helpers;
- CPU-oracled and GPU-certified WGSL Forge kinematic and molecular-dynamics steps; and
- deterministic render-physics admission, AABBs, material mass, momentum, and joint motors.

These capabilities support analytical checks, screening models, small bounded simulations,
oracles, and future solver assembly. They do **not** yet constitute:

- arbitrary 3-D finite-element structural analysis;
- tetrahedral/hexahedral mesh generation and quality repair;
- assembled sparse stiffness/mass matrices and boundary-condition elimination;
- nonlinear contact, plasticity, fracture, large deformation, or coupled multiphysics;
- 3-D compressible or turbulent CFD;
- mesh-convergence or model-form uncertainty automation; or
- certification to a named engineering code or regulator.

Several large `engineering_analysis`, `physics_simulation`, medical, and chemistry types are
configuration/data scaffolds. A struct named `FiniteElementSolver`, `TurbulenceModeling`, or
`SafetyAssessment` is not evidence that its solver or assurance process exists.

### 2.3 Biological, medical, and chemical substrate

| Area | Implemented capability | Important boundary |
|---|---|---|
| DICOM | Part-10 metadata parsing, pixel split, mmap blob store, Q42/WAL semantic records, series lookup, organ-overlay support | Not a volumetric segmentation or patient-specific biomechanics solver. |
| Anatomy assets | GLB asset catalogue and semantic-ID extraction prototype | Its direct BIN parsing and Tensor10D mapping are not a valid general compiler. |
| Bioinformatics | Smith-Waterman, Needleman-Wunsch, BLOSUM/nucleotide scoring, k-mers, MinHash, FASTA validation, phylogenetic merge, translation, peptide helpers | Sequence computation, not tissue mechanics. |
| Clinical computation | Framingham, CHA2DS2-VASc, SCORE2, renal and SOFA calculations, drug interaction/contraindication checks, FHIR observation validation, longitudinal trends | Decision-support kernels are not autonomous diagnosis or regulatory approval. |
| Organic chemistry | SMILES graph parsing, descriptors, drug-likeness filters, fingerprints, functional groups, thermochemistry, pKa and green metrics | Descriptor calculations are not a validated molecular-design claim. |
| Molecular dynamics | Lennard-Jones force field, velocity-Verlet, energy/temperature observables, refusal on missing parameters | Small classical model; no bonded biomolecular force field, solvent, long-range electrostatics, or clinical inference. |
| Governance/privacy | Sensitivity classes, delegated access, deontic/epistemic/paraconsistent logic, HE/DP, audit/provenance building blocks | Policies and cryptography still require a concrete validated workflow. |

Biological 3D work therefore starts with governed anatomy/imaging visualisation and only moves
to tissue or organ mechanics after explicit constitutive models, boundary conditions, material
provenance, uncertainty, and validation data exist.

## 3. Compiled asset bundle

### 3.1 Artifact responsibilities

| Artifact | Responsibility |
|---|---|
| Source GLB/OBJ/STL | Immutable authoring/interchange source and round-trip reference. |
| Q42 volume | Asset/component identity, units, coordinate frame, scene relationships, semantics, policy, provenance, material/load/BC references, solver/result receipts. |
| Geometry sidecar | GPU/CPU-ready vertex streams, indices, primitive ranges, hierarchy, material slots, LODs, meshlets/BVH, adjacency, and hashes. |
| Analysis-mesh sidecar | Surface/volume cells, node and element IDs, groups, constraints, material regions, quality metrics, and source-geometry correspondence. |
| Field sidecar | Dense scalar/vector/tensor results over nodes, cells, voxels, time steps, or spectral coordinates. |
| P64 | Model weights only. |
| Tensor10D | Semantic/spatiotemporal projection of an asset, observation, or result—not raw geometry or a generic ndarray. |

The sidecar layout should be selected only after fixtures measure mmap, GPU upload, and solver
access. It should nevertheless have these invariant properties:

- content digest and exact byte length;
- schema version, endian marker, scalar type, units, and coordinate frame;
- 4 KiB/page-friendly section offsets with checked bounds;
- independently checksummed sections;
- structure-of-arrays views where GPU or SIMD access benefits;
- bounded counts and overflow-safe parsing;
- immutable source-to-compiled lineage; and
- stable component/primitive/element IDs shared with Q42 facts and picking.

### 3.2 Q42 computational-asset manifest

The Q42 graph should identify at least:

- source and compiled payload digests;
- asset, scene, node, component, primitive, material, texture, and LOD identities;
- original and canonical units, handedness, axis convention, and transform chain;
- bounds, counts, topology diagnostics, and mesh-quality evidence;
- render mesh versus analysis mesh and their correspondence map;
- named sets for loads, supports, contacts, inlets/outlets, regions, and sensors;
- material-property values with units, temperature/rate dependencies, source, and uncertainty;
- requested compute tier and assurance class;
- solver/kernel versions, schedules, tolerances, convergence criteria, and adapter identity;
- result-field sidecars and summary claims;
- verification, validation, human review, signature, supersession, and rejection records.

`fea_mesh_index_id` should become a real block/asset linkage or be superseded by a documented,
versioned Q42 predicate. It must not remain a suggestive field that every writer silently zeros.

## 4. Two-axis tier model

Compute cost and safety assurance are orthogonal. A week-long simulation with an unvalidated
model can be less trustworthy than a closed-form calculation with a known domain of validity.
Every run must therefore declare both a **fidelity tier** and an **assurance class**.

### 4.1 Computational fidelity

| Tier | Name | Intended use | Examples |
|---|---|---|---|
| F0 | Asset | Inspect, query, render, measure basic geometry | Canonical GLB, AABB, picking, component graph |
| F1 | Interactive | Fast deterministic motion and rough physical feedback | PGA joints, collision/bounds admission, simple mass/momentum, coarse kinematics |
| F2 | Analytical | Screening and bounded reduced-order computation | Axial member, 1-D thermal, stress invariants, fatigue/drag estimates, 1-DOF dynamics |
| F3 | Numerical | Mesh/grid simulation with convergence evidence | Current bounded 2-D laminar CFD; future 2-D/3-D FEM, finite-volume thermal/flow |
| F4 | Coupled/high-fidelity | Expensive nonlinear, transient, stochastic, or multiphysics work | Contact/plasticity, FSI, patient-specific tissue mechanics, ensemble/UQ |

A lower tier remains available when it is sufficient. The scheduler chooses the lowest tier
that satisfies the declared purpose, error target, and assurance policy; it must not silently
downgrade a requested run.

### 4.2 Assurance

| Class | Meaning | Minimum evidence |
|---|---|---|
| A0 | Exploratory | Inputs and output labelled provisional; no decision claim. |
| A1 | Reproducible | Immutable inputs, exact versions, deterministic seed/profile, rerunnable receipt. |
| A2 | Verified | Unit/dimension checks, CPU or independent numerical oracle, residuals, regression fixtures, declared validity domain. |
| A3 | Validated | Comparison with experimental/reference data, mesh/time-step sensitivity, uncertainty budget, independent reviewer approval. |
| A4 | Safety/regulated support | Applicable standards and load cases, conservative factors, traceable material/test data, independent solver or benchmark, signed competent-human decision and change control. |

Qualia may provide A4-grade evidence orchestration, policy enforcement, provenance, and
repeatability. Software output alone does not certify a bridge, medical device, aircraft part,
biological intervention, or patient-specific decision.

### 4.3 Domain profiles

The same axes apply with domain-specific gates:

- **Engineering:** units, material provenance, load combinations, supports/contact, mesh
  quality, convergence, safety factors, code clauses, fatigue/environment, and reviewer role.
- **Biological research:** anatomical registration, constitutive model, specimen/population
  provenance, parameter uncertainty, experimental validation, and explicit non-clinical label.
- **Clinical/patient-specific:** consent, sensitivity isolation, DICOM de-identification and
  geometry registration, clinical validation, intended use, human oversight, and regulatory
  change control.
- **Chemical/molecular:** force-field/version provenance, parameter coverage, equilibration,
  ensemble, time step, energy drift, sampling convergence, and experimental comparison.

## 5. Required workflows

### 5.1 Asset compilation

1. Stream/mmap and hash the immutable source.
2. Parse the complete supported glTF scene with checked offsets/strides.
3. Canonicalise units, axes, transforms, primitive attributes, and material references.
4. Validate finite values, bounds, indices, topology, resource limits, and URI handling.
5. Emit valid-parity semantic Q42 facts.
6. Emit page-aligned compiled geometry and optional analysis-mesh sidecars.
7. Record source-to-compiled hashes, capability losses, and tool versions.
8. Differentially render the source/reference and compiled form.

### 5.2 Engineering run

1. Select purpose, F-tier, A-class, validity domain, and acceptance criteria.
2. Bind geometry/analysis mesh, units, materials, loads, supports, contacts, and environment.
3. Run SHACL plus dimensional and physical admissibility checks.
4. Estimate CPU heap, Sentinel pass, VRAM, time, and thermal budget.
5. Execute with bounded cancellation/checkpoint policy.
6. Persist residual history, convergence, warnings, and result fields.
7. Run tier-required oracle, sensitivity, validation, and uncertainty checks.
8. Emit attributed claims; require human sign-off where policy demands it.

### 5.3 Biological or medical run

Use the engineering workflow plus:

- patient/specimen and consent scope;
- imaging registration and segmentation provenance;
- anatomical-coordinate transform and uncertainty;
- constitutive/biological parameter source and population applicability;
- separation of research visualisation, decision support, and clinical intended use; and
- deontic denial of any run or disclosure outside its authorized context.

## 6. Implementation priorities

1. Fix mesh-fact parity and add exact GLB-to-Q42 regression tests.
2. Replace the desktop Anatomy parser with the canonical core importer.
3. Preserve full supported glTF scenes and introduce stable component IDs.
4. Define and benchmark the compiled geometry sidecar.
5. Wire Q42 asset manifests and the analysis-mesh link.
6. Add surface/volume mesh quality and source correspondence.
7. Expose existing F0-F2 kernels through one typed computational-asset API.
8. Add units, validity-domain, convergence, uncertainty, and evidence receipts.
9. Implement a small verified FEM/thermal vertical slice before broad 3-D claims.
10. Add biological profiles only after the common mesh/field/evidence contracts are stable.

## 7. Claims the UI and API must not make

- “GLB converted to Q42” must not imply that all geometry/material/animation data is in Q42.
- “FEA available” must not refer only to an `fea_mesh_index_id` field or solver-shaped structs.
- “High fidelity” must not imply “validated” or “safe.”
- A displayed factor of safety must name its load case, material property, units, model,
  uncertainty, and validity domain.
- A biological/anatomical model must not be labelled patient-specific without registered
  patient data and the required consent/validation trail.
- A clinical or safety-critical result must not be auto-approved by a model confidence score.

