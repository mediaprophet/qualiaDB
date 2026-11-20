# `yaml-ld-q42` v2 Specification — Unified Hypermedia Asset, HCF Document & Manifold Workspace Standard

_Version: 2.0.0 | Target: QualiaDB (`crates/qualia-core-db/src/q42/yaml_ld_q42.rs`), Synth (`synth_compile_q42`), QualiaPortal & Webizen Platform_

---

## 1. Overview & Architectural Purpose

**`yaml-ld-q42`** (`.q42.yaml`, `.hcf.yaml`, `.qapp.yaml`) is QualiaDB's human-authored declarative YAML-LD format that compiles directly into **48-byte `NQuin` records** and **`Q42LEX` lexicon entries** inside unified `.q42` v4 volumes.

Rather than scattering asset, document, and workspace metadata across disconnected JSON files (`synth.project.json`, `lod-manifest.json`, `manifest.ecopack.jsonld`) that sit outside the Q42 semantic graph, `yaml-ld-q42` v2 provides a single Linked Data YAML grammar with deterministic `compile_yaml_ld_q42_auto` dispatch across three canonical shapes:

1. **`yaml-ld-q42/asset` (`@type: HypermediaAssetManifest`)** — Unified asset, Synth project, DCAT distribution, Bill of Materials (BOM) components/organs, `InstanceState64` schema, 3-profile (`film`, `open_world_game`, `cartoon`), LOD ladder (`lod0`..`lod2`), functional ports, and licensing/commons (`PermissiveCommons` / `SocialPermissiveCommons`) manifest for **`qualia-poiesis`** ("World of Man") and **`intenteco`** ("World of God") repositories.
2. **`yaml-ld-q42/workspace` (`@type: WebizenWorkspace`)** — Declarative QApp user interface layouts, 2D CSS-grid pane placements (with packed `(x, y, w, h)` bounding boxes in the lower 32 bits of `NQuin.metadata`), semantic `data_bindings`, `theme_tokens`, and Phase 5 **Manifold World `views:`** (`Scene3D` / `Pane2D`, `manifold`, `sensitivity: Public | RightsBounded`, `requires_attestation`).
3. **`yaml-ld-q42/hcf` (`@type: HypermediaDocument`)** — Hypermedia Content Format (HCF) structured documents, specifications, publications, and creative/intellectual works with sections, typed blocks (`Paragraph`, `Figure`, `CodeBlock`, `Equation`, `Callout`), `hcf:text`, optional `hcf:markdown` projector fields, `hcf:licence`, and `hcf:depictsAsset` references.

---

## 2. Deterministic NQuin & Q42LEX Compilation Rules

Every `yaml-ld-q42` compiler invocation takes `(yaml_bytes: &[u8], namespace: u64, lamport_clock: u64)` and produces `(Vec<NQuin>, HashMap<u64, String>, &'static str)`:

1. **Surface Hashing & Lexicon Population (`Q42LEX`):**
   - Every subject IRI/DID, predicate IRI, and object surface string is hashed with `qualia_core_db::q_hash(surface)` and inserted into `lexicon: HashMap<u64, String>` so the resulting `.q42` v4 volume retains lossless reverse-lookup in `Q42LEX`.
2. **48-Byte `NQuin` Layout & Parity Invariant:**
   - Each emitted statement is a 48-byte `NQuin`:
     - `subject: u64` — `q_hash(subject_surface)`
     - `predicate: u64` — `q_hash(predicate_iri)`
     - `object: u64` — `q_hash(object_surface)`
     - `context: u64` — caller-supplied `namespace` (named graph / DID context hash)
     - `metadata: u64` — `(lamport_clock << 32) | lower_32_bits` (where `lower_32_bits` is `0` for standard statements or `(x << 24) | (y << 16) | (w << 8) | h` for 2D grid pane placements)
     - `parity: u64` — `subject ^ predicate ^ object ^ context`
3. **Auto-Dispatch Order (`compile_yaml_ld_q42_auto`):**
   - **Rule 1 (`yaml-ld-q42/asset`):** Dispatched when `@type` includes `HypermediaAssetManifest`, `PoiesisArtefact`, `SpeciesEmbodiment`, `dcat:Dataset`, `made:Artefact`, or `bio:DarwinianTaxon`, or when top-level `distributions:`, `lods:`, `components:`, or `project:` is present.
   - **Rule 2 (`yaml-ld-q42/workspace`):** Dispatched when `@type` includes `WebizenWorkspace` or when top-level `pages:` is a non-empty sequence.
   - **Rule 3 (`yaml-ld-q42/hcf`):** Dispatched when `@type` includes `HypermediaDocument` or when top-level `content:` is a non-empty sequence.
   - Unknown or empty shapes fail closed with a descriptive error.

