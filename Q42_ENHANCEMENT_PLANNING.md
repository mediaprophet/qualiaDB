# Q42 Format & Architecture Enhancement Planning

**Date:** 2026-06-11  
**Updated:** 2026-06-11 (Phase 1 complete — `e42ed480`; Phase 2 complete; Phase 3 complete; Phase 4 complete — merge DAG, `AS OF`/`AT TIME` SPARQL temporal traversal; 138 SPARQL tests)  
**Branch:** `0.0.10-dev`  
**Source:** Analysis of `local/q42-related-updates-discussion.md` mapped against the current codebase.

This document translates the architectural discussion into concrete requirements, identifies
gaps against the current codebase, and records resolved design decisions and remaining open
questions. It is a working document — not a specification — updated as each phase lands.

### Phase status

| Phase | Status | Commit |
|-------|--------|--------|
| Phase 1 — Foundational Fixes | ✅ **Complete** | `e42ed480` |
| Phase 2 — Bi-Temporal & Provenance | ✅ **Complete** | `0.0.10-dev` |
| Phase 3 — Credential-Gated Views | ✅ **Complete** (items 1–2 full; item 3 SPARQL-side) | `0.0.10-dev` |
| Phase 4 — Full Merkle-DAG | ✅ **Complete** — merge DAG, `AS OF`/`AT TIME` SPARQL, 138 tests | `0.0.10-dev` |
| Phase 5 — WASM Compute Contracts | ⏳ Future | — |

### Resolved vocabulary decisions (2026-06-11)

| Layer | Vocabulary | Notes |
|---|---|---|
| Provenance / temporal | **PROV-O + Dublin Core** | `prov:generatedAtTime` = Assertion Time; `dcterms:valid` / `prov:startedAtTime` / `prov:endedAtTime` = Valid Time |
| Rights / policy | **ODRL + SKOS** | `odrl:Permission` / `Prohibition` / `Obligation`; SKOS vocabulary for actions, purposes, jurisdictions |
| Agent structure | **W3C CogAI CG shapes** | `cog:Agent`, `cog:Goal`, `cog:Belief`, `cog:Plan`; validated by SHACL |
| Validation | **SHACL** (existing domain engines + CogAI shapes + Epistemic shapes) | Single validation pass across all layers; Closed World Assumption enforces epistemic state classification |
| Epistemic layer | **Dynamic Epistemic Logic via SHACL** | JTB model (Justified True Belief): per-agent mind contexts + objective reality context; modality predicates: `knowsDirectly`, `infersFrom`, `believesViaHearsay`; SHACL shapes classify `ObjectiveKnowledge` vs `InferredBelief` vs `HearsayBelief` |
| Spatial query | **GeoSPARQL** (WKT literals internally) | KML as import/export exchange format; `<TimeStamp>`/`<TimeSpan>` maps to PROV-O valid time |

---

## 1. Summary of Discussion Requirements

The discussion identifies seven broad capability areas needed by the Q42 file format and QualiaDB:

| # | Capability | Description |
|---|-----------|-------------|
| T | Bi-temporal tracking | Distinguish **Valid Time** (when an event occurred in reality) from **Assertion Time** (when the system recorded it). Vocabulary: **PROV-O + Dublin Core** |
| S | Spatiotemporal coordinates | Anchor data to geographic coordinates and jurisdictional bounding boxes alongside time. Internal: **GeoSPARQL** WKT. Exchange: **KML** (with `<TimeStamp>`/`<TimeSpan>` feeding PROV-O valid time) |
| V | Versioning / DAG deltas | Immutable, append-only Merkle-DAG where every change is a new cryptographically-linked node, not an overwrite |
| G | Credential-gated contextual views | One file presents different data depending on the agent's Verifiable Credentials — Attribute-Based Encryption (ABE) or equivalent |
| R | Rights Ontology | DID-anchored access policies using **ODRL** (permissions/prohibitions/obligations) + **SKOS** (controlled vocabulary for actions, purposes, jurisdictions) |
| A | Agent structure | Cognitive agent profiles expressed in **W3C CogAI CG** vocabulary (`cog:Agent`, `cog:Goal`, `cog:Belief`, `cog:Plan`), validated by **SHACL** |
| W | Swarm compute contracts | Executable logic (WASM) stored in the file; peers execute locally, return signed insights |
| L | Labor/contestability provenance | Every labelled, cleaned, or moderated data item carries the DID of the worker; agents can cryptographically contest automated assertions |
| **E** | **Epistemic modeling** | **Separate objective reality (cryptographically proven facts, shared) from per-agent subjective reality (beliefs, inferences, hearsay). Dynamic Epistemic Logic: knowledge evolves differently per agent over time as queries propagate through the network. SHACL Closed World validates epistemic state.** |

---

## 2. What Already Exists

### 2.1 NQuin (48-byte core primitive)

```
Subject   [u64]  — hashed entity IRI
Predicate [u64]  — hashed property IRI
Object    [u64]  — hashed value/IRI
Context   [u64]  — graph context + sensitivity bits[63:56]
Metadata  [u64]  — bit-packed: Quin Type[63:60], Sensitivity tier[59:56], Reserved[55:32], Lamport clock[31:0] (✅ LoRA moved to side-table — see §4.1)
Parity    [u64]  — XOR of all five fields
```

**Lamport clocks** are currently stored in `metadata` bits[60:32] via `extract_lamport_clock()` /
`set_lamport_clock()`. Per the §4.1 decision, this will shift to bits[31:0] once the LoRA
side-table migration is applied. The clock provides coarse event ordering for CRDT resolution
(`crdt.rs`); wall-clock temporal semantics live in PROV-O overlay quins (§4.2), not in this field.

### 2.2 Spatial primitives

`spatial_sieve.rs` already defines `GeoCoordinate { lat: f64, lng: f64, timestamp_ms: u64 }`
and `BoundingBox`. A Minkowski spatial overlap check (`compute_spatial_overlap_gpu_mock`) exists
but is not wired to the NQuin graph — it operates on standalone coordinate arrays.

`spatial_sieve.rs` also mentions "encodes the tuple into a 48-byte Spatial_Log Quin" in a comment
at `log_spatial_coordinate()`, but the function body is a stub returning `true`.

### 2.3 Temporal / versioning (Phase 1 delivered)

`git_bridge.rs` ✅ — `DagNode` (88-byte `#[repr(C)]`), `DagStore` with
`genesis_node/commit_node/fork_node/write_branch_pointer`, `FORK_DISPUTED` contestability flag,
serialize/deserialize for Q42 v3 volume embedding, real fast-export stream from stored nodes.

`kml_bridge.rs` ✅ (new) — `import_kml()` / `export_kml()` covering `<Placemark>`, `<Point>`,
`<TimeStamp>`, `<TimeSpan>`; GeoSPARQL predicates + PROV-O temporal quins.

`q42_volume.rs` ✅ — v3 header with `temporal_index_offset/length`, `merkle_root[32]`,
`assertion_timestamp`, `dag_root_offset/length`; `migrate_v2_to_v3()`; v2 hard-rejected.

`wal.rs` — append-only semantics; does not yet expose Merkle tree or content-addressed history.

`crdt.rs` — Last-Write-Wins resolver using Lamport clocks; `DelegatedAccess` struct with
expiration timestamps and Ed25519 proofs.

