# Tool-Chest Spec — Code, AI & Spatial Toolboxes

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Ontology:** [`qualia-ui/ontologies/code.n3`](../ontologies/code.n3) (N3 authoring → CBOR-LD runtime)

These three toolboxes serve QualiaDB / Webizen / human-centric application (QApp) development. They are **loosely coupled** to containers — the same toolbox can be used across different container and manifold types. A VibeScript editor tool works in a `code` container, a `doc` container (embedded code block), or a `vibe` manifold.

---

## 1. Toolbox: `code` (VibeScript & QApp Development)

The `code` toolbox is for writing, evaluating, debugging, and packaging VibeScript and QApp projects. It is the native equivalent of an IDE — but for Vibe, not JavaScript.

### 1.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `code` | content | live | VibeScript editing, eval, diagnose. Reaches `poet_eval`, `capability.invoke`. |
| `project` | content | missing | Project tree view — modules, ontologies, assets, tests. |
| `terminal` | content | missing | VibeScript REPL / command output. |
| `inspector` | panel | missing | Inspects active module's AST, capabilities, effects. |
| `outline` | panel | missing | Module outline — functions, types, capabilities. |
| `diagnostics` | panel | missing | Compiler diagnostics, effect-check errors, capability violations. |
| `property-sheet` | panel | missing | Edits module properties (entry point, capabilities, build target). |

### 1.2 Tool-chains

#### `vibe` — VibeScript language tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `vibe-console` | Mutate | `source: string` | Opens a VibeScript REPL session |
| `vibe-eval` | Query | `source: string`, `capabilities: [string]` | Evaluates a VibeScript expression with declared capabilities |
| `vibe-diagnose` | Query | `module_iri: iri` | Runs static analysis — effect checking, capability validation, type inference |
| `vibe-syntax-check` | Query | `source: string` | Lexer + parser check without evaluation |
| `vibe-format` | Mutate | `source: string`, `style: string` | Formats VibeScript source (indentation, spacing) |
| `vibe-outline` | Query | `module_iri: iri` | Generates module outline (functions, types, capabilities) |

#### `quin` — RDF graph statement tools

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `quin-statement` | Mutate | `subject: iri`, `predicate: iri`, `object: term` | Constructs an RDF-Star triple statement |
| `quin-inspect` | Query | `triple_iri: iri` | Inspects a triple's metadata, provenance, reifications |
| `quin-ref` | Query | `term: iri` | Finds all triples referencing a term |
| `quin-reify` | Mutate | `triple_iri: iri`, `metadata: CBOR-LD` | Adds reification metadata to a triple |
| `quin-validate` | Query | `triple_iri: iri`, `shape_iri: iri` | Validates a triple against a SHACL shape |

#### `project` — QApp project management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `new-project` | Mutate | `name: string`, `qapp_type: QAppType`, `build_target: BuildTarget` | Creates a new QApp project scaffold |
| `add-module` | Mutate | `project_iri: iri`, `module: Module` | Adds a module (VibeScript, native WASM, ontology, or hybrid) |
| `remove-module` | Mutate | `module_iri: iri` | Removes a module from the project |
| `build-project` | Mutate | `project_iri: iri`, `target: BuildTarget` | Builds the project (WASM, native, HCF, or CBOR-LD) |
| `test-project` | Query | `project_iri: iri` | Runs all tests (unit, integration, golden corpus) |
| `package-project` | Mutate | `project_iri: iri`, `target: BuildTarget` | Packages the built project for distribution |
| `deploy-project` | Mutate | `project_iri: iri`, `target_iri: iri` | Deploys to a target (local runtime, HCF bundle, or CBOR-LD serialisation) |
| `inspect-manifest` | Query | `project_iri: iri` | Views the project manifest (modules, capabilities, build targets) |

#### `hcf` — Hypermedia Content Format authoring

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `hcf-author` | Mutate | `source: yaml-ld-q42`, `assets: [iri]` | Authors an HCF document (YAML-LD-Q42 authoring form) |
| `hcf-compile` | Mutate | `source_iri: iri` | Compiles HCF from YAML-LD-Q42 to CBOR-LD |
| `hcf-validate` | Query | `bundle_iri: iri` | Validates an HCF bundle against the HCF schema |
| `hcf-inspect` | Query | `bundle_iri: iri` | Inspects an HCF bundle's structure (assets, ontologies, manifests) |
| `hcf-extract` | Query | `bundle_iri: iri`, `asset_iri: iri` | Extracts a single asset from an HCF bundle |

