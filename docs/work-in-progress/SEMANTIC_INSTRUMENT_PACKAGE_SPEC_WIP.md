# Semantic instrument package specification

**Status:** work in progress · **Version:** 0.1.0-draft · **Not a normative standard**  
**Date:** 2026-09-14  
**Overview:** [`SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md`](SEMANTIC_INSTRUMENTS_OVERVIEW_WIP.md)  
**Implementation:** [`SEMANTIC_INSTRUMENT_IMPLEMENTATION_PLAN_WIP.md`](SEMANTIC_INSTRUMENT_IMPLEMENTATION_PLAN_WIP.md)

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT** and **MAY** express
requirements for this draft. They become normative only if this document is promoted through the
project's standards process.

## 1. Scope

This specification defines a sector-independent contract for manufacturing, distributing,
collecting, resolving, installing, employing and tracing semantic instruments. It also defines how
an instrument can assess capability evidence and support achievement credentials and gap analysis.

It does not standardise the substantive rules of medicine, law, tax, engineering or any other
profession. Those belong in separately governed sector profiles and packages.

### 1.1 Foundational conformance principle

The `SemanticInstrument` is the primary knowledge object. An icon, Open Badge, credential, catalogue
card or certificate is a representation of, or claim about, that object. A conformant system MUST
therefore ensure that:

1. a badge/icon identifies and resolves an immutable instrument release rather than substituting
   artwork or a title for its semantic content;
2. the complete ontology, logic, constraints, dependencies, evidence, tests, applicability and
   execution contract are inspectable subject to declared access controls;
3. instruments of very different size and sector use the same identify, inspect, acquire, validate,
   employ, receipt and cite lifecycle;
4. installation and execution occur only after package/dependency integrity and required
   permissions are established;
5. exact package and dependency versions remain denotable after execution;
6. every result remains traceable to its instrument, context and responsible actions;
7. credentials about authors, publishers or award recipients reference the instrument but do not
   replace or absorb it.

## 2. Conformance roles

An implementation may conform as one or more of:

- **Authoring host** — creates and validates packages.
- **Publisher** — signs publication claims and makes immutable package bytes available.
- **Catalogue** — indexes package metadata without becoming the publisher or trust authority.
- **Resolver** — obtains dependencies and verifies their identity and integrity.
- **Library** — collects and installs packages for a holder.
- **Runner** — executes a validated entry point within declared limits.
- **Receipt store** — retains traceable execution and derivation records.
- **Credential issuer** — issues claims about capabilities or learning outcomes.
- **Presentation client** — renders badges and provides inspectable context.

Conformance claims MUST name the implemented roles and optional sector profiles.

## 3. Conceptual model

### 3.1 Required entity classes

| Class | Plane | Meaning |
|---|---|---|
| `SemanticInstrument` | tool | Abstract, version-independent identity of an instrument |
| `InstrumentRelease` | tool | Immutable version of the instrument definition |
| `InstrumentPackage` | artifact | Canonical bytes containing a release and embedded resources |
| `DependencyRequirement` | tool | Constraint on another ontology, package, dataset or runtime |
| `EntryPoint` | tool | Named, typed and bounded operation supplied by a release |
| `Agent` | who | Human, organisation, group or software actor |
| `CapacityGrant` | relation | Authority, professional role, mandate or delegation used for an action |
| `Attestation` | claim | Authorship, review, endorsement, publication or other signed assertion |
| `Capability` | concept | Skill, learning outcome or competence concept |
| `CapabilityRequirement` | claim | Capability and level required for a role, task or award |
| `CapabilityEvidence` | evidence | Work, observation, assessment, testimonial or prior credential |
| `CapabilityCredential` | claim | Issuer's assertion that a subject demonstrated capabilities |
| `GapReport` | claim | Contextual comparison of requirements with recognised capabilities |
| `ExecutionEvent` | activity | Employment of one entry point against one bound input set |
| `ExecutionReceipt` | evidence | Durable record of the execution identity, policy and outcome |
| `BadgePresentation` | presentation | Icon and concise human-facing rendering of an instrument or credential |

The abstract instrument, immutable release, package bytes, badge presentation and credentials MUST
have different identifiers.

### 3.2 Separation rules

1. An agent MUST NOT be identified by the identifier of an instrument, credential or result.
2. An organisation's action MUST name the acting agent or accountable organisational process where
   that information is available.
3. A software agent's action MUST name its principal/delegation chain, or explicitly declare that
   no human principal is asserted.
