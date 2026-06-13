# Contextual Workspace QApp Specification

**Version:** 1.0.0
**Target Framework:** Webizen Studio (Dioxus / Rust)
**Engine Backend:** QualiaDB `.q42` Context Hashes

---

## 1. Executive Summary

The **Contextual Workspace QApp** is a sovereign, multidimensional hypermedia environment built on top of the QualiaDB engine. It breaks away from traditional document silos by treating all data as a series of transcludable, context-bound blocks. 

Rather than isolated "Word," "Excel," or "PowerPoint" applications, the Contextual Workspace provides a unified environment where users seamlessly compose text, spread data, build presentations, and embed rich media. Everything is tethered together through deep bidirectional transclusion, ensuring that a single data block can exist in infinite intersecting contexts without ever being duplicated.

## 2. Unified Reader/Writer Environment

A core philosophy of the Contextual Workspace is the dissolution of the boundary between reading and writing.

- **The Unified Interface:** The "Reader" and "Writer" are fundamentally the same environment. Users engaging with a document as a "Reader" possess the intrinsic capability to create localized contributions.
- **Contextual Overlays:** When a Reader creates a note, highlights text, or suggests a modification, they are creating new Quins bounded to their own unique `context` hash. These contributions overlay the original document without mutating the author's root graph.
- **Bidirectional Traceability:** The author can "subscribe" to overlapping contexts, immediately seeing community contributions (e.g., margin notes, inline HTML suggestions) natively woven into their view.

## 3. Multi-Modal Capability Matrix

The Contextual Workspace abandons the concept of application-specific file formats. The canvas operates on semantic "Blocks" that can morph and interact.

### Supported Block Types
1. **Word-Processing:** Rich text blocks with dynamic typography, semantic markup, and deep contextual linking.
2. **Spreadsheets:** Tabular data blocks where individual cells can be isolated Quins, queryable via SPARQL or N3 rules, updating live across all transcluded instances.
3. **Presentations:** Spatial or linear sequences grouping blocks into "slides" for display logic, transcluding live data.
4. **HTML / Semantic Nodes:** Direct HTML injection blocks for custom rendering or legacy web compatibility.
5. **Rich Media:** First-class embedding for PDFs, raster images, vector graphics, and 3D scenes (leveraging Webizen Studio's Spatial rendering layer).

## 4. Engine Mechanics: The 56-Bit Context Hash

The magic of the Contextual Workspace relies entirely on the **QualiaDB Super-Quin architecture**.

### Deep Bidirectional Transclusion
Instead of copying a spreadsheet table to paste it into a word-processing report, the user *transcludes* it. 
- In the `.q42` engine, the `NQuin` contains a 64-bit context field (Bits [0..55] hold the World/Contract/Graph DID hash). 
- When a block is transcluded, the system merely queries the BIDX (Block Index) for the source `context` hash and renders it locally.
- If the original table updates, all transclusions automatically update, preserving absolute mechanical sympathy and zero-allocation logic.

### Context Markup Language (`cml`)
The Workspace uses `cml` natively to describe these boundaries. When a user creates a new "document", they are simply assigning a new 56-bit context hash. A document is just a query that says: *"Render all Quins intersecting with Context A, plus transclusions from Context B and Context C."*

## 5. Security and Capability Management

Because the Reader and Writer are unified, robust capability gating is required:
- **Capability Profiles (`.qchk`):** Users must present a valid `.qchk` capability profile to mutate a specific context graph.
- **Ed25519 Author-Scoped Merkle Roots:** Every block created in the workspace is signed and anchored to the user's DID, ensuring cryptographic provenance even when a block is transcluded millions of times across disparate workspaces.
- **Paraconsistent Routing:** If two readers make contradictory edits to a shared spreadsheet cell, the Paraconsistent Logic router safely isolates the contradictions into bifurcated sub-contexts for human review, rather than throwing an exception.

---

## 6. Implementation Roadmap

1. **Phase 1 (Canvas Primitives):** Map existing Dioxus `text-input`, `tab-group`, and `card-view` components to individual `context` hashes.
2. **Phase 2 (The Unified Reader):** Implement the overlay system where user annotations generate Quins with a diverging `context` hash, rendered as semi-transparent nodes over the root graph.
3. **Phase 3 (Multi-Modal Blocks):** Introduce spreadsheet cell evaluation (mapping N3 rules to cells) and presentation sequence logic.
