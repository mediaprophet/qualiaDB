# Semantic instruments — overview

**Status:** work in progress · **Branch:** `0.0.38` · **Not a normative standard**  
**Date:** 2026-09-14  
**Scope:** QualiaDB · Poet · Webizen Desktop · Webizen/WASM · Vibe · external publishers  
**Related:** [`EXPERTISE_EVAL_PACK_SCOPE_WIP.md`](EXPERTISE_EVAL_PACK_SCOPE_WIP.md) · [`core-ontologies/capability-credentials.n3`](../../core-ontologies/capability-credentials.n3) · [`core-ontologies/cml.n3`](../../core-ontologies/cml.n3)

## 1. Purpose

The controlling definition for this programme is:

> A semantic instrument is a versioned, attributable and executable artifact manufactured by one
> or more agents for a declared purpose. It combines ontologies, logical processes, constraints,
> dependencies, evidence and execution requirements, while being presented to people as a
> recognisable badge or icon.

The central thing is the **expert-authored semantic instrument**. The badge or icon is its visible
handle, not merely an award and not the substantive payload. The system resolves the full semantic
context behind that handle.

The instrument may be very small, such as a unit conversion with three rules, or enormously
complex, such as a clinical assessment, a tax treatment, a legal reasoning framework, a scientific
model or a professional qualification framework.

The badge is **not** the knowledge and is **not necessarily an award**. It is one presentation of
the instrument. The durable object is the content-addressed semantic package and its history.

### 1.1 Conceptual separation

| Element | Meaning |
|---|---|
| **Instrument** | The executable ontological model itself |
| **Badge/icon** | Its recognisable human-facing representation and activation handle |
| **Package** | The rules, shapes, ontologies, dependencies, tests and metadata behind it |
| **Credential** | Signed claims about authorship, authority, review, integrity and publication |
| **Execution receipt** | Evidence that a particular version was employed in a particular context |
| **Result** | A claim produced by that execution—not automatically a fact or authoritative decision |

### 1.2 Illustrative professional instrument

An endocrinologist, professional body or appropriately constituted team could manufacture a
diabetes-related instrument in Poet or another conformant authoring environment. Its icon may be
simple in Webizen, while inspection and employment resolve a package containing:

- the concepts and external ontologies it relies upon;
- required observations, types, units and input constraints;
- symbolic rules, calculation flows and decision boundaries;
- population, jurisdictional and temporal applicability;
- exclusions, uncertainty and fail-closed behaviour;
- source literature, legislation, standards or professional guidance;
- reference fixtures and verification tests;
- authorship, organisational authority and reviewer attestations;
- exact dependency versions and content digests;
- expected output claims and permitted interpretations.

The same pattern applies to a tax treatment, statutory interpretation, engineering calculation,
diagnostic aid, welfare assessment, scientific model or much smaller transformation.

### 1.3 Consistent external behaviour

Complexity remains encapsulated behind a consistent lifecycle. Whether an instrument contains three
elementary rules or an extensive system of interdependent ontologies and evaluators, a person or
agent must be able to:

- identify and inspect it before installation;
- see who manufactured, reviewed and published it;
- inspect its purpose, applicability and limitations;
- acquire and verify the package and its dependencies;
- grant only the data and execution permissions it requires;
- employ it through stable, shape-defined inputs and outputs;
- retain and cite the exact version used;
- trace results through the instrument, dependencies, actors and authorities;
- update, derive, fork, suspend or revoke it without rewriting history.

## 2. Why this is broader than Open Badges

Open Badges began principally as a way to express achievements, learning outcomes and
micro-credentials. That remains an important use case, especially where a person's capabilities
were acquired through work, community practice, traditional knowledge, self-directed learning or
other Recognition of Prior Learning (RPL), rather than through a formal institution.

Qualia needs two related but distinct uses of a badge-shaped object:

1. **Instrument badge** — a visual handle for an executable semantic instrument.
2. **Achievement badge** — a credential claiming that a subject has demonstrated capabilities or
   learning outcomes, possibly after an instrument assessed evidence.

An assessment instrument may therefore help produce an achievement credential, but they are not
the same object. The instrument is a tool. The credential is a claim. The person is neither.

Open Badges and Verifiable Credentials are useful presentation, exchange and attestation envelopes.
They do not replace the underlying Qualia model, in which the executable knowledge instrument and
its resolvable semantic context are first-class artifacts.

## 3. Context before certificate

A certificate or badge can be copied, screenshotted or reduced to a title. Its meaning depends on
the context behind it:

