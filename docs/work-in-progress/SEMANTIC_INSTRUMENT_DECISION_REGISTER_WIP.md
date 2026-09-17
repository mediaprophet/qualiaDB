# Semantic instruments — SI-00 decision register

**Status:** work in progress · **Packets:** SI-00 (this register) + SI-01 (ontology/shapes landed in the same turn)  
**Date:** 2026-09-15 · **Branch:** `0.0.38` · **Not a normative standard** · **Not a runtime-complete claim**  
**Overview:** [`SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md`](SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md)  
**Draft specification:** [`SEMANTIC_INSTRUMENT_PACKAGE_SPEC_WIP.md`](SEMANTIC_INSTRUMENT_PACKAGE_SPEC_WIP.md)  
**Implementation plan:** [`SEMANTIC_INSTRUMENT_IMPLEMENTATION_PLAN_WIP.md`](SEMANTIC_INSTRUMENT_IMPLEMENTATION_PLAN_WIP.md)  
**Prior scope:** [`EXPERTISE_EVAL_PACK_SCOPE_WIP.md`](EXPERTISE_EVAL_PACK_SCOPE_WIP.md)

This register inventories existing package-like, credential-like, provenance and UI models,
records what SI-01 reuses, and marks decisions that must **not** be baked into an ABI without
owner review. Provisional choices are labelled `PROVISIONAL`. Owner-review items are labelled
`HELD`.

---

## 0. Packet record

| Field | SI-00 | SI-01 |
|---|---|---|
| Objective | Baseline inventory + decision log | Core ontology, SHACL, fixtures, focused tests |
| Dependencies | none | SI-00 |
| Owned paths | this file; WIP index link | `core-ontologies/semantic-instrument*.n3`; `core-ontologies/fixtures/semantic-instruments/`; `crates/qualia-core-db/tests/semantic_instruments.rs`; `core-ontologies/DIRECTORY_INDEX.md` listing |
| Out of scope | runtime, resolver, Poet/Webizen UI, Host IDs | same; no replacement of credentials, law packages, qapp manifests, or `capability_gap` |
| Acceptance | inventory with file/test evidence; reserved decisions recorded | positive fixtures pass; each critical negative fixture fails for the intended reason; existing core-ontology gate remains green |

---

## 1. Method

Inspected in this checkout on 2026-09-15. Citations are module paths and named tests, not
marketing names. Adjacent existence is not suitability. SI-01 does **not** replace any of the
inventoried runtimes.

---

## 2. Inventory with file and test evidence

### 2.1 Q42 package / volume storage — **reuse for SI-03 carrier (PROVISIONAL adapter)**

| Surface | Path | What it actually is |
|---|---|---|
| Volume file | `crates/qualia-core-db/src/q42/q42_volume.rs` | Native Q42 container (mmap, superblock, graph blocks). |
| Logical volume manifest | `crates/qualia-core-db/src/q42/volume/manifest.rs` | `Q42VolumeManifest` — generation, segment locators, SHA-256, shared lexicon segments. Magic `Q42VOL\0\0`, version 2, 4 MiB cap. |
| Validate / verify | `q42/volume/validate.rs`, `q42/volume/verify.rs` | Integrity and set verification. Test: `volume_set_full_passes_with_shared_lexicon`. |
| Compat / lexicon on root | `q42/volume/compat.rs` | Test: `volume_root_can_carry_the_shared_lexicon`. |
| Asset envelope | `q42/asset_envelope/envelope.rs` | Versioned governed-asset envelope (AST-01): sensitivity, routing lane, 42 MiB Sentinel chunk plan, derived-from, licence policy. |

**Reuse:** SI-03 should treat Q42 volume + asset envelope as the **preferred physical carrier** for an `si:InstrumentPackage`. Do not invent a second graph store.

**HELD:** whether the canonical carrier is Q42-only, a deterministic archive, or both behind one logical package model (plan §11 item 2).

### 2.2 Ontology and CML logic routing — **reuse**

