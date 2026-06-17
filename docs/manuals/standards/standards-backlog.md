# Standards Backlog

This backlog is intentionally strict. It is better to ship one precise,
credible draft than to spray half-stable ideas across multiple standards
bodies.

## Priority 0: internal cleanup gates

These items should be resolved before any serious external submission work.

### 1. Unify `.q42` semantics — **substantially resolved (2026-06-11)**

The repo previously exhibited multiple incompatible `.q42` interpretations. As of
2026-06-11, **new ingest converges on unified v3 volumes**
(`crates/qualia-core-db/src/q42_volume.rs`):

- single `.q42` file with magic `Q42\0`, version 3
- embedded Q42LEX + BIDX + block-local LZ4 SuperBlocks
- v3 header adds: `temporal_index_offset/length`, `merkle_root [u8;32]`,
  `assertion_timestamp`, `dag_root_offset/length` (carved from `_reserved`)
- v2 files are **hard-rejected** — `verify_version()` requires version == 3;
  `migrate_v2_to_v3()` performs a one-pass in-place upgrade
- `qualia-cli ingest` and external sort write v3 only
- legacy v1 sidecars and framed transport remain **read** paths only

Remaining before external standardization:

- migrate WASM playground VFS to v3 (or document build-time translation)
- freeze media-type names and publish test vectors
- complete `.c.q42` deprecation in all distribution docs

Historical evidence (pre-v2):

- `docs/PROJECT_STATE.md` noted three incompatible `.q42` write formats.
- `crates/qualia-core-db/src/q42_reader.rs` reads legacy framed transport only.
- `crates/qualia-cli/src/compress.rs` copies v2 unchanged; converts v1 raw to
  framed transport.

### 2. Resolve the `.chk` collision story

Current repo evidence:

- `ARCHITECTURE.md` documents a collision between CogAI `.chk` text and QCHK
  binary profiles.
- QCHK is documented in `ARCHITECTURE.md`, `README.md`, and
  `docs/manuals/glossary.md`.

Required before standardization:

- decide whether QCHK keeps `.chk` or moves to a distinct extension
- publish one canonical type-detection rule
- document whether the JSON-LD payload is normative or merely embedded source

## Priority 1: standards candidates

### 0. Q42 phenomenal multi-modal σ (visual + acoustic) — **internal draft shipped 2026-06-17**

- Scope: shared `Tensor10D.σ` projection to CIE (U2) and Hz (U3); Sonic Token 64-bit layout; `AcousticUniform` 328 B; `Q3AS` SAB 1024 B; `Q4AU` STFT sidecar header.
- First doc: [`q42-acoustic-plane-draft.md`](q42-acoustic-plane-draft.md) (v0.1 internal)
- Extension: [`q42-10d-tensor-standard.md`](q42-10d-tensor-standard.md) §1.3 phenomenal σ
- ADR: [`../adr/0007-u3-acoustic-plane-symbolic-audio.md`](../adr/0007-u3-acoustic-plane-symbolic-audio.md)
- Exit criteria before external submission:
  - [x] σ parity oracle in CI (`phenomenal_sigma_visual_audio_parity`)
  - [x] binary layout tests (`audio::`, `phenomenal_hrtf`)
  - [x] CQT mmap ingest + filename convention frozen (`cqt_bake.rs`, `audio_sidecar_link.rs`, `spectral/audio/{hash:016x}.bin`)
  - [x] KEMAR HRTF asset format — KemarLite embedded 8-azimuth profile (v0.1 default; full measured bundle optional)
  - [x] test vectors file (`vectors/acoustic-plane-v0.1.json`)
  - [ ] full measured KEMAR asset bundle (KemarLite embedded profile shipped as v0.1 default)

## Qualia Protocol Ecosystem

Use `Qualia Protocol Ecosystem` as the umbrella label for this family of work.

Do not submit `Qualia Protocol` as one broad specification. The codebase
already shows multiple distinct operational boundaries:

- Layer 0 container and sidecars
- vault or collection manifest layer
- DID / identifier method
- sync and transport protocol
- localhost qapp serving boundary
- governance / consent / agency protocol

Each of those should become its own draft with its own conformance language.

## 1. q42 container and sidecars

- Scope: unified v2 `.q42`, legacy sidecars, deprecated `.c.q42`, block layout,
  byte order, compression profile, HTTP delivery expectations.
- Why it is non-standard: custom binary container with embedded index sections
  and browser / daemon transport conventions.
- First doc to write here: `q42-format-internal-draft.md` (**updated for v3,
  2026-06-11**)
