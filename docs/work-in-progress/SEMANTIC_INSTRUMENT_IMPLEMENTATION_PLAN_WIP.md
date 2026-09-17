# Semantic instruments — implementation plan and requirements

**Status:** work in progress · **Programme:** SI-0 · **Not an implementation-complete claim**  
**Date:** 2026-09-14  
**Overview:** [`SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md`](SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md)  
**Draft specification:** [`SEMANTIC_INSTRUMENT_PACKAGE_SPEC_WIP.md`](SEMANTIC_INSTRUMENT_PACKAGE_SPEC_WIP.md)  
**Prior scope:** [`EXPERTISE_EVAL_PACK_SCOPE_WIP.md`](EXPERTISE_EVAL_PACK_SCOPE_WIP.md)

## 1. Outcome

Deliver a sector-independent semantic-instrument system in which authorised agents can manufacture,
review, package, publish and distribute ontology-backed logic; other agents can collect, resolve,
install and employ it; and every material use is traceable to exact package and dependency bytes.

The first vertical slices prove both sides of the model:

1. **Instrument use:** package and run one existing clinical or wellbeing calculation through a
   generic runner, represented by an icon and producing a receipt.
2. **Capability recognition:** define a role/qualification requirement graph, recognise formal and
   experiential evidence, compute a gap report, and optionally issue an Open Badges-compatible
   capability credential referencing the assessment instrument.

## 2. Programme rules

1. Extend existing Q42, CML, capability, credential, qapp, provenance and catalogue infrastructure.
   Do not build a parallel database or trust system.
2. The package definition, badge presentation, publisher attestation, capability credential,
   execution receipt and result claim remain distinct types.
3. New capabilities use directory-backed libraries with focused files. Apply the repository's
   500/1,200-line ownership thresholds and two-tier allocation model.
4. Cold package construction may allocate within explicit budgets. Runner kernels and per-element
   evaluation paths obey Tier-1 zero-heap requirements.
5. Every packet owns an explicit file set. Concurrent packets MUST NOT edit the same implementation
   file without a declared handoff.
6. A landed document or UI shell is not implementation completion. Completion requires the listed
   tests, fixtures and evidence.
7. Existing native clinical and assessment behaviour remains until equivalence and safety gates pass.
8. Test fixtures use labelled reference profiles, never realistic-looking partial patient defaults.
9. Network retrieval, secrets and personal data remain explicit capabilities.
10. No packet may represent a signature, issuer reputation, institutional logo or catalogue listing
    as proof of substantive truth.

## 3. Target module boundaries

Indicative paths are proposed for ownership planning; the first packet may adjust them after an
ownership and dependency review.

```text
crates/qualia-core-db/src/semantic_instruments/
    mod.rs                    public domain types and routing
    manifest.rs               validated in-memory manifest model
    canonical.rs              deterministic canonical encoding/digest
    package.rs                cold package build/read
    dependency.rs             requirements and lock model
    resolver.rs               bounded resolution planning and verification
    validation.rs             core conformance gates
    registry.rs               installed-release index
    execution.rs              generic runner preflight and dispatch
    receipt.rs                execution receipt model/encoding
    capability_profile.rs     capability/outcome and award bindings
    errors.rs                 stable error/held vocabulary

core-ontologies/semantic-instruments.n3
core-ontologies/semantic-instrument-shapes.n3
core-ontologies/semantic-instrument-capability-profile.n3

crates/poet/src/browser/semantic_instruments/
    catalog/ authoring/ dependencies/ validation/ publication/ receipts/

crates/webizen-studio/src/components/semantic_instruments/
    library/ inspector/ permissions/ runner/ receipts/

crates/webizen-desktop/src/commands/semantic_instruments/
    package/ registry/ resolver/ execution/ publication/
```

The canonical domain types belong in `qualia-core-db`; Poet and Webizen consume them through host
bindings and MUST NOT define incompatible manifest structures.

## 4. Delivery graph