#### `n3-ontology` — N3 ontology authoring and compilation

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `n3-author` | Mutate | `source: string`, `prefixes: [Prefix]` | Creates or edits an N3 ontology source file |
| `n3-compile` | Mutate | `source_iri: iri` | Compiles N3 to CBOR-LD (`ontology.cbor`) |
| `n3-validate` | Query | `source_iri: iri` | Validates N3 against the tool-chest ontology schema |
| `n3-import` | Query | `source_iri: iri` | Resolves and lists all `@import` dependencies |
| `n3-prefix-check` | Query | `source_iri: iri` | Checks prefix declarations for lexicon compliance |
| `ontology-load` | Mutate | `cbor_iri: iri`, `registry: iri` | Loads a compiled CBOR-LD ontology into the OntologyRegistry |
| `ontology-query` | Query | `registry: iri`, `term: iri` | Queries the OntologyRegistry for a term, class, or property |

#### `capability` — capability declaration and Sentinel interaction

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `declare-capability` | Mutate | `scope: string`, `reason: string`, `optional: bool` | Adds a capability declaration to a module or QApp |
| `inspect-sentinel` | Query | `module_iri: iri` | Shows the Sentinel's view of a module's capabilities |
| `test-capability-gating` | Query | `module_iri: iri`, `scope: string` | Tests whether a capability is granted for a module |
| `grant-capability` | Mutate | `module_iri: iri`, `scope: string` | Grants a capability (requires `capability:invoke` — Sentinel enforced) |
| `revoke-capability` | Mutate | `module_iri: iri`, `scope: string` | Revokes a capability |
| `audit-capabilities` | Query | `qapp_iri: iri` | Audits all capability declarations and grants for a QApp |

### 1.3 Directory structure

```
toolboxes/code/
├── mod.rs
├── ontology.n3                  ← imports code.n3
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── vibe/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   ├── quin/
│   │   ├── mod.rs
│   │   └── tools/               ← 5 tools
│   ├── project/
│   │   ├── mod.rs
│   │   └── tools/               ← 8 tools
│   ├── hcf/
│   │   ├── mod.rs
│   │   └── tools/               ← 5 tools
│   ├── n3_ontology/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   └── capability/
│       ├── mod.rs
│       └── tools/               ← 6 tools
```

### 1.4 Capability scope

- `graph:read`, `graph:mutate` — reading/writing the project's knowledge graph
- `poet:eval` — evaluating VibeScript
- `poet:diagnose` — static analysis
- `capability:invoke` — capability management (Sentinel-gated)
- `ontology:load`, `ontology:query` — ontology registry interaction
- `build:compile` — building projects
- `build:package` — packaging projects

---

## 2. Toolbox: `ai` (Symbolic + Neural + Neuro-Symbolic Stack)

The `ai` toolbox is for building, running, and inspecting the AI stack. It is **not** just LLMs — it covers the full symbolic → neural → bridge → agent pipeline. The toolbox is loosely coupled: the same tools work in an `agent-console` container, a `doc` container (for NLP annotation), or a `graph` container (for GraphRAG).

### 2.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `agent-console` | content | missing | Agent runtime console — task planning, execution, verification. |
| `nlp-pipeline` | content | missing | NLP extraction pipeline view — text in, structured substrate out. |
| `graphrag` | content | missing | GraphRAG query and retrieval view. |
| `diagnostics` | panel | missing | Agent diagnostics — traces, capability usage, grounding scores. |
| `sentinel-tray` | panel | partial | Sentinel status — capability enforcement, agency claim evaluation. |
| `corpus-browser` | panel | missing | Golden corpus browser — verified examples for evaluation. |
| `inspector` | panel | missing | Inspects active AI component (gazetteer, FST, transformer, etc.). |

### 2.2 Tool-chains