| Surface | Path | Evidence |
|---|---|---|
| CML vocabulary | `core-ontologies/cml.n3` | `cml:LogicApplication`, `cml:Modality` (`N3Logic`, `SHACL`, deontic, epistemic, LTL, …), `cml:executionSurface`, `cml:recognitionBasis`. Shapes: `cml:LogicApplicationShape`. |
| Values / agency | `core-ontologies/values.n3`, `agency.n3` | `values:Agent`, `NaturalPerson`, `LegalPerson`, `ArtificialAgent`, `actsFor`, `principal`, `hasNoPrincipal`. Living-safe: a person is **not** `owl:Thing`. |
| Accountability | `core-ontologies/agent-accountability.n3` | Signature/assertion ≠ substantiation. Test lane: `crates/qualia-core-db/tests/agent_honesty_guard.rs` (`overclaimed_completion_is_flagged_and_routed_to_review`). |
| N3 parser | `crates/qualia-core-db/src/modalities/logic/n3_parser.rs` | Cold parse of triples and `{ } => { }` rules. Tests: `tests/n3_logic_tests.rs`, `tests/deontic_smoke.rs`. |
| SHACL data validator | `modalities/logic/shacl/validate.rs` | Graph-level SHACL over `NQuin`s (cold, `Vec`-backed). Complementary to `query/shacl_compiler.rs` VM lowering. |
| Core-ontology CI gate | `crates/qualia-core-db/tests/validate_core_ontologies.rs` | **Values-credential corpus only** (`un-instruments/`, `regional/`, `mutable/`). Requires `values:ValuesCredential`. **Not** the seam for `si:` files. |
| Index builder | `core-ontologies/tools/build_index.py` | Same corpus dirs; rdflib Turtle. Exits 0 (report). |

**Reuse:** SI-01 subclasses/references `cml:LogicApplication` on `si:EntryPoint`. Framing remains living-SHACL for people; artifact-OWL only for pack/catalog machinery.

### 2.3 Capability / RPL and gap analysis — **reuse, do not replace**

| Surface | Path | Evidence |
|---|---|---|
| Capability ontology | `core-ontologies/capability-credentials.n3` | `cap:Capability` ⊑ `skos:Concept`; `cap:LearningClaim`; `cap:CapabilityRequirement`; `cap:Gap`; `cap:LearningClaimShape` (basis + capability + heldBy). |
| Gap kernel | `crates/qualia-core-db/src/modalities/capability_gap.rs` | Zero-heap `capability_gap`, `requirements_met`, `learning_path_cost`, `estimate_capability`. Tests: `gap_is_the_set_difference`, `experiential_equivalence_closes_the_gap`, `a_star_finds_the_shortest_learning_path`. |
| Juridical capacity | `crates/qualia-core-db/src/modalities/capacity.rs` | `CapacityStatus::{Intact,Impaired,UnderDuress}` — **legal/cognitive capacity**, not skill and not host permission. |

**Reuse:** SI-11 must call `capability_gap` / `learning_path_cost`. SI-01 adds `si:GapReport` and `si:CapabilityCredential` as **profile types** over `cap:*`, not a parallel gap engine.

**Conflict:** the word *capability* is already used for host/qapp permissions (see §4).

### 2.4 W3C and native credentials — **reuse as envelopes; do not consolidate in SI-01**