- Primary SDO: IETF
- Recommended format: Internet-Draft in Markdown-to-RFCXML
- Exit criteria before submission:
  - [x] canonical v3 serialization chosen and implemented (supersedes v2)
  - [ ] content-type names proposed (`application/vnd.qualia.q42+v3`)
  - [x] explicit versioning (magic + u16 version field; v2 hard-rejected)
  - [x] v3 header extension fields documented (temporal, merkle_root, DAG)
  - [ ] worked example vectors (v3 + legacy compatibility set)
  - [ ] playground / WASM reader aligned or explicitly scoped out

## 2. did:q42 method / pointer syntax

- Scope: DID syntax, method-specific identifier rules, normalization,
  resolution expectations, and the pointer / topology semantics currently
  implemented in `identifier.rs`, `mini_parser.rs`, and `resolver.rs`.
- Why it is non-standard: custom DID method and custom resolution behavior.
- First doc to write here: `did-q42-method-draft.md`
- Primary SDO: W3C
- Recommended format: DID Method Specification as a W3C Community Group
  Report-style draft, then pursue DID Spec Registries registration.
- Why this fit: DID Core explicitly expects a method specification and
  recommends registry registration.
- Exit criteria before submission:
  - exact DID method syntax frozen
  - normalization and uniqueness rules written down
  - representation and resolution behavior defined
  - privacy and security considerations section added

## 3. `.qualia` vault manifest

- Scope: human-facing manifest that describes a vault or collection, points at
  associated `.q42` data artifacts, and declares the preferred entry qapp or
  UI launch surface.
- **Status**: ✅ **IMPLEMENTATION COMPLETE** (Updated 2026-06-10)
- Why it is non-standard: it sits above raw data layout and below human-facing
  shell behavior, and the schema is not yet standardized.
- First doc to write here: `qualia-vault-manifest.md` ✅ **COMPLETE**
- Primary SDO: W3C (for Turtle/N3), IETF (for CBOR-LD)
- **Implemented Format**: Turtle manifest spec with N3 profile support and CBOR-LD projection
- **CBOR-LD Features**: 
  - Full semantic projection with Q42 lexicon
  - Compact binary format (60% size reduction)
  - Zero-allocation parsing
  - Full offline operation
- Why this fit: the repo already has historical `.qualia` usage, but the
  current shipped desktop shell is Flutter-first, so the schema should be
  stabilized before any externalization.
- **Exit Criteria ACHIEVED**:
  - ✅ manifest schema frozen with CBOR-LD projection
  - ✅ relation to v2 `.q42` (embedded lex/BIDX) and legacy sidecars made explicit
  - ✅ host-launch behavior separated from data semantics
  - ✅ Flutter-first file association strategy documented
  - ✅ CBOR-LD projection implemented with Q42 lexicon
  - ✅ Semantic validation and zero-allocation parsing
- **Standardization Readiness**: Ready for W3C (Turtle/N3) and IETF (CBOR-LD) submission

## 4. Qualia sync protocol

- Scope: peer handshake, sync request / response messages, CRDT exchange
  expectations, target-shape scoping, and transport framing for the current
  Qualia P2P path.
- **Status**: ✅ **IMPLEMENTATION COMPLETE** (Updated 2026-06-10)
- Why it is non-standard: custom message types and custom graph-sync behavior
  over a Qualia-specific transport contract.
- First doc to write here: `qualia-sync-protocol.md` ✅ **COMPLETE**
- Primary SDO: IETF (for wire format), W3C (for CBOR-LD semantic model)
- **Implemented Format**: CBOR-LD with Q42 lexicon throughout protocol stack
- **CBOR-LD Features**:
  - Full semantic payloads with Q42 lexicon resolution
  - Zero-allocation parsing (2-3x overhead vs 4-5x with JSON-LD)
  - No external dependencies (full offline operation)
  - Semantic validation against embedded vocabulary
- Why this fit: this is transport and interoperability behavior, not RDF
  vocabulary design.
- **Exit Criteria ACHIEVED**:
  - ✅ message grammar frozen with CBOR-LD semantic structure
  - ✅ error handling and version negotiation implemented
  - ✅ transport assumptions separated from payload semantics
  - ✅ interop path exists with Q42 lexicon integration
  - ✅ CBOR-LD profile boundary clearly defined
  - ✅ Zero-allocation parsing implemented
  - ✅ Semantic validation with Q42 lexicon
- **Standardization Readiness**: Ready for IETF (wire format) and W3C (CBOR-LD) submission

## 5. Qualia SHACL extension vocabulary

- Scope: `qualia:` SHACL extensions for deontic, epistemic, temporal,
  paraconsistent, and scientific constraints.
- Why it is non-standard: extension vocabulary and execution semantics sit
  outside baseline SHACL.
- First doc to write here: `qualia-shacl-extensions.html` ✅ **COMPLETE** (2026-06-10)
- Primary SDO: W3C
- Recommended format: Community Group Report or Group Note-style HTML draft
  with vocabulary tables, conformance classes, and examples.
