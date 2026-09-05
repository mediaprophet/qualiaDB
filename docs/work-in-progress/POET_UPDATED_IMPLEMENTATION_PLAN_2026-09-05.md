# POET Updated Implementation Plan

**Status:** Work in progress  
**Date:** 2026-09-05  
**Branch baseline:** `0.0.36-dev`  
**Reconciled through:** `4eade061`  
**Frozen host contract:** `vibe-host-0.1` at `6dc2b8b8`

## Objective

Continue POET as a better human product while preserving the completed UX and
Health work, the frozen Vibe host facade, live capability bindings, ontology
intent, and contributions from every agent. Use the least expensive agent that
meets both the packet's difficulty rating and functional role. Agent identity,
vendor, or price does not confer ownership or authority; evidence and
project-owner decisions do.

## Platform-neutral routing

### Difficulty ratings

| Rating | Meaning | Typical work | Required control |
|---|---|---|---|
| `D1` | Clerical and deterministic | Indexing, formatting, extraction, link checks, status updates from supplied evidence | Exact scope and mechanical verification |
| `D2` | Bounded implementation | Focused UI/CSS, projections over accepted contracts, tests, fixtures, browser UAT | One packet, narrow files, focused regression |
| `D3` | Cross-module reasoning | Reconciliation, contract-preserving refactors, ontology joins, UI/catalog/invoke audits | Explicit invariants and independent verification |
| `D4` | High-assurance engineering | Authorization, clinical computation, ABI, permissions, storage, lifecycle, ingestion architecture | Strong model, adversarial tests, independent review |
| `D5` | Human-governed critical decision | Clinical claims, private keys, licence ambiguity, destructive migration, public ABI approval, gate closure | Project owner or qualified independent expert |

Difficulty is not a prestige ranking. A `D4` agent should not perform `D2` work
when a lower-cost agent can complete it with the same evidence.

### Functional roles

| Tag | Function |
|---|---|
| `COORD` | Reconcile plans, dependencies, status, and handoffs |
| `SPEC` | Define acceptance criteria, contracts, and normative wording |
| `UX` | Interaction design, accessibility, visual hierarchy, and UAT |
| `FE` | POET/browser implementation and projections |
| `RUST` | Core, host, ABI, persistence, and lifecycle implementation |
| `ONT` | RDF, SHACL, ontology, provenance, and mapping semantics |
| `DATA` | Dataset manifests, ingestion, validation, and licence metadata |
| `SEC` | Consent, authority, permissions, threat cases, and fail-closed behavior |
| `CLIN` | Clinical algorithms, units, applicability, provenance, and non-advice boundaries |
| `QA` | Test design, regression evidence, browser/native verification, and claim audit |
| `OPS` | Branch, release, packaging, and deployment coordination authorized by the owner |

An agent may hold several roles for one packet, but `D4` work requires a
separate reviewer or `D5` gate; self-review is not sufficient evidence.

### Cross-platform examples

| Difficulty | Grok/xAI style | OpenAI example | Gemini example | Other/local agents |
|---|---|---|---|---|
| `D1` | Planning/research function with exact output schema | Cost-sensitive/fast model at low or medium reasoning | Flash-class model | Deterministic script or small local model |
| `D2` | Named implementation or UX function | GPT-5.6 Luna, high | Flash-class model with high reasoning/tools | Coding model proven on the focused test |
| `D3` | Coordinator plus the relevant specialist function | GPT-5.6 Terra, high | Pro-class model, medium/high | Strong coding/reasoning model with long-context tools |
| `D4` | Security, systems, clinical, or architecture specialist plus independent review | GPT-5.6 Sol, high; GPT-5.5 medium/high for continuity | Pro-class model, high | High-capability model plus separate reviewer |
| `D5` | Project-owner gate support only | Human/expert decision support only | Human/expert decision support only | Qualified human reviewer |

These are examples, not fixed endorsements. Model names and prices change.
Before assigning a platform, confirm that the selected agent can read the
repository, edit safely, run the required tools, preserve concurrent changes,
and produce the packet's evidence. Grok-bot lane names may continue to describe
functions such as Capt./coordination, Neo/Rust seams, Vibe/language, Marvin/
ontology, and davinci/monet UX/visual work; other platforms should receive the
same role tag and acceptance contract.

### Escalation rules

- Escalate from `D1-D2` to `D3` when a task crosses ownership boundaries or an
  accepted contract is unclear.
- Escalate to `D4` for authority, clinical computation, key material,
  permissions, destructive migration, public ABI, or new host capabilities.
