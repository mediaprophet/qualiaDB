# QualiaDB Architecture

_Branch: `0.0.6-dev`_

QualiaDB is a zero-allocation, mechanically sympathetic semantic database and multi-agent collaboration ecosystem. It bridges the string-heavy reality of the Semantic Web with hardware-aligned execution paths, enforcing strict constraints to ensure bounded memory and deterministic performance.

## 1. Core Principles & Constraints

- **Zero-Heap in Hot Paths**: No `Vec`, `String`, or `Box` allocations inside evaluator loops. Callers supply fixed-size output buffers (`&mut [T]`) or use `[T; N]` stack arrays.
- **42MB Prolog Sentinel & SlgArena**: The core Virtual Machine executes within a strictly bounded 42MB memory envelope. The `SlgArena` manages state structurally without dynamic growth.
- **The 48-byte Super-Quin**: Every semantic datum fits into a `QualiaQuin` (6 `u64` fields: subject, predicate, object, context, metadata, parity). Hashes and bit-packing replace string pointers.
- **Mechanical Sympathy**: Data layouts and evaluation paths are designed for maximum CPU cache efficiency, avoiding pointer chasing and random memory access.

## 2. Universal Translator: CLI Ingestion Pipeline

The `qualia-cli` is the entry point for sovereign data ingestion into `.q42` vaults.
- **Formats**: Prioritizes Cognitive AI Chunks (`.chk`) and CBOR-LD (`.cbor` / `.cbor-ld`).
- **Zero-Allocation Parsing**: Data is pull-parsed sequentially. Values are hashed directly into Quins using FNV-1a.
- **Multi-Pass External Sorter**: Handles datasets larger than RAM by buffering up to ~50MB chunks, sorting by object hash, and writing to disk. A K-Way Merge then emits the final `.q42` stream.
- **BIDX Indexing**: A `.q42.bidx` sidecar tracks block ranges for binary-search resolution.

## 3. Webizen VM & Modality Logics

The Webizen VM natively interprets SHACL constraints and N3Logic rules through a set of integrated reasoning modalities. These modalities expand the capability of the semantic engine without allocating memory.

- **Deontic Logic** (`deontic_logic.rs`): Handles rights, obligations, and defeaters (`OP_OBLIGATE`, `OP_PERMIT`, `OP_FORBID`).
- **Epistemic Logic** (`epistemic.rs`): Handles knowledge states, nested claims, and certainty (`OP_KNOWS`, `OP_BELIEVES`, `OP_COMMON_KNOWLEDGE`).
- **Temporal Logic (LTL)** (`temporal_ltl.rs`): Handles Linear Temporal Logic trace verification (`Globally`, `Finally`, `Next`, `Until`, `Release`).
- **Paraconsistent & Dialectical Logic**: Isolates contradictions (`paraconsistent.rs`) and synthesizes them (`dialectical.rs`) without causing logic explosions.
- **Spatio-Temporal Logic**: Allen Interval logic for time-span intersections.

## 4. Phase 5: Domain-Specific Scientific Primitives

The database engine includes native `SlgOpcode` hardware-aligned instructions for scientific evaluation, bypassing the need for slow external RPCs.

- **Clinical Engine (`clinical_engine.rs`)**: Risk scoring (Framingham, CHA₂DS₂-VASc, SCORE2), eGFR, CrCl, Pharmacokinetics (one-compartment models), and SOFA scores.
- **Bioinformatics (`bioinformatics.rs`)**: Zero-allocation DNA-to-Protein translation, Isoelectric point calculation, Peptide cleavage prediction, and Sequence alignment.
- **Organic Chemistry (`organic_chemistry.rs`)**: SMILES parsing, ADMET heuristics (BBB permeation), Ligand efficiency calculations, TPSA, LogP, and isotopic mass distribution.

## 5. Agent & LLM Layer

The `llm_agent.rs` coordinates interactions with local (llama.cpp) or remote LLM backends.
- **Agent Intent Validation**: Every LLM intent must be declared and pre-validated by the Webizen VM against the Rights Ontology.
- **Provenance Citation**: All LLM outputs must cite a specific `QualiaQuin` provenance hash. Ungrounded outputs are rejected.
- **Memory Budgeting**: The LLM runtime operates within a strict 128MB ceiling to protect the 42MB VM footprint.

## 6. MCP & Dynamic Profiles (Upcoming Architecture)

To manage domain specificity:
- **MCP Mediation Layer**: The Model Context Protocol establishes an explicit "Intent Frame" restricting the LLM's agency to a defined purpose (e.g., "ClinicalDiagnostic").
- **Dynamic Capability Profiles**: Users can load domain-specific declarative profiles that selectively map relevant ontologies (e.g., SNOMED, Bio2RDF) and activate subsets of the Scientific Primitives, maintaining a compact memory footprint.