#### `symbolic` — deterministic symbolic AI

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `run-gazetteer` | Query | `text: string`, `ontology_iri: iri` | Runs Aho-Corasick gazetteer matching over text, returns matches with byte spans |
| `run-fst` | Query | `word: string`, `dictionary: iri` | Runs FST morphology lookup |
| `run-coref-sieve` | Query | `text: string`, `entities: [Entity]` | Runs multi-pass coreference resolution sieve |
| `run-frame-semantics` | Query | `text: string`, `frame_ontology: iri` | Identifies frame elements and roles |
| `run-temporal-parser` | Query | `text: string` | Extracts and normalises temporal expressions (TimeML-style) |
| `run-geo-parser` | Query | `text: string`, `gazetteer: iri` | Extracts and normalises geographic entities (GeoSPARQL-compatible) |
| `run-quantity-normalizer` | Query | `text: string` | Detects, converts, and normalises quantities (QUDT-compatible) |
| `run-relation-extractor` | Query | `text: string`, `ontology: iri` | Extracts RDF-Star triples from parsed text |
| `build-gazetteer` | Mutate | `entries: [GazetteerEntry]`, `ontology: iri` | Builds an Aho-Corasick automaton from lexicon entries |
| `build-fst` | Mutate | `entries: [FstEntry]` | Builds an FST from morphology entries |

#### `neural` — neural model interaction

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `run-embedder` | Query | `text: string`, `model: string` | Produces vector embedding for text |
| `run-transformer` | Query | `prompt: string`, `model: string`, `constraints: [string]` | Runs transformer inference (local WASM or remote API) |
| `run-classifier` | Query | `text: string`, `model: string`, `labels: [string]` | Runs neural classification |
| `run-reranker` | Query | `query: string`, `candidates: [string]`, `model: string` | Reranks retrieval candidates by relevance |
| `run-constrained-decode` | Query | `prompt: string`, `grammar: string`, `model: string` | Runs constrained decoding with grammar/ontology constraints |
| `load-model` | Mutate | `model_iri: iri`, `runtime: string` | Loads a neural model into the runtime |
| `unload-model` | Mutate | `model_iri: iri` | Unloads a neural model |

#### `bridge` — neuro-symbolic bridge

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `extract-substrate` | Query | `text: string`, `ontology: iri` | Runs the full symbolic pipeline to produce a verified structured substrate |
| `build-graph-index` | Mutate | `triples: [Triple]`, `entities: [Entity]` | Builds a graph index from extracted triples and entities |
| `graphrag-query` | Query | `query: string`, `graph_index: iri`, `top_k: int` | Retrieves relevant graph context for grounding |
| `verify-grounding` | Query | `output: string`, `substrate: iri` | Verifies that a neural output is grounded in the symbolic substrate |
| `feedback-loop` | Mutate | `output: string`, `graph: iri`, `verifier: Actor` | Feeds neural output back into the symbolic graph for verification |
| `inspect-substrate` | Query | `substrate_iri: iri` | Inspects a structured substrate (triples, entities, annotations) |

#### `agent` — agent runtime

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `agent-plan` | Query | `task: string`, `capabilities: [string]` | Plans an agent task — selects tools, estimates steps |
| `agent-execute` | Mutate | `plan_iri: iri`, `capabilities: [string]` | Executes an agent plan through the IntentBus |
| `agent-verify` | Query | `execution_iri: iri` | Verifies an agent execution — checks grounding, capabilities, outputs |
| `agent-trace` | Query | `execution_iri: iri` | Shows the full execution trace (tool calls, results, errors) |
| `load-golden-corpus` | Mutate | `corpus_iri: iri` | Loads a golden corpus for agent evaluation |
| `evaluate-agent` | Query | `execution_iri: iri`, `corpus: iri` | Evaluates agent performance against the golden corpus |
| `sentinel-inspect` | Query | `agent_iri: iri` | Shows the Sentinel's view of an agent's capabilities and agency claims |
| `sentinel-gate` | Mutate | `action: string`, `agent_iri: iri`, `claim: Claim` | Evaluates an agency claim for an agent action (gates execution) |

### 2.3 AI stack layer diagram