- Escalate to `D5` before clinical claims, key-vault/signing operations, licence
  decisions, destructive migrations, ABI approval, or closing Review Gate A/B.
- A higher-cost agent may audit lower-cost work; it should not reimplement valid
  work without a recorded defect.

## Cost controls

Every implementation session must:

1. Execute one packet only.
2. Read at most 12 implementation files, excluding the governing instructions.
3. Change at most 6 implementation files plus focused tests and WIP records.
4. Add no more than 700 net lines unless the project owner approves a split.
5. Run focused tests first and one broader check/build only after they pass.
6. Stop after two materially different failed approaches.
7. Avoid whole-repository formatting, unrelated repairs, package installation,
   external publishing, and dataset downloads.
8. Record baseline commit/status, changed files, exact verification, UAT,
   remaining gaps, and next packet.
9. Preserve concurrent work and never rewrite historical ledger entries.
10. Apply the Rust decomposition register's touch-before-grow rule. Do not hide
    an incomplete module move with path workarounds or warning suppression.

## Programme sequence

```text
Phase 0  Re-baseline and close the Health programme
   |
REVIEW GATE A
   |
Phase 1  Reconcile post-freeze POET chrome and Tool Chest
   |
Phase 2  Container / Manifold / Link and visual twins
   |
Phase 3  Governed assets and portable applications
   |
Phase 4  Webizen Desktop host and lifecycle
   |
REVIEW GATE B
   |
Phase 5  Broader domain restoration
```

## Phase 0 - Health completion and current baseline

### `RBL-01` - Concurrent-change baseline audit

**Difficulty/functions:** `D3` - `COORD`, `SPEC`, `QA`  
**Outcome:** Establish a current, evidence-backed baseline after the recent
multi-agent commits without changing product behavior.  
**Read:** WIP reconciliation, implementation ledger, current git history,
product-integrity tests, surface inventory, Health module routes.  
**Write:** WIP status/register only.  
**Acceptance:** every completion claim links to code/test/UAT evidence; plan
documents are not counted as implementation; unrelated `Cargo.lock` work is
preserved.  
**Verify:** current focused POET tests, surface-inventory test, `trunk build`.

**Result recorded 2026-09-05:** Audit completed at `4eade061` with the fetched
remote synchronized. `cargo test -p poet --test product_integrity` and
`cargo test -p poet --test surface_inventory` both stopped at compilation with
16 `E0583` errors. `browser/registration.rs` declares child modules whose
tracked implementation files are siblings under `browser/`. No test cases ran.
The unrelated modified `Cargo.lock` was preserved. The broader build was not
run because it reaches the same crate compilation gate.

### `FIX-REG-01` / `RM-01` - Restore the registration library

**Difficulty/functions:** `D2` - `RUST`, `QA`  
**Dependency:** `RBL-01`.  
**Outcome:** Give the existing purpose-specific registration files one coherent
module owner and make their links resolve without moving behavior, widening
capability bindings, or changing the frozen host facade.  
**Read:** `browser/registration.rs`, its 16 `register_*_toolbox.rs` siblings,
and the focused test entry points.  
**Write:** registration library/module routing only, plus a focused regression
if the current tests do not cover registration completeness. Permanent path
attributes and warning suppression are not accepted repairs.  
**Acceptance:** the registration split compiles; no toolbox is omitted or
duplicated; `Cargo.lock` and concurrent work remain untouched.  
**Verify:** `cargo test -p poet --test product_integrity`, then
`cargo test -p poet --test surface_inventory`, then `trunk build` if both pass.

**Result recorded 2026-09-05:** Completed as a behavior-preserving filesystem
decomposition. The router is now `browser/registration/mod.rs`; all 16 toolbox
modules are its directory-backed children. Product integrity passed 9/9,
surface inventory passed 1/1, and `trunk build` passed. No path attributes or
warning suppression were added, and the pre-existing `Cargo.lock` was unchanged.

### `RM-02` - Decompose the POET style asset

**Difficulty/functions:** `D2` - `FE`, `UX`, `QA`  
**Dependency:** `RM-01`.  
**Outcome:** Replace the 3,322-line embedded stylesheet in `browser/css.rs`
with purpose-specific CSS assets and a small deterministic composition point.
Preserve cascade order and runtime injection behavior.  
**Acceptance:** no visual redesign is mixed into the move; style groups have
clear ownership; focused suites and `trunk build` pass; desktop and mobile
screenshots establish visual parity.  
**Dependency rule:** complete this before adding further POET or Health styles.