---

## 3. Shape 1: `yaml-ld-q42/asset` (`@type: HypermediaAssetManifest`)

A single `manifest.q42.yaml` at the root of an asset repository (`C:\github\poiesis\<thing-slug>\manifest.q42.yaml` or `C:\github\eco-intent\<species-slug>\manifest.q42.yaml`) serves as:
- The **Synth project descriptor** (read by `synth/src/project.rs` for `name`, `brief`, `project.working`, `project.delivered`, and `project.sources`).
- The **DCAT package & distribution index** (compiled via `synth_compile_q42` or `qualia-cli ingest` into `dist/index.q42`).
- The **LOD, profile, component/organ, port, and `InstanceState64` manifest**.

### 3.1 Example `manifest.q42.yaml`

```yaml
---
"@context":
  "@vocab": "https://qualiadb.org/schema/q42#"
  dct: "http://purl.org/dc/terms/"
  dcat: "http://www.w3.org/ns/dcat#"
  prov: "http://www.w3.org/ns/prov#"
  odrl: "http://www.w3.org/ns/odrl/2/"
  qudt: "http://qudt.org/schema/qudt/"
  made: "urn:qualia:made:"
  q42state: "urn:qualia:state:"
"@id": "urn:qualia:design:rover-field-utility-4wd:rev1"
"@type":
  - "HypermediaAssetManifest"
  - "made:Artefact"
  - "dcat:Dataset"

name: "rover-field-utility-4wd"
title: "NPWS 4WD Field Utility & Ecological Survey Rover"
brief: "Made-World Hypermedia (.hmc) Digital Twin — 1:1 SI metric articulated 4WD utility vehicle"
did: "did:q42:design:rover_field_utility_4wd"
domain: "made_world"
fiction: false
quality_profile: "production16"
landing_page: "https://github.com/qualia-poiesis/rover-field-utility-4wd"
licence_policy: "PermissiveCommons"

project:
  working: "ontology"
  delivered: "dist"
  sources:
    - "manifest.q42.yaml"
    - "ontology/artefact.ttl"
    - "ontology/shapes.shacl.ttl"
    - "source/rover-field-utility-4wd.vibe"
    - "source/actuation.vibe"

dimensions_si:
  length_m: 4.92
  width_m: 1.88
  height_m: 1.96
  wheelbase_m: 2.85
  curb_mass_kg: 2280.0

profiles:
  - id: "film"
    target: "linear_render"
    resource: "dist/rover-field-utility-4wd-pbr.10d"
  - id: "open_world_game"
    target: "realtime_streaming"
    resource: "dist/lods/rover-field-utility-4wd-lod0.10d"
  - id: "cartoon"
    target: "stylized_realtime"
    shading_model: "Stylized"
    resource: "dist/rover-field-utility-4wd.10d"

components:
  - id: "chassis-frame"
    did: "did:q42:design:rover_field_utility_4wd#part-chassis-frame"
    class: "made:StructuralFrame"
    label: "Ladder-frame high-tensile steel chassis"
  - id: "wheel-assembly-fl"
    did: "did:q42:design:rover_field_utility_4wd#part-wheel-assembly-fl"
    class: "made:WheelAssembly"
    label: "Front-left alloy rim, ventilated brake disc & all-terrain pneumatic tire"

ports:
  - id: "aux-dc-bus-24v"
    did: "did:q42:design:rover_field_utility_4wd#port-aux-dc-bus-24v"
    medium: "electrical_dc"
    direction: "bidirectional"
    unit: "qudt:Watt"
    rated_capacity: 2400.0

instance_state_64:
  id: "q42state:rover_instance_state_64"
  struct_size_bytes: 64
  axes: "vec4<f32>: [steering_angle_rad, suspension_compression_norm, mud_dust_weathering, battery_soc_norm]"
  stages:
    - id: "factory_new"
      min: 0.00
      max: 0.15
    - id: "field_service"
      min: 0.15
      max: 0.75
    - id: "weathered_veteran"
      min: 0.75
      max: 1.00

distributions:
  - id: "hmc-container"
    title: "Hypermedia Container (rover-field-utility-4wd.hmc)"
    format: "application/vnd.qualia.hmc"
    path: "dist/rover-field-utility-4wd.hmc"
    derived_from: "source/rover-field-utility-4wd.vibe"
    sha256: "87433b98ff8cee0a96b36cee213c72e116e45f2fa0d54e543e8eb0304468d4ba"
    bytes: 46096680
  - id: "artefact-q42"
    title: "Sealed NQuin Design, BOM & Physics Ontology (artefact.q42)"
    format: "application/vnd.qualia.q42"
    path: "dist/artefact.q42"
    derived_from: "ontology/artefact.ttl"
  - id: "index-q42"
    title: "Sealed NQuin Package Index (index.q42)"
    format: "application/vnd.qualia.q42"
    path: "dist/index.q42"
    derived_from: "manifest.q42.yaml"

lods:
  - id: "lod0"
    file: "dist/lods/rover-field-utility-4wd-lod0.10d"
    distance_band_m: "0-25m"
    vertices: 39521
    triangles: 65000
    sha256: "bc87c0d730f28981672646bee589f1f78f533580d2784b160b9b6df5fd2ed65b"
  - id: "lod1"
    file: "dist/lods/rover-field-utility-4wd-lod1.10d"
    distance_band_m: "25-90m"
    vertices: 14296
    triangles: 17798
  - id: "lod2"
    file: "dist/lods/rover-field-utility-4wd-lod2.10d"
    distance_band_m: ">90m"
    vertices: 3183
    triangles: 3898
```