```text
SI-01 vocabulary + shapes ─┬─> SI-03 canonical package ─> SI-05 resolver/registry ─┐
                           │                                                       │
SI-02 UX/interaction ──────┼──────────────────────────────> SI-09 Poet UI          ├─> SI-12 UAT
                           │                                                       │
SI-04 attestations/VC ─────┼─> SI-07 publication/catalog ─> SI-10 Webizen UI       │
                           │                                                       │
SI-06 runner + receipts <──┴──── SI-03 + SI-05 ───────────> SI-08 reference packs ┘

SI-01 + SI-04 + SI-06 ─────────> SI-11 capability/RPL vertical slice ────────────> SI-12 UAT
```

Packets in the same wave may run concurrently only when their file ownership does not overlap.

## 5. Swarm packet contract

Every implementation packet MUST begin with a short packet record containing:

| Field | Requirement |
|---|---|
| Packet ID | Stable `SI-NN` identifier |
| Objective | One verifiable outcome |
| Inputs | Specifications, APIs and prior packet evidence consumed |
| Dependencies | Packet IDs that must be accepted first |
| Owned paths | Exclusive files/directories for the packet turn |
| Out-of-scope paths | Adjacent files the packet must not change |
| Budget | New-file line target, memory/runtime limit and expected test duration |
| Acceptance | Exact tests, fixtures, inspection or UAT evidence |
| Handoff | Public APIs, unresolved questions and next eligible packets |

Agents MUST inspect existing implementations before proposing replacements. A packet that discovers
a contract conflict stops mutation, records the evidence and requests an integration decision.

Each handoff reports:

- changed files;
- public API or ontology terms introduced;
- tests run and exact outcomes;
- allocation/resource evidence where applicable;
- known limitations and held items;
- compatibility impact;
- whether dependent packets are unblocked.

## 6. Work packets

### SI-00 — Baseline and decision register

**Dependencies:** none  
**Ownership:** documentation only

Inventory existing package-like and credential-like models, identify duplicate `Credential` and
`LawPackage` concepts, record compatibility decisions, and establish the semantic-instrument
decision log. Include current tests and source-line evidence.

**Acceptance:** inventory covers Q42 volume, lexicon pack, qapp package, CML, capability gap, W3C and
native VCs, Open Badge codec, law packages, provenance/receipts, WellFair assessments, Poet and both
Webizen surfaces.

### SI-01 — Core ontology and SHACL shapes

**Dependencies:** SI-00  
**Proposed ownership:** `core-ontologies/semantic-instrument*.n3` plus ontology-validation tests

Define the classes and separation rules in specification §3, manifest properties, lifecycle
dimensions, agent/capacity actions, dependency requirements, entry points, attestations, badge
assets, executions, receipts and capability-profile links.

Reuse `cml:LogicApplication`, `cap:Capability`, PROV-O, SHACL, values/agency terms and existing time
vocabularies. Add closed-world shapes for critical package fields. Do not assert that every
instrument or actor is an `owl:Thing` where living-safe framing applies.

**Acceptance:** positive fixtures validate; fixtures missing digest, shapes, purpose, honesty policy,
tests or accessible icon fail; who/tool/claim merge fixtures fail; existing core ontology suite
remains green.

### SI-02 — UX information architecture and interaction contract

**Dependencies:** SI-00  
**Proposed ownership:** one Poet/Webizen semantic-instrument UX specification and wireflow fixtures

Specify common cards, badge behaviour, context inspection, collection, resolution, installation,
permissions, run forms, receipts, updates and held/error states. Define compact, standard and
administrative views. Include keyboard, screen-reader, localisation, low-bandwidth and offline
requirements.

**Acceptance:** every lifecycle transition is reachable; artwork never carries trust state alone;
Poet and Desktop parity table is complete; Webizen/WASM degradations are explicit; five user
walkthroughs pass on paper: author, reviewer, publisher, learner/worker and instrument user.

### SI-03 — Manifest, canonical package and icon assets

**Dependencies:** SI-01  
**Proposed ownership:** `qualia-core-db/src/semantic_instruments/{manifest,canonical,package}.rs`

Implement the manifest model, bounded validation, deterministic canonical encoding, content digest,
logical package reader/builder and badge asset records. Reuse Q42 volume/container facilities where
they satisfy the contract; document any adapter boundary.

Cold builders use bounded scratch and caller-selected output locations. Readers enforce byte and
entry limits before allocation.