**Result recorded 2026-09-05:** Completed with 14 purpose-specific assets, all
below 500 lines, and a 43-line Rust composition module. The assembled normalized
CSS hash matches the original. The focused CSS test, product integrity 9/9,
surface inventory 1/1, `trunk build`, and desktop/mobile browser checks passed.

### Rust touch-before-grow gate

Use `RUST_MODULE_DECOMPOSITION_REGISTER_2026-09-05.md` for subsequent work.
Before a packet adds behavior to a Rust file above 1,000 lines, first split the
touched responsibility behind stable imports/re-exports. This is a scoped gate,
not a requirement to refactor every large file before product work resumes.

### `HLT-R1` - Independent consent-contract review

**Difficulty/functions:** `D4` - `SEC`, `RUST`, `SPEC`, followed by `D5` review  
**Outcome:** Validate the existing `HLT-03` implementation rather than
reimplementing it.  
**Read:** consent contract, governance module, Health disclosure projection,
record API, existing tests, deontic/delegation facilities.  
**Acceptance:** principal and scope are immutable; expiry fails closed; only
the authorized principal can revoke; replay and reactivation fail; private
keys are absent; UI projection cannot broaden authority.  
**Write:** tests and the smallest defect repair only if evidence identifies a
real issue.  
**Stop:** any ABI, key-vault, missing-authority, or destructive change goes to
`D5` before editing.

### `HLT-07` - Clinical calculator workflow integrity

**Difficulty/functions:** `D4` - `CLIN`, `RUST`, `FE`, `QA`, followed by `D5` clinical review  
**Outcome:** Complete calculator forms against the real native algorithms.  
**Acceptance:** every required input and unit is explicit; patient values are
never fabricated as defaults; incomplete/inapplicable inputs cannot calculate;
the result names algorithm/version and provenance; the UI states that output
is not a diagnosis.  
**Verify:** native boundary-value tests and browser UAT using known fixtures.

### `HLT-08` - Health completion UAT pack

**Difficulty/functions:** `D2` - `QA`, `UX`  
**Dependency:** `HLT-R1`, `HLT-07`.  
**Outcome:** Exercise add measurement, reload, trend/table, correction, grant,
revocation, report-text ingestion, and offline recovery.  
**Write:** focused UAT document/tests and direct defects found by those tasks.  
**Acceptance:** each workflow has exact evidence and no completion claim exceeds
the observed host behavior.

### Review Gate A

The project owner or independent expert reviews Health architecture, consent,
clinical behavior, data contracts, screenshots, browser UAT, and status claims.
Do not begin governed assets or portable-app implementation before acceptance.

## Phase 1 - Post-freeze POET and Tool Chest

### `PFT-01` - Standalone/live semantics audit

**Difficulty/functions:** `D3` - `SPEC`, `FE`, `RUST`, `QA`  
**Outcome:** Verify that standalone POET, daemon-backed QualiaDB, and desktop
host behavior are visibly and semantically distinct.  
**Focus:** `requires_daemon`, local extraction, local Sentinel inspection,
local graph query, daemon fallbacks, action labels, and result provenance.  
**Acceptance:** local DOM/query results are never represented as live
`GraphDatabase.sparql`; denied/error states do not become success merely because
a fallback exists; every result identifies its source.

### `PFT-02` - Honest semantics repair

**Difficulty/functions:** `D2` - `FE`, `UX`, `QA` if label/state-only; `D4` - `RUST`, `SPEC` if a host/capability contract changes.  
**Dependency:** `PFT-01`.  
**Outcome:** Apply only the defects recorded by the audit.  
**Verify:** focused action-policy tests, product-integrity tests, web build, and
offline/daemon browser UAT.

### `PFT-03` - Select the next Tool Chest chain

**Difficulty/functions:** `D5` - project-owner decision supported by `COORD`, `UX`, `SPEC`  
**Outcome:** Choose one chain from the inventory based on user value and existing
live `ALL_BOUND` support.  
**Acceptance:** selected chain, user task, capability IDs, gated functions, and
UAT host are written before implementation begins.

### `PFT-04` - Implement the selected chain

**Difficulty/functions:** `D2` - `FE`, `UX`, `QA` when all bindings exist;
`D4` - `RUST`, `SPEC`, `QA` for an approved minimal catalog or invoke seam.  
**Dependency:** `PFT-03`.  
**Acceptance:** a person completes the task without reading capability strings;
every visible action is live or explicitly gated; no host widening or invented
dotted ID.

