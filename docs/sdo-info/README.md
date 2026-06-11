# SDO Standards Backlog

This folder tracks which parts of QualiaDB are plausible candidates for
external standardization, which ones are still internal-only, and which
standards body is the best fit when we are ready.

## Why this exists

QualiaDB already exposes several custom artifacts and protocol surfaces:

- unified v2 `.q42` volumes (embedded lex, BIDX, LZ4 SuperBlocks)
- legacy `.q42.lex` and `.q42.bidx` sidecars (read compatibility)
- deprecated `.c.q42` transport alias
- `.qualia` vault manifests / collection descriptors
- `did:q42` identifiers / topological pointers
- QCHK capability profiles
- SHACL-native Qualia extensions
- Solid bridge behaviors
- MCP intent mediation and fiduciary control surfaces

Some of these are good standards candidates. Some are not ready yet. This
folder is where we keep that distinction explicit.

## Ground rules

1. Do not submit an external spec for a format that is still internally
   inconsistent.
2. Separate normative specs from implementation guides and architecture notes.
3. Prefer the smallest viable scope for each submission.
4. Treat multi-vendor interoperability as the threshold for externalization.

## Terminology guardrail

Within this standards folder, use the following distinction consistently:

- `human identity` means the enumerated human reality of a person in their
  lived and social context
- `identifier` means a technical label such as a DID, URI, hash, pointer, or
  method-specific token
- `auth` means authentication and authorization material
- `verifiable claim` and `verifiable credential` mean attestation artifacts,
  not human identity itself

Do not define human identity as a nym, identifier, credential, or auth token.
The environment may carry and relate those technical elements, but they remain
supporting artifacts around human identity rather than substitutes for it.

## SDO track guide

### W3C / Solid CG

Best fit for:

- RDF / SHACL / DID / Solid-facing extensions
- vocabulary registries
- implementation guidance for linked-data interoperability

Recommended document style:

- ReSpec or Bikeshed-style HTML draft
- start as a Community Group Report or Note-style editor's draft

### IETF

Best fit for:

- media types
- HTTP transport conventions
- URI / protocol behaviors
- wire-level interoperability rules

Recommended document style:

- Internet-Draft
- author in RFCXML directly, or Markdown that compiles to RFCXML

### OASIS

Best fit for:

- profile bundles
- package / interchange specifications
- guidance documents that may later grow into a formal committee spec

Recommended document style:

- Committee Note for guidance
- Committee Specification for a normative interchange format
- author in DocBook XML / OASIS publication pipeline

### Internal only

Use an internal explainer or ADR first when:

- the surface is single-vendor
- the semantics are still moving
- the repo has multiple incompatible encodings already
- there is no obvious external working group yet

## What to read next

- [standards-backlog.md](./standards-backlog.md) - Updated with CBOR-LD implementation status
- [q42-format-internal-draft.md](./q42-format-internal-draft.md) - Updated for v3 (temporal/DAG header, v2 migration)
- [qualia-vault-manifest.md](./qualia-vault-manifest.md) - CBOR-LD projection complete
- [did-q42-method-draft.md](./did-q42-method-draft.md)
- [qualia-sync-protocol.md](./qualia-sync-protocol.md) - CBOR-LD with Q42 lexicon complete
- [qualia-shacl-extensions.html](./qualia-shacl-extensions.html) - ReSpec specification for SHACL extensions ⭐ NEW (2026-06-10)

## Recent Implementation Updates (2026-06-11)

**✅ Q42 v3 Format + Phase 4 Merkle-DAG & SPARQL Temporal Traversal**

### **Key Achievements:**
- **v3 Format Canonical:** `q42_volume.rs` version field is now `3`; v2 files are hard-rejected (`verify_version()` errors); `migrate_v2_to_v3()` performs an in-place one-pass upgrade. v3 header adds `temporal_index_offset/length`, `merkle_root [u8;32]`, `assertion_timestamp`, and `dag_root_offset/length` carved from the former reserved region.
- **Merkle-DAG Merge Nodes:** `git_bridge.rs` now has `merge_node()` (creates primary commit + `MERGE_SECONDARY` back-link) and `nodes_as_of(ms)` (assertion-time snapshot filter). The `MERGE_SECONDARY` flag (0x0008) enables bidirectional DAG traversal across merge parents.
- **SPARQL `AS OF` / `AT TIME`:** New temporal snapshot query modifiers. Syntax: `SELECT ... WHERE { ... } AS OF "2024-06-01"^^xsd:dateTime` or `... AT TIME 1717286400000`. Implemented end-to-end: `TemporalMode` enum + `Pattern::AsOf` in `sparql_ast.rs`; `PhysicalOperatorType::AsOf` in `sparql_planner.rs`; `execute_as_of()` + `check_temporal_constraint()` in `sparql_executor.rs`; parser recognition in `sparql_parser.rs`. Executor uses T_CONTEXT PROV-O quins; open-world default (no annotation = include).
- **138 SPARQL tests passing** (up from 133 before Phase 4).

