# Tool-Chest Architecture: Specification & Design Guide

> *"A workshop, not a webpage. A socket set, not a menu bar."*

**Copyright (c) 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn <timothy.holborn@gmail.com>  
Assignment: [`COPYRIGHT.md`](../../COPYRIGHT.md) · Licence: [`LICENSE`](../../LICENSE) (CC BY-NC-ND 4.0)

**Status:** Normative for the `qualia-ui/tool-chest/` tree.  
**Target Runtimes:** Native Rust AOT (Webizen Desktop), `wasm32-unknown-unknown`, Mobile FFI

---

## 1. Overview

The **Tool-Chest** is the top-level UI system that manages **Toolboxes**. It is the native Rust replacement for the legacy `Canvas_Workbench` JavaScript toolbox registry. There is no DOM, no HTML, and no JavaScript. All UI components are native Rust structs that emit VibeScript payloads (CBOR-LD) through the [`IntentBus`](core/intent_bus.rs).

### Design principles

1. **Physical workshop metaphor.** The UI is modelled on a physical workshop, not a web page. A tool-chest holds toolboxes; a toolbox holds tool-chains; a tool-chain holds tools. This metaphor guides layout, interaction, and code organisation.
2. **Plugin-style installation.** Each toolbox is a self-contained directory that can be added, removed, or updated independently. The tool-chest discovers and loads toolboxes at startup.
3. **Single-purpose files.** No file exceeds 800 lines. Each file has one clear responsibility. Sub-directories group related modules.
4. **Strict decoupling.** Tools never touch database logic directly. They emit [`VibeScriptPayload`]s to the [`IntentBus`], which routes to the QualiaDB backend.
5. **WASM-compatible.** All code targets `wasm32-unknown-unknown` with no platform-native C bindings in core crates.
6. **Ontology-driven.** Every toolbox, tool-chain, and tool carries an ontology that defines its domain vocabulary, capability scopes, and parameter schemas. Ontologies are authored in **N3** (Notation3) for human readability and version control, then **compiled to CBOR-LD** for runtime use. The runtime never parses N3 — it loads pre-compiled CBOR-LD ontology files.

---

## 2. Hierarchy

```
Manifold (work surface)
 └── Container (typed occupant)
      └── Tool-Chest (system)
           └── Toolbox (plugin)
                └── Tool-Chain (grouped set)
                     └── Tool (individual unit)
```

The tool-chest sits inside a **manifold** — a switchable work surface (virtual desktop). Containers (doc, sheet, code, map, etc.) are placed on manifolds by tools. The tool-chest is the furniture that holds the toolboxes; tools are what the user picks up to place containers or run Vibe.

| Level | Analogy | Rust artefact | Directory |
|:------|:--------|:-------------|:----------|
| **Manifold** | A work surface (virtual desktop) | `Manifold` struct | `qualia-ui/manifolds/<name>/` |
| **Container** | A typed occupant on a manifold (doc, sheet, code, map) | `Container` trait + impl | `qualia-ui/containers/<type>/` |
| **Tool-Chest** | The workshop chest itself | `ToolChest` registry struct | `qualia-ui/tool-chest/` |
| **Toolbox** | A removable toolbox drawer (e.g. "Socket Set Drawer", "Paint Drawer") | `Toolbox` plugin trait + impl | `qualia-ui/tool-chest/toolboxes/<name>/` |
| **Tool-Chain** | A grouped set within a drawer (e.g. "Metric Sockets", "Allen Keys", "Oil Brushes") | `ToolChain` module | `qualia-ui/tool-chest/toolboxes/<name>/chains/<chain>/` |
| **Tool** | A single socket, allen key, or brush | `Tool` struct + VibeScript emission logic | `qualia-ui/tool-chest/toolboxes/<name>/chains/<chain>/tools/<tool>.rs` |

### 2.0 Manifolds

Manifolds are switchable work surfaces, like a pager of virtual desktops. A container on one manifold can target another (sub-manifold). See [`poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) for the product vocabulary.

| Manifold | Role |
|:---------|:-----|
| `research` | GIS, clinical, rights alignment |
| `media` | `.10d` kinematics, grapheme, acoustic |
| `social` | Chat graphs, live peers, field notes |
| `settings` | Capabilities, fiduciary VM, sub-manifolds |
| `vibe` | VibeScript console + diagnose (human door into Qualia) |

### 2.0.1 Containers

Containers are typed occupants placed on manifolds by tools. Every container has a **ContainerKind** (`content`, `panel`, or `widget`) that discriminates its role. The container ontology is authored in N3 and compiled to CBOR-LD — see [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) and [`poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) for the full vocabulary.

#### Content containers (hold media)

| Container | Honesty | Tool folder | Qualia it reaches |
|:----------|:--------|:-----------|:------------------|
| `doc` | live | `rich_text` | `nlp.analyze`, gazetteer, `Document.ingest`, later HCF |
| `code` | live | `vibe` | `poet_eval`, diagnose, `capability.invoke` |
| `sheet` | live (range) | `sheet` | `Sheet.sum_range`, `Sheet.stats`, P64 latents |
| `ontology` | partial | `aura` | `SHACL.validate`, `SHACL.extensions`, catalog TTL |
| `map` | live (scene) | `gis` | `Render.scene`, hull, `Manifold.project`, GeoSPARQL |
| `media` | live (scene) | `media` | kinematics, vision ahash, grapheme, later Forge |
| `social` | live (LWW) | `social` | `Social.lww`, chat graph, later Pulse |
| `health` | live (Framingham ref.) | `health` | ClinicalRisk, anatomy renderer, Wellfair (consent) |
| `3d` | present | `mesh` | `/gpu-viewport`, `webizen-render`, `.10d` / vocal tract |
| `webrtc` | present | `rtc` | desktop `webrtc` crate; no fake stream |
| `webview` | present | `webframe` | `<q-web-frame>`, capability-gated fetch |
| `portal` | present | `portal` | wormhole IRI; later multi-tenant |
| `latex` | missing | `latex` | SymbolicAlgebra, CAS wasm |
| `graph` | missing | `graph` | SPARQL, `quin.statement`, RDF 1.2 |
| `triad` | missing | `triad` | q42↔p64↔d10, `qualia-audio` |
| `pulse` | missing | `pulse` | `pulse.publish` allowlist |
| `rights` | missing | `rights` | deontic, DID, fiduciary sign |
| `wallet` | missing | `wallet` | identity / key vault |
| `anatomy` | missing | `anatomy` | Wellfair anatomy `RenderScene` |
| `listen` | missing | `listen` | `qualia-audio`, EnCodec |
| `vision` | missing | `vision` | `ComputerVision.ahash`, `qualia-vision` |
| `finance` | missing | `econ` | Black–Scholes, portfolio |
| `slide` | missing | `slide` | office toolbox "presentation" |

#### Panel containers (hold UI chrome)

| Panel | Role | Observes |
|:------|:-----|:---------|
| `inspector` | Inspects active content container's semantic structure | active content container |
| `property-sheet` | Edits properties of the selected element | active content container |
| `outline` | Structural overview (document outline, sheet tree, scene graph) | active content container |
| `tool-palette` | Floating/docked quick-access tools from active toolbox | (independent) |
| `aura-tray` | Ontological shape & schema inspector (SHACL, validation) | active content container |
| `pulse-panel` | Collaborative stream & telemetry monitor | (subscribes to Pulse) |
| `graph-panel` | Interactive RDF / knowledge graph preview (read-only) | active graph |

Panels dock to a manifold edge (left, right, top, bottom) or float.

#### Widget containers (small UI elements)

| Widget | Role | Attached to |
|:-------|:-----|:------------|
| `mini-map` | Zoomed-out overview of manifold layout | the manifold |
| `status-bar` | Budget, gas, Sentinel state, connection status | (manifold-level) |
| `breadcrumb` | Navigation path through manifold hierarchy | (manifold-level) |
| `progress-indicator` | Long-running operation progress | active operation |
| `capability-badge` | Visual Sentinel indicator for active container/tool scope | active container or tool |

Widgets are lightweight — no media, no tool-chains.

**Honesty labels:** `live` = wired to real engine; `partial` = some bindings work; `present` = UI exists but engine not wired; `missing` = not yet built.

### 2.1 Tool-Chest

The tool-chest is the system-level container and registry. It:

- Discovers toolboxes by scanning `toolboxes/` subdirectories at startup.
- Maintains an ordered registry of installed toolboxes.
- Provides the [`IntentBus`] instance to all toolboxes.
- Manages toolbox lifecycle: load, activate, deactivate, unload.
- Renders the top-level toolbox selection surface (native UI, not DOM).

### 2.2 Toolbox

A toolbox is a plugin. Each toolbox:

- Lives in its own directory: `toolboxes/<domain>/`.
- Implements the `Toolbox` trait (defined in `core/toolbox.rs`).
- Declares its metadata: id, label, domain, icon (native vector, not emoji), version.
- Contains one or more tool-chains.
- Has no direct access to QualiaDB — all interactions go through the [`IntentBus`].
- Can be hot-loaded and hot-unloaded at runtime (capability-gated).

**Example toolboxes** (aligned with the qualia-27062026 inventory — see [`poet-hypercanvas-tools-and-containers-2026-08-16.md`](../../TechDesign/poet-hypercanvas-tools-and-containers-2026-08-16.md)):

| Toolbox | Places / actions | First real tools |
|:--------|:-----------------|:----------------|
| `office` | +doc, +ontology, +slide | `rich_text` |
| `sheet` | +sheet, import, resonance | `sheet` |
| `epistemic` | tag objective / subjective / intersubjective / normative | set node.epistemic |
| `image` | +media, marker, heatmap | `media`, vision |
| `spatial` | +map, +3d, +portal, pin, track | `gis`, `mesh` |
| `communication` | +social, +webrtc, +webview | `social`, rtc |
| `rights` | authors group, fiduciary, DID sign | `rights` |
| `health` | +health, pathology, 10D anatomy | `health`, anatomy |
| `code` | +vibe cell, `requires[]`, `quin.statement` | `vibe` — **never** `<<[` overlay |
| `ai` | co-author, extractor, sentinel, triad | local `AgentRuntime` only (no Ollama) |
| `graph` | +graph, SPARQL, SHACL | `graph`, `aura` |
| `audio` | +listen, +triad, synthesis | `listen`, `triad` |
| `finance` | +finance, portfolio | `econ` |
| `latex` | +latex, CAS | `latex` |
| `image-editing` | +image-canvas, layers, brushes, filters, masks | `image` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `audio-production` | +audio-timeline, mixer, synth, MIDI, effects | `audio` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `video-production` | +video-timeline, preview, colour, transitions | `video` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `3d-editing` | +viewport-3d, outliner, rigging, animation, materials | `3d` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `hypermedia` | +interactive-timeline, 2nd screen, HbbTV, social | `hypermedia` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `portals` | +portal-viewport, world-building, portals, avatars | `portals` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |
| `productions` | +production-timeline, DMX, projection, cue-stack | `productions` (see `TOOLBOX_HYPERMEDIA_SPEC.md`) |

### 2.3 Tool-Chain

A tool-chain is a grouped set of tools within a toolbox that share a purpose or workflow. The metaphor is a socket set, an allen-key set, or a paint set.

- Lives in: `toolboxes/<domain>/chains/<chain_name>/`.
- Contains 2 or more related tools.
- Declares a grouping label and optional ordering.
- May share state or context within its parent toolbox.

**Examples by toolbox:**

| Toolbox | Tool-Chain | Tools (examples) |
|:--------|:-----------|:-----------------|
| `rich_text` | `formatting` | bold, italic, underline, strikethrough |
| `rich_text` | `annotation` | tag-entity, tag-temporal, tag-spatial, tag-relation |
| `code` | `vibe` | `vibe_console`, `vibe_eval`, `vibe_diagnose` |
| `code` | `quin` | `quin.statement`, `quin.inspect`, `quin.ref` |
| `graph` | `query` | sparql-star, pattern-match, nquin-lookup |
| `graph` | `edit` | add-triple, retract-triple, add-reification, commit-transaction |
| `spatial` | `kinematics` | look-at-ik, foot-placement, motor-blend |
| `spatial` | `animation` | blend-2d, keyframe-edit, anim-notify |
| `audio` | `synthesis` | phoneme-graph, vocal-tract-config, audio-speak |
| `rights` | `capability` | grant-scope, revoke-scope, inspect-sentinel |
| `epistemic` | `tagging` | tag-objective, tag-subjective, tag-intersubjective, tag-normative |

### 2.4 Tool

A tool is the smallest unit — a single action or interaction surface. Each tool:

- Lives in: `toolboxes/<domain>/chains/<chain>/tools/<tool_name>.rs`.
- Constructs and emits a [`VibeScriptPayload`] when activated.
- Has no side effects beyond emitting to the [`IntentBus`].
- May carry local UI state (e.g. a colour picker's selected colour) but does not persist it directly — persistence is an intent.
- Has an optional tool-level ontology fragment (N3 source, CBOR-LD compiled) that defines the tool's parameter schema and capability requirements.

---

## 3. Ontology Layer

Each level of the hierarchy (toolbox, tool-chain, tool) may carry an ontology. Ontologies are **authored in N3** (Notation3) and **compiled to CBOR-LD** for runtime. The runtime never parses N3 directly — it loads pre-compiled `.cbor` files.

### 3.1 Why N3 for authoring?

- **Human-readable and diff-friendly** — N3 is a superset of Turtle with rules, making it ideal for version control and collaborative authoring.
- **RDF-native** — N3 is part of the same Linked Data stack as Q42 graphs, nquins, and CBOR-LD. No impedance mismatch.
- **Rule-capable** — N3 rules can express validation constraints (Aura/SHACL-like) that the runtime applies via the compiled CBOR-LD form.
- **Aligns with QualiaDB's `ontology_loader`** — the existing `qualia_core_db` module already handles RDF ontology loading; N3 is a natural source format.

### 3.2 Compilation pipeline

```
Authoring                    Compilation                 Runtime

ontology.n3  ──[n3-to-cbor]──▶  ontology.cbor  ──[load]──▶  OntologyRegistry
   (N3)                        (CBOR-LD)                    (in-memory)
```

1. **Author** — A domain expert writes `ontology.n3` using standard N3/Turtle syntax with QualiaDB prefixes.
2. **Compile** — A build-time tool (`n3-to-cbor`) parses the N3, validates it against the tool-chest ontology schema, and serialises to CBOR-LD (`ontology.cbor`).
3. **Install** — The compiled `.cbor` file ships alongside the Rust code in the toolbox directory.
4. **Load** — At toolbox activation, the registry loads the CBOR-LD ontology into an `OntologyRegistry` that tools query for vocabulary terms, parameter schemas, and capability scopes.

### 3.3 Ontology scope by hierarchy level

| Level | Ontology file | Defines |
|:------|:-------------|:--------|
| **Toolbox** | `ontology.n3` → `ontology.cbor` | Domain vocabulary, prefix declarations, shared classes/properties, capability namespace |
| **Tool-Chain** | `chains/<chain>/ontology.n3` → `chains/<chain>/ontology.cbor` | Tool-chain-specific terms, parameter constraints shared across tools in the chain |
| **Tool** | `chains/<chain>/tools/<tool>.n3` → `chains/<chain>/tools/<tool>.cbor` | Tool parameter schema, required capabilities, action type mapping |

Tool-chain and tool ontologies **import** their parent ontology via N3 `@import` or prefix declarations. The compiled CBOR-LD preserves these links as context references.

### 3.4 N3 authoring example (toolbox-level)

```n3
# toolboxes/rich_text/ontology.n3
# Rich Text Toolbox Ontology
# Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

@prefix toolchest: <https://qualiadb.org/schema/toolchest#> .
@prefix rich_text: <https://qualiadb.org/schema/toolchest/rich_text#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix vibe: <https://qualiadb.org/schema/vibe#> .

# ── Toolbox declaration ──────────────────────────────────────────

rich_text:Toolbox a toolchest:Toolbox ;
    rdfs:label "Rich Text Toolbox" ;
    toolchest:domain "rich_text" ;
    toolchest:version "0.1.0" ;
    toolchest:capabilityScope "graph:read" ;
    toolchest:capabilityScope "graph:mutate" .

# ── Tool-chain: formatting ───────────────────────────────────────

rich_text:FormattingChain a toolchest:ToolChain ;
    rdfs:label "Formatting" ;
    toolchest:parent rich_text:Toolbox ;
    toolchest:tool rich_text:BoldTool ,
                   rich_text:ItalicTool ,
                   rich_text:UnderlineTool .

# ── Tool: bold ───────────────────────────────────────────────────

rich_text:BoldTool a toolchest:Tool ;
    rdfs:label "Bold" ;
    toolchest:parent rich_text:FormattingChain ;
    toolchest:actionType vibe:Mutate ;
    toolchest:targetKind toolchest:ComponentId ;
    toolchest:requiresCapability "graph:mutate" ;
    toolchest:parameter [
        a toolchest:Parameter ;
        rdfs:label "selection_span" ;
        toolchest:paramType "array" ;
        toolchest:paramMin 2 ;
        toolchest:paramMax 2 ;
        rdfs:comment "Character offset pair [start, end]"
    ] .

# ── Tool-chain: annotation ───────────────────────────────────────

rich_text:AnnotationChain a toolchest:ToolChain ;
    rdfs:label "Annotation" ;
    toolchest:parent rich_text:Toolbox ;
    toolchest:tool rich_text:TagEntityTool ,
                   rich_text:TagTemporalTool ,
                   rich_text:TagSpatialTool .

rich_text:TagEntityTool a toolchest:Tool ;
    rdfs:label "Tag Entity" ;
    toolchest:parent rich_text:AnnotationChain ;
    toolchest:actionType vibe:Annotate ;
    toolchest:targetKind toolchest:Iri ;
    toolchest:requiresCapability "graph:mutate" ;
    toolchest:requiresCapability "aura:validate" ;
    toolchest:parameter [
        a toolchest:Parameter ;
        rdfs:label "entity_iri" ;
        toolchest:paramType "iri" ;
        rdfs:comment "IRI of the entity to tag"
    ] ;
    toolchest:parameter [
        a toolchest:Parameter ;
        rdfs:label "selection_span" ;
        toolchest:paramType "array" ;
        toolchest:paramMin 2 ;
        toolchest:paramMax 2
    ] ;
    toolchest:parameter [
        a toolchest:Parameter ;
        rdfs:label "confidence" ;
        toolchest:paramType "f32" ;
        toolchest:paramMin 0.0 ;
        toolchest:paramMax 1.0 ;
        toolchest:paramDefault 1.0
    ] .
```

### 3.5 CBOR-LD compiled form

The compiled `ontology.cbor` uses CBOR-LD with the tool-chest context (`https://qualiadb.org/schema/toolchest#`). The compiler:

1. Parses N3 into an RDF graph.
2. Validates against the tool-chest ontology schema (required properties, type constraints).
3. Serialises to CBOR-LD with term compaction using the tool-chest context.
4. Writes `ontology.cbor` alongside the N3 source.

The runtime loads `ontology.cbor` via `qualia_core_db::ontology_loader` and registers terms in an `OntologyRegistry` that tools query at activation time.

---

## 4. Directory Structure

```
qualia-ui/tool-chest/
├── TOOL_CHEST_SPEC.md              ← this document
├── core/
│   ├── mod.rs                      ← re-exports
│   ├── intent_bus.rs               ← IntentBus trait, VibeScriptPayload, Provenance
│   ├── toolbox.rs                  ← Toolbox trait, ToolboxMetadata
│   ├── tool_chain.rs               ← ToolChain trait, ToolChainMetadata
│   ├── tool.rs                     ← Tool trait, ToolMetadata
│   ├── registry.rs                 ← ToolChest registry: discovery, load, unload
│   ├── manifest.rs                 ← Toolbox manifest parsing (CBOR-LD manifest)
│   └── ontology.rs                 ← OntologyRegistry: load & query CBOR-LD ontologies
├── ontologies/
│   ├── mod.rs
│   ├── toolchest.n3                ← base tool-chest ontology schema (N3 source)
│   └── toolchest.cbor              ← compiled CBOR-LD (shipped at install time)
├── toolboxes/
│   ├── mod.rs                      ← re-exports all installed toolboxes
│   ├── rich_text/
│   │   ├── mod.rs
│   │   ├── ontology.n3             ← toolbox ontology (N3 authoring source)
│   │   ├── ontology.cbor           ← toolbox ontology (compiled CBOR-LD runtime)
│   │   ├── manifest.cbor           ← toolbox metadata (CBOR-LD)
│   │   ├── chains/
│   │   │   ├── mod.rs
│   │   │   ├── formatting/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── ontology.n3      ← tool-chain ontology (N3, optional)
│   │   │   │   ├── ontology.cbor    ← tool-chain ontology (compiled, optional)
│   │   │   │   └── tools/
│   │   │   │       ├── mod.rs
│   │   │   │       ├── bold.rs
│   │   │   │       ├── bold.n3      ← tool ontology fragment (N3, optional)
│   │   │   │       ├── bold.cbor    ← tool ontology fragment (compiled, optional)
│   │   │   │       ├── italic.rs
│   │   │   │       └── underline.rs
│   │   │   └── annotation/
│   │   │       ├── mod.rs
│   │   │       ├── ontology.n3
│   │   │       ├── ontology.cbor
│   │   │       └── tools/
│   │   │           ├── mod.rs
│   │   │           ├── tag_entity.rs
│   │   │           ├── tag_entity.n3
│   │   │           ├── tag_entity.cbor
│   │   │           └── tag_temporal.rs
│   ├── rdf_graph/
│   │   └── ... (same pattern)
│   └── ... (other toolboxes)
└── lib.rs                          ← crate root
```

### Rules

- Each directory has a `mod.rs` that re-exports its children.
- No file exceeds 800 lines. Split proactively.
- Each tool is a single file with one `Tool` impl.
- Toolbox metadata lives in a `manifest.cbor` (CBOR-LD), not inline Rust constants, so it can be read at discovery time without compiling the toolbox.
- **Ontology files are paired:** `.n3` is the authoring source, `.cbor` is the compiled runtime form. Both are version-controlled. The `.cbor` is regenerated by the build tool when the `.n3` changes.
- **Tool-chain and tool ontologies are optional** — they inherit from the parent toolbox ontology if not present. Only author a sub-ontology when the chain or tool needs terms or constraints not in the parent.
- **The runtime never parses N3.** It loads `.cbor` files only. N3 parsing is a build-time concern.

---

## 6. Core Traits (Summary)

Full definitions live in their respective files under `core/`. This section is a quick reference.

### 6.1 `Toolbox` trait

```rust
pub trait Toolbox: Send + Sync {
    fn metadata(&self) -> &ToolboxMetadata;
    fn tool_chains(&self) -> &[ToolChainEntry];
    fn activate(&self, intent_bus: &dyn IntentBus);
    fn deactivate(&self);
}
```

### 6.2 `ToolChain` trait

```rust
pub trait ToolChain: Send + Sync {
    fn metadata(&self) -> &ToolChainMetadata;
    fn tools(&self) -> &[ToolEntry];
}
```

### 6.3 `Tool` trait

```rust
pub trait Tool: Send + Sync {
    fn metadata(&self) -> &ToolMetadata;
    /// Called when the user activates this tool. Constructs a
    /// VibeScriptPayload and pushes it to the IntentBus.
    fn invoke(&self, intent_bus: &dyn IntentBus);
}
```

---

## 7. Manifest Format

Each toolbox ships a `manifest.cbor` file (CBOR-LD) at its root. The manifest references the toolbox ontology:

```yaml
# Authoring form (yaml-ld-q42) — compiled to manifest.cbor
"@context": "https://qualiadb.org/schema/toolchest#"
"@id": "toolchest:rich_text"
"@type": "Toolbox"
"label": "Rich Text"
"domain": "rich_text"
"version": "0.1.0"
"ontology": "asset://toolchest/rich_text/ontology.cbor"
"tool_chains":
  - "@id": "toolchest:rich_text/formatting"
    "label": "Formatting"
    "ontology": "asset://toolchest/rich_text/chains/formatting/ontology.cbor"
    "tools":
      - "@id": "toolchest:rich_text/formatting/bold"
        "label": "Bold"
      - "@id": "toolchest:rich_text/formatting/italic"
        "label": "Italic"
```

The registry reads manifests at startup to build the toolbox index without loading tool implementations. Tool implementations are loaded lazily on first activation. The `ontology` field points to the compiled CBOR-LD ontology file, which the `OntologyRegistry` loads at activation time.

---

## 8. Interaction Flow

```
User activates a Tool (native UI)
  │
  ▼
Tool::invoke(&intent_bus)
  │  constructs VibeScriptPayload { action_type, target_identifier, parameters }
  ▼
IntentBus::dispatch(payload)
  │  1. Capability gating (Sentinel)
  │  2. Provenance stamping
  │  3. Route to QualiaDB backend
  ▼
IntentReceipt { dispatch_id, status }
  │
  ▼
Tool updates native UI state based on receipt
```

No tool, tool-chain, or toolbox ever calls QualiaDB directly. The [`IntentBus`] is the sole boundary.

---

## 9. Lexicon Compliance

- **"nquins"** is the established nomenclature. Do not use "qualiaquins".
- **"identity"** means an enumerated state involving the use of multiple cryptography-supported identifiers and related datasets of an agent & entity-centric basis.
- Do not use the word "sovereign" or "sovereignty". Use language defined in human rights instruments when referring to rights or modalities.
- Demo identities must be Timothy Charles Holborn (`did:qualia:timothy_charles_holborn`).

---

## 10. File Size Enforcement

Per project rule:

- No file exceeds **800 lines**.
- Code generation produces single-purpose files.
- Sub-directories group related modules.
- Re-exports via `mod.rs` maintain a clean public API.

If a file approaches 800 lines during development, split it immediately into focused sub-modules.

---

## 11. Relationship to Existing Specs

| Document | Relationship |
|:---------|:-------------|
| [`TechDesign/scriptingLang.md`](../../TechDesign/scriptingLang.md) | VibeScript language spec — defines capability namespaces and invoke ids |
| [`TechDesign/FileFormat.md`](../../TechDesign/FileFormat.md) | HCF, CBOR-LD, Q42, D10, p64 formats — manifests & ontologies use CBOR-LD |
| [`TechDesign/agent_vibescript_requirements.md`](../../TechDesign/agent_vibescript_requirements.md) | Agent readiness — binding names, invoke table |
| [`core/intent_bus.rs`](core/intent_bus.rs) | Foundation implementation — `VibeScriptPayload`, `IntentBus` trait |
| `Canvas_Workbench/` | Legacy JS implementation — conceptual reference only, not to be imported |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | Manifold / container / tool-chest / toolbox / tool vocabulary, presentation & composition concepts |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — content/panel/widget kinds, manifold relationships, WorkArtifact state duality, composition DAG |
| [`qualia-ui/ontologies/presentation.n3`](../ontologies/presentation.n3) | Presentation ontology — display modality, form factor, accessibility, i18n, hardware tier |
| [`qualia-ui/ontologies/provenance.n3`](../ontologies/provenance.n3) | Provenance ontology — contributors, sources, constituencies, derivative chains, credits |
| [`qualia-ui/ontologies/agency.n3`](../ontologies/agency.n3) | Agency ontology — entities, actors, contracts, claims, delegation chains, claim validity |
| [`qualia-ui/ontologies/document.n3`](../ontologies/document.n3) | Document ontology — document types, templates, context markup, datasource bindings, temporal status, forms, publishing |
| [`qualia-ui/ontologies/code.n3`](../ontologies/code.n3) | Code ontology — QApp types, modules, capabilities, build targets, 10D scenes, AI stack (symbolic/neural/bridge/agent) |
| [`TOOLBOX_CODE_SPEC.md`](TOOLBOX_CODE_SPEC.md) | Code, AI & Spatial toolbox definitions — 3 toolboxes, 15 tool-chains, 105+ tools |
| [`TOOLBOX_INVESTIGATION_SPEC.md`](TOOLBOX_INVESTIGATION_SPEC.md) | Investigation & Forecast toolbox definitions — 2 toolboxes, 11 tool-chains, 97 tools |
| [`TOOLBOX_RESEARCH_SPEC.md`](TOOLBOX_RESEARCH_SPEC.md) | Research toolbox definition — 1 toolbox, 8 tool-chains, 75 tools |
| [`TOOLBOX_EPISTEMICS_SPEC.md`](TOOLBOX_EPISTEMICS_SPEC.md) | Epistemics toolbox definition — 1 toolbox, 7 tool-chains, 58 tools |
| [`qualia-ui/ontologies/investigation.n3`](../ontologies/investigation.n3) | Investigation ontology — cases, evidence, hypotheses, timelines, scenarios, constituencies, links |
| [`qualia-ui/ontologies/research.n3`](../ontologies/research.n3) | Research ontology — projects, scope, constraints, questions, corpus, dynamics, dark links, inferences, findings, synthesis |
| [`qualia-ui/ontologies/epistemics.n3`](../ontologies/epistemics.n3) | Epistemics ontology — subjective/objective reality, fiction/non-fiction categories, intentionality, behaviour grounding, sentiment, agent perspective, spatio-temporal/social context |
| [`qualia-ui/ontologies/ungrounded-generation.n3`](../ontologies/ungrounded-generation.n3) | Ungrounded generation ontology — 18 causes (training, context, attention, sampling, retrieval, alignment, inference-environment), 12 consequences (software-agent-scoped), cause-consequence matrix (split from epistemics.n3) |
| [`qualia-ui/ontologies/agent-nomenclature.n3`](../ontologies/agent-nomenclature.n3) | Agent nomenclature ontology — agent-type classification (natural, legal, software), applicableAgentType scoping, nomenclature mapping, appendable record support (split from epistemics.n3) |
| [`qualia-db-standards/agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md) | Project rule — agent nomenclature isolation table (natural vs legal vs software), enforcement rules |
| [`TOOLBOX_HYPERMEDIA_SPEC.md`](TOOLBOX_HYPERMEDIA_SPEC.md) | Hypermedia asset toolbox definitions — 7 toolboxes, 52 tool-chains, 339 tools (image, audio, video, 3D, hypermedia, portals, productions) |
| [`qualia-ui/ontologies/hypermedia.n3`](../ontologies/hypermedia.n3) | Core hypermedia ontology — asset types, provenance, composition elements, timeline, cross-domain references |
| [`qualia-ui/ontologies/image-editing.n3`](../ontologies/image-editing.n3) | Image domain ontology — layers, filters, brushes, masks, colour profiles |
| [`qualia-ui/ontologies/audio-production.n3`](../ontologies/audio-production.n3) | Audio domain ontology — tracks, clips, synthesis, MIDI, effects, tempo map |
| [`qualia-ui/ontologies/video-production.n3`](../ontologies/video-production.n3) | Video domain ontology — timeline tracks, transitions, effects, colour grades, generators |
| [`qualia-ui/ontologies/spatial-3d.n3`](../ontologies/spatial-3d.n3) | 3D domain ontology — scenes, meshes, rigs, animation, materials, cameras, lights, narratives |
| [`qualia-ui/ontologies/interactive-hypermedia.n3`](../ontologies/interactive-hypermedia.n3) | Interactive domain ontology — HbbTV packages, 2nd screen, triggers, social layers, sync models |
| [`qualia-ui/ontologies/portal-worlds.n3`](../ontologies/portal-worlds.n3) | Portal domain ontology — worlds, portal links, avatars, environment objects, physics bodies |
| [`qualia-ui/ontologies/production-events.n3`](../ontologies/production-events.n3) | Production domain ontology — DMX universes, fixtures, projection surfaces, cue stacks, show control |
| [`qualia-ui/ontologies/selfhood.n3`](../ontologies/selfhood.n3) | Selfhood ontology — inner experience, private keys, biometric baselines, sensory pacing, personal values. Scoped to NaturalAgent only |
| [`qualia-ui/ontologies/personhood.n3`](../ontologies/personhood.n3) | Personhood ontology — social relations, legal status, rights/obligations, fiduciary relationships, causality consequences. Applicable to NaturalPerson and LegalPerson |
| [`qualia-ui/ontologies/duty-of-care.n3`](../ontologies/duty-of-care.n3) | Duty of care ontology — umbrella for all protective relations: guardianship, custodial, professional, stewardship, safety, public service, family, co-resident, principal-agent. Care-subject, scope, parties, rights, responsibilities, legal basis, consent, jurisdiction, duty origin (constituted/circumstantial/fraudulent), Good Samaritan, fraudulent assumption, verification requirements |
| [`qualia-ui/ontologies/care-scope.n3`](../ontologies/care-scope.n3) | Care scope ontology — agreements (terms, status, lifecycle, amendments), legal domains (family, tax, immigration, criminal, etc.), care contexts (school, medical, sports, childcare, etc.), scoped task assignments, software agent delegation |
| [`qualia-ui/ontologies/guardianship.n3`](../ontologies/guardianship.n3) | Guardianship ontology — formal legal guardianship (subClass of DutyOfCare), ward vulnerability, structure, authority, review, family roles, intervention/challenge/replacement |
| [`qualia-ui/ontologies/adversarial-conduct.n3`](../ontologies/adversarial-conduct.n3) | Adversarial conduct ontology — strategies (compromise, coerce, undue influence, pervert, subvert, deceive, isolate), methods (drugs, sexual compromise, media manipulation, blackmail, financial pressure, bribery, digital compromise, legal abuse, disinformation, agent insertion, supply chain, witness tampering, insider exploitation, weaponised software agents), targets (persons, projects, event histories, circumstances, outcomes, relations, information, systems), harm scope (direct, collateral, social, informational), coordinated multi-vector patterns with phases, evidence for retrospective evaluation, vulnerabilities for prospective protection, consequences, specific named conduct types with legal analogues (usurpation, self-dealing, dereliction of oversight, shadow director evasion, tunneling, fraudulent misrepresentation, negligent misstatement, deceptive omission, astroturfing, greenwashing/tech-washing, wrongful trading, front-running, wash trading/spoofing, jurisdictional arbitrage, sybil fraud, privilege escalation abuse, asymmetric information hoarding) |
| [`qualia-ui/ontologies/production-document.n3`](../ontologies/production-document.n3) | Production document ontology — scripts (scenes, characters, dialogue, action), functional specs (features, requirements, acceptance criteria, use cases), style guides (visual, audio, code, brand, editorial, narrative), book publications (chapters, topics, prologue/epilogue), research reports (knowledge queries, findings, methodology), production briefs (scope, deliverables, timeline, budget), document sections, entities, style rules, agent processing directives, document layouts, knowledge query language |
| [`qualia-ui/ontologies/game-design.n3`](../ontologies/game-design.n3) | Game design ontology — game types (action, adventure, RPG, strategy, simulation, serious, sandbox, narrative, multiplayer, AR), perspectives, mechanics (movement, combat, resource management, progression, puzzle, social, exploration, construction, stealth, time manipulation, physics, procedural), objectives (primary, secondary, hidden, fail/win conditions), game characters (PC, NPC, companion, antagonist, quest giver, vendor, crowd), AI behaviour (behaviour trees, state machines, utility AI, GOAP, scripted sequences, nav meshes, LLM-driven), LLM agent bindings (character definition, grounding context, generation constraints, fallback behaviour), dialogue trees (scripted, dynamic, hybrid modes), quests (main, side, fetch, escort, investigation, collection), items and inventory (weapons, armour, consumables, key items, crafting, currency, cosmetics, documents), progression (XP/levels, skill trees, achievements, unlocks, reputation), game states (save, checkpoint, session, replay, shared) |
| [`qualia-ui/ontologies/game-world.n3`](../ontologies/game-world.n3) | Game world ontology — real-world grounding (historical, geographic, cultural, contemporary, future, fictional, hybrid; fidelity levels; time period; geographic scope; sources; epistemic modality), world structures (single, parallel universe, layered reality, alternate timeline, pocket dimension, multiverse), coordinate systems (real-world, celestial, abstract, hybrid, layered; coordinate mapping), world physics model, world time model, world layers and parallel world linking, game worlds (regions, points of interest, fast travel, encounter zones, real-world and celestial references), game design documents (GDD — subClass of ProductionDocument; vision, target audience, monetisation, art style, audio style) |
| [`qualia-ui/ontologies/learning-core.n3`](../ontologies/learning-core.n3) | Learning core ontology — knowledge domains (nested, overlapping), skills (cognitive, physical, social, technical, creative, computational, meta; sub-skills, prerequisites), proficiency levels (Bloom, Dreyfus, CEFR, AQF, custom), skill assessments (examination, practical, portfolio, peer, self, observation, automated, interview, project; confidence, validity, epistemic mode), skill profiles (appendable history, current level), skill gaps (magnitude, priority, recommended actions), recognition of prior learning (RPL — evidence sources, outcomes, assessor, confidence, qualification granted), qualifications (degree, diploma, certificate, licence, micro-credential, digital badge, professional cert; issuer, expiry, RPL linkage), learning events (intentional, incidental, experiential, social, reflective; source types, outcomes, confidence), assurance levels (verified, supported, indicated, claimed, unknown; computed from assessment history quality) |
| [`qualia-ui/ontologies/learning-experience.n3`](../ontologies/learning-experience.n3) | Learning experience ontology — learning paths (linear, branching, open, adaptive; target skill/domain/level), learning activities (lesson, exercise, reading, video, interactive, quiz, assessment, project, discussion, game-based, simulation, mentorship, reflection, media-analysis; difficulty, duration, prerequisites), adaptive learning engines (Bayesian knowledge tracing, item response theory, rule-based, ML-based; adaptation factors, explanation, transparency), tutorial environments (interactive document, virtual lab, immersive, game-based, augmented, mixed; feedback modes, real-world grounding), incidental learning trackers (data source monitoring, inferred events, recommendations, confidence), elearning toolbox components (lesson viewer, exercise editor, quiz builder, progress tracker, skill radar, gap analyser, adaptive panel, feedback display, path navigator, portfolio viewer, peer review, mentor chat, badge display, timeline, assurance meter, RPL wizard, media analyser, incidental log), learning manifolds (self-paced, cohort, instructor-led, blended, adaptive) |
| [`qualia-ui/ontologies/learning-experience-modality.n3`](../ontologies/learning-experience-modality.n3) | Learning experience modality ontology — experience modes (direct, lived, simulated, observational, interpersonal, educational, mediated), knowledge types (propositional/knowing-that, practical/knowing-how, experiential/knowing-what-it-is-like [natural agents only], interpersonal/knowing-about-others'-experiences), experience records (lived experience distinct from learning events; duration, intensity, repetition, consequence level, documentation, confidence), skill development trajectories (exposure → novice → practitioner → proficient → expert → master; mode mix, plateaus, breakthroughs, decay), experiential transfer (full, partial, limited, none; transfer by knowledge type, gaps, adaptation required, evidence), mode-to-knowledge-type matrix, life activities (parenting, sport, arts, food, community, travel, professional, health, hobby, life events, upbringing — learning as byproduct not purpose), upbringing factors (birthplace, language, family profession, faith tradition, culture, socioeconomic, family characteristics, education, community — background circumstances that confer knowledge without intentional learning), knowledge conferral (immersion-based acquisition, conferral depth: exposure → familiar → integrated → transformative, recognisability, assessability, RPL evidence) |
| [`qualia-ui/ontologies/faith-systems.n3`](../ontologies/faith-systems.n3) | Faith systems ontology — faith systems (text-based religion, oral tradition, practice-based, place-based, philosophical-spiritual, eclectic-syncretic, historical-ancient, personal-eclectic; transmission modes: text, oral, practice, place, mixed; system eras: ancient, classical, medieval, early-modern, modern, timeless), sacred texts (scriptural, canonical, commentary, inspirational, liturgical; original language), oral traditions (story, song, ceremony, teaching, law-custom, genealogy; custodianship, protocols, seasonal, place-based), sacred places (natural feature, ceremonial ground, built structure, ancestral land, pilgrimage site, burial site; access protocols, restricted knowledge), faith communities (institution, congregation, people-nation, order-tradition, informal gathering, diaspora; gathering place, leader), practices and rituals (daily observance, seasonal ceremony, life-cycle rite, pilgrimage, initiation, healing, offering, communal gathering; frequency, protocols), moral frameworks (codified, embodied-in-practice, narrative, philosophical; virtues, prohibitions, obligations, relation to land, relation to community), cosmology (creation narrative, time models: linear/cyclic/timeless/layered, divine nature: monotheistic/polytheistic/pantheistic/non-theistic/ancestral/animistic), indigenous knowledge protocols (access levels: public/community-only/initiated-only/custodian-only/gender-restricted/seasonal/sacred-secret; consent, attribution, commercial use), faith adherence (cultural identification, nominal, practicing, devoted, leader-custodian, seeker; origin: inherited/chosen/syncretic/reversion; community, start date) |
| [`qualia-ui/ontologies/values.n3`](../ontologies/values.n3) | Values ontology — values (moral, ethical, political, environmental, cultural, social, professional, spiritual, aesthetic, epistemic; value conflicts, priority), projected values (statements, mission, codes, charters, platforms, brand, pledges, policy; projection form, date, source, epistemic mode), enumerated values (behaviour-derived, evidence-based, confidence, assessor, observation period, epistemic mode), values gap (aspiration, misrepresentation, transition, contextual-variation, hypocrisy [natural agents only] / institutional inconsistency [legal persons]; magnitude, assessor, date), values instruments (human rights, constitutional, professional code, religious moral, cultural customary, environmental, organisational, community charter, international law, philosophical; authority, jurisdiction, transmission mode), values agreements (parties, referenced instruments, agreement values, governed content, persistence: while-relationship/content-persists/indefinite/binding-on-successors, related agreements/contracts/care agreements, status lifecycle, dispute resolution, issued credentials), values credentials (holder, issuer, values, instrument, basis: authority-issued/community-affirmed/agreement-issued/behavioural-enumeration/self-declared, assurance: verified/supported/self-only/contested, revocable, required-for, related agreement), values networks (entities, agreements, instruments, credentials, conflicts) |
| [`qualia-ui/ontologies/obligations.n3`](../ontologies/obligations.n3) | Obligations ontology — obligations (confidentiality, secrecy, privileged communication, human dignity, moral, community, safety, fiduciary, professional ethics, statutory, contractual; strength: absolute/strong/standard/weak; conflicts), confidentiality instruments (NDA, deed of secrecy, contractual clause, suppression order, sealed record, professional code, national security, cultural protocol; parties, scope, duration, exceptions, penalties, mutual), human dignity (bodily integrity, identity privacy, trauma history, vulnerability, cultural-spiritual, autonomy/self-determination, reputation/honour, freedom from degradation; protected information, obligation), zero-knowledge validation (range proof, set membership, set non-membership, credential verification, attribute proof, relationship proof, obligation proof; protected data, revealed data, verification result, links to confidentiality instruments and dignity aspects), coercive circumstances (physical threat, psychological pressure, economic coercion, legal coercion, institutional pressure, familial pressure; threat levels: imminent/serious/moderate/low; endangered parties; ZKP options), disclosure exceptions (imminent harm, child protection, vulnerable adult, court order, law enforcement, public health, public interest, consent, whistleblower, self-defence; mandatory/permissive; disclose-to; proportionality; ZKP-first), disclosure exclusions (law enforcement, courts, social workers, medical professionals, mental health specialists, parliamentarians, journalists, legal professionals, clergy, defence/national security, researchers, auditors; rules, oversight, jurisdiction), obligation conflict resolution (ZKP, disclosed, withheld, partial disclosure, deferred, escalated; rationale, date) |
| [`qualia-ui/ontologies/adversarial-relational.n3`](../ontologies/adversarial-relational.n3) | Adversarial relational methods ontology — relational aggression (social exclusion, gossip/rumour spreading, relational sabotage, friendship weaponisation, triangulation, flying monkey recruitment, competitive victimhood/DARVO), psychological manipulation (gaslighting with 5 variants: interpersonal/institutional/medical/racial/collective-orchestrated; emotional blackmail/FOG, trauma bonding, narcissistic abuse pattern: idealise→devalue→discard→hoover, silent treatment, infantilisation, reality distortion), sexual manipulation (sexual coercion, sexual deception/rape by fraud, reproductive coercion/contraceptive sabotage, sexual content weaponisation/revenge porn/deep-fakes, sexual exploitation of dependency, sexual boundary testing/grooming), false claims & fabrication (false allegations to authorities, fabricated evidence, perjury/false sworn statements, false medical/psychiatric claims, misleading true claims, false counter-claims/DARVO), content double binds (illegal sexual content, illegal recordings, classified content, privacy-protected content, self-incriminating content; reporting paths, ZKP options, mischaracterisation risk), social harm patterns (coercive control, post-separation abuse, workplace mobbing, institutional abuse, online harassment campaigns, family system abuse, community ostracism), coercive control phases (grooming→consolidation→crisis→post-separation), proxy perpetration (willing/coerced/unwitting/partially-aware; layered chains), profession-correlated access (12 roles), core principle (friendly/good-faith/injury test), accountability evasion (feigned incompetence, discrimination weaponisation, marginalised group weaponisation with protective bias types & intergenerational impact, provocation-response inversion, ontological invisibility), instrumentalisation factors (11 contextual factors: social bias, wealth/resource, family network, psychiatric vulnerability, substance dependency, sexual leverage, professional authority, institutional blind spots, legal status, child dependency, isolation) |
| [`qualia-ui/ontologies/adversarial-relational-scenarios.n3`](../ontologies/adversarial-relational-scenarios.n3) | Adversarial relational scenarios ontology — contextual scenario patterns showing how methods and factors intersect in practice: wealth-family abuse (concentrated resources, family acting as unit, legal abuse, child access control), substance-driven exploitation (parent exploiting children for substances, no support, intergenerational risk), sexual network leverage (strategic relationship cultivation, premeditated attacks, network deniability), adolescent relational aggression (schoolyard exclusion → online harassment → suicide/self-harm/substance use/trafficking cascade), corporate shield weaponisation (sole directorships, phoenixing, psychiatric vulnerability as legal shield, organised crime insulation), adolescent online commodification (isolation → online vulnerability → grooming → commodification → trafficking pipeline), proxy harassment via protective bias (untouchable instrument, victim double bind, protective bias weaponised), single-parent intergenerational transmission (primary mechanism for adversarial conduct transmission across generations, triangulation, infantilisation, gaslighting, normalisation) |
| [`qualia-ui/ontologies/settings.n3`](../ontologies/settings.n3) | Settings ontology — setting (base class, scope: global/profile/session/container, value types, provenance, validity), preferences (natural persons: accessibility, sensory pacing, aesthetic, workflow, language, privacy, communication), configuration (legal persons: policy, compliance, security, operational, integration), parameters (software agents: threshold, mode, allocation, timeout), capability management (name, holder, grantor, scope, status: active/suspended/revoked/expired/pending, constraints, budget), access control (capability-based, selfhood protection: non-delegable, non-transferable), ontology loading (modules, prefixes, load order, status, imports), runtime parameters (profiles: sovereign-native/wasm-web/edge, memory, cache, processing mode), fiduciary VM config (sandbox strictness: strict/permissive/locked, budget enforcement, event bridge, legacy JS), profile management (personal, work, shared-device, institutional, agent, guest), settings manifold (sub-manifolds: preferences, capabilities, access-control, ontologies, runtime, fiduciary-vm, profiles) |
| [`qualia-ui/ontologies/communications.n3`](../ontologies/communications.n3) | Communications manifold ontology — pulse events (source, channel, payload types: VibeScript/graph-mutation/notification/telemetry/agent-message/presence/sync, priority: critical/high/normal/low, provenance), channels (direct, topic, request-response, stream, group, federation; transports: WebRTC, WebSocket, MQTT, HTTP, SSE, internal, DAG sync; capability-gated, encrypted), interaction patterns (conversation, request-response, broadcast, collaboration, negotiation, delegation, notification, presence), protocol handlers (state, retry policy, encryption), routing rules (match, actions: deliver/queue/drop/redirect/transform/batch, priority), agent communication (PersonCommunication: natural persons, InstitutionalTransmission: legal persons, AgentDispatch: software agents), communications manifold (sub-manifolds: conversations, channels, notifications, presence, pulse-inspector) |
| [`qualia-ui/ontologies/social.n3`](../ontologies/social.n3) | Social manifold ontology — social graph (owner, edges), social edges (source, target, type, strength: strong/moderate/weak/severed, context, provenance; enumerable characteristics: duration, interaction frequency, reciprocity, power dynamics, disclosure level, support types, conflict history, tags), edge types (personal: friendship/family/professional/community-member/acquaintance/adversarial; institutional: membership/partnership/sponsorship; computational: connection/federation), communities (formal, informal, place-based, interest-based, event-based, online, faith; members, leaders, rules, values), reputation (dimensions: trustworthiness/competence/reliability/integrity/conduct; levels: high/moderate/low/negative/unknown; evidence-based, context-dependent), social context (relationships, communities, norms, obligations), social protocols (initiation, confidentiality, conflict resolution), social manifold (sub-manifolds: social-graph, communities, reputation, social-context, protocols) |
| [`qualia-ui/ontologies/social-connections.n3`](../ontologies/social-connections.n3) | Social connections ontology — connection requests (ZKP-verified workflow, status: pending/verifying/guardian-review/accepted/declined/blocked/expired/withdrawn), ZKP claims (age proof with age ranges, identity uniqueness, credential, community membership, conduct clear), risk assessment (levels: none/low/moderate/high/critical; indicators: new-account, no-shared-contacts, no-community-overlap, unverified-identity, duplicate-account, conduct-history, trolling-pattern, grooming-pattern, identity-mismatch, social-engineering, phishing-vector, network-weaponisation, coercive-control-entry; evidence-based), settings recommendations (disclosure level, monitoring, guardian notification, initial period, capability scope), social-vector attack types (fake-account, trolling, grooming, phishing, social-engineering, coercive-control-entry, network-weaponisation, reputation-manipulation, isolation-tactics, radicalisation), vulnerable person categories (minor, under-guardianship, recently-separated, DV-survivor, whistleblower, public-figure, housing-insecure, cognitive-impairment, new-immigrant, substance-recovery, elderly), vulnerable person protection (mandatory for structural vulnerabilities, opt-in for situational; approval modes: always/age-based/risk-based/community-based/network-based/none; max disclosure level; monitoring: passive/active/off; alert triggers: grooming-pattern, bullying-pattern, adult-contact-minor, risky-connection, location-sharing, isolation-pattern, identity-mismatch, coercive-control-pattern, network-weaponisation, retaliation-pattern, financial-exploitation) |
| [`TechDesign/poet-hypercanvas-tools-and-containers-2026-08-16.md`](../../TechDesign/poet-hypercanvas-tools-and-containers-2026-08-16.md) | Complete container & toolbox inventory with honesty labels |
| [`TechDesign/native-presentation-and-vibe-beyond-webview-2026-08-16.md`](../../TechDesign/native-presentation-and-vibe-beyond-webview-2026-08-16.md) | WebView exit strategy, Vibe-as-JS-replacement, CBOR-LD wire plan |
| [`qualia-db-standards/vibescript-core.md`](../../qualia-db-standards/vibescript-core.md) | Normative VibeScript 0.1 spec — capability namespaces, binding table |
| [`consult/branding.md`](../../consult/branding.md) | Aura = ontology/schema; Pulse = events; Poet = engine |

---

## 12. Implementation Phases

| Phase | Scope | Status |
|:------|:------|:-------|
| **1 — Foundation** | `core/intent_bus.rs`: `VibeScriptPayload`, `IntentBus` trait, `Provenance` | ✅ Done |
| **2 — Traits & Ontology** | `core/toolbox.rs`, `core/tool_chain.rs`, `core/tool.rs`, `core/registry.rs`, `core/manifest.rs`, `core/ontology.rs`, base `ontologies/toolchest.n3` | Pending |
| **3 — First Toolbox** | Port `rich_text` toolbox with `formatting` and `annotation` tool-chains | Pending |
| **4 — Toolbox Ports** | Port remaining toolboxes from qualia-27062026 inventory (graph, audio, finance, latex, epistemic, code, ai) | Pending |
| **5 — Native Rendering** | Native UI rendering layer (WGPU, no DOM); presentation context negotiation via `presentation.cbor` | Pending |

---

## 13. Toolbox Definition: `office` (Hypermedia Documents)

The `office` toolbox is the primary toolbox for authoring, editing, and publishing hypermedia documents. It is the native equivalent of Microsoft Word, Apple Pages, or Google Docs — but built on Vibe, CBOR-LD, and the context graph.

**Ontology:** [`qualia-ui/ontologies/document.n3`](../ontologies/document.n3) (N3 authoring → CBOR-LD runtime)

### 13.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `doc` | content | live (gazetteer) | Rich text, context markup, annotations. The primary editing surface. |
| `ebook` | content | missing | Published e-book derivative (EPUB, PDF). Flattened from a doc manifold. |
| `webpage` | content | missing | Published webpage derivative. Interactive, live datasource links. |
| `print-document` | content | missing | Print-ready derivative (PDF, PostScript). Static, paginated. |
| `form` | content | missing | Interactive form with fields, validation, submission workflow. |
| `inspector` | panel | missing | Inspects active document's semantic structure, context graph, provenance. |
| `outline` | panel | missing | Document outline (sections, headings, TOC). |
| `aura-tray` | panel | partial | SHACL shapes, validation results, ontology context for active document. |
| `property-sheet` | panel | missing | Edits properties of selected element (paragraph style, markup properties). |

### 13.2 Tool-chains

#### `formatting` — text formatting

Tools for character-level and paragraph-level formatting. Analogous to the formatting toolbar in Word/Pages/Docs.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `bold` | Mutate | `selection_span: [start, end]` |
| `italic` | Mutate | `selection_span: [start, end]` |
| `underline` | Mutate | `selection_span: [start, end]` |
| `strikethrough` | Mutate | `selection_span: [start, end]` |
| `subscript` | Mutate | `selection_span: [start, end]` |
| `superscript` | Mutate | `selection_span: [start, end]` |
| `heading` | Mutate | `selection_span: [start, end]`, `level: 1-6` |
| `paragraph-style` | Mutate | `selection_span: [start, end]`, `style_name: string` |
| `list` | Mutate | `selection_span: [start, end]`, `list_type: ordered|unordered` |
| `indent` | Mutate | `selection_span: [start, end]`, `direction: increase|decrease` |
| `alignment` | Mutate | `selection_span: [start, end]`, `align: left|center|right|justify` |
| `font-family` | Mutate | `selection_span: [start, end]`, `family: string` |
| `font-size` | Mutate | `selection_span: [start, end]`, `size: f32` |
| `text-colour` | Mutate | `selection_span: [start, end]`, `colour: string` |
| `highlight` | Mutate | `selection_span: [start, end]`, `colour: string` |

#### `structure` — document structure

Tools for organising document structure: sections, tables, footnotes, citations, table of contents.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `insert-section` | Mutate | `position: int`, `title: string`, `required: bool` |
| `insert-table` | Mutate | `position: int`, `rows: int`, `cols: int` |
| `insert-footnote` | Mutate | `position: int`, `content: string` |
| `insert-endnote` | Mutate | `position: int`, `content: string` |
| `insert-citation` | Annotate | `position: int`, `source_iri: iri`, `citation_style: string` |
| `insert-toc` | Mutate | `position: int`, `depth: int` |
| `insert-page-break` | Mutate | `position: int` |
| `insert-hyperlink` | Mutate | `selection_span: [start, end]`, `target_iri: iri` |
| `insert-bookmark` | Mutate | `position: int`, `label: string` |

#### `context-markup` — semantic annotation and source linking

Tools for adding context markup — linking concepts, terms, claimed facts, and statements to sources. This is the core of the document's context graph.

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `tag-term` | Annotate | `selection_span`, `term_iri: iri`, `ontology_binding: iri` | Links a term to an ontology entry |
| `tag-entity` | Annotate | `selection_span`, `entity_iri: iri`, `confidence: f32` | Links a named entity to a knowledge graph node |
| `tag-claimed-fact` | Annotate | `selection_span`, `source_iris: [iri]`, `confidence: f32` | Links a factual claim to supporting/refuting sources |
| `tag-statement` | Annotate | `selection_span`, `provenance: ProvenanceEntry`, `agency_claim: Claim` | Links a statement to its assertion context |
| `tag-statistic` | Annotate | `selection_span`, `datasource_binding: iri`, `snapshot: StatusSnapshot` | Links a statistic to a datasource with temporal status |
| `tag-citation` | Annotate | `selection_span`, `source_iri: iri`, `citation_style: string` | Links a citation to an academic or legal source |
| `tag-definition` | Annotate | `selection_span`, `definition_iri: iri`, `glossary: iri` | Links a term definition to an ontology or glossary |
| `tag-quote` | Annotate | `selection_span`, `source_iri: iri`, `speaker: iri` | Links a direct quotation to its source utterance |
| `link-markup` | Annotate | `from_markup: iri`, `to_markup: iri`, `relation: string` | Creates an edge between two markup nodes (e.g. "supports", "refutes") |
| `append-markup` | Annotate | `markup: ContextMarkup`, `appended_by: Actor`, `append_scope: AppendScope` | Appends markup from a 3rd-party or contributor to the context graph |
| `refresh-temporal-status` | Query | `markup_iri: iri` | Refreshes the present-status snapshot for a statistic or datasource-linked markup |
| `view-temporal-delta` | Query | `markup_iri: iri` | Shows the delta between created-at and present status |

#### `media-embed` — embedding hypermedia assets

Tools for embedding other containers and media within a document.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `embed-image` | Mutate | `position: int`, `image_iri: iri`, `caption: string`, `alt_text: string` |
| `embed-audio` | Mutate | `position: int`, `audio_iri: iri`, `caption: string` |
| `embed-video` | Mutate | `position: int`, `video_iri: iri`, `caption: string` |
| `embed-sheet` | Mutate | `position: int`, `sheet_iri: iri`, `link_type: live|snapshot` |
| `embed-graph` | Mutate | `position: int`, `graph_iri: iri`, `link_type: live|snapshot` |
| `embed-map` | Mutate | `position: int`, `map_iri: iri`, `link_type: live|snapshot` |
| `embed-3d` | Mutate | `position: int`, `mesh_iri: iri` |
| `embed-form` | Mutate | `position: int`, `form_iri: iri` |
| `embed-manifold` | Mutate | `position: int`, `manifold_iri: iri`, `link_type: reference|embed` |

#### `template` — document types and templates

Tools for selecting, applying, and creating document templates.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `select-document-type` | Mutate | `document_type: DocumentType` | Sets the document type (research, news, legal, academic, etc.) |
| `apply-template` | Mutate | `template_iri: iri` | Applies a template's structure, styles, and datasource bindings |
| `create-template` | Mutate | `from_document: iri`, `template_name: string`, `origin: supplied|custom|community` | Creates a new template from the current document |
| `add-section-rule` | Mutate | `template_iri: iri`, `section: TemplateSection` | Adds a section rule to a template |
| `add-style-definition` | Mutate | `template_iri: iri`, `style: StyleDefinition` | Adds a style definition to a template |
| `add-datasource-binding` | Mutate | `template_iri: iri`, `binding: DatasourceBinding` | Binds a datasource to a template |
| `validate-against-type` | Query | `document_iri: iri` | Validates the document against its type's requirements (citations, jurisdiction, etc.) |

#### `datasource` — connecting to data services

Tools for binding datasources to documents and querying them.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `bind-datasource` | Mutate | `binding: DatasourceBinding` | Binds a datasource to the document |
| `unbind-datasource` | Mutate | `binding_iri: iri` | Removes a datasource binding |
| `query-datasource` | Query | `binding_iri: iri`, `query: string` | Queries a bound datasource |
| `refresh-datasource` | Query | `binding_iri: iri` | Refreshes the datasource content (per refresh policy) |
| `list-bindings` | Query | `document_iri: iri` | Lists all datasource bindings for a document |
| `set-jurisdiction` | Mutate | `binding_iri: iri`, `jurisdiction: string` | Sets jurisdiction scope for a binding (legal documents) |

#### `publish` — flattening manifolds to derivatives

Tools for publishing a document manifold as a flat derivative.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `publish-print` | Mutate | `manifold_iri: iri`, `output: PrintDocument`, `styling: StyleDefinition` | Flattens to print-ready PDF |
| `publish-webpage` | Mutate | `manifold_iri: iri`, `output: Webpage`, `include_context_graph: bool` | Flattens to interactive webpage |
| `publish-ebook` | Mutate | `manifold_iri: iri`, `output: Ebook`, `format: epub|pdf` | Flattens to e-book |
| `publish-hcf` | Mutate | `manifold_iri: iri`, `output: HCFBundle` | Flattens to HCF bundle (canonical, fully interactive) |
| `publish-markdown` | Mutate | `manifold_iri: iri`, `output: Markdown` | Flattens to plain markdown |
| `publish-plain` | Mutate | `manifold_iri: iri`, `output: Plain` | Flattens to plain text (accessibility fallback) |
| `view-credits` | Query | `manifold_iri: iri` | Generates and displays the credits summary from provenance graph |

#### `review` — collaboration and review

Tools for collaborative review, track changes, and version comparison.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `track-change` | Mutate | `selection_span`, `change_type: insert|delete|modify`, `author: Actor` | Records a tracked change |
| `accept-change` | Mutate | `change_iri: iri` | Accepts a tracked change |
| `reject-change` | Mutate | `change_iri: iri` | Rejects a tracked change |
| `add-comment` | Annotate | `selection_span`, `content: string`, `author: Actor` | Adds a review comment |
| `resolve-comment` | Mutate | `comment_iri: iri` | Resolves a comment |
| `compare-versions` | Query | `version_a: iri`, `version_b: iri` | Shows diff between two document versions |
| `request-review` | Mutate | `document_iri: iri`, `reviewer: Actor`, `scope: string` | Requests a review from an actor |

#### `forms` — interactive form elements

Tools for creating and managing forms within documents.

| Tool | Action | Parameters |
|:-----|:-------|:-----------|
| `insert-form` | Mutate | `position: int`, `form_iri: iri` | Inserts a form at a position in the document |
| `add-field` | Mutate | `form_iri: iri`, `field: FormField` | Adds a field to a form |
| `set-field-validation` | Mutate | `field_iri: iri`, `validation: string` | Sets validation rule for a field |
| `bind-field-datasource` | Mutate | `field_iri: iri`, `binding: DatasourceBinding` | Binds a field to a datasource (e.g. entity-select) |
| `set-submission-workflow` | Mutate | `form_iri: iri`, `workflow: SubmissionWorkflow` | Sets the form's submission workflow |
| `submit-form` | Mutate | `form_iri: iri`, `data: CBOR-LD`, `agency_claim: Claim` | Submits form data (requires agency claim if configured) |

### 13.3 Document type examples

| Document type | Required datasources | Requires citations | Temporal status | Example use |
|:--------------|:---------------------|:-------------------|:----------------|:------------|
| **Research** | research-corpus, gazetteer, ontology | yes | yes | Academic research with checked sources, embedded graphs and maps |
| **News article** | public-stats, news-wire | yes | yes | Article relying on public statistics that may change over time |
| **Legal** | legislation, case-law (jurisdiction-scoped) | yes | yes | Legal document reviewing case info against legislation in a jurisdiction |
| **Academic paper** | academic-corpus | yes | no | Formal paper with citations, peer-review workflow |
| **General** | (none) | no | no | Free-form document, no mandatory bindings |
| **Form** | (varies by field) | no | no | Interactive form with validation and submission |

### 13.4 Context markup flow

```
1. Author writes text in a doc container
2. Author (or NLP agent) selects a span and applies a context-markup tool:
   - tag-term: links "hyperinflation" to an ontology term
   - tag-claimed-fact: links "unemployment is 4.2%" to a public-stats datasource
   - tag-entity: links "Timothy Holborn" to a knowledge graph node
3. The markup node is added to the document's ContextGraph
4. A ProvenanceEntry records who added it, when, and with what confidence
5. An AgencyClaim is evaluated (was the contributor authorised?)
6. For datasource-linked markup (statistics):
   - A StatusSnapshot is taken at creation time (createdAtStatus)
   - The present-status can be refreshed later (presentStatus)
   - The delta shows what changed
7. 3rd-party audiences or contributors can append markup (append-markup tool)
   - Their markup is tagged with appendedBy and appendScope
8. When published:
   - HCF/Webpage: includes the full context graph
   - Print/Ebook: context graph is omitted (static derivative)
```

### 13.5 Directory structure

```
toolboxes/office/
├── mod.rs
├── ontology.n3                  ← imports document.n3
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── formatting/
│   │   ├── mod.rs
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── bold.rs
│   │       ├── italic.rs
│   │       └── ... (15 tools)
│   ├── structure/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (9 tools)
│   ├── context_markup/
│   │   ├── mod.rs
│   │   ├── ontology.n3          ← context markup vocabulary
│   │   ├── ontology.cbor
│   │   └── tools/
│   │       └── ... (12 tools)
│   ├── media_embed/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (9 tools)
│   ├── template/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (7 tools)
│   ├── datasource/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (6 tools)
│   ├── publish/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (7 tools)
│   ├── review/
│   │   ├── mod.rs
│   │   └── tools/
│   │       └── ... (7 tools)
│   └── forms/
│       ├── mod.rs
│       └── tools/
│           └── ... (6 tools)
```

### 13.6 Capability scope

The `office` toolbox requires:

- `graph:read` — reading the document's knowledge graph
- `graph:mutate` — modifying document content and structure
- `aura:validate` — validating context markup against ontologies
- `nlp:analyze` — running NLP extraction on document text (for agent-driven annotation)
- `datasource:query` — querying bound datasources
- `datasource:refresh` — refreshing datasource content
- `provenance:write` — recording provenance entries for contributions
- `agency:evaluate` — evaluating agency claims for contributions
- `publish:flatten` — flattening manifolds to derivative containers