## Phase 2 - Shared product language

### `DES-01` - Davinci/Monet delta audit

**Difficulty/functions:** `D2` - `UX`, `QA`  
**Outcome:** Compare the chrome and visual plans with completed `UX-01` to
`UX-04`; create a checklist of genuine unmet acceptance criteria.  
**Acceptance:** completed accessibility, honest state, narrow layout, and
container hierarchy work is retained; no redesign is justified by plan overlap
alone.  
**Status (2026-09-05):** chrome delta landed on existing surfaces (`15-studio-chrome.css`,
diagnose glow, preview handle kinds, volume chips, CML+twins, media surfaces). UX-01–UX-04 not reopened. Map/G-COORD remains gated.

### `DES-02` - Motion and gated-state contract

**Difficulty/functions:** `D2` - `UX`, `SPEC`  
**Outcome:** Publish named entrance, dwell, and exit beats; reduced-motion
behavior; diagnose glow; and disabled-versus-broken visual rules.  
**Acceptance:** no free-tween system and no celebratory success motion for
denied/fault outcomes.  
**Status (2026-09-05):** contract in `poet-motion-contract.md`; beats and gated≠broken
are CSS-backed (`data-beat`, `data-honesty`, `data-volume-state`).

### `ONT-01` - Container/Manifold/Link shape contract

**Difficulty/functions:** `D3` - `ONT`, `SPEC`, with `D5` review for protected semantics  
**Outcome:** Reuse existing vocabularies to define content-shaped Container,
nested Manifold, typed Link, optional Volume/Position, and Layout/Stage/Timeline
references.  
**Acceptance:** persons and living/natural existence remain SHACL-first and are
not forced into an artifact-oriented OWL class taxonomy; technical terminology
remains formally correct.

### `COORD-01` - G-COORD activation

**Difficulty/functions:** `D4` - `ONT`, `RUST`, `SPEC`, `QA`  
**Dependency:** explicit project-owner gate after `ONT-01`.  
**Outcome:** Define the smallest approved shape/bind/dialect path for coordinate
system, realm, position, and time.  
**Stop:** no DNS/IP replacement claim and no host widening.

## Phase 3 - Governed assets and portable applications

### High-assurance packets

Use `D4` (`DATA`, `ONT`, `RUST`, `SEC`) for asset schema, source/licence policy, bounded ingestion,
evidence-preserving Quin mapping, portable manifest contracts, projection
adapters, permissions, and package lifecycle. These correspond to the earlier
`AST-01` to `AST-04` and `APP-01` to `APP-03` boundaries.

### Bounded product packets

Use `D2` (`FE`, `UX`, `QA`) for asset inspection UI, evidence browsers, Health packaging proof,
focused-app rendering, documentation, and UAT after their contracts are
accepted. These correspond to `AST-05` to `AST-07` and `APP-04` to `APP-06`.

Unknown licences, ABI changes, or clinical interpretations stop at `D5`.

## Phase 4 - Webizen Desktop

Use `D4` (`RUST`, `SEC`, `SPEC`, `QA`) for registry/storage contracts, package installation/removal,
permissions, managed paths, process lifecycle, and host parity. Use `D2` (`FE`, `UX`, `QA`) for
registry views, launch/stop UI, asset inspection, logs, accessibility, and UAT
after those contracts exist. Preserve the frozen four-operation Vibe facade and
script hot-edit behavior.

### Review Gate B

The project owner or independent expert reviews package security, permission
enforcement, managed paths, process lifecycle, cross-projection parity, and UI
architecture before broader restoration.

## Phase 5 - Broader restoration

After Gate B, continue bounded Project, Knowledge, and Studio workflows at
`D2`. Route governance authority, device authority, and social trust/security
contracts to `D4`. Apply `D5` review where rights, safety, cryptography,
clinical use, licensing, or destructive state changes are involved.

## Session handoff template

```text
Packet:
Platform and agent/model:
Difficulty and functional roles:
Reasoning/effort setting:
Baseline commit and git status:
User outcome delivered:
Files changed:
Tests and exact results:
Browser/native UAT:
Live/local/gated behavior:
Security/clinical/ABI/licence impact:
Known gaps and unrelated failures preserved:
Recommended next packet:
```

## Starting instruction

The next implementation session should execute `HLT-R1` only: independently
review the existing consent contract against authorization, immutable scope,
expiry, replay, one-way revocation, and private-key boundaries. Do not
reimplement valid work. Any ABI, key-vault, missing-authority, or destructive
change stops for project-owner review before editing.