### 2.4 Identity / DID layer

`web_civics.rs` — derives a Webizen ULA IPv6 address from an Ed25519 public key (Cryptokey
Routing).

`webizen_identity.rs` — Ed25519 sign/verify wired (✅ in this branch).

`sparql_did.rs` — SPARQL extension for DID resolution; exists as a module.

`identifier.rs`, `profiles.rs`, `key_vault.rs` — identity management scaffolding.

### 2.5 Rights / deontic layer

`deontic_logic.rs` — `OP_PERMIT` / `OP_FORBID` opcodes exist. N3Logic Rights Ontology rules are
evaluated at inference time (`validate_intent` in `orchestrator.rs`). Maps onto ODRL:
`OP_PERMIT` → `odrl:Permission`, `OP_FORBID` → `odrl:Prohibition`.

`fiduciary_crypto.rs` — ML-DSA signatures for fiduciary operations (✅ commitment-based in this
branch).

### 2.6 Cryptography

`zk_proofs.rs` — ZK proof generation scaffolding (✅ hash-chain approach in this branch).

No ABE implementation currently exists in the codebase.

### 2.7 SHACL validation and domain engines

`sparql_shacl.rs` — core SHACL engine. Domain-specific SHACL engines exist for biosciences,
biomedical, and organic chemistry (wired to `SlgOpcode` and WASM, 149 tests as of 2026-06-05).
This infrastructure is the natural home for CogAI SHACL shapes — a fourth shape graph loaded
at daemon startup. No structural changes needed; CogAI shapes are additive.

### 2.8 Q42 file format (Phase 1 delivered v3)

`q42_volume.rs` defines:
- `Q42_MAGIC = [0x51, 0x34, 0x32, 0x00]` ("Q42\0")
- `Q42VolumeHeader` ✅ v3 — `temporal_index_offset/length`, `merkle_root [u8;32]`,
  `assertion_timestamp u64`, `dag_root_offset/length` — all carved from former `_reserved`
- `verify_version()` hard-rejects v2; `migrate_v2_to_v3()` one-pass migration
- SuperBlock structure (LZ4-compressed NQuin arrays)
- BIDX (Block Index) for range queries on object hashes

**Still missing:** temporal index section populated (header fields exist but builder always
writes 0); spatial index; DAG section populated by ingest pipeline.

---

## 3. Requirements vs. Current State — Gap Analysis

| Req | Status | Gap |
|-----|--------|-----|
| **T** Bi-temporal | ✅ Done | `temporal_graph.rs` write helpers (`assert_temporal`, T_CONTEXT constants). `sparql_filter.rs` has `prov_predicates` constants + `ProvenanceFilter` for SPARQL joins. |
| **S** Spatiotemporal | ✅ Done | `kml_bridge.rs` produces GeoSPARQL quins from KML. `spatial_sieve.rs::log_spatial_coordinate()` writes real GeoHash-64 + SPATIAL_CONTEXT quins. |
| **V** Merkle-DAG versioning | ✅ Done | `DagStore`/`DagNode` in `git_bridge.rs`. `merge_node()` + `MERGE_SECONDARY` flag. `nodes_as_of()` for assertion-time snapshots. WAL→DagStore linking via `checkpoint_to_dag()`. SPARQL `AS OF`/`AT TIME` via `Pattern::AsOf` + `PhysicalOperatorType::AsOf`. Ingest pipeline wiring pending. |
| **G** Credential-gated views | ✅ MVP | AES-256-GCM per-layer keys in `key_vault.rs`; X25519 ECDH encapsulation; `deontic_logic.rs` VC-to-key policy evaluation. CP-ABE deferred. |
| **R** Rights Ontology | ✅ Done | `ontologies/rights_ontology.ttl` (ODRL+SKOS). `deontic_logic.rs` runtime opcodes wired. Daemon startup loading pending. |
| **A** Agent structure | ✅ MVP | `ontologies/cogai_shapes.ttl` created. `epistemic.rs` implements JTB per-agent contexts. Full `sparql_shacl.rs` loader + `validate_intent()` wiring pending (Phase 2 remaining). |
| **W** WASM compute contracts | ⚠️ Partial | `wasm_bridge.rs` exists. No Q42 WASM block type. Phase 5. |
| **L** Labor / contestability provenance | ✅ Done | `provenance.rs` — `label_with_worker_did()`, `contest_assertion()`, `write_activity()`. `DagNode::FORK_DISPUTED` flag for DAG-level contestability. |

---

## 4. Design Decisions

Most decisions are resolved (marked ✅). Two open items (§4.4, §4.5) are the only remaining
blockers before Phase 1 implementation begins.

---

### 4.1 ✅ NQuin Metadata Bit-Layout Conflict — RESOLVED

**Decision: Option B — move LoRA state out of NQuin into a side-table / LORA_CONTEXT overlay.**

**Reasoning:** The 48-byte NQuin is the atomic performance primitive; its layout must not be
compromised by sparse, inference-time state. LoRA triggers are inference-specific and sparse
relative to base graph operations — they do not belong in the struct. Moving them out frees the
top 16 bits entirely.

**New `metadata` field layout (64 bits, no ABI change to NQuin size):**

```
Bits 63:60  —  Quin Type flag (4 bits, up to 16 distinct quin types for future use)
Bits 59:56  —  Sensitivity / access tier (4 bits, links to ODRL subgraph layer)
Bits 55:32  —  Reserved (24 bits, available for Phase 2 temporal/spatial flags)
Bits 31:0   —  Lamport clock (32 bits, clean — no overlap)
```

**LoRA side-table:** `LoRAAdapterManager` (in `lora/adapter_manager.rs`) holds a
`HashMap<u64, u64>` mapping quin content-hash → active adapter ID. When an inference batch
needs LoRA context, the manager looks up by hash, not by struct field. For persistent LoRA
attribution, a `LORA_CONTEXT` overlay quin is written:
```
(quin_hash, q_hash("lora:activeAdapter"), adapter_id_hash, LORA_CONTEXT, lamport, parity)
```

**Migration:** No `.q42` file migration required. The metadata field is read-only metadata; no
persisted files encode the LoRA bits (they were runtime-only). The Lamport clock bits shift
from 60:32 to 31:0 — any persisted files with Lamport values need a one-pass rewrite of the
metadata field. A `q42 migrate-meta` CLI subcommand will handle this.

**`lib.rs` / `lora/` changes required:**
- Remove `set_context_trigger()` / `get_context_trigger()` from `NQuin`
- Update `set_lamport_clock()` / `extract_lamport_clock()` to use bits 31:0
- Add `set_quin_type()` / `get_quin_type()` for bits 63:60
- Update `lora/adapter_manager.rs`: replace metadata-bit lookup with HashMap lookup
- Update `LocalLlmAgent::warm_lora_for_prompt()`: remove metadata-bit write

---

### 4.2 ✅ Bi-Temporal Design Strategy — RESOLVED

**Decision:** PROV-O + Dublin Core predicates stored as quins in a dedicated `T_CONTEXT` overlay
graph. No NQuin struct changes. The metadata bits retain the Lamport clock for CRDT ordering
only; wall-clock temporal semantics live entirely in the graph layer.

**Predicate mapping:**