**Acceptance:** canonical byte/digest golden vectors; encode/decode round-trip; unknown non-critical
field preservation; zip/path traversal and oversized-entry negative tests if an archive carrier is
used; icon digest/media/dimension/accessibility checks; cleanup tests on success, error and unwind.

### SI-04 — Attestation and credential profiles

**Dependencies:** SI-01  
**Proposed ownership:** semantic-instrument credential profile modules and fixtures; minimise changes
to shared credential runtimes

Define and implement profiles for authorship, contribution, review, endorsement, publication,
withdrawal and capability awards. Bind immutable release digests. Extend Open Badges representation
to model nested achievement/outcome/evidence and image data rather than only adding a context/type.

Provide adapters to the existing W3C ML-DSA and native Ed25519/ML-DSA paths. Preserve the origin-not-
truth boundary. Record status/revocation without erasing historical claims.

**Acceptance:** issue/verify and tamper/expiry/revocation tests; role separation tests; Open Badge
JSON fixture validation and image binding; an achievement credential references the assessment
instrument instead of using it as the credential subject.

### SI-05 — Dependency resolver, lock and local registry

**Dependencies:** SI-03  
**Proposed ownership:** `semantic_instruments/{dependency,resolver,registry}.rs`

Implement dependency requirements, deterministic resolution planning, lock records, content
verification and installed-release lookup. Begin with local files/Q42 volumes and explicit catalogue
records; add network transports only behind a capability gate.

**Acceptance:** exact and compatible resolution; missing optional versus required behaviour; digest
mismatch; cycle detection; revoked critical dependency; offline held state; deterministic lock;
no activation before complete verified closure.

### SI-06 — Generic runner and execution receipts

**Dependencies:** SI-03, then integrate SI-05  
**Proposed ownership:** `semantic_instruments/{execution,receipt,errors}.rs` and narrow WebizenVM seams

Implement preflight, shape binding, capability/budget checks, modality/native dispatch and receipts.
The initial runner may support a deliberately small set of already implemented evaluator kinds,
but dispatch is declared by entry point rather than by new per-instrument Host IDs.

Hot dispatch remains caller-buffered and bounded. Cold receipt serialisation may allocate within a
declared budget. Input/output commitments protect restricted data.

**Acceptance:** deterministic reference run; missing input held; invalid shape, dependency or
capability refused; cancellation and resource-limit outcomes; receipt binds exact release,
dependency lock, runner, input/output commitments and parent/child execution order.

### SI-07 — Publication, catalogue and distribution

**Dependencies:** SI-03, SI-04, SI-05  
**Proposed ownership:** catalogue/resource APIs and package publication adapters

Publish immutable package bytes and mutable discovery records. Support direct file import and local
catalogue first, then commons/peer or HTTP transports. Catalogue metadata exposes purpose,
provenance, permissions, dependencies, compatibility and limitations before download.

**Acceptance:** publish/discover/download/verify round-trip; same release cannot accept different
bytes; catalogue inclusion remains distinct from endorsement; licence and size visible before
download; interrupted download cannot register a release as resolved.

### SI-08 — Reference instrument packages

**Dependencies:** SI-01, SI-03, SI-06; SI-04/05 before signed distribution  
**Proposed ownership:** new fixture directories only

Create three labelled reference packages:

1. a simple, non-sensitive deterministic instrument demonstrating shapes and units;
2. a clinical/wellbeing uplift of an existing hard-coded implementation, such as PHQ-9/GAD-7 or a
   selected ClinicalRisk score, retaining its safety and licensing boundaries;
3. a non-clinical package, such as a fictional law or tax-training example, to prove sector
   independence without purporting to give professional advice.

**Acceptance:** all validate, resolve and run; native-uplift output matches existing behaviour within
declared tolerance; incomplete inputs fail closed; receipts reproduce; every fixture is visibly
labelled as reference/test data.

### SI-09 — Poet manufacturing environment

**Dependencies:** SI-02, SI-03, SI-05; publication functions require SI-04/07  
**Proposed ownership:** `poet/browser/semantic_instruments/` plus narrow routing seams

Build the ontology/dependency election, manifest editor, visual logic flow, shapes/rules editor,
capability framework editor, fixture runner, validation panel, version comparison, review,
sign/publish and receipt inspection flows.