4. A signature MUST be represented as evidence of origin and integrity, not truth.
5. Authorship, contribution, review, endorsement, accreditation, publication and issuance MUST be
   independently expressible and MUST NOT be inferred from one another.
6. A run result MUST be a contextual claim and MUST NOT silently mutate the input facts.
7. Revocation or suspension of a release or credential MUST NOT erase the subject, author, package
   bytes already lawfully retained, or historical receipts.
8. `cap:Capability` (a competence), an acting `CapacityGrant`, and a runtime `HostCapabilityGrant`
   MUST use distinct types and predicates. Possessing a skill does not grant data access; receiving
   execution permission does not confer professional competence.

## 4. Package structure

### 4.1 Logical file set

A canonical package contains:

```text
manifest
graphs/
    instrument-definition.q42
shapes/
    input-shapes.q42
    output-shapes.q42
rules/
    ... declarative or bytecode resources
tests/
    manifest + input/output fixtures
assets/
    badge/icon resources
attestations/
    optional detached credentials and proofs
```

The physical carrier MAY be a Q42 volume, deterministic archive, content-addressed directory or
other registered encoding. Conformant encodings MUST yield the same logical resource graph and
must define their canonical digest procedure.

### 4.2 Manifest fields

| Field | Card. | Requirement |
|---|---:|---|
| `format_version` | 1 | Package format version |
| `instrument_id` | 1 | Stable abstract instrument IRI/DID |
| `release_id` | 1 | Immutable release IRI derived from or bound to version and digest |
| `version` | 1 | Semantic version |
| `name` | 1..* | Human label with language tag or locale |
| `description` | 1..* | Purpose-oriented description |
| `domains` | 1..* | Resolvable concept identifiers; labels are not identifiers |
| `purpose` | 1 | Intended use |
| `prohibited_uses` | 0..* | Uses explicitly excluded by the publisher |
| `framing` | 1 | `living-SHACL`, `artifact-OWL` or declared split/mixed framing |
| `entry_points` | 1..* | Named operations with shapes, effects and budgets |
| `dependencies` | 0..* | Exact or compatible requirements with digests and licences |
| `citations` | 1..* | Sources supporting the method or its provenance |
| `applicability` | 1 | Explicit constraints or an explicit unrestricted declaration |
| `honesty_policy` | 1 | Fail-closed and interpretation requirements |
| `resource_limits` | 1 | Memory, cycles/time, output and recursion/depth limits |
| `capability_requests` | 0..* | Least-privilege host/data/network capabilities |
| `test_manifest` | 1 | Test vectors and expected outcomes |
| `assets` | 1..* | At least one accessible badge/icon presentation |
| `licence` | 1 | Package licence plus per-resource exceptions |
| `authors` | 1..* | Agent references; capacity claims remain separate |
| `content_digest` | 1 | Digest of canonical package content excluding detached signatures |
| `uplift_from` | 0..* | Prior release, native kernel or external model being migrated |
| `supersedes` | 0..* | Releases explicitly superseded by this release |

Unknown fields MUST be preserved by round-tripping tools unless a declared canonicalisation profile
forbids them. A runner MAY ignore unknown non-critical fields. Unknown fields marked critical MUST
make validation fail closed.

### 4.3 Badge assets

Each asset record MUST include:

- resource path or content-addressed locator;
- media type;
- byte length and cryptographic digest;
- dimensions or scalable-vector declaration;
- purpose such as `catalog`, `compact`, `maskable`, `print` or `accessibility`;
- accessible text or language-keyed label;
- creator and licence where different from the package;
- whether institutional marks or protected symbols are present.

An asset MUST be covered by the release digest. Presentation clients MUST display verification and
lifecycle state separately from the artwork. Artwork MUST NOT be interpreted as proof.

## 5. Entry-point contract

Every entry point MUST declare:

- stable name and operation kind;
- input and output graph shapes;
- required and optional inputs;
- units, datatypes and controlled concepts;
- rules or executable resource identifiers;
- deterministic/non-deterministic classification and random-seed requirements;
- allowed side effects, normally none;
- requested host capabilities and data sensitivity classes;
- maximum memory, execution steps/time and output size;
- incomplete-input behaviour;
- error and held-state vocabulary;
- interpretation and safety notices;
- receipt disclosure profile.

A runner MUST validate inputs before execution. Missing required observations MUST NOT be replaced
with patient-, taxpayer-, client- or learner-looking defaults. The entry point MUST return a held or
error result without manufacturing facts.