### 3.2 Emitted NQuin Predicates for `yaml-ld-q42/asset`

| Section / Field | Subject | Predicate | Object |
| :--- | :--- | :--- | :--- |
| `@type` | `<@id>` | `rdf:type` | `<type>` (plus `HypermediaAssetManifest` & `dcat:Dataset`) |
| `did` | `<@id>` | `owl:sameAs` / `dct:identifier` | `<did>` |
| `name` / `title` / `brief` | `<@id>` | `q42:slug` / `dct:title` / `dct:description` | literal string |
| `domain` / `quality_profile` | `<@id>` | `q42:domain` / `q42:qualityProfile` | `"made_world"` / `"production16"` |
| `landing_page` / `licence_policy` | `<@id>` | `dcat:landingPage` / `odrl:hasPolicy` | URL / policy tag |
| `project.{working,delivered,sources}` | `<@id>` | `synth:workingDir` / `synth:deliveredDir` / `synth:hasSource` | relative path |
| `dimensions_si.<metric>` | `<@id>` | `q42:dimension:<metric>` | value string |
| `profiles[]` | `<@id>` $\to$ `<@id>#profile-<id>` | `q42:hasDeliveryProfile`, `q42:profileTarget`, `q42:profileResource`, `q42:shadingModel` | profile node & attributes |
| `components[]` (or `organs[]`) | `<@id>` $\to$ `<did_or_part_id>` | `dct:hasPart`, `made:hasComponent`, `rdf:type`, `dct:identifier`, `rdfs:label` | component node & attributes |
| `ports[]` | `<@id>` $\to$ `<did_or_port_id>` | `made:hasPort`, `made:portMedium`, `made:portDirection`, `qudt:unit`, `made:ratedCapacity` | port node & attributes |
| `instance_state_64` | `<@id>` $\to$ `<schema_id>` | `q42state:hasInstanceStateSchema`, `q42state:structSizeBytes`, `q42state:environmentalStressAxes`, `q42bio:hasDevelopmentalStage` | schema & stage bounds |
| `distributions[]` | `<@id>` $\to$ `<@id>#dist-<id>` | `dcat:distribution`, `rdf:type` (`dcat:Distribution`), `dct:title`, `dct:format`, `dcat:accessURL`, `prov:wasDerivedFrom`, `q42:sha256`, `dcat:byteSize` | distribution node & attributes |
| `lods[]` | `<@id>` $\to$ `<@id>#lod-<id>` | `q42:hasLodLevel`, `dcat:accessURL`, `q42:distanceBandMeters`, `q42:vertexCount`, `q42:triangleCount`, `q42:sha256` | LOD node & attributes |
| `epistemic_zone` / `embodiment_mode` | `<@id>` | `q42:epistemicZone` / `q42:embodimentMode` | `"reality" \| "hypothesis" \| "mixed_creative"` / `"computational_only" \| "archetype_inline" \| "archetype_instance"` |
| `labels.<lang>` | `<@id>` | `rdfs:label` | `"<utf8_label>@<lang>"` (full UTF-8 / UTF-16 Unicode in `Q42LEX`, zero ASCII restriction) |
| `coordinates_10d` | `<@id>` | `q42:coord10d:{q,v,w,x,y,z,t,alpha,mu,sigma}` | 10D coordinate value |
| `archetype_ref` | `<@id>` $\to$ `<@id>#archetype-ref` | `q42:hasArchetypeRef`, `q42:archetypeDigest`, `q42:archetypeEntityId`, `q42:taxonOrClassIri`, `q42:proceduralSeed`, `q42:ageDays`, `q42:ontogeneticStage` | Mode-B reference to a canonical Mode-A Archetype `.10d` (`FIELD_KIND_ARCHETYPE_REF = 0x0301`) |
| `computational_strata[]` | `<@id>` $\to$ `<@id>#stratum-<id>` | `q42:hasComputationalStratum`, `q42:stratumLevel` (`L0..=L6`), `q42:constituentKind`, `q42:interactionRegime`, `q42:embodimentState`, `q42:taxonOrOntologyIri`, `q42quantum:quantumRegime`, `q42quantum:hamiltonianOperator`, `q42quantum:dephasingTimePicoseconds`, `q42quantum:relaxationTimePicoseconds`, `q42quantum:vonNeumannEntropy`, `q42quantum:tunnelingSplittingMev`, `q42quantum:chernInvariant`, `q42chem:hartreeEnergy`, `q42chem:homoLumoGapEv`, `q42chem:gtoBasisSet`, `q42chem:xcFunctional`, `q42chem:qForgeStateVectorNQuin`, `q42logic:governedByN3Rule`, `q42logic:belnapTruthStatus` | 7-layer multi-scale nested strata (`MSTR` `0x0304`: organs, follicles/tissues, symbiotic fungi/bacteria/parasites, cells, molecular pathways, and `L6` **Quantum Physics** — quantum dynamics, coherence/entanglement, tunneling, spin/radical-pair dynamics, QED, topological phases, and the quantum chemistry subset) |