- what capabilities or outcomes were considered;
- how those capabilities were demonstrated;
- which ontology defines the terms;
- what assessment method and thresholds were applied;
- who authored, reviewed, endorsed or issued each claim;
- which professional, cultural, organisational or legal capacity they exercised;
- what evidence, limitations and applicability conditions were present;
- which exact package and dependency versions were used;
- whether the credential is current, suspended, revoked or superseded.

Consequently, a badge presentation MUST resolve to the full context graph or to a durable locator
and digest from which that graph can be obtained. A badge that cannot expose its meaning is only a
picture.

## 4. Agents and accountable capacities

An instrument may be manufactured, reviewed or published by:

- a natural person acting personally;
- a natural person acting in a professional or delegated capacity;
- an organisation acting through authorised people or software;
- a group, community or cooperative using a declared governance process;
- a software agent operating for a human or organisational principal;
- a software agent without a human principal, if that fact is explicit and policy permits it.

Identity, agency and capacity remain separate:

- **who** identifies the agent;
- **capacity** records the role, licence, mandate or delegation under which the action occurred;
- **action** records authorship, review, publication, execution or issuance;
- **claim** records what the actor asserted;
- **proof** binds an actor to an action but does not make the assertion true.

Three similar words must also remain separate throughout the product:

- **capability** means a skill, competence or learning outcome held by an agent;
- **capacity** means the role, licence, mandate or authority under which an agent acts;
- **execution permission** means access granted to an instrument, such as reading a graph or using
  the network.

An organisation MUST NOT be modelled as if it acted without people or systems. A software agent
MUST NOT silently inherit the qualifications of its operator or organisation. Delegation and
responsibility need an inspectable chain.

## 5. The semantic instrument

A complete instrument normally contains or references:

- a stable identifier and semantic version;
- name, description, domains and visual assets;
- declared purpose and prohibited interpretations;
- input and output SHACL shapes;
- ontology, vocabulary, dataset and package dependencies;
- N3, deontic, epistemic, temporal, paraconsistent or other symbolic rules;
- bounded native or WASM kernels where declarative rules are insufficient;
- applicability constraints such as jurisdiction, population, period and units;
- citations, licences and source provenance;
- test vectors, expected results and tolerance rules;
- resource limits and least-privilege capability requests;
- authorship, contribution, review and publication records;
- one or more signed attestations or credentials;
- revocation, suspension, supersession and vulnerability information.

Dependencies may be embedded or externally resolvable. In either case their exact content digests
and compatibility constraints travel with the instrument.

## 6. Lifecycle

The common lifecycle is:

1. **Author** — create ontology mappings, shapes, rules, tests, citations and visual identity.
2. **Validate** — check structure, dependency closure, examples, policy and sector profile.
3. **Review** — record independent technical, professional, community or legal assessment.
4. **Attest** — sign distinct claims about authorship, review, endorsement and publication.
5. **Package** — produce an immutable, content-addressed bundle and manifest.
6. **Publish** — place the bundle in a catalogue, commons, institutional repository or peer store.
7. **Collect** — save it to a personal or organisational library without activating it.
8. **Resolve** — obtain and verify all dependencies under an explicit network and trust policy.
9. **Install** — register the verified package locally and show its permissions and limitations.
10. **Employ** — invoke a named entry point against shape-valid inputs.
11. **Record** — issue an execution receipt linking inputs, package, dependencies, policy and result.
12. **Cite or present** — use the package or its result by stable identifier and digest.
13. **Revise** — publish a new version or a declared derivation without changing old content.
14. **Suspend, revoke or supersede** — stop future reliance while retaining historical traceability.

Collection, installation, trust and activation are separate decisions. A user can inspect or retain
an instrument without granting it access to records or allowing it to execute.

## 7. Capability recognition and gap analysis

An achievement or qualification credential is a contextual claim that a subject has demonstrated
one or more capabilities or learning outcomes. Recognition MUST support multiple bases, including:

- formal education;
- prior learning;
- experiential learning;
- supervised practice;
- peer or community attestation;
- assessment of produced work;
- traditional or culturally governed knowledge;
- equivalence or articulation from another capability framework.

The model recognises what is present before computing what remains. Given a target role or task:

```text
required capabilities
    minus demonstrated or recognised-equivalent capabilities
    equals unresolved requirements
```

The output is a **gap report**, not a judgement about a person's worth. It distinguishes:

- demonstrated capabilities;
- recognised equivalents and the authority for each mapping;
- unresolved or missing evidence;
- genuinely unmet prerequisites;
- ambiguous mappings requiring human review;
- possible learning pathways, costs and dependencies.

This is particularly important for Sustainable Development Goal contexts and developing regions,
where useful competence may exist without a conventional institutional certificate. Institutional
status is one possible source of evidence, not the definition of capability.

## 8. Sector profiles

The core package and provenance model is sector-independent. Sector profiles add stricter rules.

| Sector | Typical additions |
|---|---|
| Learning and employment | outcomes, proficiency levels, assessment evidence, prerequisite graphs, RPL and gap analysis |
| Health and diagnostics | populations, units, reference ranges, exclusions, clinical authority, safety escalation and `not_diagnosis` boundaries |
| Law | jurisdiction, commencement and repeal, authority hierarchy, precedent, conflicts and `not_legal_advice` boundaries |
| Tax and accounting | jurisdiction, tax period, entity type, currency, source rulings, audit tolerances and versioned legislation |
| Engineering and science | dimensions, units, calibration, uncertainty, test vectors, physical assumptions and reproducibility |
| Welfare and public administration | eligibility policy, effective dates, evidence rules, review rights and non-discrimination constraints |
| Cultural or community knowledge | community authority, access protocol, attribution, permitted audiences and non-extractive use conditions |

A sector profile may strengthen the core requirements but must not weaken provenance, dependency
integrity, human-rights protections or historical traceability.

## 9. Badge and icon behaviour

The icon is a first-class presentation asset bound to the package digest. It should make an
instrument recognisable across Poet, Webizen and exported credentials without becoming an opaque
trust signal.

The default badge interaction should expose:

- name, purpose and current version;
- publisher and principal/delegation chain;
- authors, reviewers and their distinct capacities;
- installed, held, active, suspended, revoked or superseded status;
- dependency and network requirements;
- requested data access and execution capabilities;
- applicability and important limitations;
- verification state and available evidence;
- previous uses and resulting receipts, subject to access control.

Colour, seals and institutional logos MUST NOT substitute for verification. Unverified and
self-asserted instruments may still be collected and studied, but their state must be visible.

## 10. User environments

### Poet

Poet is the primary manufacturing environment. It should support ontology election and ingestion,
shape editing, rule authoring, dependency declaration, fixtures, validation, review requests,
versioning, signing and publication. A visual flow and a source representation must remain
round-trippable or explicitly identify any lossy boundary.

### Webizen Desktop and Webizen/WASM

Webizen is the library and employment environment. It should support discovery, inspection,
collection, dependency resolution, installation, permission grants, execution and receipt review.
Administrative authoring capabilities should have Desktop parity where the user does not use Poet.

### Other tools

External tools may manufacture or consume instruments if they implement the same package,
validation, signature and receipt contracts. Poet and Webizen are reference environments, not
exclusive authorities.

## 11. Trust and safety principles

1. Signature proves origin and integrity, not correctness or truth.
2. Authorship, review, endorsement, accreditation and issuance are different predicates.
3. Professional capacity is time- and jurisdiction-bounded and is not personal identity.
4. Software-produced content identifies the software agent and its principals or lack thereof.
5. Incomplete required inputs fail closed; defaults cannot impersonate observed facts.
6. Results remain claims with modality, uncertainty and applicability.
7. Dependency failure, digest mismatch or revoked critical dependencies prevents execution.
8. Past receipts remain interpretable after revocation or supersession.
9. Private evidence and personal records remain access-controlled and are not bundled into public
   package definitions.
10. Cultural and community knowledge follows its declared governance and access conditions.

## 12. Existing Qualia foundations

This programme should consolidate existing work rather than create parallel systems:

- `cml:LogicApplication` and subject-matter-selected evaluator routing in `core-ontologies/cml.n3`;
- capability claims, RPL and gap analysis in `core-ontologies/capability-credentials.n3` and
  `modalities/capability_gap.rs`;
- W3C/native credential issue and verification in `identity/credentials/` and
  `crypto/verifiable_credential.rs`;
- Q42 volumes, lexicon manifests and content-addressed resources;
- Vibe law packages and QualiaDB modality evaluators;
- qapp manifests for icons, permissions and content-addressed WASM;
- provenance, temporal graphs, receipts and authority attestations;
- current Health and Domain Lab instruments that provide migration fixtures.

The accompanying specification defines the shared contract. The implementation plan divides its
delivery into bounded, dependency-aware work packets suitable for parallel agents.