The UI must distinguish source text, generated graph and executable form. Round-trip loss or
unsupported constructs are shown before save. Empty states say held/not yet and provide the next
action.

**Acceptance:** scripted author-to-publish walkthrough; keyboard-only path; source/visual round-trip
tests; invalid package cannot publish; validation evidence remains inspectable; no hard-coded new
Host method for a reference package.

### SI-10 — Webizen library, inspector and runner UX

**Dependencies:** SI-02, SI-05, SI-06, SI-07  
**Proposed ownership:** new Webizen Studio components and Desktop command directory

Build badge/card discovery, full-context inspector, collection/install controls, dependency review,
least-privilege grants, shape-derived input binding, run progress/cancellation, result/receipt view,
history, update, suspend and remove flows. Provide WASM capability-aware degradation.

**Acceptance:** collect without activate; inspect before network fetch; permission denial is usable;
offline resolved package runs; unresolved package is held; revoked version cannot start a new run;
old receipts remain readable; desktop and WASM share fixture semantics.

### SI-11 — Capability/RPL, gap analysis and award vertical slice

**Dependencies:** SI-01, SI-04, SI-06; UI integrates after SI-09/10  
**Proposed ownership:** `semantic_instruments/capability_profile.rs`, capability fixtures and narrow
extensions to existing `capability_gap`

Model a target job or micro-qualification with capability outcomes, levels and prerequisites.
Evaluate formal, experiential, portfolio and peer/community evidence. Reuse existing zero-heap gap
and learning-path kernels. Produce satisfied, equivalent, unresolved and unmet results. Allow a
declared issuer workflow to issue a capability credential referencing the assessment receipt.

**Acceptance:** formal and RPL evidence can satisfy the same capability under explicit policy;
unavailable evidence is unresolved rather than falsely absent; unauthorised equivalence does not
pass; dependency-ordered gap path is deterministic; public award omits private evidence; no award
is issued merely because the evaluator returned a proposed pass.

### SI-12 — Conformance, security, accessibility and UAT

**Dependencies:** SI-01 through SI-11 as applicable  
**Ownership:** new integration/conformance suites and UAT records; fixes return to owning packet

Run cross-platform golden fixtures, malformed-package fuzz/property tests, resource and allocation
checks, signature/tamper tests, privacy review, accessibility review and sector-specific safety UAT.
Verify that published documentation matches live capability.

**Acceptance:** specification §15 matrix has evidence; reference packages pass in QualiaDB, Poet and
Webizen Desktop, with applicable WASM tests; security and accessibility findings are closed or
explicitly held; migration decision for each native demo is recorded.

## 7. Wave plan

| Wave | Parallel packets | Gate |
|---|---|---|
| 0 — reconcile | SI-00 | Existing types and conflicts evidenced; terminology decision accepted |
| 1 — contracts | SI-01 · SI-02 | Ontology/shapes and UX state model accepted before broad code |
| 2 — foundations | SI-03 · SI-04 | Canonical package and attestation profiles pass golden fixtures |
| 3 — runtime | SI-05 · SI-06 | Verified dependency closure and reproducible receipts |
| 4 — distribution/examples | SI-07 · SI-08 · SI-11 core | At least two sectors and capability flow work without UI |
| 5 — human surfaces | SI-09 · SI-10 | Poet/Webizen end-to-end walkthroughs and parity evidence |
| 6 — release gate | SI-12 | Conformance, safety, accessibility and migration decisions complete |

No wave number implies that all code must be serial. A packet may begin when its named dependencies
and ownership boundaries are satisfied.

## 8. Requirements traceability matrix

