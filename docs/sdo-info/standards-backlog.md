# Standards Backlog

This backlog is intentionally strict. It is better to ship one precise,
credible draft than to spray half-stable ideas across multiple standards
bodies.

## Priority 0: internal cleanup gates

These items should be resolved before any serious external submission work.

### 1. Unify `.q42` semantics

Current repo evidence shows multiple incompatible interpretations of `.q42`:

- `docs/PROJECT_STATE.md` notes three incompatible `.q42` write formats.
- `crates/qualia-cli/src/ingest.rs` writes raw SuperBlock-oriented output.
- `crates/qualia-cli/src/compress.rs` emits compressed browser-delivery
  `.c.q42`.
- `crates/qualia-core-db/src/ingest.rs` writes compressed block-stream data.
- `crates/qualia-core-db/src/q42_reader.rs` expects compressed block reads.
- `crates/qualia-core-db/src/storage.rs` positions `SuperBlockWriter` as the
  canonical low-level writer.

Required before standardization:

- choose one canonical on-disk format
- define whether `.c.q42` is a separate transport profile or just a content
  coding
- define exact roles for `.q42.lex` and `.q42.bidx`
- freeze reader / writer expectations across CLI, daemon, browser, and docs

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

- Scope: `.q42`, `.c.q42`, `.q42.lex`, `.q42.bidx`, block layout, byte order,
  compression profile, HTTP delivery expectations.
- Why it is non-standard: custom binary container with custom sidecars and
  browser / daemon transport conventions.
- First doc to write here: `q42-format-internal-draft.md`
- Primary SDO: IETF
- Recommended format: Internet-Draft in Markdown-to-RFCXML
- Why this fit: media types, HTTP transport behavior, and binary interchange
  are a better fit for IETF than W3C.
- Exit criteria before submission:
  - canonical serialization chosen
  - content-type names proposed
  - explicit versioning and compatibility story
  - worked example with raw and compressed forms

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
- Why it is non-standard: it sits above raw data layout and below human-facing
  shell behavior, and the schema is not yet defined.
- First doc to write here: `qualia-vault-manifest.md`
- Primary SDO: internal first
- Recommended format: internal Turtle manifest spec first, with optional N3
  profile support and optional CBOR-LD projection later
- Why this fit: the repo already has historical `.qualia` usage, but the
  current shipped desktop shell is Flutter-first, so the schema should be
  stabilized before any externalization.
- Exit criteria before submission:
  - manifest schema frozen
  - relation to `.q42`, `.q42.lex`, `.q42.bidx`, and `.qchk` made explicit
  - host-launch behavior separated from data semantics
  - Flutter-first file association strategy documented

## 4. Qualia sync protocol

- Scope: peer handshake, sync request / response messages, CRDT exchange
  expectations, target-shape scoping, and transport framing for the current
  Qualia P2P path.
- Why it is non-standard: custom message types and custom graph-sync behavior
  over a Qualia-specific transport contract.
- First doc to write here: `qualia-sync-protocol.md`
- Primary SDO: IETF
- Recommended format: Internet-Draft in Markdown-to-RFCXML
- Why this fit: this is transport and interoperability behavior, not RDF
  vocabulary design.
- Exit criteria before submission:
  - message grammar frozen
  - error handling and version negotiation written down
  - transport assumptions separated from payload semantics
  - at least one interop path exists outside the current daemon

## 5. Qualia SHACL extension vocabulary

- Scope: `qualia:` SHACL extensions for deontic, epistemic, temporal,
  paraconsistent, and scientific constraints.
- Why it is non-standard: extension vocabulary and execution semantics sit
  outside baseline SHACL.
- First doc to write here: `qualia-shacl-extensions.md`
- Primary SDO: W3C
- Recommended format: Community Group Report or Group Note-style HTML draft
  with vocabulary tables, conformance classes, and examples.
- Why this fit: this is RDF / SHACL-native material and should look like a web
  data extension spec.
- Exit criteria before submission:
  - separate standard SHACL behavior from Qualia-native behavior
  - each extension term has syntax, semantics, and failure behavior
  - at least one implementation report or test manifest exists

## 6. Qualia qapp loopback protocol

- Scope: localhost / loopback asset serving and host-embedded qapp access
  boundary, including URL model, lifecycle, and trust assumptions.
- Why it is non-standard: it is currently a product-specific app hosting
  boundary rather than a general web standard.
- First doc to write here: `qualia-qapp-loopback-protocol.md`
- Primary SDO: internal first
- Recommended format: internal explainer or ADR first
- Why this fit: the design is still tightly coupled to current desktop /
  Flutter host behavior and is not ready for external submission.
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

## Suggested file backlog for this folder

- [x] `q42-format-internal-draft.md`
- [x] `did-q42-method-draft.md`
- [x] `qualia-vault-manifest.md`
- [x] `qualia-sync-protocol.md`
- [ ] `qualia-shacl-extensions.md`
- [ ] `qualia-qapp-loopback-protocol.md`
- [ ] `solid-webizen-bridge-profile.md`
- [ ] `qchk-capability-profile.md`
- [ ] `mcp-fiduciary-mediation-explainer.md`
- [ ] `webizen-protocol-split.md`
- [ ] `webizen-execution-model.md`

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

The initial `q42` internal draft should reflect the implementation as it
exists, while explicitly naming contradictions that must be resolved:

- raw SuperBlock `.q42` vs framed compressed `.q42`
- `.c.q42` as transport artifact vs `.q42` as on-disk artifact
- object-hash BIDX implementation vs subject-hash wording in some docs
- `q42` / `qla` naming drift in older storage comments