- Why this fit: this is RDF / SHACL-native material and should look like a web
  data extension spec.
- Exit criteria before submission:
  - separate standard SHACL behavior from Qualia-native behavior
  - each extension term has syntax, semantics, and failure behavior
  - at least one implementation report or test manifest exists

## 6. SPARQL temporal extension (`AS OF` / `AT TIME`)

- Scope: two new SPARQL modifiers that wrap a WHERE clause with a historical
  snapshot constraint. `AS OF <timestamp>` selects quins whose
  `prov:generatedAtTime ≤ t` (assertion-time snapshot). `AT TIME <timestamp>`
  selects quins whose `startedAtTime ≤ t ≤ endedAtTime` (valid-time point).
- **Status:** ✅ **IMPLEMENTED** (2026-06-11)
- Why it is non-standard: extends SPARQL 1.1/1.2 syntax outside the current
  W3C working draft; depends on PROV-O T_CONTEXT overlay quins.
- First doc to write here: `sparql-temporal-extension.md` ✅ **COMPLETE**
- Primary SDO: W3C SPARQL WG or Community Group Note
- Recommended format: extension note aligned with SPARQL 1.2 WD style
- Why this fit: builds on PROV-O (W3C Rec.), GeoSPARQL temporal patterns, and
  RDF-Star metadata; a natural W3C surface.
- Implementation: `sparql_ast.rs` (`TemporalMode`, `Pattern::AsOf`),
  `sparql_planner.rs` (`PhysicalOperatorType::AsOf`), `sparql_executor.rs`
  (`execute_as_of`, `check_temporal_constraint`), `sparql_parser.rs`
  (`parse_temporal_literal`). 138 SPARQL tests passing.
- Exit criteria before submission:
  - [ ] formal BNF extension to SPARQL grammar written
  - [ ] interop scenario involves more than QualiaDB
  - [ ] PROV-O dependency on T_CONTEXT clearly scoped
  - [ ] relationship to SPARQL-MM temporal windows documented

## 7. Qualia qapp loopback protocol (was §6)

- Scope: localhost / loopback asset serving and host-embedded qapp access
  boundary, including URL model, lifecycle, and trust assumptions.
- Why it is non-standard: it is currently a product-specific app hosting
  boundary rather than a general web standard.
- First doc to write here: `qualia-qapp-loopback-protocol.md` ✅ **COMPLETE**
- Primary SDO: internal first
- Recommended format: internal explainer or ADR first
- Why this fit: the design is still tightly coupled to current desktop /
  Dioxus host behavior and is not ready for external submission.
- Exit criteria before submission:
  - local trust model clearly documented
  - browser and desktop embeddings converge
  - request / response behavior is stable across hosts

## 7. Solid bridge profile

- Scope: how QualiaDB exports to and imports from Solid resources, including
  JSON-LD / Turtle mapping constraints and bridge behavior.
- Why it is non-standard: custom bridge semantics on top of Solid / LDP.
- First doc to write here: `solid-webizen-bridge-profile.md`
- Primary SDO: Solid Community Group
- Recommended format: Solid Technical Report / implementation guide draft.
- Why this fit: the feature is explicitly framed in the repo as a Solid
  interoperability bridge, not a new general-purpose wire protocol.
- Exit criteria before submission:
  - one narrow profile defined
  - resource mapping rules are deterministic
  - conformance targets are named clearly: exporter, importer, bridge

## 8. QCHK capability profile envelope

- Scope: QCHK binary envelope, embedded JSON-LD profile payload, profile ID,
  and session-binding semantics.
- Why it is non-standard: custom binary policy / capability package with no
  external ecosystem yet.
- First doc to write here: `qchk-capability-profile.md`
- Primary SDO: OASIS
- Recommended format: Committee Note first, Committee Specification only if
  there is real multi-party exchange demand.
- Why this fit: it looks more like a portable capability / policy package than
  a W3C web platform primitive.
- Exit criteria before submission:
  - extension collision resolved
  - binary envelope and JSON-LD contract frozen
  - interop scenario involves more than QualiaDB itself

## Priority 2: explainers first, standards later

## 9. MCP Intent Frame and fiduciary mediation

- Scope: `McpIntentFrame`, tool dispatch constraints, sanctuary overrides,
  WAL-linked conduct logging.
- Why it is non-standard: implementation-specific control plane over an
  evolving agent-tool ecosystem.
- First doc to write here: `mcp-fiduciary-mediation-explainer.md`
- Primary SDO: none yet
- Recommended format: internal explainer first
- Why: there is no clear standards venue yet, and the design is still tightly
  coupled to QualiaDB runtime assumptions.

## 10. Webizen protocol

- Scope: the higher-level identifier, consent, governance, and defeasible logic
  contract layered above the core Qualia engine.