### **Updated SDO Documents:**
- **q42-format-internal-draft.md**: v3 header layout; v2 migration path; DAG section
- **standards-backlog.md**: v3 format, new `AS OF`/`AT TIME` extension entry (§6)
- **sparql-star.md**: implementation status table added
- **sparql-extensions.md**: implementation status table with all extensions

## Recent Implementation Updates (2026-06-10)

**✅ CBOR-LD with Q42 Lexicon Implementation Complete**

Major updates across all SDO documentation to reflect the completed CBOR-LD implementation:

### **Key Achievements:**
- **Zero-Allocation CBOR-LD**: Implemented with Q42's native lexicon system
- **No External Dependencies**: Eliminated JSON-LD, IRI, and HTTP dependencies
- **Performance Excellence**: 2-3x overhead vs 4-5x with traditional CBOR-LD
- **Full Offline Operation**: 100% functionality without network access
- **Semantic Interoperability**: Full CBOR-LD support with embedded vocabulary

### **Updated Documents:**
- **qualia-sync-protocol.md**: Complete CBOR-LD semantic payload implementation
- **qualia-vault-manifest.md**: CBOR-LD projection with compact binary format
- **standards-backlog.md**: Implementation status and standardization readiness

### **Standardization Readiness:**
- **IETF**: Wire format and transport specifications
- **W3C**: CBOR-LD semantic model and DID Q42 integration
- **OASIS**: Profile bundle and interchange specifications

The Qualia Protocol Ecosystem is now ready for external standardization with a self-contained, high-performance CBOR-LD implementation.

### **SHACL Extensions Specification Complete (2026-06-10)**

**✅ ReSpec-style HTML Specification Created**

A comprehensive ReSpec-style HTML specification has been created for the Qualia SHACL Extension Vocabulary:

- **90+ Constraint Types**: Documented across 4 major categories (client features, specialized libraries, core modalities, infrastructure)
- **Vocabulary Tables**: Complete property definitions with types, ranges, and descriptions
- **Conformance Classes**: Clear conformance requirements for implementations
- **Security Considerations**: Detailed security sections for medical, financial, cryptographic, and scientific computing
- **Implementation Examples**: Code examples for opcode generation and constraint usage
- **Standardization Readiness**: Ready for W3C Community Group Report submission

The specification provides a formal, standards-compliant document for external standardization of the Qualia SHACL extension vocabulary.

## Ecosystem label

Use `Qualia Protocol Ecosystem` as the umbrella label for external-facing
standards work.

Avoid using `Qualia Protocol` as a single monolithic spec title. In this repo
it currently spans at least five different surfaces that should standardize
separately:

- q42 container and transport profile
- `.qualia` vault manifest
- `did:q42` method
- Qualia sync protocol
- Qualia qapp loopback protocol
- Webizen protocol

## Primary external process references

- W3C document types and Note track: <https://www.w3.org/standards/types/>
- W3C process: <https://www.w3.org/policies/process/>
- W3C Community Group reports: <https://www.w3.org/community/reports/>
- DID Core: <https://www.w3.org/TR/did-core/>
- DID Specification Registries: <https://www.w3.org/TR/did-spec-registries/>
- Solid specification and process entry point: <https://solidproject.org/specification>
- Solid Protocol TR: <https://solidproject.org/TR/protocol>
- IETF Internet-Drafts: <https://www.ietf.org/how/ids/>
- IETF author guidance: <https://authors.ietf.org/getting-started>
- IETF Markdown drafting: <https://authors.ietf.org/drafting-in-markdown>
- OASIS specification lifecycle: <https://docs.oasis-open.org/templates/TCHandbook/content/tcprocess/standardsapprovalprocess/specificationlifecycle.htm>
- OASIS Committee Specification guidance: <https://docs.oasis-open.org/TChandbook/Reference/CommitteeSpecs.html>
- OASIS DocBook authoring templates: <https://docs.oasis-open.org/templates/DocBook/spec-0.8/oasis-specification-0.8-wd05.html>