| Surface | Path | Evidence |
|---|---|---|
| W3C VC + ML-DSA-65 | `crates/qualia-core-db/src/identity/credentials/mod.rs` | JSON-LD `Credential` (`@context`, `credentialSubject`, `Proof`). `VcRuntime::issue` / `verify_credential`. Tests: `issue_verify_roundtrip`, `tampered_credential_fails_closed`, `unsigned_credential_fails_closed`. |
| Native quin VC | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` | Distinct `Credential` (issuer/subject `u64`, claim `NQuin`s, Ed25519, `q42-vc-v1` digest). Comment: verification authenticates **origin, not truth**. Tests: `issue_and_verify_roundtrip`, `tampered_claim_fails_verification`. Ungrounded artificial issuer rejected (`verify_grounded`). |
| WellFair records | `crates/wellfare-core/src/credentials.rs` | `CredentialRecord` — holder/anatomy path, not the engine VC runtime. |

**HELD:** consolidation path for the two engine `Credential` types (plan §11 item 4). SI-04 may **adapt** both; it must not silently merge them.

### 2.5 Open Badges transport — **reuse codec; incomplete profile**

| Surface | Path | Evidence |
|---|---|---|
| Open Badges v3 codec | `identity/credentials/codecs.rs` | `OpenBadgeCodec` adds context `https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json` and type `OpenBadgeCredential`, then JSON-encodes. Tests in the same file (`OpenBadgeCodec` encode/decode). |
| PDF carrier | same file | `PdfCodec` embeds credential JSON as a `/Metadata` stream. |

**Gap:** the codec does **not** yet model nested achievement/outcome/evidence or bind an accessible image to a package digest. That is SI-04 work. Version/profile conformance strategy is **HELD** (plan §11 item 5).

### 2.6 Icons and presentation metadata — **reuse patterns, not types**

| Surface | Path | Evidence |
|---|---|---|
| Q42 portable app icons/permissions | `q42/app_manifest/{mod,manifest,permissions}.rs` | `PortableAppManifest`: identity, entry projections, required capabilities, permission intents, assets. Tests: `round_trip_preserves_manifest`, `deterministic_hash_is_stable`. `PermissionKind` is **runtime access** (`NetworkEgress`, `CameraCapture`, …). Presentation hints must not grant authority (`authority_from_presentation_hints`). |
| Cooperative qapp | `crates/qualia-cooperative-core/src/qapp_package/manifest.rs` | Distinct `QappManifest` + `IconRef` (`src`, `sizes`, `purpose`) + `Capability` enum (`ReadRecords`, `Camera`, …). Tests: construction / JSON round-trip in the same file. |
| Poet icon session | `crates/poet/src/browser/icon_registry.rs`, `icon_session.rs` | UI glyph/session icons — not package badge assets. |
| Lexicon catalog chips | `crates/webizen-studio/src/lexicon_catalog.rs` | Held-gate copy; not instrument badges. |

**Reuse:** digest + media type + accessible text + purpose, following qapp `IconRef` / app-manifest assets. **Do not** type a badge as a qapp.

### 2.7 Law and other signed packages — **reuse as a dependency kind, not the instrument type**

| Surface | Path | Evidence |
|---|---|---|
| QualiaDB law package | `crates/qualia-core-db/src/governance/law_packages.rs` | `LawPackage` { `law_id`, `name`, `author_did`, `licence`, `nature`, `content_hash`, Ed25519 `signature` }. Tests: `law_package_construction`, `law_nature_physical_vs_fictional`, plus sign/verify tests in the same module. |
| Vibe law package | `crates/vibe/src/law_package.rs` | Distinct `LawPackage` wrapping a `LawDecl` (condition/consequence text, `LawKind`, optional signature). Same T72 intent, different ABI. |

**Reuse:** a law package may be an `si:DependencyRequirement` of kind `law-package`. An instrument is not a `LawPackage`.

**HELD / conflict:** two `LawPackage` types. Do not invent a third. SI-03 documents an adapter if a law package is embedded.

### 2.8 Execution receipts and provenance — **reuse PROV-O + labour provenance; do not reuse the LLM receipt type as the instrument receipt**

| Surface | Path | Evidence |
|---|---|---|
| PROV write helpers | `governance/provenance.rs` | Named graphs `PROVENANCE_CONTEXT` / `CONTEST_CONTEXT`; `prov:wasAttributedTo`, `wasGeneratedBy`, contestability. |
| 10D provenance section | `container_10d/provenance_section.rs` | `validate_provenance`. |
| LLM execution receipt | `inference/runtime/receipt/execution.rs` | `ExecutionReceipt` — backend, model instance, decode counters, artifact cleanup. **LLM decode evidence**, not instrument employment. Test: serde round-trip in the same file. |
| Q42 verification receipt | `q42/q42_volume.rs` (`Q42VerificationReceipt`) | Volume integrity receipt. |
| Agent honesty | `tests/agent_honesty_guard.rs` | Completion claims require verification provenance. |

**Reuse:** PROV-O predicates and the origin-not-truth rule. SI-06 introduces `si:ExecutionReceipt` as a **new class**. It must not be the LLM `ExecutionReceipt` struct.

### 2.9 Poet authoring — **later SI-09; existing workbench is the landing surface**

| Surface | Path | Notes |
|---|---|---|
| Ontology workbench | `crates/poet/src/browser/ontology_views/` | N3 editor, SHACL shapes, ShEx, library, personhood (living-safe comment in `mod.rs`). |
| Logic workbench | `poet/src/browser/logic_workbench/` | Modality authoring; tests in `logic_workbench/tests.rs`. |
| Publication panel | `poet/src/browser/publication_panel.rs` | Existing publish chrome — not instrument-package publish. |
| Health calculators | `poet/src/browser/health_views/calculators/` | Hard-coded clinical scores; first uplift candidates for SI-08 (`upliftFrom`). |
| Instrument panel | `poet/src/browser/instrument_panel/` | **Name collision:** Poet chrome “instrument”, not `si:SemanticInstrument`. |
| Lexicon bay | `poet/src/browser/lexicon_bay/` | `vibe:LexiconPack` held-gate. |
| SHACL actions | `poet/src/browser/shapes_actions.rs` | `SHACL.validate` toolchest bind. |

No Poet UI in this turn.

### 2.10 Webizen Desktop / WASM collection and execution — **later SI-10**

| Surface | Path | Notes |
|---|---|---|
| Desktop commands | `crates/webizen-desktop/src/commands/` | Vibe host, poet harness; no semantic-instrument command dir yet (plan proposes `commands/semantic_instruments/`). |
| Studio library | `crates/webizen-studio/src/components/wellfair/semantic_library/` | Import/provenance UI copy; not instrument packages. |
| Lexicon catalog | `webizen-studio/src/lexicon_catalog.rs` | Collect/inspect pattern for packs. |
| WASM ontology MCP | `crates/webizen-lite-wasm/src/lib.rs` | `wasm-ontology` profile: parse N3, query quins, validate SHACL, modal eval. Native Q42 v3 volumes are **not** loadable here (`session.rs` held message). |

WASM degradation: inspect ontology/shapes; full Q42 package bytes are a Desktop/native path unless SI-03 defines a WASM-safe encoding. **Recorded, not decided.**

### 2.11 Lexicon pack — **dependency kind, not the instrument**

| Surface | Path | Evidence |
|---|---|---|
| Shape (standards) | `docs/manuals/standards/lexicon-pack-shape-G-LEXICON-0.md` | `vibe:LexiconPack`: `packId`, `packSemVer`, `framing`, `conceptIds`. Artifact-OWL ok. |
| Bind | `GraphDatabase.lexicon_manifest` | Studio: `lexicon_catalog.rs` `INVOKE_ID`. |
| Q42 lexicon | `q42/q42_lexicon.rs`, volume lexicon segments | Content-addressed dictionary, not logic. |

A lexicon pack is an `si:DependencyRequirement` of kind `lexicon-pack`.

### 2.12 Expertise eval pack (predecessor draft)

`vibe:ExpertiseEvalPack` in [`EXPERTISE_EVAL_PACK_SCOPE_WIP.md`](EXPERTISE_EVAL_PACK_SCOPE_WIP.md) is the prior name for a pack-first evaluation. SI-01 treats `si:SemanticInstrument` as the successor **concept**. No `vibe:ExpertiseEvalPack` class is minted in SI-01 (avoids a second live IRI). Mapping is `PROVISIONAL` (§6).

### 2.13 WellFair assessments / authority attestations

Poet `health_views/authority_attestations.rs` plus WellFair credential records. These are **sector surfaces** and attestation UIs. SI-08 may uplift a labelled calculator; SI-01 does not migrate them.

---

## 3. Reuse map (what SI-01/later packets should call)

| Concern | Reuse | Do not |
|---|---|---|
| Q42 package/volume storage | `q42/volume/*`, `q42_volume.rs`, `asset_envelope` | New store or parallel CID scheme in SI-01 |
| Ontology / CML routing | `cml:LogicApplication`, N3 parser, SHACL validator | New modality opcodes; per-score Host IDs |
| Capability / RPL / gap | `cap:*`, `capability_gap.rs` | Replace `cap:Gap` or the zero-heap kernel |
| W3C / native credentials | both existing `Credential` runtimes as envelopes | Merge ABIs in this turn |
| Open Badges | `OpenBadgeCodec` | Treat OB as the instrument |
| Icons / presentation | qapp/app-manifest asset fields as a pattern | Collapse badge into credential or qapp |
| Law / signed packages | existing `LawPackage` types as dependencies | Third `LawPackage` |
| Receipts / provenance | `governance/provenance.rs`, PROV-O | Reuse LLM `ExecutionReceipt` as `si:ExecutionReceipt` |
| Poet authoring | ontology_views + logic_workbench (SI-09) | New Host methods |
| Webizen collect/run | studio library + desktop commands (SI-10) | WASM-only Q42 v3 requirement |

---

## 4. Conflicts, duplicate types, missing seams

### 4.1 Duplicate / colliding types (do not silently unify)

| Name | Instance A | Instance B | SI stance |
|---|---|---|---|
| `Credential` | `identity/credentials/mod.rs` (W3C JSON-LD, ML-DSA) | `crypto/verifiable_credential.rs` (native quin, Ed25519) | **HELD** consolidation. SI-04 adapters only. |
| `LawPackage` | `governance/law_packages.rs` | `vibe/src/law_package.rs` | **HELD**. Instrument ≠ law package. |
| `Capability` | `cap:Capability` (skill) | `qapp_package::Capability` and `PermissionKind` (runtime access) | Ontology keeps them disjoint (`si:HostCapabilityGrant`). |
| `ExecutionReceipt` | LLM decode receipt | spec `si:ExecutionReceipt` | New class; different identifier. |
| `instrument` | Poet `instrument_panel` chrome | `si:SemanticInstrument` | Different planes; do not rename Poet chrome in SI-01. |
| `Gap` | `cap:Gap` (one unmet requirement) | spec `GapReport` (the comparison document) | `si:GapReport` **contains** `cap:Gap`s. |
| Values credential vs instrument | `values:ValuesCredential` (human-rights undertaking) | `si:SemanticInstrument` | Unrelated classes. Do not put `si:` files through `validate_core_ontologies` instrument scanner. |

### 4.2 Missing seams (honest)

1. No canonical semantic-instrument package bytes, digest procedure, or lockfile (SI-03/SI-05).
2. No attestation profiles bound to a release digest (SI-04).
3. No generic runner or instrument receipt encoder (SI-06).
4. Open Badges codec does not carry nested evidence or an accessible digested icon.
5. WASM ontology profile cannot mmap native Q42 v3 volumes.
6. `vibe:ExpertiseEvalPack` was never landed as an `.n3` class.
7. Identifier-fabric VC shape (`idf:VerifiableCredentialShape`) is cited in expertise-eval scope as reuse; SI-01 does not mint a parallel envelope IRI.

### 4.3 Validation-path seam (SI-01)

The correct CI seam for `si:` files is **not** `validate_core_ontologies` (that gate would HARD-fail anything that is not a `values:ValuesCredential`).

SI-01 adds `crates/qualia-core-db/tests/semantic_instruments.rs`, which:

- parses the new ontology files with `N3Parser` (same engine path as `cml.n3` consumers);
- checks required class/predicate declarations;
- closed-world validates fixtures against the critical SHACL fields;
- leaves `validate_core_ontologies` unchanged.

Root `core-ontologies/*.n3` files (`cml.n3`, `capability-credentials.n3`, now `semantic-instrument*.n3`) are the vocabulary spine for non-UN instruments. `DIRECTORY_INDEX.md` lists them.

---

## 5. Decisions that remain owner-review (`HELD`)

Copied from the implementation plan §11, with SI-00 notes. Implementation agents must not bake these into a permanent ABI.

| ID | Decision | SI-00 note |
|---|---|---|
| D1 | Final public terminology and ontology namespace for `SemanticInstrument` | **Owner 2026-09-15:** `si:` = `https://ns.webizen.org/semantic-instrument/`. Web of Data / Webizen outputs live on `ns.webizen.org`, not `ns.webcivics.net` (that host remains `values:` / `cml:` / `cap:`). HTTPS is the frontdoor. Decentralisation path already specified: `did:webizen` multi-transport (local `.q42`, IPFS, WebTorrent, then `https://ns.webizen.org/`). See `docs/manuals/standards/did-webizen-method.md` and standards-backlog §2. A later `did:webizen:ont:semantic-instrument@<semver>` alias is compatible; not minted in SI-01 fixtures. |
| D2 | Canonical physical carrier | **Owner 2026-09-15 (revised):** the *instrument* is credential-like (a collectable with a depicting visual), **not** a dataset volume. Large Q42 volumes are associated dependencies. Collectable shape (from the hypermedia suite in `docs/manuals/standards/hypermedia-content-format-hcf.md`): `.hcf` document and/or `.hmc` archive carrying a `.10d`/`.d10` badge visual plus a **small** `.q42` semantic payload. SI-03 must not treat a lexicon/ontology volume as the instrument package. Extension naming: spec table uses `.d10`; implemented container magic is `.10d` (`10d\0`) — do not silently pick one. |
| D3 | Canonicalisation and digest algorithms; digest-derived release ids | Fixtures use labelled `sha256:` strings. No algorithm ABI in SI-01. |
| D4 | Consolidation of W3C vs native `Credential` | Native quin VC remains the in-stack attestation path. W3C JSON-LD is an optional export adapter, not a second source of truth. Types stay distinct until SI-04 writes adapters; they are not merged. |
| D5 | Open Badges version/profile and external conformance tests | **Owner 2026-09-15:** the Qualia-native instrument/capability profile is **not** bound to existing badge/VC standards as the ceiling. Those efforts described a weaker object (achievement presentation) than this stack implements (executable, attributable, content-addressed instruments). Open Badges / W3C VC remain optional **export/exchange** envelopes. External conformance tests are not a gate on the native profile. |
| D6 | Catalogue discovery protocol | Not this turn. |
| D7 | First safety-reviewed clinical/wellbeing uplift fixture | Candidates: Framingham / PHQ-9 / GAD-7 (expertise-eval + HLT-07). **HELD** for owner. |
| D8 | Organisation and community authority; multi-party publication | Ontology distinguishes org vs acting agent vs process; governance of multi-party publication **HELD**. |
| D9 | Sector-profile governance; who may publish an endorsed profile | Vocabulary allows `si:SectorProfile`; endorsement authority **HELD**. |
| D10 | Compatibility/version resolution beyond exact pins | SI-01 records `si:versionConstraint` + `si:expectedDigest`. Resolver policy is SI-05. |

Additional **HELD** from inventory:

| ID | Decision |
|---|---|
| D11 | Award/credential envelope | **Owner 2026-09-15 (with D5):** `si:CapabilityCredential` is a Qualia-native claim profile (SI-01: ⊑ `cap:LearningClaim`, subject = learner/worker, `si:assessmentInstrument` → release). Native issue/verify is the path. Open Badge / W3C VC encodings are optional exports of that claim, not the definition of it. |
| D12 | WASM-safe package encoding vs Desktop-only Q42 v3. |
| D13 | Whether Poet `instrument_panel` is renamed later to avoid the English collision. |

---

## 6. Provisional choices used by SI-01 (not ABI)

These are required so the ontology can be written. They are **not** final.

1. **Namespace** `si:` `https://ns.webizen.org/semantic-instrument/` — D1 (owner). `values:`/`cml:`/`cap:` stay on `ns.webcivics.net`.
2. **Successor of `vibe:ExpertiseEvalPack`** is `si:SemanticInstrument`. No second class minted.
3. **`si:GapReport`** is the report; `cap:Gap` remains a single unmet requirement.
4. **`si:CapabilityCredential`** ⊑ `cap:LearningClaim`; subject is the learner/worker; `si:assessmentInstrument` points at a release.
5. **`si:HostCapabilityGrant`** is execution permission; not `cap:Capability`; not `values:JuridicalCapacity`; not qapp `Capability`.
6. **`si:CapacityGrant`** is professional role / licence / mandate / delegation (time-bounded).
7. **Named SHACL PropertyShapes** (not nested blank nodes) so `N3Parser` can stream them.
8. **Fixture digest strings** are labelled test values, not a chosen canonicalisation.
9. **Core-ontology SCHEMA_VERSION** left at `3`. This is not a values-credential schema bump.
10. **Fixture IRIs are quoted strings**, not `<angle>` IRIs. The current `N3Parser` statement splitter treats `.` as statement end even inside angle-bracket IRIs (`example.org`, `1.0.0`). Fixing that splitter is out of SI-01 scope (`n3_parser.rs` is a shared hot file). SI-03 must not assume angle-IRI round-trip until that is repaired or a Turtle parser is used.

---

## 7. Separation rules SI-01 must keep testable

From the specification §3.2 and the eleven programme rules. Encoded as classes, disjointness shapes, and N3 flags:

1. Badge/icon identifies a release; artwork is not proof.
2. Author/publisher credential does not replace the package.
3. Learner capability credential references the assessment instrument; it is not the instrument.
4. Signature proves origin/integrity, not substantive truth.
5. Authorship, review, endorsement, accreditation, publication, issuance are distinct predicates.
6. Human, organisation, and software-agent roles remain distinguishable.
7. Software agent exposes principal/delegation or `values:hasNoPrincipal true`.
8. Capability ≠ professional capacity ≠ runtime permission.
9. Missing evidence is not automatically missing capability (`si:unresolvedEvidence`).
10. Living/person subjects must not collapse into artifact/tool identities.
11. Revocation/supersession stops future reliance without erasing historical authorship or receipts.

---

## 8. What SI-02 and SI-03 may now do

**SI-02 (UX contract)** is unblocked: types and lifecycle dimensions exist in the ontology (`si:authoringState`, `si:publicationState`, `si:resolutionState`, `si:installationState`, `si:trustState`, `si:activationState`). Do not collapse them in chrome.

**SI-03 (manifest / canonical package / icon assets)** is unblocked for a **logical** collectable matching SI-01 properties. Carrier direction is now: hypermedia collectable (HCF/HMC) + `.10d` visual + small `.q42`; volumes are dependencies. Do not freeze `.hcf` vs `.hmc` vs `.10d` vs `.d10` wire magic until the one remaining D2 clarification below is answered. Digest algorithm remains D3.

SI-04+ still wait on SI-01 acceptance plus their listed dependencies.

---

## 9. Owner asks still open

Recorded 2026-09-15: D1 `ns.webizen.org`; D2 instrument ≠ volume (HCF/HMC + `.10d` visual + small `.q42`); D5/D11 native profile, badge/VC optional export.

Still need owner input only on:

1. **D2 remainder (SI-03 proceeding):** `.hmc` (`bundle` QBDL) is the shipping collectable; `.hcf` is the document member; `assets/badge.10d` is the visual; `graphs/instrument-definition.n3` is the inspectable graph; native builds also embed a small `graphs/instrument.q42`. Owner may still say HCF-alone is the unit.
2. **D7** — first safety-reviewed clinical/wellbeing uplift (name a kernel), or SI-08 starts non-clinical only.
3. **D9** — who may publish an *endorsed* sector profile: open, named bodies, or held.

D3/D8/D10/D12/D13 are implementation or later chrome; they are not being asked as blockers. `.d10` vs `.10d` is an existing spec/code split to reconcile in SI-03, not a new coinage.
