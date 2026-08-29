# POET Knowledge & Semantic Library Specification

**Document ID:** `POET-SPEC-007`  
**Status:** Canonical Domain Specification  
**Scope:** Visual knowledge graph exploration, Semantic Library management, dataset ingestion, ontology mapping, DID RDF Documents, and dual-layer serialization in POET.

---

## 1. Overview & Semantic Substrate

The Semantic Library is POET's foundational knowledge layer, bridging human conceptual thinking and machine-verifiable ontologies. It manages multi-modal datasets, RDF/Turtle ontologies, SHACL validation shapes, DID RDF documents, and verified knowledge Quins.

```
+-----------------------------------------------------------------------------------+
|                        KNOWLEDGE & SEMANTIC LIBRARY TOPOLOGY                      |
+-----------------------------------------------------------------------------------+
|  [Visual Knowledge Graph Explorer] <===> [Semantic Library Asset Hub]             |
|  - Interactive 2D/3D node-link canvas     - Multi-modal dataset ingestion         |
|  - Force-directed ontology navigation    - Metadata, licensing & sensitivity tags|
|  - Clickable entity relationship edges    - Local graph publication               |
|                                                                                   |
|  [Visual Ontology Mapping Editor]  <===> [DID Document & Serialization Engine]    |
|  - Drag-and-drop property mapping         - CBOR-LD / N3 zero-heap compute format  |
|  - External schema -> Qualia ontology     - Dynamic Turtle (.ttl) / JSON-LD export|
|  - Transform rules & lossy warnings       - Recursive DID graph resolution        |
+-----------------------------------------------------------------------------------+
```

---

## 2. Visual Knowledge Graph Explorer

- **Interactive Node-Link Canvas:** Real-time force-directed layout rendering entities as nodes and predicates/properties as directional edges.
- **Node Filtering & Grouping:** Filter by class type, sensitivity level, author DID, or search terms with dynamic clustering.
- **Entity Inspector Drawer:** Clicking any node opens a slide-over panel displaying all incoming/outgoing Quins, literal values, Lamport timestamps, and provenance metadata.

---

## 3. Decentralized Identifier (DID) Documents & Dual-Layer Serialization

Every Decentralized Identifier in the system resolves to an attached **RDF Graph Document**:
- **Recursive DID Graphs:** A DID Document can declare verification methods, authentication keys, service endpoints, authorizations, and child/related DIDs, creating a decentralized linked graph.
- **Dual-Layer Serialization Architecture:**
  1. **Compute & Storage Tier (CBOR-LD / N3 / Super-Quins):** Compact binary encoding (`[NQuin]`) optimized for zero-heap evaluation, deterministic hashing, 42MB Sentinel budget compliance, and GPU SIMD pipelines.
  2. **User Presentation Tier (Turtle / JSON-LD / Visual Tree):** On-demand dynamic serialization into human-readable **Turtle (`.ttl`)**, JSON-LD, or expandable UI tree views, allowing human users to inspect, understand, and audit the exact RDF structure.

---

## 4. Semantic Library Asset Management

- **Dataset Ingestion:** Multi-format ingest supporting Turtle (`.ttl`), N-Triples (`.nt`), JSON-LD, CML documents, CSV/TSV tables, and raw text extracts.
- **Classification & Provenance:** Every ingested item carries sensitivity tags (`Public`, `Restricted`, `Classified`, `Secret`), publisher DID, and cryptographic content hash.
- **Library Search & Query:** Full-text fuzzy search combined with structured SPARQL query execution over local Quins.

---

## 5. Visual Ontology Mapping Editor

- **Mapping Interface:** Visual dual-column canvas for connecting fields from external schemas (e.g., Schema.org, FHIR, DCAT) to internal Qualia ontology terms.
- **Transformation Rules:** Define conversion rules (e.g., unit scaling, string normalization, type coercion) with live preview of transformed Quins.
- **Lossy Mapping Guard:** Explicit warnings when a mapping drops fidelity or loses semantic precision, requiring human user confirmation.

---

## 6. SHACL Shape Validation & Error Navigation

- **Constraint Validation:** Automated validation against W3C SHACL shape definitions (e.g., `minCount`, `maxCount`, `datatype`, `pattern`, `nodeKind`).
- **Visual Error Navigation:** Clicking a validation error jumps directly to the violating node in the graph explorer, highlighting the exact property and constraint failure.

---

## 7. Knowledge Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-KNOW-001` | **Interactive Graph Explorer** | 2D/3D force-directed node-link graph visualizer with zoom, pan, and entity selection. | `ontology_views`, `icon_graph.rs` |
| `POET-KNOW-002` | **Entity Inspector Drawer** | Contextual slide-over drawer displaying incoming/outgoing Quins, literals, and provenance. | `semantic_library_view.rs` |
| `POET-KNOW-003` | **Multi-Format Ingestion Hub** | Ingestion pipeline for Turtle, JSON-LD, CML, CSV, and text with sensitivity tagging. | `semantic_library_render.rs` |
| `POET-KNOW-004` | **Visual Ontology Mapper** | Dual-column visual mapping editor connecting external data fields to Qualia ontology terms. | `dataset_views`, `ontology_views` |
| `POET-KNOW-005` | **Lossy Mapping Guard** | Warn user of fidelity loss in field transformations and require explicit acknowledgment. | `dataset_views` |
| `POET-KNOW-006` | **SHACL Validation & Navigation**| Run SHACL shape validation with clickable links jumping directly to offending graph nodes. | `shacl_compiler.rs`, `ontology_views` |
| `POET-KNOW-007` | **Natural Person Modeling Guard**| Enforce that natural human beings are represented via arrays of DIDs across artifacts, never `owl:Thing`. | `ontology_views`, `AGENTS.md` |
| `POET-KNOW-008` | **DID Document Dual Serialization** | Store and compute DID RDF documents in CBOR-LD/N3 while rendering in Turtle/JSON-LD for users. | `qualia-core-db`, `poet` |