| Bi-temporal concept | Predicate | Stored as |
|---|---|---|
| Assertion Time (when recorded) | `prov:generatedAtTime` | `(entity, q_hash("prov:generatedAtTime"), encode_ms(t), T_CONTEXT, 0, parity)` |
| Valid Time start | `prov:startedAtTime` | same pattern in T_CONTEXT |
| Valid Time end | `prov:endedAtTime` | same pattern in T_CONTEXT |
| Validity period (compact) | `dcterms:valid` | ISO 8601 interval hash in object field |
| Who asserted it | `prov:wasAttributedTo` | DID hash in object field |
| Activity that produced it | `prov:wasGeneratedBy` | Activity quin hash |

KML `<TimeStamp>` imports populate `prov:generatedAtTime`; `<TimeSpan begin/end>` populates
`prov:startedAtTime` / `prov:endedAtTime` — both paths converge on the same T_CONTEXT quins.

SPARQL temporal joins use the `T_CONTEXT` graph as a named graph:
```sparql
SELECT ?s WHERE {
  GRAPH <urn:q42:context:temporal> {
    ?s prov:startedAtTime ?t1 ; prov:endedAtTime ?t2 .
    FILTER(?t1 <= "2025-01-01"^^xsd:dateTime && ?t2 >= "2025-01-01"^^xsd:dateTime)
  }
}
```

**New module required:** `temporal_graph.rs` — write helpers `assert_temporal(entity, t_valid_start, t_valid_end, t_assert)` and read helpers for T_CONTEXT queries.

---

### 4.3 ✅ Spatial Encoding — RESOLVED

**Decision:** GeoSPARQL internally; KML as the import/export exchange format.

**Internal representation** — GeoHash-64 in the `object` field of spatial quins:
```
(entity_hash, q_hash("geo:hasGeometry"), geohash_u64, SPATIAL_CONTEXT, ts, parity)
(entity_hash, q_hash("geo:asWKT"), wkt_literal_hash, SPATIAL_CONTEXT, ts, parity)
```
GeoHash-64 encodes lat/lon to ~1mm precision in a u64. Spatial range queries are range scans on
`object` within `SPATIAL_CONTEXT`. `spatial_sieve.rs::log_spatial_coordinate` implements this.

GeoSPARQL predicates (`geo:hasGeometry`, `geo:asWKT`, `geo:sfWithin`, `geo:sfIntersects`) are
used in SPARQL queries — this makes the spatial layer compatible with any GeoSPARQL client.

**KML exchange:**
- KML `<Placemark>` → graph entity with `geo:hasGeometry` quin
- KML `<Polygon>` → `BoundingBox` in `spatial_sieve.rs` → WKT POLYGON literal
- KML `<TimeStamp>` → `prov:generatedAtTime` in T_CONTEXT (feeds bi-temporal layer)
- KML `<TimeSpan begin/end>` → `prov:startedAtTime` / `prov:endedAtTime` in T_CONTEXT
- KML `<NetworkLink>` → DID-anchored remote graph pointer

KML export is the inverse: reconstruct `<Placemark>` + `<TimeSpan>` from spatial + temporal
context quins. A `kml_bridge.rs` module handles both directions.

**Jurisdictional bounding boxes** are stored as named `BoundingBox` entities with
`dcterms:spatial` + `odrl:spatial` constraints linking geographic extent to ODRL access policies.

---

### 4.4 ✅ Q42 Format Version Strategy — RESOLVED

**Decision: Option B — Migration required. v3 builds refuse to open v2 files without a `q42 migrate-meta` run.**

**Reasoning:** The Q42 format serves as a cryptographic ledger for legal investigations, biomedical
data, and rights-bearing assertions. Option A (silent bit reinterpretation) would allow a v3 engine
to silently misread a v2 file's Lamport clock bits as a valid temporal sequence — compromising the
truth of the graph without raising an error. A file's bit-layout must be mathematically verified
before any read/write operations occur. Strict enforcement is the only position consistent with
the data integrity requirement at the core of the format.

**v3 `Q42VolumeHeader` additions:**
```rust
pub struct Q42VolumeHeader {
    // existing fields (v2) ...
    pub format_version: u16,         // 3 for v3; v3 builds reject version < 3
    pub temporal_index_offset: u64,  // 0 if no temporal index present
    pub temporal_index_length: u64,
    pub merkle_root: [u8; 32],       // SHA3-256 of DAG root; [0u8;32] if no history yet
    pub assertion_timestamp: u64,    // ms since Unix epoch when volume was last written
    pub reserved: [u8; 32],          // reserved for spatial/rights extension fields
}
```

**`q42 migrate-meta` subcommand** performs a one-pass rewrite:
1. Reads v2 header, verifies magic `[0x51, 0x34, 0x32, 0x00]`
2. For each NQuin: shifts Lamport clock bits from [60:32] → [31:0]; clears bits [63:32]
3. Writes v3 header with updated `format_version = 3` and `assertion_timestamp = now()`
4. v3 build opens the file; if `format_version < 3` → hard error with migration hint

---

### 4.5 ✅ Merkle-DAG Versioning — RESOLVED

**Decision: Option B — Minimal Merkle-DAG with `DagNode` structure in `git_bridge.rs`, from day one.**

**Reasoning:** A linear WAL hash chain structurally enforces a single timeline, which would make
it impossible for a human agent to cryptographically contest an automated classification while
preserving both the original assertion and the dispute simultaneously. Contestability requires
branching at the data structure level — the Right of Recourse is not retrofittable onto a linear
log. The extra 2–3 weeks of Phase 1 effort is load-bearing, not gold-plating.

**`DagNode` structure (implemented in `git_bridge.rs`):**
```rust
#[repr(C)]
pub struct DagNode {
    pub parent_hash:   [u8; 32],   // SHA3-256 of parent DagNode; [0u8;32] = genesis
    pub quins_merkle:  [u8; 32],   // Merkle root of all NQuin hashes in this commit
    pub author_did:    u64,        // q_hash of the committing agent's DID
    pub timestamp:     u64,        // assertion time (ms since Unix epoch)
    pub message_hash:  u64,        // q_hash of optional commit description
    pub flags:         u32,        // bit 0 = merge node (two parents); bits 31:1 reserved
    pub _pad:          u32,
}
// Total: 88 bytes, repr(C) for mmap-safe serialisation
```

**Branch model:** a branch is a named pointer stored as a quin in `BRANCHES_CONTEXT`:
```
(branch_name_hash, q_hash("dag:pointsTo"), dag_node_hash, BRANCHES_CONTEXT, ts, parity)
```

**Contestability fork:** when an agent disputes a classification, `git_bridge.rs::fork_node()`
creates a new `DagNode` with `parent_hash = disputed_node_hash` and `flags |= FORK_DISPUTED`.
Both the original and the dispute branch coexist in the DAG. SPARQL queries can traverse either
path or show both via the `BRANCHES_CONTEXT` named graph.

**Merge nodes** reference two parents: `parent_hash` = primary parent; a second `DagNode` with
`flags |= MERGE_SECONDARY` points to the secondary parent. Conflicting NQuins between branches
are written to `CONTEST_CONTEXT` for human review.

---

### 4.6 ✅ Credential-Gated Views — RESOLVED

**Decision: Option B — AES-256-GCM per-layer keys with X25519 ECDH key encapsulation.**