---

## 4. Shape 2: `yaml-ld-q42/workspace` (`@type: WebizenWorkspace`)

Compiles QApp 2D pane layouts and Phase 5 Manifold World 3D/2D views into `NQuin`s and `Q42LEX` lexicon entries:

```yaml
---
"@context": "https://webizen.org/q42"
"@id": "did:q42:app:poiesis-inspector"
"@type": "WebizenWorkspace"

theme_tokens:
  primary: "#58a6ff"
  bg: "#0d1117"

pages:
  - name: "Asset Digital Twin Inspector"
    url_path: "/inspect"
    panes:
      - component_id: "bom-tree-pane"
        x: 1
        y: 1
        w: 4
        h: 8
        data_bindings:
          - "did:q42:design:rover_field_utility_4wd#bom"
    views:
      - manifold: "did:q42:design:rover_field_utility_4wd"
        kind: "Scene3D"
        sensitivity: "Public"
      - manifold: "did:q42:design:rover_field_utility_4wd"
        kind: "Pane2D"
        sensitivity: "RightsBounded"
        requires_attestation: true
```

---

## 5. Shape 3: `yaml-ld-q42/hcf` (`@type: HypermediaDocument`)

Compiles structured documents, specifications, and creative/intellectual works into `NQuin`s and `Q42LEX` lexicon entries:

```yaml
---
"@context":
  "@vocab": "https://qualiadb.org/schema/hcf#"
"@id": "doc:poiesis_design_manual"
"@type": "HypermediaDocument"
title: "Poiesis Engineering Design Manual"
author: "Timothy Holborn"
date: "2026-11-20"
licence: "PermissiveCommons"
summary: "Engineering and physical substrate manual for made-world assets."
depicts:
  - "did:q42:design:rover_field_utility_4wd"
tags:
  - "poiesis"
  - "yaml-ld-q42"
content:
  - "@type": "Section"
    heading: "1. Structural Chassis Frame"
    blocks:
      - "@type": "Paragraph"
        text: "The ladder-frame chassis is authored in 1:1 SI metres."
        markdown: "The **ladder-frame chassis** is authored in `1:1` SI metres."
```