| Requirement | Primary packets | Required evidence |
|---|---|---|
| Agent/principal/capacity attribution | SI-01, SI-04 | ontology fixtures + signature tests |
| Badge is a resolvable handle | SI-03, SI-07, SI-10 | asset/digest tests + inspector UAT |
| Full semantic context survives presentation | SI-01, SI-03, SI-04 | Open Badge and package round-trips |
| Inspect before installation | SI-02, SI-07, SI-10 | no-execution catalogue/inspector UAT |
| Downloadable dependencies | SI-05, SI-07 | resolution lock and interrupted-fetch tests |
| Collectable without activation | SI-05, SI-10 | state-transition tests and UAT |
| Employable through generic runner | SI-06, SI-08 | cross-sector reference runs |
| Stable shape-defined input/output | SI-01, SI-03, SI-06 | compatibility and negative-shape fixtures |
| Use is denotable and traceable | SI-06 | reproducible signed/anchored receipt |
| Exact version retained and citable | SI-03, SI-05, SI-06, SI-07 | immutable release and receipt lookup tests |
| Derive/fork/update without history rewrite | SI-03, SI-04, SI-05 | derivation and supersession lineage tests |
| Professional review distinct from origin | SI-01, SI-04, SI-10 | conflicting/multiple-attestation fixture |
| RPL and informal competence recognised | SI-11 | experiential-equivalence fixture |
| Honest gap analysis | SI-11 | satisfied/equivalent/unresolved/unmet output test |
| Poet manufacturing workflow | SI-02, SI-09 | author-to-publish walkthrough |
| Webizen employment workflow | SI-02, SI-10 | collect-to-receipt walkthrough |
| Sector independence | SI-08, SI-11 | clinical + non-clinical + learning fixtures |
| Bounded and least privilege | SI-03, SI-05, SI-06, SI-12 | limits, allocation and permission tests |
| Revocation without historical erasure | SI-04, SI-05, SI-06 | post-revocation receipt interpretation test |

## 9. First reference scenarios

### Scenario A — health instrument

An authorised health-domain team packages an existing, well-scoped assessment or risk calculation.
The badge opens its sources, population, required fields, units, safety wording, tests and reviewers.
Webizen resolves the dependencies, obtains consent, runs it locally and stores a restricted result
plus a minimally disclosing receipt. The result does not become a diagnosis.

### Scenario B — RPL and employment pathway

A community infrastructure role declares capabilities and prerequisites. A person presents formal
credentials, portfolio evidence and community attestations. The instrument recognises explicitly
authorised equivalences and returns demonstrated capabilities, unresolved evidence and remaining
gaps. A training path is proposed. An authorised issuer may award a capability credential after its
review policy completes.

### Scenario C — law or tax instrument

A publisher packages jurisdiction- and period-specific rules with source instruments, citations,
tests and professional review. Webizen prevents employment outside the declared jurisdiction/period
unless an authorised override is recorded. The output is labelled as a computed claim and retains
the exact legal/tax package release used.

## 10. Global acceptance gate

SI-0 is ready for standards-candidate review only when:

1. one canonical package format and digest procedure are implemented;
2. full dependency closure can be resolved and locked;
3. distinct agent roles and attestations survive encode/decode;
4. the Open Badge representation carries a digest-resolvable semantic context and accessible image;
5. one generic runner executes at least two sector-distinct packages;
6. every execution produces a reproducible, privacy-aware receipt;
7. the RPL scenario produces a four-way capability result: satisfied, equivalent, unresolved and
   unmet;
8. Poet can manufacture and publish a package without editing Rust;
9. Webizen can inspect, collect, install and employ that package without rebuilding the host;
10. malformed, tampered, incomplete, unauthorised and revoked cases fail closed;
11. desktop/WASM parity and declared degradations have evidence;
12. accessibility, licensing, human-rights and sector-safety reviews are recorded.

Until then, the documents and partial surfaces remain explicitly work in progress.

## 11. Decisions reserved for SI-00 and owner review

Implementation agents must not silently settle these cross-cutting choices:

1. Final public terminology and ontology namespace for `SemanticInstrument`.
2. Canonical physical carrier: Q42-native volume, deterministic archive, or both through one
   logical package model.
3. Canonicalisation and digest algorithms, including whether a release identifier is digest-derived.
4. Consolidation path for the existing W3C and native `Credential` types.
5. Exact Open Badges version/profile and external conformance-test strategy.
6. Catalogue discovery protocol and the boundary between mutable listings and immutable bytes.
7. Which existing clinical/wellbeing evaluator is the first safety-reviewed uplift fixture.
8. Organisation and community authority representation, including multi-party publication.
9. Sector-profile governance and who may publish an endorsed profile.
10. Compatibility/version-resolution semantics beyond exact pinned dependencies.

SI-00 records the decision, alternatives, rationale, authority and affected packets. Work that does
not depend on a reserved decision may continue; work that would bake one into an ABI remains held.