**Reasoning:** `aes-gcm` and `curve25519-dalek` are already in the workspace. The functional
outcome is identical to CP-ABE for the target threat model (file owner as key authority, DID-
identified agents presenting Verifiable Credentials). CP-ABE's additional 8–12 weeks of
cryptographic engineering introduces unacceptable risk for 0.x releases.

**Layer model:**

Named subgraph layers, each with its own AES-256-GCM key:

| Layer constant | Example content | ODRL action required |
|---|---|---|
| `PUBLIC` | Open data, ontology quins | none |
| `PROFESSIONAL` | Employment, affiliation | `odrl:use` with professional VC |
| `LEGAL` | Case records, testimony | judge/solicitor VC |
| `MEDICAL` | Health records | clinician VC + purpose constraint |
| `FIDUCIARY` | Financial, asset records | fiduciary VC |

**Key encapsulation flow:**
1. Subgraph key `K_layer` ∈ AES-256 stored in `key_vault.rs`
2. Agent presents VC to `deontic_logic.rs` → VC attributes evaluated against ODRL policy
3. If policy satisfied: `key_vault.rs` generates capsule = `X25519(agent_pubkey, HKDF(K_layer))`
4. Agent decapsulates with their DID private key → recovers `K_layer` → decrypts subgraph quins
5. Capsule is one-use, time-bounded (TTL in ODRL constraint)

**CP-ABE** is deferred to a future research phase after Phase 3 stabilises.

---

### 4.7 ✅ Rights Ontology — Format and Anchoring — RESOLVED

**Decision:** ODRL + SKOS, as a Turtle ontology file (`ontologies/rights_ontology.ttl`) loaded
at daemon startup. The deontic opcodes in `deontic_logic.rs` are the runtime evaluation layer;
the Turtle file is the vocabulary layer — they are complementary, not alternative.

**Vocabulary stack:**

```
ODRL (W3C Rec.)         — policy structure: Permission, Prohibition, Obligation, Constraint
SKOS (W3C Rec.)         — controlled vocabulary: Actions, Purposes, Jurisdictions, Agent roles
Dublin Core (DCMI)      — resource metadata: rights, subject, publisher linkage
deontic_logic.rs        — runtime opcodes: OP_PERMIT, OP_FORBID, OP_OBLIGE (map to ODRL classes)
```

**ODRL mapping to existing code:**

| `deontic_logic.rs` | ODRL class | SKOS role |
|---|---|---|
| `OP_PERMIT` | `odrl:Permission` | `skos:Concept` in actions vocabulary |
| `OP_FORBID` | `odrl:Prohibition` | same |
| `OP_OBLIGE` | `odrl:Obligation` | same |
| N3Logic rule | `odrl:Constraint` | `skos:scopeNote` on constraint concept |
| DID party | `odrl:uid` | `skos:prefLabel` for human-readable identity |

**ODRL parties are DID URIs** — `odrl:permission [ odrl:party <did:example:alice> ]`. This
means the same DID identifier works across ODRL policies, PROV-O attribution, and CogAI agent
profiles. No separate identity namespace required.

**SKOS vocabulary sections in `rights_ontology.ttl`:**
- `q42:Actions` — read, write, infer, label, contest, compute, share, export
- `q42:Purposes` — research, clinical, commercial, governance, personal
- `q42:Jurisdictions` — topical + geographic (linked to GeoSPARQL bounding boxes via §4.3)
- `q42:AgentRoles` — human, agent, swarm-peer, fiduciary, rights-orchestrator

**Terminology anchor** — the ontology formally defines:
- `q42:HumanCentricControl` (preferred); `owl:deprecated q42:Sovereign`
- `q42:DataAgency` (preferred); `owl:deprecated q42:DataOwnership`
- `q42:AccessPolicy` (preferred); `owl:deprecated q42:Permission` in ACL sense

**Dialect forks** — a community override is a separate `skos:ConceptScheme` that
`skos:broaderTransitive` the base vocabulary. The SHACL shape for policies allows either the
base scheme or any registered override scheme as valid `odrl:Action` values.