Native/WASM kernels MAY be used, but the manifest MUST bind their bytes, ABI, supported target and
semantic contract. An opaque executable without inspectable purpose, shapes, provenance and tests
is not a conformant semantic instrument.

## 6. Dependencies

### 6.1 Dependency record

A dependency MUST declare:

- identifier and dependency kind;
- version constraint;
- expected digest when an exact release is required;
- resolver locations or catalogue hints;
- licence and redistribution rule;
- required/optional status;
- purpose within the instrument;
- compatibility or vocabulary mapping where relevant;
- transitive dependency policy;
- offline/cache policy;
- revocation and vulnerability policy.

Dependencies can include ontology volumes, lexicon packs, rule packages, datasets, unit systems,
native/WASM modules, sector profiles and other semantic instruments.

### 6.2 Resolution

Resolution MUST be deterministic for a fixed manifest, catalogue snapshot and policy. The resolver
MUST verify bytes before registration and MUST record the chosen release and digest. A digest
mismatch, missing required dependency, unresolved critical licence, incompatible ABI or revoked
critical release MUST prevent activation.

Network retrieval MUST be an explicit capability. A package that is collected but unresolved may
be displayed as **held / dependencies not yet resolved**; it MUST NOT be presented as executable.

Dependency cycles MUST be detected. A sector profile may prohibit cycles or define a bounded,
deterministic strongly-connected-component policy.

## 7. Authorship, review and publication

### 7.1 Attestation predicates

At minimum the model MUST distinguish:

- `authoredBy`;
- `contributedBy` with contribution role;
- `technicallyReviewedBy`;
- `professionallyReviewedBy`;
- `communityAttestedBy`;
- `endorsedBy`;
- `accreditedBy`;
- `publishedBy`;
- `derivedFrom`;
- `withdrawnBy`.

Each attestation identifies its subject release/digest, issuer, acting capacity, issuance time,
validity where applicable, evidence and proof. Multiple, even conflicting, attestations MAY coexist.
A catalogue MUST NOT flatten them into one generic `verified` flag.

### 7.2 Credential envelopes

Attestations MAY use W3C Verifiable Credentials, Open Badges profiles, Qualia native credentials or
other registered proof envelopes. The signed subject for publication is the immutable
`InstrumentRelease` and its digest, not the author's person.

Open Badges export SHOULD preserve the full instrument locator and digest. Achievement credentials
MUST identify the learner/worker as credential subject and MUST reference, rather than become, the
assessment instrument used.

## 8. Capability, learning and qualification model

### 8.1 Requirements

A capability requirement MUST declare:

- capability concept identifier;
- required level or observable outcome;
- whether it is mandatory, elective or advisory;
- prerequisite relationships;
- acceptable recognition bases;
- acceptable evidence and assessment methods;
- equivalence policy and required mapping authority;
- validity or recency requirements;
- jurisdiction, language or cultural context where material.

### 8.2 Evidence and recognition

Evidence MAY arise from formal education, employment, portfolios, observed performance, community
practice, prior credentials, tests or other declared sources. Evidence MUST retain provenance,
sensitivity and access policy. Public credentials SHOULD reference private evidence through
commitments or controlled locators rather than disclose it.

RPL and experiential recognition MUST be first-class. Formal institutional issuance MUST NOT be a
hard-coded prerequisite unless the particular role, law or safety profile actually requires it.

### 8.3 Gap report

A gap analysis MUST identify:

- target role, task, qualification or instrument release;
- requirement-set release and digest;
- capability evidence and credentials considered;
- satisfied requirements and recognition basis;
- recognised equivalences and mapping authority;
- unresolved evidence questions;
- unmet mandatory and elective requirements;
- possible dependency-ordered learning paths;
- run time, evaluator release and receipt identifier.

Absence of accessible evidence is not proof of absence of capability. It MUST be reported as
`unresolved` unless the assessment contract justifies a stronger conclusion.

### 8.4 Award

An instrument MAY propose an award result. Issuance occurs only through the declared issuer policy,
which may require human review, organisational approval, multi-party consensus or automatic
issuance. The execution receipt, assessment evidence and issuer decision remain separately
traceable.

## 9. Lifecycle and state

Package state is multi-dimensional:

| Dimension | Example values |
|---|---|
| Authoring | draft · structurally valid · test-valid · review-ready |
| Publication | unpublished · published · withdrawn · superseded |
| Resolution | unresolved · partially resolved · resolved · integrity-failed |
| Installation | not collected · collected · installed |
| Trust | unassessed · self-asserted · reviewed · endorsed · locally trusted |
| Activation | inactive · active · suspended · policy-refused |

Clients MUST NOT collapse these dimensions into a single status. In particular, `published` does
not mean `trusted`, and `installed` does not mean `active`.

Mutable catalogue metadata MUST point to immutable releases. Republishing different bytes under the
same release identifier or digest is forbidden.

## 10. Execution and receipts

### 10.1 Preflight

Before execution, a runner MUST establish:

- package and entry-point identity;
- package integrity and allowed lifecycle state;
- complete, verified dependency closure;
- input shape validity;
- applicability decision;
- user/agent authority and consent;
- least-privilege capability grant;
- resource budget;
- required sector-specific safety gates.

### 10.2 Receipt fields

An execution receipt MUST bind:

- receipt and execution-event identifiers;
- package, release and content digest;
- entry point;
- resolved dependency lock and digest;
- runner identity, version and execution target;
- invoking agent and applicable delegation/capacity;
- input commitment or authorised input references;
- start/end time and logical clock;
- policy, consent and applicability decisions;
- resource budget and actual use;
- status, warnings and error/held reasons;
- output commitment and controlled result references;
- parent receipts for composite executions;
- signature or provenance anchor.

Receipts MUST minimise disclosure. A public receipt may carry commitments while the detailed input
and result remain in a restricted or sanctuary graph.

### 10.3 Composition

An instrument MAY call another instrument only when the dependency and capability relationship is
declared. The parent receipt MUST reference child receipts in deterministic order. Failure and
uncertainty MUST propagate according to declared composition rules; a failed child cannot silently
be treated as a successful fact.

## 11. Distribution and catalogue requirements

A catalogue record MUST expose enough information to decide whether to collect the package without
executing it:

- stable instrument and release identifiers;
- name, icon, purpose, domains and sector profile;
- version, digest, size and download locations;
- authorship and publisher claims without collapsing distinct roles;
- dependency count, download size and online/offline status;
- requested capabilities and sensitivity access;
- lifecycle, verification and local trust states;
- licence, citations and important limitations;
- compatibility with the local runner.

Catalogue search results are discovery claims. Catalogue inclusion is not endorsement.

## 12. Poet and Webizen interaction requirements

### 12.1 Poet authoring

Poet MUST provide reachable flows for:

1. creating or importing an instrument;
2. electing and converting ontology dependencies;
3. editing metadata, shapes, logic and visual assets;
4. defining capabilities, learning outcomes and prerequisite graphs;
5. attaching citations, evidence and licences;
6. creating complete labelled fixtures;
7. validating and comparing versions;
8. requesting and recording reviews;
9. signing and publishing an immutable release;
10. running locally and inspecting receipts.

### 12.2 Webizen library and employment

Webizen Desktop and Webizen/WASM MUST, within platform capability, provide:

1. catalogue discovery and direct package import;
2. context inspection before collection or activation;
3. dependency and permission review;
4. collection, resolution, installation and removal as distinct operations;
5. badge/icon surfaces with accessible status indicators;
6. entry-point invocation through shape-derived forms or graph bindings;
7. execution progress, held/error states and cancellation;
8. receipt, result, provenance and version inspection;
9. update, suspension, revocation and supersession handling;
10. export or citation without losing the package identity and digest.

Desktop administrative authoring SHOULD provide parity with Poet for organisations that do not use
Poet, while retaining the same underlying package contracts.

## 13. Security, rights and governance

1. Packages execute with no capabilities by default.
2. Network, filesystem, personal-record, secret, device and external-service access require
   explicit grants.
3. Untrusted package parsing and validation MUST be bounded and non-recursive where required by the
   QualiaDB execution contract.
4. Public package definitions MUST NOT contain private assessment evidence or personal records.
5. Sector profiles MUST define escalation for safety-critical outputs.
6. Human-rights, non-discrimination, accessibility and contestability requirements apply to the
   package and to uses of its results.
7. Cultural/community knowledge packages MUST be able to declare audience, custodianship,
   attribution, non-derivation and non-commercial or other governance constraints.
8. Licences and dependency terms MUST be visible before distribution or execution.
9. Automated updates MUST NOT cross a breaking semantic version, expand permissions or replace a
   locally pinned digest without consent.
10. A result affecting a person MUST remain contestable and traceable to its instrument and inputs.