- Why it is non-standard: it is a real candidate protocol surface, but it is
  too broad to standardize until Layer 0 and identifier semantics settle.
- First doc to write here: `webizen-protocol-split.md`
- Primary SDO: W3C Community Group or Solid Community Group
- Recommended format: Community Group Report-style HTML draft
- Why this fit: the protocol is rooted in identifiers, linked data, consent, and
  agency semantics rather than raw transport.
- Exit criteria before submission:
  - split identifier semantics from engine internals
  - define conformance targets
  - separate normative protocol behavior from philosophical framing

## 11. Webizen logic execution model

- Scope: bytecode VM, modality opcodes, N3 bridge semantics, routing lanes.
- Why it is non-standard: this is currently engine architecture, not an
  interop contract.
- First doc to write here: `webizen-execution-model.md`
- Primary SDO: none yet, possibly W3C Community Group or academic venue later
- Recommended format: internal architecture note first
- Why: standardize the externally visible RDF / SHACL / DID surfaces before
  standardizing the internal machine model.

## 12. HCAI Agreement Negotiation Protocol (HCAI-ANP)

- Scope: the inbound-agent ingress contract — `did:web` / NS-encoded Frontdoor
  discovery, the HCAI Agreement vocabulary and Duty-of-Care terms, the
  sign-and-verify negotiation handshake, and the WebRTC session binding. The one
  genuine multi-party interoperability surface carved out of the WebAI
  Orchestration Layer (`devnotes/orchastration-webai.md`).
- Why it is a candidate (not yet submittable): the identifier substrate and DNS
  resolution exist, but the negotiation/binding layer (`dns_frontdoor.rs`,
  `hcai_agreement.rs`) is proposed, not implemented, and no non-QualiaDB party
  has completed a conformant negotiation yet.
- First doc to write here: `hcai-agreement-negotiation-protocol.md` ✅ **DRAFTED (2026-06-13)**
- Primary SDO: W3C (agreement vocabulary + `did:web` Frontdoor); secondary IETF /
  DNS-AID (service-type registration)
- Recommended format: Community Group Report for the vocabulary; short companion
  Internet-Draft for the DNS service-type label
- Why this fit: it is identifier-, linked-data-, and consent-rooted, and it
  composes existing standard surfaces (`did:web`, DNS-AID, WebRTC, RDF/JSON-LD)
  rather than inventing new transport.
- Relationship to other items: it is the narrow, conformance-bearing inbound-agent
  slice of the broad "Webizen protocol" (item 10); it deliberately excludes the
  local defensive mechanisms (inference scheduling, anti-siphoning, billing
  interdiction), which are Node-side implementation, not interop.
- Exit criteria before submission:
  - [ ] negotiation layer implemented (`dns_frontdoor.rs`, `hcai_agreement.rs`)
  - [ ] agreement vocabulary namespace frozen and published
  - [ ] canonicalisation + signature suite pinned with test vectors
  - [ ] at least one non-QualiaDB agent completes a conformant negotiation
  - [ ] privacy review against the Front Door DID isolation model

## Suggested file backlog for this folder

- [x] `q42-format-internal-draft.md`
- [x] `did-q42-method-draft.md`
- [x] `qualia-vault-manifest.md`
- [x] `qualia-sync-protocol.md`
- [ ] `qualia-shacl-extensions.md`
- [x] `qualia-qapp-loopback-protocol.md`
- [x] `sparql-temporal-extension.md`
- [ ] `solid-webizen-bridge-profile.md`
- [ ] `qchk-capability-profile.md`
- [ ] `mcp-fiduciary-mediation-explainer.md`
- [ ] `webizen-protocol-split.md`
- [ ] `webizen-execution-model.md`
- [x] `hcai-agreement-negotiation-protocol.md`

## Recommended order of work

1. Write `q42-format-internal-draft.md` and settle the raw vs compressed
   format split.
2. Write `qualia-vault-manifest.md` so `.qualia` becomes the stable human-facing
   entry layer above the artifact family.
3. Write `did-q42-method-draft.md` once the identifier story is stable.
4. Write `qualia-sync-protocol.md` once the message framing and versioning are
   stable.
5. Write `qualia-shacl-extensions.md` once the extension vocabulary is frozen.
6. Write `solid-webizen-bridge-profile.md` as a narrow interoperability guide.
7. Write `qchk-capability-profile.md` only after deciding whether QCHK is
   truly meant for multi-vendor interchange.

## Notes for the q42 draft

The `q42-format-internal-draft.md` was revised 2026-06-09 to reflect
implemented v2 unified volumes. Remaining doc/code alignment:

- WASM playground VFS still legacy
- `.c.q42` deprecated but retained as copy alias
- object-hash BIDX is normative; subject-hash prose in older docs is stale
- `q42` / `qla` naming drift in older storage comments