**Signing** — the canonical `rights_ontology.ttl` is signed with the **founding DID of the
Peace Infrastructure Project (established 2020)** via `fiduciary_crypto.rs`. The signature is
stored as a `prov:wasAttributedTo` annotation quin at daemon startup, binding the network
mathematically to the vocabulary definitions. Community dialect files (e.g., an indigenous
cooperative's contextual override) carry their own community DID's signature and
`skos:broaderTransitive` the root ontology — they extend without overriding the root definitions.

---

### 4.8 🟢 Labor Provenance and Contestability

**Labor provenance:**  
Each NQuin that has been labeled, cleaned, or moderated by a human agent should carry
that agent's DID. The most natural representation:
```
(data_quin_hash, q_hash("LABELLED_BY"), worker_did_hash, PROVENANCE_CONTEXT, timestamp, parity)
(data_quin_hash, q_hash("LABELLED_AT"), encode_ms(t_assert), PROVENANCE_CONTEXT, ..., parity)
```
This is an extension of Option A for bi-temporal (§4.2) and requires no NQuin struct changes.

**Contestability:**  
When an automated swarm computation produces a result (appends a quin), the affected DID must
be able to "fork" that result with a signed dispute quin:
```
(disputed_quin_hash, q_hash("CONTESTED_BY"), agent_did_hash, CONTEST_CONTEXT, t_assert, parity)
(disputed_quin_hash, q_hash("CONTEST_REASON"), reason_hash, CONTEST_CONTEXT, ..., parity)
```
The SPARQL engine must be able to query "is this assertion contested?" and return both
the original and any dispute records. This is additive — no struct changes required.

---

### 4.9 ✅ Homomorphic Encryption and ZK Encrypted Search — RESOLVED

**Decision: FHE is out of scope for all 0.x releases. ZK encrypted search (MIRACL approach) is the priority privacy path.**

**FHE** — retained as a stub + design note in `zk_proofs.rs` only. The computational overhead
(10³–10⁶× vs plaintext) makes it unsuitable for swarm compute in any 0.x timeframe.

**ZK encrypted search** — agents prove a predicate (e.g., "this graph contains a spatial quin
within bounding box X") without revealing the underlying data. `zk_proofs.rs` has the proof
scaffolding. Integration with the SPARQL query path is a 3–6 month investment after Phase 3
stabilises — it builds on the AES-GCM layer key infrastructure from §4.6.

**Deployment value:** agents in the swarm can verify semantic links or spatial states for
another agent's Q42 file without that agent exposing plaintext — directly enabling the
compute-to-data model described in the discussion.

---

### 4.10 ✅ W3C CogAI CG — Agent Structure Layer — RESOLVED

**Decision:** Load W3C Cognitive AI CG SHACL shapes as a fourth domain shape graph in
`sparql_shacl.rs`, alongside the existing biosciences, biomedical, and organic chemistry engines.

**Why CogAI applies here:**

The Webizen agent model (`orchestrator.rs`, `llm_agent.rs`, `agency.rs`) implements an agent
with beliefs (the knowledge graph), goals (the inferred task), plans (the orchestration pipeline),
and actions (validated by `validate_intent`). CogAI vocabulary makes this structure explicit and
queryable as linked data.

**Key CogAI concepts and their QualiaDB mapping:**

| CogAI concept | QualiaDB equivalent | Stored as |
|---|---|---|
| `cog:Agent` | Webizen identity | `(did_hash, rdf:type, q_hash("cog:Agent"), AGENT_CONTEXT, ts, parity)` |
| `cog:Belief` | Graph assertion | PROV-O annotated quin in T_CONTEXT |
| `cog:Goal` | Orchestrator task | `(agent_hash, cog:hasGoal, goal_hash, AGENT_CONTEXT, ts, parity)` |
| `cog:Plan` | Inference pipeline | `(goal_hash, cog:hasPlan, plan_hash, AGENT_CONTEXT, ts, parity)` |
| `cog:Action` | Validated opcode | Links to `odrl:Permission` — goal can only be pursued if ODRL permits action |
| Working memory | SlgArena | The 42 MB arena is the agent's working memory at inference time |

**Integration with validate_intent():**

`orchestrator.rs::validate_intent()` currently checks N3Logic Rights Ontology rules. With CogAI:
1. Check ODRL policy for the requested action (existing path)
2. Check SHACL CogAI shapes — does the agent's current `cog:Goal` graph conform to the shape?
3. If both pass → inference proceeds. If CogAI shape fails → the goal is malformed, not just
   forbidden; write a different conduct violation type to the WAL.

**New `cog:Belief` + PROV-O intersection:**

A `cog:Belief` is an assertion held by an agent at a given time. This maps naturally onto
PROV-O bi-temporal quins — a belief formed at T1 about an event at T2:
```
(agent_hash, cog:holdsBelief, belief_quin_hash, AGENT_CONTEXT, t_assert, parity)
(belief_quin_hash, prov:generatedAtTime, t_assert, T_CONTEXT, 0, parity)
(belief_quin_hash, prov:startedAtTime, t_valid, T_CONTEXT, 0, parity)
```

**Files affected:** `sparql_shacl.rs` (load CogAI shape graph), new `ontologies/cogai_shapes.ttl`,
`orchestrator.rs` (CogAI pre-flight check in `validate_intent`).

---

### 4.11 ✅ Epistemic Layer — Dynamic Epistemic Logic via SHACL — RESOLVED

**Source:** `local/epistomology-notes.md` — design discussion covering the John/Jane/Frank scenario.

**Core problem:** Standard databases maintain one "God's eye" table — if a row exists, it is
*objective reality* for everyone. A Human-Centric architecture must model that John observing an
event, Frank querying about it, and Jane inferring it from Frank's query each produces a
**different, valid epistemic state** — not one "true" value that overwrites the others.

**Decision: Dynamic Epistemic Logic stored as NQuins, validated by SHACL shapes (Closed World Assumption). No OWL reasoning engine — executable mindware only.**

#### Epistemic context model

Three named-graph tiers:

| Context constant | Meaning | Producers |
|---|---|---|
| `OBJECTIVE_CONTEXT` | Cryptographically verified facts (signed sensor, notarized document) | Hardware/signatures |
| `EPISTEMIC_CONTEXT` (base) | Shared epistemic metadata (query events, propagation records) | `epistemic.rs` |
| `agent_epistemic_context(did)` | Per-agent mind space — beliefs + inferences scoped to this agent | Each agent independently |

Per-agent context is derived at runtime:
```rust
pub fn agent_epistemic_context(did_hash: u64) -> u64 {
    q_hash("urn:qualia:context:epistemic:agent") ^ did_hash
}
```
XOR-folding the base context with the DID hash gives a stable, deterministic, non-colliding context per agent without string allocation.

#### JTB model (Justified True Belief) predicate vocabulary

```
P_COG_BELIEVES          = q_hash("https://www.w3.org/community/cogai/ont#believes")
P_COG_QUERIES           = q_hash("https://www.w3.org/community/cogai/ont#queries")
P_COG_OBSERVES          = q_hash("https://www.w3.org/community/cogai/ont#observes")
P_COG_INFERS            = q_hash("https://www.w3.org/community/cogai/ont#infers")
P_KNOWS_DIRECTLY        = q_hash("urn:qualia:epistemic:knowsDirectly")
P_INFERS_FROM           = q_hash("urn:qualia:epistemic:infersFrom")
P_BELIEVES_VIA_HEARSAY  = q_hash("urn:qualia:epistemic:believesViaHearsay")
P_WAS_DERIVED_FROM      = q_hash("http://www.w3.org/ns/prov#wasDerivedFrom")
```

#### John/Jane/Frank scenario — quin representation

```
// t₁: John directly observes the event (cryptographic witness)
(john_did, P_KNOWS_DIRECTLY,   matter_hash,         AGENT_CONTEXT_JOHN,  t1, parity)
(john_did, P_WAS_GENERATED_BY, crypto_sensor_hash,  OBJECTIVE_CONTEXT,   t1, parity)

// t₂: Frank queries about the matter — his query is itself an epistemic event
(frank_did, P_COG_QUERIES,     matter_hash,          AGENT_CONTEXT_FRANK, t2, parity)

// t₃: Jane observes Frank's query event (not the original matter)
(jane_did,  P_COG_OBSERVES,    frank_query_hash,     AGENT_CONTEXT_JANE,  t3, parity)
// t₄: Jane infers her own version of the matter from what she observed
(jane_did,  P_INFERS_FROM,     jane_variant_hash,    AGENT_CONTEXT_JANE,  t4, parity)
(jane_did,  P_WAS_DERIVED_FROM,frank_query_hash,     AGENT_CONTEXT_JANE,  t4, parity)
```

Jane's `AGENT_CONTEXT_JANE` quins **never overwrite** John's `AGENT_CONTEXT_JOHN` quins.
SPARQL can query either agent's timeline independently or compare them.

#### SHACL epistemic shapes (Closed World)

`ObjectiveKnowledgeShape`: passes only when `prov:wasGeneratedBy` points to a `q42:CryptoSensor`
or `q42:SignedObjectiveEvent`. This upgrades a belief to *knowledge*.

`InferredBeliefShape`: passes when at least one `prov:wasDerivedFrom` exists — even without
cryptographic proof. Correct for Jane's case.

`HearsayBeliefShape`: passes when `q42:believesViaHearsay` is present. Lowest epistemic weight.

**Why SHACL over OWL:**
- OWL uses Open World Assumption — absence of proof doesn't prove absence. Dangerous for legal/medical contexts.
- SHACL uses Closed World Assumption — if the graph doesn't have a `prov:wasGeneratedBy` cryptographic link, the state is **definitively classified** as `InferredBelief`, not "might be Knowledge somewhere."
- This makes the epistemic engine an executable **validation circuit**, not a philosophical reasoner.

#### Integration with `deontic_logic.rs`

Once SHACL classifies a belief's epistemic state, deontic rules can gate access:
```
OP_PERMIT read IF subgraph CONFORMS_TO ObjectiveKnowledgeShape
OP_CONDITIONALLY_PERMIT read IF subgraph CONFORMS_TO InferredBeliefShape AND agent_clearance >= LEGAL
OP_FORBID write_over IF subgraph CONFORMS_TO ObjectiveKnowledgeShape AND requestor != original_author
```

This separates *what is known* (epistemic layer) from *who may act on it* (deontic layer) cleanly.

#### New module: `epistemic.rs`

```rust
pub enum EpistemicState { ObjectiveKnowledge, InferredBelief, HearsayBelief, Unknown }

pub fn agent_epistemic_context(did_hash: u64) -> u64
pub fn assert_objective_knowledge(agent_did, proposition_hash, proof_hash, ts) -> [NQuin; 3]
pub fn assert_inferred_belief(agent_did, proposition_hash, source_hash, ts) -> [NQuin; 3]
pub fn assert_hearsay_belief(agent_did, proposition_hash, informant_did, ts) -> [NQuin; 3]
pub fn record_query_observation(observer_did, query_hash, ts) -> NQuin
pub fn classify_epistemic_state(quins, agent_did, proposition_hash) -> EpistemicState
```

**Files required:** new `epistemic.rs`, new `ontologies/epistemic_shapes.ttl`, updates to
`provenance.rs` (epistemic predicate constants), `lib.rs` (module declaration), `sparql_shacl.rs`
(load epistemic shapes as 5th domain), `deontic_logic.rs` (Phase 3: epistemic gate in OP_PERMIT).

---

## 5. Proposed Implementation Phases

Ordered by impact, dependencies, and effort. Each phase is additive and backward-compatible
where possible.

### Phase 1 — Foundational Fixes ✅ COMPLETE (commit `e42ed480`)

1. ✅ **NQuin metadata bit-layout fix** — `set_context_trigger`/`get_context_trigger` removed;
   LoRA → `LoRAAdapterManager.active_adapter_by_hash: HashMap<u64,u64>`; Lamport → bits[31:0];
   `set_quin_type` (bits[63:60]), `set_sensitivity_tier` (bits[59:56]) added; `q42 migrate meta`
   CLI subcommand live.

2. ✅ **Q42 v3 volume header** — `temporal_index_offset/length`, `merkle_root[32]`,
   `assertion_timestamp`, `dag_root_offset/length` in header; `migrate_v2_to_v3()` migration;
   v2 hard-rejected; `sizeof == 256` compile-time assert.

3. ✅ **`git_bridge.rs` — real DagNode Merkle-DAG** — `DagNode` (88-byte `#[repr(C)]`),
   `DagStore` with genesis/commit/fork/branch, `FORK_DISPUTED` flag, serialize/deserialize.
   Phase 1 ships the full data structure; ingest pipeline wiring is Phase 2.

4. ✅ **Ontology files** — `ontologies/rights_ontology.ttl` (ODRL+SKOS+Human-Centric anchors),
   `ontologies/cogai_shapes.ttl` (W3C CogAI SHACL shapes incl. `InferenceIntentShape`).
   Runtime loading (`sparql_shacl.rs` + `orchestrator.rs`) is Phase 2.

5. ✅ **KML bridge** — `kml_bridge.rs`: `import_kml()` / `export_kml()`; `<Placemark>`,
   `<Point>`, `<TimeStamp>`, `<TimeSpan>`; GeoSPARQL + PROV-O quins; GeoHash-64; round-trip
   tested. Polygon + NetworkLink support is Phase 2.

**Files delivered:** `lib.rs`, `q42_volume.rs`, `lora/adapter_manager.rs`, `llm_agent.rs`,
`git_bridge.rs` (rewrite), new `kml_bridge.rs`, `qualia-cli/src/main.rs`, new `ontologies/`

---

### Phase 2 — Bi-Temporal & Provenance Quins ✅ COMPLETE

Phase 1 shipped all the data structures; Phase 2 wired them into the runtime graph layer.

#### 2.1 `temporal_graph.rs` (new module)
```rust
pub fn assert_temporal(
    entity: u64, t_valid_start: u64, t_valid_end: Option<u64>, t_assert: u64
) -> [NQuin; 4]
// returns quins for: generatedAtTime, startedAtTime, endedAtTime, wasAttributedTo
```
Context: `T_CONTEXT = q_hash("urn:qualia:context:temporal")`.
Predicate hashes pre-computed as `const` via `q_hash()`.
SPARQL joins on `T_CONTEXT` — extend `sparql_filter.rs` to handle `prov:` predicates natively.

#### 2.2 `provenance.rs` (new module)
```rust
pub fn label_with_worker_did(data_hash: u64, worker_did: u64, ts: u64) -> [NQuin; 2]
pub fn contest_assertion(disputed_hash: u64, agent_did: u64, reason_hash: u64, ts: u64) -> [NQuin; 3]
pub fn write_activity(activity_hash: u64, label_hash: u64, start_ms: u64, end_ms: u64) -> [NQuin; 3]
```
Context: `PROVENANCE_CONTEXT = q_hash("urn:qualia:context:provenance")`.
`contest_assertion` also calls `git_bridge::DagStore::fork_node()` to register the dispute branch.

#### 2.3 Wire `spatial_sieve.rs::log_spatial_coordinate()`
Replace stub body with GeoHash-64 encoding (reuse `kml_bridge::encode_geohash_64`) and write
two `SPATIAL_CONTEXT` quins: `geo:hasGeometry` + `geo:asWKT`. Currently returns `true` without
writing anything.

#### 2.4 Wire CogAI SHACL shapes into `sparql_shacl.rs`
Load `ontologies/cogai_shapes.ttl` as the fourth domain shape graph at daemon startup alongside
the existing biosciences/biomedical/organic-chemistry engines. No new engine code — reuse the
existing `ShaclCompiler` path with `DomainTarget::CogAI`.

#### 2.5 Wire CogAI pre-flight into `orchestrator.rs::validate_intent()`
After the existing ODRL N3Logic check, run the `InferenceIntentShape` from `cogai_shapes.ttl`:
1. Check `q42:inferenceAuthorizedBy` — must point to a `q42:HumanCentricControl` party
2. Check `q42:provenanceCitations` — must have ≥ 1 citation  
3. On violation → write CogAI-typed WAL entry (distinct violation type from ODRL rejection)

Write `cog:Agent`, `cog:Goal`, `cog:Belief` quins to `AGENT_CONTEXT` as part of this flow.

#### 2.6 Load ontology TTL files at daemon startup
`daemon.rs` / `orchestrator.rs` startup sequence:
1. Parse `ontologies/rights_ontology.ttl` → ingest into named graph `urn:qualia:ontology:rights`
2. Parse `ontologies/cogai_shapes.ttl` → register with `sparql_shacl.rs` shape compiler
3. Both paths use existing `rio_turtle` parser

#### 2.7 KML Phase 2 — `<Polygon>` and `<NetworkLink>`
Extend `kml_bridge.rs::import_kml()`:
- `<Polygon><outerBoundaryIs>` → WKT `POLYGON((lon lat, ...))` literal; `BoundingBox` for
  `spatial_sieve.rs`; link to ODRL `odrl:spatial` constraint via jurisdiction lookup
- `<NetworkLink><Link><href>` → DID-anchored remote graph pointer quin:
  `(subject, q_hash("schema:url"), href_hash, SPATIAL_CONTEXT, ts, parity)`

#### 2.8 Ingest pipeline DagStore wiring
`crates/qualia-core-db/src/ingest.rs` — after each `SuperBlock` is written, call
`DagStore::commit_node()` and update `Q42VolumeHeader::merkle_root` with the new DAG root hash.
On first ingest into a fresh file, call `DagStore::genesis_node()`.

#### 2.9 `epistemic.rs` — Dynamic Epistemic Logic module (§4.11)

New module implementing the JTB epistemic layer:

```rust
// Named graph contexts
pub const OBJECTIVE_CONTEXT: u64 = q_hash("urn:qualia:context:objective");
pub const EPISTEMIC_CONTEXT:  u64 = q_hash("urn:qualia:context:epistemic");
pub fn agent_epistemic_context(did_hash: u64) -> u64

// Epistemic state classification
pub enum EpistemicState { ObjectiveKnowledge, InferredBelief, HearsayBelief, Unknown }
pub fn classify_epistemic_state(quins: &[NQuin], agent_did: u64, proposition: u64) -> EpistemicState

// Write helpers
pub fn assert_objective_knowledge(agent_did: u64, proposition: u64, proof_hash: u64, ts: u64) -> [NQuin; 3]
pub fn assert_inferred_belief(agent_did: u64, proposition: u64, source_hash: u64, ts: u64) -> [NQuin; 3]
pub fn assert_hearsay_belief(agent_did: u64, proposition: u64, informant_did: u64, ts: u64) -> [NQuin; 3]
pub fn record_query_observation(observer_did: u64, query_hash: u64, ts: u64) -> NQuin
```

Also: new `ontologies/epistemic_shapes.ttl` with `ObjectiveKnowledgeShape`,
`InferredBeliefShape`, `HearsayBeliefShape` SHACL NodeShapes.

**Files primarily affected:** new `temporal_graph.rs`, new `provenance.rs`, new `epistemic.rs`,
`spatial_sieve.rs`, `sparql_shacl.rs`, `orchestrator.rs`, `kml_bridge.rs`, `ingest.rs`, `daemon.rs`,
new `ontologies/epistemic_shapes.ttl`

---

### Phase 3 — Credential-Gated Views (6–10 weeks)

1. **Layered AES-GCM subgraph encryption** (§4.6, Option B) — Define 4–8 named subgraph
   layers (e.g., `MEDICAL`, `LEGAL`, `FINANCIAL`, `PUBLIC`). Each layer has an AES-256-GCM
   key stored in `key_vault.rs`. Key encapsulation uses X25519 ECDH + HKDF.

2. **VC-to-key mapping in `deontic_logic.rs`** — Extend the deontic engine to evaluate
   an agent's presented Verifiable Credentials and release the relevant subgraph key if
   the VC attributes satisfy the policy.

3. **Selective disclosure / query-time decryption** — SPARQL queries on encrypted subgraphs
   first check deontic rules, then decrypt only the relevant blocks for the requesting DID.

**Files primarily affected:** `deontic_logic.rs`, `key_vault.rs`, `query_engine.rs`, `sparql_executor.rs`

---

### Phase 4 — Full Merkle-DAG Versioning ✅ COMPLETE

1. ✅ **`git_bridge.rs` — merge DAG** — `merge_node(primary_parent, secondary_parent, quins, ...)` creates a two-node merge structure: a primary commit node + a `MERGE_SECONDARY` back-link node whose `quins_merkle` encodes the primary hash for bidirectional traversal. `nodes_as_of(ms)` returns all DAG node hashes with `timestamp ≤ ms` for assertion-time snapshots. Conflict quins are written to `CONTEST_CONTEXT` (from `provenance.rs`).

2. ✅ **Branch support** — A branch is a named pointer stored as a quin in `BRANCHES_CONTEXT`. `DagStore::write_branch_pointer()` + `branch_tip()` shipped in Phase 1.

3. ✅ **Merge / contestability branches** — `FORK_DISPUTED` (Phase 1) + `MERGE_SECONDARY` (Phase 4). `fork_node()` creates contestability branches; `merge_node()` resolves them.

4. ✅ **SPARQL temporal traversal** — `TemporalMode` enum (`AsOf`, `AtTime`); `Pattern::AsOf { inner, timestamp_ms, mode }` in `sparql_ast.rs`; `PhysicalOperatorType::AsOf` in `sparql_planner.rs`; `execute_as_of()` + `check_temporal_constraint()` in `sparql_executor.rs`. Parser (`sparql_parser.rs`) recognizes `AS OF <timestamp>` / `AT TIME <timestamp>` after the WHERE clause closing brace; accepts integer ms or `"YYYY-MM-DD"^^xsd:dateTime`. Executor uses T_CONTEXT PROV-O quins (`prov:generatedAtTime`, `startedAtTime`, `endedAtTime`); open-world default: no annotation = include.

5. ✅ **WAL→DagStore linking** — `wal.rs` 32-byte `prev_dag_hash` header; `checkpoint_to_dag()`; `buffered_count()` (Phase 4 item from previous session).

**Test count:** 138 SPARQL tests (up from 133), 8 git_bridge tests (up from 5).

**Remaining:** ingest pipeline wiring (`ingest.rs` §2.8 — `DagStore::commit_node()` after SuperBlock flushes); CP-ABE (deferred Phase 5+).

**Files delivered:** `git_bridge.rs` (merge_node, nodes_as_of, MERGE_SECONDARY), `sparql_ast.rs` (TemporalMode, Pattern::AsOf), `sparql_planner.rs` (PhysicalOperatorType::AsOf), `sparql_executor.rs` (execute_as_of, check_temporal_constraint), `sparql_parser.rs` (AS OF / AT TIME parsing, parse_temporal_literal), `wal.rs`

---

### Phase 5 — WASM Compute Contracts (10–16 weeks)

1. **Q42 block type for WASM modules** — Add `BLOCK_TYPE_WASM = 0x03` to the v3 format.
   A WASM block stores: entry-point hash, access policy hash, executable bytes.

2. **Compute-to-data invocation** — When a peer requests computation, `wasm_edge.rs` loads
   the WASM block, sandboxes it, and passes it the local quin graph as a read-only view.
   The result is signed by the peer's DID and appended as provenance quins.

3. **Swarm routing** — The peer-to-peer dispatch layer (`daemon_swarm.rs`) broadcasts
   compute requests to peers holding the target Q42 files.

---

### Future / Research

- **CP-ABE** (§4.6, Option C) — deferred; requires dedicated cryptography engineering
- **FHE compute** (§4.9) — deferred; research track only
- **ZK encrypted search** — integrate with `zk_proofs.rs` after Phase 3 stabilizes

---

## 6. Files Impacted by Phase 1–2

A preliminary list of files that will need changes across Phases 1–2:

| File | Status | Changes |
|------|--------|---------|
| `lib.rs` | ✅ Phase 1 | NQuin v3 bit-layout; sensitivity tier constants; LoRA removed |
| `q42_volume.rs` | ✅ Phase 1 | v3 header; `migrate_v2_to_v3`; v2 hard-rejected |
| `lora/adapter_manager.rs` | ✅ Phase 1 | `active_adapter_by_hash` side-table |
| `git_bridge.rs` | ✅ Phase 1 | Full `DagNode`/`DagStore`; contestability fork |
| `kml_bridge.rs` | ✅ Phase 1 | KML↔NQuin; GeoSPARQL + PROV-O quins; GeoHash-64 |
| `ontologies/rights_ontology.ttl` | ✅ Phase 1 | ODRL+SKOS; Human-Centric anchors; PIP DID |
| `ontologies/cogai_shapes.ttl` | ✅ Phase 1 | CogAI SHACL shapes + InferenceIntentShape |
| `qualia-cli/src/main.rs` | ✅ Phase 1 | `q42 migrate meta` subcommand |
| `spatial_sieve.rs` | 🔄 Phase 2 | Wire `log_spatial_coordinate` to GeoHash-64 + SPATIAL_CONTEXT quins |
| `sparql_shacl.rs` | 🔄 Phase 2 | Load `cogai_shapes.ttl` as 4th domain shape graph |
| `orchestrator.rs` | 🔄 Phase 2 | CogAI pre-flight; `cog:Agent/Goal/Belief` quin writes |
| `ingest.rs` | 🔄 Phase 2 | Wire `DagStore::commit_node()` after each SuperBlock write |
| `daemon.rs` | 🔄 Phase 2 | Load ontology TTLs at startup |
| new: `temporal_graph.rs` | 🔄 Phase 2 | `assert_temporal()`; T_CONTEXT predicate consts |
| new: `provenance.rs` | 🔄 Phase 2 | Labor DID labelling; contestability write helpers |
| `kml_bridge.rs` | 🔄 Phase 2 | `<Polygon>` + `<NetworkLink>` support |
| `sparql_filter.rs` | ✅ Phase 2 | `prov_predicates` constants; `ProvOPredicate` enum; `ProvenanceFilter` helpers; 6 tests |
| new: `epistemic.rs` | ✅ Phase 2.9 | Dynamic Epistemic Logic: per-agent mind contexts, JTB predicates, SHACL classifier |
| new: `ontologies/epistemic_shapes.ttl` | ✅ Phase 2.9 | ObjectiveKnowledgeShape, InferredBeliefShape, HearsayBeliefShape SHACL shapes |
| `crdt.rs` | ✅ Phase 2 | Verified: Lamport clock correctly uses bits[31:0]; LWW unaffected |
| `deontic_logic.rs` | ✅ Phase 3 | VC attribute evaluation for subgraph key release; ODRL policy table; `evaluate_vc_for_subgraph_key_release()` |
| `key_vault.rs` | ✅ Phase 3 | `SubgraphLayer` enum; `SubgraphKey` (AES-256-GCM, zeroize); `generate_layer_key()` (HKDF-SHA256); `encapsulate_for_recipient()` / `decapsulate()` (X25519 ECDH) |
| `wal.rs` | ✅ Phase 4 | `prev_dag_hash` header; `checkpoint_to_dag()`; `buffered_count()`; full DAG-linking test suite |
| `git_bridge.rs` | ✅ Phase 4 | `MERGE_SECONDARY` flag; `merge_node()` (two-node primary+back-link); `nodes_as_of()` snapshot filter; 8 tests |
| `sparql_ast.rs` | ✅ Phase 4 | `TemporalMode` enum (AsOf/AtTime); `Pattern::AsOf { inner, timestamp_ms, mode }` |
| `sparql_planner.rs` | ✅ Phase 4 | `PhysicalOperatorType::AsOf { input, timestamp_ms, mode }`; `plan_pattern()` case |
| `sparql_executor.rs` | ✅ Phase 4 | `execute_as_of()` + `check_temporal_constraint()` (T_CONTEXT PROV-O lookup; open-world default) |
| `sparql_parser.rs` | ✅ Phase 4 | `AS OF`/`AT TIME` parsing after WHERE `}`; `parse_temporal_literal()` (int ms + ISO 8601); 5 new tests |
| `ingest.rs` | ⏳ Phase 4 remaining | Wire `DagStore::commit_node()` after each SuperBlock flush; update `Q42VolumeHeader::merkle_root` |

---

## 7. Terminology Notes

The following terminology preferences should be used consistently throughout the codebase,
documentation, and the Rights Ontology:

| Avoid | Use instead | Reason |
|-------|------------|--------|
| "sovereign" / "sovereignty" | "agency", "Human-Centric control", "data agency" | Owner's explicit preference; "sovereign" has been misappropriated by extractive systems |
| "DID = identity" | "DID = persistent pointer / URI" | DIDs are durable identifiers; the semantic payload and agency live in the Q42 graph, not the DID itself |
| "permissions" (as in ACL) | "access policy" / "deontic rules" | Permissions implies centralized grant; the Q42 model is rule-based and embedded in the data |
| "data owner" | "rights orchestrator" | The entity who sets up access rules, not a property owner in the proprietary sense |
| "anonymized data" | "pseudonymous data with DID binding" | True anonymization is not achievable when DIDs are present; be precise |

---

## 8. Decisions Checklist

### ✅ Resolved (8/10)

| # | Decision | Outcome |
|---|---|---|
| §4.1 | NQuin metadata bit-layout | Option B — LoRA → side-table; Lamport → bits[31:0]; Quin Type → bits[63:60] |
| §4.2 | Bi-temporal model | PROV-O + Dublin Core quins in T_CONTEXT overlay graph |
| §4.3 | Spatial encoding | GeoSPARQL WKT internally; GeoHash-64 in `object` field; KML import/export |
| §4.4 | Format versioning | Option B — migration required; v3 builds hard-reject v2; `q42 migrate-meta` CLI |
| §4.5 | Versioning / DAG | Option B — Minimal Merkle-DAG with `DagNode` in `git_bridge.rs` from day one; branching supports contestability |
| §4.6 | Credential-gated views | AES-256-GCM per layer + X25519 ECDH encapsulation; CP-ABE deferred |
| §4.7 | Rights Ontology format | ODRL + SKOS Turtle at `ontologies/`; signed by Peace Infrastructure Project founding DID (2020) |
| §4.8 | Labor provenance | PROV-O `wasAttributedTo` + `PROVENANCE_CONTEXT` overlay quins |
| §4.9 | FHE / ZK scope | FHE out of scope for all 0.x; ZK encrypted search (MIRACL) is priority after Phase 3 |
| §4.10 | Agent structure | W3C CogAI CG SHACL shapes as fourth domain engine in `sparql_shacl.rs` |

| §4.11 | Epistemic Layer | Dynamic Epistemic Logic via SHACL CWA; per-agent mind contexts; JTB vocabulary; `epistemic.rs` module |

✅ **All 11 decisions resolved. Phase 1 complete (`e42ed480`). Phase 2 (incl. §4.11 Epistemic Layer) in progress.**

---

## 9. Relation to Existing Work

### What is NOT duplicating existing implementations

- `temporal_ltl.rs` evaluates LTL temporal logic on trace sequences. It is a *reasoning* layer
  over quin sequences, not bi-temporal storage. Phase 2 adds the storage layer that feeds it.

- `deontic_logic.rs` enforces *who can act*. Phase 3 adds *what they can see* (key release).
  These are complementary, not duplicate.

- `zk_proofs.rs` (hash-chain approach) provides proof generation. Phase 5 ZK encrypted search
  would use it as a building block.

### What the discussion implicitly assumes exists but does not yet

- A formal `.q42` file format specification document (not just Rust code). This should be
  written as `Q42_FORMAT_SPEC.md` before Phase 2, so external implementations can target it.

- A test suite for temporal and spatial queries. The codebase now has 640+ tests (138 SPARQL, 8 git_bridge, domain/shacl/etc.). `AS OF`/`AT TIME` parser tests and `merge_node`/`nodes_as_of` tests are in place. GeoHash bi-temporal round-trip tests still pending.