```
Layer 4: Agent Runtime
  ┌─────────────────────────────────────────────────┐
  │  agentRuntime · goldenCorpus · diagnostics       │
  │  sentinel                                        │
  └────────────────────┬────────────────────────────┘
                       │ orchestrates
Layer 3: Neuro-Symbolic Bridge
  ┌────────────────────┴────────────────────────────┐
  │  substrateExtractor · graphIndexer · graphRAG    │
  │  groundingVerifier · feedbackLoop                │
  └────────┬───────────────────────────────┬─────────┘
           │ feeds substrate to            │ feeds outputs back
Layer 2:   ▼                               ▼
  ┌─────────────────┐         ┌──────────────────────┐
  │  Neural Layer    │         │  Symbolic Layer       │
  │  embedder        │◀────────│  gazetteer            │
  │  transformer     │         │  fst                  │
  │  classifier      │         │  corefSieve           │
  │  reranker        │         │  frameSemantics       │
  │  decoder         │         │  temporalParser       │
  │                  │         │  geoParser            │
  │                  │         │  quantityNormalizer   │
  │                  │         │  relationExtractor    │
  └─────────────────┘         └──────────────────────┘
```

### 2.4 Directory structure

```
toolboxes/ai/
├── mod.rs
├── ontology.n3                  ← imports code.n3 (AI stack section)
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── symbolic/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   ├── neural/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   ├── bridge/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   └── agent/
│       ├── mod.rs
│       └── tools/               ← 8 tools
```

### 2.5 Capability scope

- `nlp:analyze` — running NLP extraction pipelines
- `nlp:build` — building gazetteers, FSTs, and other symbolic resources
- `neural:infer` — running neural model inference
- `neural:load` — loading/unloading neural models
- `graph:read`, `graph:mutate` — graph index building and querying
- `agent:plan`, `agent:execute` — agent runtime
- `agency:evaluate` — Sentinel agency claim evaluation
- `capability:invoke` — capability management (Sentinel-gated)

---

## 3. Toolbox: `spatial` (10D Browser, 3D Design & Rendering)

The `spatial` toolbox is for building and interacting with 10D scenes — the Q42 10D volumetric tensor manifold `[q, v, w, x, y, z, t, α, μ, σ]`. It covers scene graph construction, mesh manipulation, rendering, camera control, and 10D manifold navigation. The toolbox is loosely coupled: tools work in a `scene` container, a `mesh` container, or embedded in a `doc` container (e.g. a 3D figure in a research document).

**Normative reference:** [`q42-10d-tensor-standard.md`](../../TechDesign/q42-10d-tensor-standard.md) (v1.2) — the 10D tensor is a 40-byte, zero-heap, stack-allocated structure using f32 per dimension.

### 3.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `scene` | content | missing | 10D scene graph view — the primary spatial workspace. |
| `mesh3d` | content | partial | 3D mesh viewer and editor. Reaches `webizen-render`. |
| `portal` | content | missing | AR/VR portal — immersive view of a scene. |
| `viewport` | panel | missing | Render viewport — camera output, swapchain. |
| `scene-tree` | panel | missing | Scene node hierarchy (tree view). |
| `inspector` | panel | missing | Inspects selected node — transform, material, mesh, semantic link. |
| `property-sheet` | panel | missing | Edits node properties (transform values, material uniforms). |

### 3.2 Tool-chains