## 14. Sector profiles

A sector profile MUST define additional manifest constraints, evidence requirements, authority
rules, UI notices, test requirements and output modalities. Initial profiles SHOULD cover:

- capability/learning and employment;
- health/clinical support;
- law and regulation;
- tax/accounting;
- science/engineering;
- welfare/public administration.

Profiles MUST reuse the core package and receipt formats. They MUST NOT introduce a separate
distribution lifecycle unless an external standard makes an adapter necessary.

## 15. Minimum conformance tests

A conformant implementation MUST test:

1. canonical package digest round-trip;
2. manifest and graph-shape validation;
3. icon digest and accessible-label validation;
4. dependency closure, cycle and digest-mismatch handling;
5. signature success, tamper failure and expired/revoked status;
6. separation of author, reviewer, publisher, issuer and subject;
7. incomplete input held/fail-closed behaviour;
8. deterministic execution or declared seeded/non-deterministic behaviour;
9. receipt reproduction and dependency-lock binding;
10. collection without activation and least-privilege default;
11. historical receipt interpretation after supersession/revocation;
12. RPL equivalence plus unresolved-evidence behaviour in gap analysis;
13. restricted evidence non-disclosure in public credentials and receipts;
14. Poet/Webizen package compatibility for the same fixture;
15. at least one negative test for every critical manifest field.

## 16. Compatibility and migration

The first implementation SHOULD adapt, not replace, these existing facilities:

- `vibe:ExpertiseEvalPack` draft concepts;
- `cap:LearningClaim`, `CapabilityRequirement` and capability-gap evaluators;
- `cml:LogicApplication` and registered QualiaDB modalities;
- W3C/native Verifiable Credential runtimes and Open Badge codec;
- Q42 volume and lexicon-pack manifests;
- qapp icon, capability and content-addressed WASM concepts;
- Vibe/QualiaDB law packages;
- WellFair assessment instruments and authority attestations;
- temporal/provenance graphs and policy receipts.

Legacy hard-coded evaluators remain available as explicitly named `uplift_from` adapters until a
reference package demonstrates semantic equivalence, safety review and surface UAT.

## 17. Illustrative manifest skeleton

This example is informative. SI-01 and SI-03 own the final vocabulary, encoding and
canonicalisation decisions.

```json
{
  "format_version": "0.1",
  "instrument_id": "https://example.org/instruments/community-water-assessment",
  "release_id": "https://example.org/instruments/community-water-assessment/releases/1.0.0",
  "version": "1.0.0",
  "name": { "en": "Community water assessment" },
  "description": { "en": "Evaluates declared water observations against a cited rule set." },
  "domains": ["https://example.org/concepts/water-quality"],
  "purpose": "training-and-field-screening",
  "prohibited_uses": ["sole-basis-for-regulatory-enforcement"],
  "framing": "mixed-split",
  "entry_points": [
    {
      "name": "assess",
      "input_shape": "shapes/input.q42#AssessmentInput",
      "output_shape": "shapes/output.q42#AssessmentClaim",
      "rules": ["rules/assessment.q42"],
      "resource_limits": { "memory_bytes": 1048576, "steps": 100000, "output_quins": 128 },
      "host_capability_requests": []
    }
  ],
  "dependencies": [
    {
      "id": "https://example.org/ontologies/water-observations",
      "version": "2.1.0",
      "digest": "sha256:EXAMPLE",
      "required": true,
      "purpose": "observation terms and units"
    }
  ],
  "citations": ["https://example.org/guidelines/water-assessment-2026"],
  "applicability": { "jurisdiction": ["example"], "valid_from": "2026-01-01" },
  "honesty_policy": {
    "incomplete_input": "held",
    "result_kind": "screening-claim",
    "required_notice": "This result requires contextual professional review."
  },
  "test_manifest": "tests/tests.json",
  "assets": [
    {
      "path": "assets/badge.svg",
      "media_type": "image/svg+xml",
      "digest": "sha256:EXAMPLE",
      "purpose": ["catalog", "compact"],
      "accessible_text": { "en": "Community water assessment instrument" }
    }
  ],
  "licence": "https://spdx.org/licenses/CC-BY-4.0.html",
  "authors": ["did:example:community-water-group"],
  "content_digest": "sha256:EXAMPLE"
}
```

The badge artwork is only one digested resource. Authorship, professional review and publication
proofs are detached attestations over `release_id` plus `content_digest`; they are not inferred from
the `authors` list or from the appearance of the asset.