#### `scene` — scene graph construction

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-scene` | Mutate | `name: string` | Creates a new empty 10D scene |
| `add-node` | Mutate | `scene_iri: iri`, `parent: iri?`, `node: SceneNode` | Adds a node to the scene graph |
| `remove-node` | Mutate | `node_iri: iri` | Removes a node (and its children) |
| `set-transform` | Mutate | `node_iri: iri`, `transform: Transform` | Sets a node's transform (position, rotation, scale, temporal/semantic offset) |
| `set-mesh` | Mutate | `node_iri: iri`, `mesh_iri: iri` | Assigns a mesh to a node |
| `set-material` | Mutate | `node_iri: iri`, `material: Material` | Assigns a material (shader, uniforms) to a node |
| `add-light` | Mutate | `scene_iri: iri`, `light: Light`, `parent: iri?` | Adds a light source (directional, point, spot, ambient) |
| `add-camera` | Mutate | `scene_iri: iri`, `camera: Camera`, `parent: iri?` | Adds a camera (perspective, orthographic, or manifold) |
| `link-semantic` | Mutate | `node_iri: iri`, `semantic_iri: iri` | Links a scene node to a knowledge graph node or ontology term (makes it 10D) |
| `duplicate-node` | Mutate | `node_iri: iri`, `parent: iri?` | Duplicates a node and its subtree |

#### `mesh` — mesh manipulation

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `import-mesh` | Mutate | `file_iri: iri`, `format: string` | Imports a 3D mesh (glTF, OBJ, PLY, point cloud) |
| `export-mesh` | Mutate | `mesh_iri: iri`, `format: string` | Exports a mesh |
| `edit-vertex` | Mutate | `mesh_iri: iri`, `vertex_id: int`, `position: [f32;3]` | Edits a single vertex position |
| `subdivide-mesh` | Mutate | `mesh_iri: iri`, `levels: int` | Subdivides mesh faces |
| `decimate-mesh` | Mutate | `mesh_iri: iri`, `target_ratio: f32` | Reduces mesh complexity |
| `compute-normals` | Mutate | `mesh_iri: iri` | Recomputes vertex normals |
| `add-rig` | Mutate | `mesh_iri: iri`, `skeleton: iri` | Assigns a skeletal rig for skinned animation |
| `add-blend-space` | Mutate | `mesh_iri: iri`, `blend_space: BlendSpace` | Adds a blend space for morph targets |

#### `render` — rendering and viewport

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `render-scene` | Mutate | `scene_iri: iri`, `camera_iri: iri`, `viewport: iri` | Renders a scene to a viewport |
| `set-viewport` | Mutate | `viewport_iri: iri`, `resolution: [int;2]`, `format: string` | Configures viewport resolution and pixel format |
| `set-clear-colour` | Mutate | `viewport_iri: iri`, `colour: [f32;4]` | Sets the viewport clear colour |
| `add-post-process` | Mutate | `viewport_iri: iri`, `shader: iri`, `uniforms: CBOR-LD` | Adds a post-processing pass (bloom, SSAO, tone mapping) |
| `capture-frame` | Query | `viewport_iri: iri` | Captures the current frame as an image asset |
| `set-render-budget` | Mutate | `viewport_iri: iri`, `budget_ms: f32` | Sets a frame time budget (rendering degrades to maintain budget) |

#### `manifold-nav` — 10D manifold navigation

Navigation across the Q42 10D tensor `[q, v, w, x, y, z, t, α, μ, σ]`. A `manifoldCamera` can navigate all 10 dimensions; a standard `perspectiveCamera` navigates only x, y, z.

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `navigate-spatial` | Mutate | `camera_iri: iri`, `delta: [f32;3]` | Moves camera along x, y, z (semantic topology) |
| `navigate-temporal` | Mutate | `camera_iri: iri`, `delta: f32` | Moves camera along t (temporal state / provenance ledger) |
| `navigate-quantum` | Mutate | `camera_iri: iri`, `delta: f32` | Moves camera along q (epistemic context — shift between ground truth and what-if branches) |
| `navigate-topology` | Mutate | `camera_iri: iri`, `v_class: f32` | Changes camera's topological class v (Euclidean→Cyclic→Hyperbolic→Boundary) |
| `navigate-manifold` | Mutate | `camera_iri: iri`, `w_index: f32` | Changes camera's manifold domain w (Biological→Legal→Personal→Environmental→Socioeconomic) |
| `navigate-spectral` | Mutate | `camera_iri: iri`, `alpha: f32`, `mu: f32`, `sigma: f32` | Moves camera along α (amplitude), μ (modulation), σ (EMF spectral signature) |
| `focus-node` | Mutate | `camera_iri: iri`, `node_iri: iri` | Focuses the camera on a specific scene node's Tensor10D |
| `orbit` | Mutate | `camera_iri: iri`, `target: [f32;3]`, `azimuth: f32`, `elevation: f32` | Orbits the camera around a target point in x, y, z |
| `set-manifold-camera` | Mutate | `camera_iri: iri`, `axes: [string]` | Configures which of the 10 dimensions the camera can navigate |
| `set-spectral-perception` | Mutate | `camera_iri: iri`, `range: SpectralPerceptionRange` | Sets the spectral perception range (human visible/audible, agent full spectrum, agent custom) |

#### `design` — 3D layout and spatial UI

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `place-ui-element` | Mutate | `scene_iri: iri`, `element: iri`, `position: [f32;3]` | Places a spatial UI element (panel, label, handle) in the scene |
| `snap-to-grid` | Mutate | `node_iri: iri`, `grid_size: f32` | Snaps a node's position to a grid |
| `snap-to-surface` | Mutate | `node_iri: iri`, `target_mesh: iri` | Snaps a node to the surface of a mesh |
| `measure-distance` | Query | `node_a: iri`, `node_b: iri` | Measures the distance between two nodes |
| `align-nodes` | Mutate | `nodes: [iri]`, `axis: string`, `mode: string` | Aligns multiple nodes along an axis |
| `group-nodes` | Mutate | `nodes: [iri]`, `group_name: string` | Groups nodes into a named set |
| `set-spatial-layout` | Mutate | `scene_iri: iri`, `layout: SpatialLayout` | Applies a spatial layout (grid, radial, hierarchical) to child nodes |

### 3.3 Q42 10D Volumetric Tensor

The "10D" refers to the Q42 10D volumetric tensor `[q, v, w, x, y, z, t, α, μ, σ]` — a 40-byte, zero-heap, stack-allocated structure (f32 per dimension). **Normative reference:** [`q42-10d-tensor-standard.md`](../../TechDesign/q42-10d-tensor-standard.md) v1.2.

#### Structural & Quantum Identifiers

| Dim | Symbol | Description |
|:----|:-------|:------------|
| q | Quantum Context | Epistemic superposition index. q=0: collapsed ground truth / verified fact. q>0: parallel epistemic contexts, pending GSR resolutions, what-if branches |
| v | Topological Class | Geometric physics rules. v=0: Euclidean. v=1: Cyclic/Toroidal. v=2: Hyperbolic/Tree. v≥3: Boundary clique |
| w | Manifold Index | Domain bifurcation. w=0: Biological/Medical. w=1: Legal/Jurisdictional. w=2: Personal/Agency. w=3: Environmental/Sensor. w=4: Socioeconomic/Wellbeing |

#### Spacetime Dimensions

| Dim | Symbol | Description |
|:----|:-------|:------------|
| x | Semantic Topology X | 3D spatial coordinate — related concepts are physically clustered |
| y | Semantic Topology Y | 3D spatial coordinate |
| z | Semantic Topology Z | 3D spatial coordinate |
| t | Temporal State | Time or state-version — enables historical state queries (biomarker normal at t=0, critical at t=1) |

#### Spectral-Logical Payload (EMF spectrum)

| Dim | Symbol | Description |
|:----|:-------|:------------|
| α | Spectral Amplitude | Linear intensity, energy density, trust/consensus weight. In audio: linear gain staging. NOT gamma-encoded |
| μ | Spectral Modulation | Phase, frequency modulation, bit-packed metadata for DIDs and cryptographic provenance |
| σ | Spectral Signature | **EMF spectral profile** — shared truth index for both vision (U2) and hearing (U3). NOT colour space alone. `fract(σ)` maps to λ_nm = 400 + fract(σ)×300 for human-visible projection. Agents that perceive more spectrum (UV, IR, radio, X-ray) use the full σ range without human-range clamping |

#### σ phenomenal projection

The same σ field projects into two modalities without duplicating storage:

- **Visual (U2):** λ_nm → CIE 1931 XYZ → linear sRGB (human-visible 400-700nm only)
- **Auditory (U3):** same λ_nm → f_hz = lerp(1760, 110, t) (synaesthetic EMF→audio mapping)

Integer wraps on σ must not change either projection. Full SPD (vision) and STFT/CQT (audio) live in mmap sidecars — each node carries a 64-bin preview derived from σ.

#### Spectral perception: human vs agent

| Perceiver | Range | Notes |
|:----------|:------|:------|
| Human visible | 400-700nm | σ projected via CIE 1931 XYZ → sRGB |
| Human audible | 20Hz-20kHz | σ projected via synaesthetic λ→f mapping |
| Agent full spectrum | Full EMF | σ accessed directly — UV, IR, radio, microwave, X-ray. Portal projection is a human-facing fallback, not canonical |
| Agent custom | Defined by capability | e.g. IR sensor array, radio telescope, multi-spectral imager |

#### Topological distance metrics (normative)

Distance is selected by the query's `⌊Q.v⌋`. GPU and CPU MUST compute identical results.

| ⌊Q.v⌋ | Metric | Formula |
|:-------|:-------|:--------|
| 0 | Euclidean | d = √(Δx² + Δy² + Δz² + Δt² + Δα² + Δμ² + Δσ²) |
| 1 | Cyclic/Toroidal | d = √(c(Δx)² + c(Δy)² + c(Δz)²), c(δ) = min(\|δ\|, 1−\|δ\|) |
| 2 | Hyperbolic | d = ln(e^\|Δx\| + e^\|Δy\| + e^\|Δz\|) |
| ≥3 | Boundary clique | d = 0 if Q.v == N.v else 1 |

Note: q, v, w are NOT part of the euclidean metric — only x, y, z, t, α, μ, σ.

#### Hardware capability tiers

| Tier | Hardware | Power | Memory |
|:-----|:---------|:------|:-------|
| 0 — Strict Edge | Mobile CPUs, Pi | <1W idle, <5W active | ≤48MB |
| 1 — Mainstream | Laptops, NPU | <10W idle, <20W active | ≤256MB |
| 2 — High-Performance | GPUs (A2000, Apple Silicon) | <10W idle, <50W active | ≤2GB |
| 3 — GSR/QPU | Scarce QPUs | Variable | Stateless |

A `manifoldCamera` can navigate all 10 dimensions. A standard `perspectiveCamera` navigates only x, y, z.

### 3.4 Directory structure

```
toolboxes/spatial/
├── mod.rs
├── ontology.n3                  ← imports code.n3 (scene section)
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── scene/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   ├── mesh/
│   │   ├── mod.rs
│   │   └── tools/               ← 8 tools
│   ├── render/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   ├── manifold_nav/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   └── design/
│       ├── mod.rs
│       └── tools/               ← 7 tools
```

### 3.5 Capability scope

- `render:scene` — rendering scenes to viewports
- `render:viewport` — viewport management
- `scene:read`, `scene:mutate` — scene graph reading and modification
- `mesh:import`, `mesh:export` — mesh file I/O
- `mesh:mutate` — mesh editing
- `manifold:navigate` — 10D manifold camera navigation
- `asset:load` — loading mesh/texture assets

---

## 4. Loose coupling

These three toolboxes are **loosely coupled** to containers. The same tool can be activated in different container contexts:

| Tool | Works in `code` | Works in `doc` | Works in `scene` | Works in `graph` |
|:-----|:----------------|:---------------|:-----------------|:-----------------|
| `vibe-eval` | ✅ primary | ✅ embedded code block | ✅ scene script | ✅ graph query script |
| `run-gazetteer` | ✅ test | ✅ annotate text | — | ✅ populate graph |
| `graphrag-query` | ✅ dev | ✅ research doc | — | ✅ primary |
| `add-node` | — | ✅ 3D figure | ✅ primary | — |
| `render-scene` | ✅ preview | ✅ embedded 3D | ✅ primary | — |
| `quin-statement` | ✅ test | ✅ context markup | ✅ semantic link | ✅ primary |

The tool-chest registry resolves which tools are available in which container context based on the container's kind and the toolbox's capability scope — not hard-coded mappings.

---

## 5. Relationship to existing specs

| Document | Relationship |
|:---------|:-------------|
| [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md) | Parent spec — hierarchy, core traits, ontology layer |
| [`qualia-ui/ontologies/code.n3`](../ontologies/code.n3) | Code ontology — QApp, modules, capabilities, scenes, AI stack |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — container types, manifolds, composition |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation, composition |
| [`TechDesign/scriptingLang.md`](../../TechDesign/scriptingLang.md) | VibeScript language spec — grammar, types, effects |
| [`qualia-db-standards/vibescript-core.md`](../../qualia-db-standards/vibescript-core.md) | Normative VibeScript 0.1 spec — capability namespaces |
| [`TechDesign/FileFormat.md`](../../TechDesign/FileFormat.md) | HCF, CBOR-LD, Q42 formats |
| [`TechDesign/q42-10d-tensor-standard.md`](../../TechDesign/q42-10d-tensor-standard.md) | **Normative 10D tensor standard** — [q,v,w,x,y,z,t,α,μ,σ], EMF spectral model, distance metrics, hardware tiers |
| [`qualia-ui/ontologies/presentation.n3`](../ontologies/presentation.n3) | Presentation ontology — now includes §5 Spectral Perception (human vs agent EMF spectrum) |
| [`README.md`](../../README.md) | NLP project overview — symbolic AI, neuro-symbolic bridge |
