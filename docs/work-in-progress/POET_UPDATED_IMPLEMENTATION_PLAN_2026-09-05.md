# POET Updated Implementation Plan

**Current cross-repository programme:** [ST programme, 2026-10-11](#st-programme-2026-10-11--storytold-apps-poet-and-webizen-os). Read this first for Storytold integration, contextual orbs, supervised agents, and replacement of the Desktop UI. Its execution packets are in [the playbook, section 18](../POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md#18-st-programme-execution-packets-2026-10-11). The September material below is retained history; its old checkout paths and Desktop-UI destination are not instructions for the ST programme.

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
Phase 2  Container / Manifold / Link and visual aspects
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

**Result recorded 2026-09-05 (Grok, HLT-R1):** Independent review of
`governance/consent_contract.rs` plus Poet disclosure projection. Acceptance
items that already held: signed digest covers principal/scope; tamper fails
signature; `now >= expires_at` fails closed; only `grant.principal_did` can
revoke; grant stores verifying key + signature, not the signing secret.

Defects repaired (smallest change, no Host/ABI/key-vault widen):

1. `ReplayDetected` was unused. Bounded `ConsentLedger` (32 slots) now rejects
   duplicate `(grant_id, nonce)` and remembers revocations so omitting a receipt
   cannot reactivate.
2. `authorize_category_access` takes a receipt slice (not `Option`) and denies
   on any verified receipt for that grant.
3. `ConsentScope::from_labels` fails closed on empty/unknown labels
   (`clinical_notes`, `owl:Thing`).
4. Poet `project_shares` no longer defaults missing scope to "All categories"
   or missing expiry to Active.
5. Grantable UI categories match the five contract flags only.

Verification: `cargo +stable test -p qualia-core-db --lib consent_contract`
(12 passed); `cargo +stable test -p poet --lib health_views` (27 passed).
Share projection moved to `health_views/share_projection.rs` so `model.rs`
stays under 1,000 lines (746). `consent_contract.rs` is 733 lines after the
ledger addition — split is the next RM touch if this file grows again.

`D5` Review Gate A is **not** closed by this packet. Next product packet:
`HLT-07`.

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

**CLOSED 2026-09-06** — see `docs/work-in-progress/GATE_A_CLOSE_2026-09-06.md`.
The project owner accepted Health architecture, consent, clinical fail-closed
behavior, data contracts, source UAT contracts, and offline browser evidence,
with documented residuals. Governed assets and portable-app packets may begin
under their own acceptance rules. Review Gate B remains open.

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
diagnose glow, preview handle kinds, volume chips, CML+aspects, media surfaces). UX-01–UX-04 not reopened. G-COORD map remaps landed (Earth/Cosmos/Fiction).

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
**Status (2026-09-05):** remap bind landed on live `Cosmic.*` + `GraphDatabase.sparql`.
No new Host id. UTF-8 labels. QDNF remains design-only — do not implement the
network fabric until that spec is finished.

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

---

# ST programme 2026-10-11 — Storytold apps, Poet, and Webizen OS

## ST-A. Authority, status, and scope

**Status:** implementation specification written; no ST product packet is complete merely because this plan exists. **Planning baseline:** QualiaDB commit `9463dc62`, branch `0.0.40.8-dev`; Poet commit `3b0e463`, now on the same branch name. Recheck commits and dirty files before every session. At planning time the user-owned QualiaDB changes in `render/portal/body_scene.rs` and `render/portal/mod.rs` must not be overwritten.

The principal's requirements are: integrate all eleven locally supplied Storytold apps into Poet, native and WASM; maintain modified apps as independent repositories; move actions into toolboxes/tool chains and contextual menus; replace the contextual wheel with connected command orbs; express object-focused interaction as microscopic attention within a macroscopic Construct; use the upgraded Qualia graphics engine; provide first-class supervised-agent access; harmonise reusable app capabilities into QualiaDB; replace Webizen Desktop as the UI with the OS shell while retaining useful plumbing outside that obsolete UI.

For ST packets this chapter supersedes September instructions to work in `C:\Projects\qualia-27062026`, implement new Poet UI under `qualiaDB/crates/poet`, or make Webizen Desktop the final UI. Existing invariant, rights, privacy, licence, and ABI rules remain in force. This precedence is limited to the new programme, not unrelated work. No implementation session may treat this plan as authority to push, publish, spend on providers, delete user files, migrate production data, or change a protected ABI. User approval of the planning task is not approval of those actions.

Start with `ST-00`; execute packets from playbook section 18. Use the existing [session ledger](../POET_IMPLEMENTATION_SESSION_LEDGER.md) as the single append-only handoff record. Earlier completion rows concern earlier programmes and do not prove a new ST packet complete.

## ST-B. Verified source map and limits of the evidence

| Repository / path | Verified responsibility | Important qualification |
|---|---|---|
| `C:\github\qualiaDB` | Engine, Q42, Vibe, graphics, governance and reusable computation | Multiple crates; shared ownership is not a mandate to put everything in `qualia-core-db` |
| `C:\github\poet\wasm` | Extracted Poet browser UI, toolchest, containers, manifolds | The old `qualiaDB/crates/poet` directory remains but is not a listed workspace member |
| `C:\github\poet\native` | `poet-cli`: REPL, format, lint, eval | Not the graphical native Poet application |
| `C:\github\webizen-os-shell` | Dioxus shell, Halo, launcher, StageHost | StageHost is placeholder content; fixed peer/mixnet defaults are not real status |
| `C:\github\webizen-desktop` | Tauri host with native services and command wrappers | Extract useful implementations; do not retain this executable as a mandatory dependency |
| `qualiaDB/crates/webizen-studio/src/components/poet` | Older Dioxus Poet presentation | Reconcile useful behaviour, do not grow another independent Poet frontend |
| `qualiaDB/crates/webizen-render` | SDK/serde adapter to canonical graphics | Canonical drawing/device ownership is core `render/gpu`, not a second renderer |
| `C:\github\storytold\<slug>-main` | Eleven supplied upstream source snapshots | Treat these as read-only migration sources; some may not carry Git history |

This planning session inspected source, not runtime behaviour. Do not inherit build success, upstream command counts, latency, browser parity, or test percentages from this chapter. Measure the baseline in `ST-00`, retaining failures. Existing OS-shell README claims do not prove hosting works. Its current lockfile includes Dioxus Desktop, Wry, Tao and WebView2. The selected component framework is Dioxus; eliminating webviews is a separate decision, not a claimed consequence of removing Tauri.

Existing contracts to reuse:

- `qualia-core-db/src/q42/app_manifest/{manifest,codec,permissions,project}.rs`: canonical `PortableAppManifest`, v1 `Q42APP` wire contract and four host-neutral projection kinds.
- `q42/app_registry/`: bounded inspection/quarantine, not executable loading; current capacity is 32.
- `qualia-client-core/src/qapp_{registry,manifest,install}.rs`: legacy package, compiled capabilities, install/signature/lifecycle mechanisms to adapt, not a competing new app format.
- `q42/{journal.rs,asset_envelope/,volume/}` and `hypermedia/`: recovery, asset obligations, bounded range access and semantic lineage.
- `render/gpu/portal_gpu/{picking,settings,render_frame}.rs`, `render/navigation.rs`, `render/scene_graph/`, `render/hud/`, `render/lod_chain.rs`: semantic pick, camera, frame, scene, browser HUD and geometric LOD building blocks.
- Poet `wasm/src/tool_chest/core/{tool,tool_chain,manifest,registry,intent_bus}.rs`, `wasm/src/browser/{radial_menu,radial_gesture,command_palette,manifest}.rs` and `wasm/src/browser/topbar/{mod,menu}.rs`: current integration targets.
- Desktop `src/{supervisor,runtime,native_surface,mic_capture,companion_gateway,telemetry_bridge}.rs` and `src/commands/{poet,vibe_host,poet_daemon,poet_render}.rs`: extraction seams.

See [ADR 0012](../manuals/adr/0012-construct-is-the-distributable-composition.md) for Construct semantics and [ADR 0013](../manuals/adr/0013-portable-application-manifest-reconciliation.md) for manifest reconciliation. A Construct remains an observer's coherent scope; an app, subject or project is not automatically a Construct. Toolbox manifest, checkpoint, HCF/HMC composition and portable application package are distinct.

## ST-C. Ownership and dependency direction

### C1. QualiaDB substrate

Own semantic identity/graph/validation, Q42 assets and recovery, portable app identity and authority, command execution receipts, supervision policy, shared graphics/device management, Vibe, and reusable text, geometry, colour, imaging, media-time, DSP and computation. A primitive need not be graph-based to belong here. Split capability libraries by concern and features; no UI toolkit is allowed in the engine contract or reusable kernel crates.

### C2. Webizen host

Own window/surface creation, platform file handles, clipboard, notifications, devices, capture, native acceleration adapters, app process/worker lifecycle, gateway transports, OS key-provider handles and updates. The host implements authorised mechanisms, not a competing graph, permissions engine or private app store. Domain mail/wallet/health logic goes to its domain owner, not the general host.

### C3. Presentation and apps

The OS shell owns the operating-environment presentation. Poet owns authoring and Construct composition. Both use a shared Dioxus component library, common tokens, accessible controls and one contextual command model. The Qualia renderer draws specialised content/spatial surfaces. The shared UI library cannot depend on the shell executable, Tauri, Studio or an app engine.

Each fork owns specialised domain state/behaviour, format interoperability, an app adapter and tests. Reusable algorithms migrate into QualiaDB capability libraries; app adapters consume them. Standalone upstream egui/React builds remain regression references during transition, not permanent alternative embedded UI kits.

### C4. Planned crate homes (new paths, not present implementation claims)

| Planned home | Purpose and allowed dependencies |
|---|---|
| `qualiaDB/crates/qualia-app-contract` | Portable command/context/session/surface DTOs and validators; serde/CBOR utilities only, no engine/UI/process runtime |
| `webizen-os-shell/crates/webizen-host` | UI-independent platform and lifecycle implementations; narrow Qualia contracts/core features and target-specific OS adapters |
| `webizen-os-shell/crates/webizen-ui` | Dioxus components + pure menu/orb/navigation models; contract crate, optional graphics adapter, no legacy desktop dependency |
| `poet/shared` | Poet-specific composition model shared by CLI/browser/native projection; contract and UI dependencies only where required |
| `<app-repo>/crates/poet-adapter` | One engine/session adapter; engine crates + app contract, no global shell state |
| `<app-repo>/apps/poet-wasm` | Worker/WASM execution entry, not another ribbon shell |
| `<app-repo>/apps/poet-worker` | Native managed worker entry, no stable Rust dynamic-library ABI assumption |

`ST-02` must check for an existing suitable contract crate before creating the planned crate. If it reuses one, record one explicit replacement path for every consumer; never create duplicate contracts. Root Cargo files may become workspace+package roots to accommodate host/UI children. New shared libraries have independent narrow feature gates. Do not turn the graphics adapter or widgets into a default inference/CUDA dependency.

Local fork working-directory default is `C:\github\poet-<slug>` for the eleven slugs below. Local creation/copy may proceed after `ST-01` records the source and correct parent; remote owner, public repository creation, final branding and relicensing require the principal. Keep source notices; choose no remote or new licence implicitly.

## ST-D. Execution modes and loading decisions

| Mode | Meaning | Mandatory proof |
|---|---|---|
| Browser-local | App WASM executes in a dedicated worker where supported; no daemon needed | Offline open/edit/checkpoint/reopen/export |
| Native-backed browser | Explicitly connected authorised native services enhance the same composition | Transport/source label, daemon-loss handling, unchanged document meaning |
| Native OS shell | App native worker + host device/surface services | Managed lifecycle, same state IDs/schema and policy as browser |

Native plugin/app execution uses managed worker processes with versioned, length-delimited CBOR IPC. Do not load third-party Rust dylibs into the core engine. Browser-local uses message-based workers and transferable buffers; agents and UI use typed command envelopes. Isolation/resource enforcement must pass the reviewed loader packets before external code is enabled. A worker boundary alone is not a sandbox.

Large data crosses boundaries as authorised payload handles or bounded binary chunks, never base64/JSON per-frame pixel/sample buffers. Native frame/surface presentation uses the existing shared-device renderer or a validated binary frame path, not PNG polling for interactive content. Keep GPU ownership explicit: external workers do not get forged device pointers. CPU output/renderer upload is the initial portable seam; shared-GPU execution is enabled only through reviewed host APIs.

Loading states: discovered, validating, incompatible, quarantined, ready, starting, running, stopping, stopped, failed, revoked. Inspection never executes package code. Missing optional capabilities degrade explicitly; missing required capabilities prevent launch. Updates stage beside the active version; quiesce/checkpoint sessions, validate migration and activate atomically. Failed migration keeps old package/state usable. Uninstall removes only exact managed package artifacts with explicit confirmation; it does not delete user documents/assets.

Do not invent an archive extension or replace `PortableAppManifest`. Package resources reference command catalogue, toolbox manifests, surface descriptors and engine entries under validated relative paths. Use verified governed resources/HCF/HMC where appropriate. The v1 manifest has bounded list ceilings; do not put thousands of command rows into its 64 required-capability rows. Required capabilities declare service families; command catalogue is a separate digest-verified resource. A required manifest/schema change needs an explicit versioned review, not unknown fields smuggled into the codec.

## ST-E. Shared command, context and session contracts

These are the normative v1 interface shapes for `ST-02`, not proof that exports already exist. Native Rust names follow the table; wire names are snake_case. Use explicit schema versions and tagged enums; unknown versions/tags fail closed. Identifiers are nonempty opaque strings in cold authoring/transport. Runtime maps compile to bounded hash/slot tables with collision detection and full-identity resolution; they do not modify the NQuin ABI. All u64 identities/revisions/ticks crossing JSON/JavaScript use validated decimal strings, not lossy JS numbers.

| Type | Required fields |
|---|---|
| `CommandDescriptor` | `schema_version`, `command_id`, `command_version`, `app_id`, `upstream_id`, `label_key`, `description_key`, `input_schema`, `output_schema`, `target_kinds`, `effects`, `required_capabilities`, `undo_policy`, `interaction`, `menu`, `toolbox`, `tool_chain`, `orb_group`, `shortcuts` |
| `CommandTarget` | `session_id`, `document_id`, `document_revision`, `object_ids`, tagged `selection`, optional `subtarget`; selection order retained |
| `InteractionContext` | `context_id`, `principal_id`, `construct_id`, `manifold_id`, `container_id`, target, `focus_origin`, `editing_mode`, optional `pointer_hit`, `navigation_depth`, `capability_snapshot_revision` |
| `CommandRequest` | `request_id`, descriptor ID/version, target, validated `params`, `expected_revision`, `actor_id`, optional `delegation_id`, optional `approval_id`, `idempotency_key` |
| `Availability` | available / disabled / hidden with reason code, localised reason and missing prerequisite; disabled is not an execution grant |
| `CommandReceipt` | request/actor/app/command/target identities, before/after revision, status, effects, result or job handle, warnings, provenance reference, authority decision reference, actual resource/cost fields or explicit unavailable |
| `AppSessionDescriptor` | app/package/engine/schema versions and digests, session/document IDs, open mode, revision, dirty/durable state, execution source, capabilities and payload references |
| `SurfaceDescriptor` | surface ID/version, content/panel/widget kind, document binding, view type, required capabilities, input kinds, accessibility projection, state schema, resource budget class |
| `JobDescriptor` | job/request/actor/target IDs, state, completed/total units when known, cancellability, external-effect state, bounded diagnostics, result/provenance references |

Command IDs use `<app-id>::<upstream-id>` with a validated separator rule; shared command aliases reference concrete IDs and supported target contracts. Original IDs remain stored for CLI/script compatibility. No duplicate global ID is accepted. Existing human-readable parameter descriptions are discovery inputs, not valid JSON Schema automatically: write strict schemas from handlers and tests; reject unknown keys, malformed values, bad IDs, NaN/infinity and over-budget arrays/strings.

`effects` is a set drawn from read, document_mutation, workspace_mutation, filesystem_write, network_disclosure, device_capture, external_charge and package_lifecycle. Multiple effects are allowed. Interaction is discrete, gesture, query, view or job. Undo is none, reversible_document or compensating_external; do not promise undo for remote disclosure/payment/deletion.

App adapter methods: `describe`, `open`, `availability`, `inspect`, `execute`, `begin_gesture`, `update_gesture`, `commit_gesture`, `cancel_gesture`, `snapshot`, `restore`, `poll_job`, `cancel_job`, `close`. Each consumes a typed request and returns a typed result/receipt; programmatic calls never open dialogs. Blocking codec/render/export work is a job. Gestures begin against a revision snapshot, update previews, commit exactly once and cancel to the starting state. Closing a view releases view resources, not its document.

The dispatch boundary revalidates targets, expected revision, schemas, capability availability, principal/delegation/approval and effects. UI filtering is advisory. Do not bypass this boundary from MCP, CLI, shortcuts, native menus, or a worker. Existing frozen `vibe-host-0.1` stays unchanged; new catalogue bindings must be separately reviewed and added consistently to real discovery/invoke/WASM coverage. Never manufacture a dotted ID to make a button appear functional.

## ST-F. One UI command system and macro/micro navigation

### F1. Context resolution

Menu bar resolves the active focused document/selection; right-click orb resolves the hit target. Hit-testing order is: active modal, UI control, specialised content subtarget, container frame, manifold background. A header click means container actions, not image/clip deletion. Empty background means placement/navigation. Clicking within a selected object group preserves the group; clicking outside retargets only according to the adapter's explicit selection policy. Text selection survives opening any menu. Resolve a context snapshot on open and validate again on execution. Asynchronous picks carry pointer request ID, surface/frame revision and camera revision; ignore late results for a superseded request or destroyed target.

Adapters provide text span/grapheme positions, cells/ranges, layer/path/entity IDs, track/clip/time ranges, and scene semantic IDs. Do not interpret screen geometry as data identity. GPU mesh IDs resolve through the existing retained frame snapshot. Renderer semantic anchors are not automatically editable scene-object handles.

### F2. Contextual menu, toolchest and palette

Shared menu model supplies stable `Poet | File | Edit | <domain menus> | View | Window | Help`; domain groups change on deliberate focus, not hover. Include checked/radio states, shortcuts, disabled reasons and a target title. Domain menu contributions merge by declared group/order; deterministic conflict detection, no label-based dispatch. Save document, save workspace checkpoint, export and close view are separate commands. Undo label names the document operation; workspace layout undo is a different history.

Toolbox and tool-chain assignments reuse the same descriptors. A tool chain remains a grouped set of tools; automation is a separately typed workflow with dependencies and failure semantics. Search palette includes installed/available commands, explains disabled ones, and dispatches IDs. Native macOS menu is an adapter over the same model; browser and other native hosts render the shared components. Shortcut conflict resolution is deterministic: active modal, focused editor, container, workspace, shell. Never capture OS-reserved keys silently.

### F3. Orb graph

Replace radial sectors with `OrbNode` (ID, context snapshot, target summary, group ID, placement, expanded state) and `OrbEdge` (parent, child, submenu/parameters/retarget link). Root opens on right-click; touch long-press and keyboard context key are equivalent. No action executes merely by expansion, hover, pinning or camera movement. Child groups retain context except an explicit retarget edge; show the changed target. One root and one expanded path by default, with explicitly pinned groups; cycles refused and depth bounded to the existing Construct nesting ceiling (six). Pinning is local presentation state, not a grant or permanent user-data copy.

Use stable group ordering, up to eight primary entries plus More; never adaptive profiling/ranking. Dismiss outside/Escape, back/collapse, return focus, viewport-edge clamping, pointer capture, pen handling and touch movement cancellation are covered. Closing a container is not deleting its document. Destructive/external actions require the dispatch policy even if displayed on the orb. The accessible tree/list has the identical command IDs and availability. Orb appearance may be spherical, but actionable labels cannot hide on its back; do not require rotation to discover commands.

### F4. Attention scale

Macroscopic means the observer's current Construct composition. Microscopic means a focused document, object, subpart, command group or parameters within it. Scale is relative; a film sequence can be macro relative to a keyframe. `AttentionFrame` stores context, focus target, view/camera, selection, breadcrumb and previous frame. Right-click opens the orb without camera motion. Focus/Explore explicitly pushes a frame; Back restores view/selection without undoing edits. An overview/minimap or backdrop preserves global location. Semantic depth and geometric LOD are independent. No new global app/scene/session occupant kind is introduced.

### F5. Graphics integration

Use core PortalGpu/shared device; webizen-render remains an adapter. Object framing derives target and distance from bounds, not the origin-oriented `camera_frame_node` alone. Maintain camera/selection histories and reduced-motion snap transitions. Use normal HUD/screen-space labels and focus controls above spatial orb geometry; essential text/approval controls must not receive bloom/temporal ghosting. UI HUD is not proof of native accessibility; provide host-accessible controls. New graph layout is bounded and deterministic and does not add unlimited render objects.

Handle resize, device-pixel ratio, occlusion, depth-tested picking, camera changes, asynchronous WASM readback, device loss and resource release. Native fallback may use bounded CPU/binary-frame rendering; browser fallback uses the same command model with 2D orb/tree presentation. Lack of a GPU must not make consent or supervision unusable. Do not claim true spatial object editing merely because a point anchor is pickable. Temporal history invalidation follows existing camera/reset APIs. Render frame budgets are negotiated/recorded, never borrowed silently from inference or capture.

## ST-G. First-class supervised software agents

Agents are principals' delegated actors, not UI automation with unrestricted engine access. Agent inspect/discovery uses the same command/context/session contracts, via a transport adapter; screenshots are optional visual evidence, not the authority channel.

`TaskDelegation` records principal, actor, app/package version, goal, exact target scopes, allowed command/effect/capability sets, expiry, maximum steps, CPU/GPU/memory/storage/network budgets, optional external-cost currency/minor-unit ceiling, approval policy and revocation state. Scope references stable objects/graphs/assets; document text and display hints never create authority. Missing required accounting/enforcement means refuse the corresponding delegated operation, not invent available budget.

Modes: Observe (read only), Propose (draft/preview only), ApproveEachAction, BoundedDelegation. Drafts live in an isolated proposal overlay with an immutable base revision and digest; they cannot modify authoritative state or initiate external effects. `Approval` binds proposal digest, actor, target/revision, params, effects, cost/disclosure summary and expiry. Changed material parameters/targets/revision invalidate approval. No string 'approved', opened orb or expanded submenu is a grant. Actual grants remain host/core decisions.

Lifecycle: submitted, validating, awaiting_approval, approved, running, paused, cancelling, completed, failed, cancelled, revoked. Pause prevents new dispatch but may let an indivisible in-flight operation finish; cancellation acknowledgement states what actually stopped. Revocation blocks the next effect boundary and subsequent commands. External request already sent may be irreversible or still billable: receipt states that explicitly. Retries of external operations reuse an enforceable idempotency key or require a new approval; transport timeout is not proof of failure.

Supervision UI offers inspect/propose, exact target, diff/preview, planned effects, budget source/remaining state, approve/revise/reject, pause/cancel/revoke and chronological receipts. Human and agent changes have separate attribution. Cost unavailable stays unavailable, not zero. Journals include Who/When/Why/What/Where/Cost only when measured; do not claim signed audit unless signing is wired and verified. Private keys never enter schemas, logs or browser state.

Explicit negative tests: denied command via CLI/MCP; wrong actor/app/version; expired/revoked delegation; outside scope; stale approval; altered params; duplicate external request; budget exhaustion; injected instructions in imported content; fabricated command/capability; missing accounting; concurrent revocation. The existing DesktopSupervisor is operations plumbing, not the policy implementation. Gate G2 requires independent security review before delegated mutation/external effects are enabled.

## ST-H. Authoritative state, persistence and collaboration

An editable domain state is not the semantic projection alone. Maintain one logical document identity binding domain snapshot, semantic facts, immutable originals, process/version parameters, opaque interchange parts, derived previews and provenance. Preserve app-specific precision and full editable representation. CML annotations anchor to stable content spans/objects and revision mappings, not guessed pixel coordinates.

Commit protocol: verify expected revision and authority; build bounded draft; validate domain invariants and semantic delta; stage immutable payloads; append prepared transaction with before revision and payload references; append durable commit containing domain/graph references; publish new revision; acknowledge saved only after durability. Recovery publishes complete commits only, discards/quarantines incomplete staging, and retains prior state. Extend existing Q42 journal/versioned sidecars if required; do not claim it already atomically stores arbitrary app payloads. Scope a schema/codec change explicitly; no two independent stores each claiming truth. Failure injection must cover every step including graph publication and acknowledgement.

Browser durable state uses OPFS/range access and quota checks; native uses authorised file/storage services. localStorage is only small preferences/legacy checkpoint import, never bulk document/asset storage. Original source files are immutable by default. Asset relocation/relinking checks content digest/permissions. Cache eviction never drops an only copy. Authenticated actor replaces default principal attribution when present. Unknown fields/interchange parts are preserved or a clear loss report prevents silent overwrite.

Read-only shared views and revision-checked single-writer authoring ship first. Concurrent editing is not claimed from LWW layout conflict handling. Domain concurrency then specifies rich-text sequence operations, cell/formula conflicts, object/layer order, timeline ranges and CAD constraint conflicts independently. Binary/plugin payloads remain immutable versioned assets. Convergence, causality, stale-target, deletion-vs-edit, reordering and reconnect tests are mandatory. Realtime audio/render execution consumes immutable snapshots outside the collaboration merge loop.

## ST-I. Primitive harmonisation programme

Classify every app dependency/function as reuse_existing, promote_to_qualia, retain_domain, platform_adapter or unsupported_upstream. Record exact existing owner/API, semantics, precision, CPU/GPU oracle, memory class, licence, callers and replacement test. A common name is not evidence of equivalence. No blanket copying app engines into `qualia-core-db`. Move algorithms with attribution and focused tests; switch both old/new consumers only after equivalence. Compatibility wrappers preserve older state and script semantics.

| Primitive family | Required shared capability | Acceptance boundary |
|---|---|---|
| Identity/assets | Stable document/part IDs, payload references, semantic lineage, derived obligations | Cross-app joins resolve the same identities, not file-path aliases |
| Text/layout | Unicode shaping/bidi/IME, styles, pagination, anchors, rich-text structures | Layout and editable source retained; no plain-text/HTML flattening |
| Sheet/calc | Typed values/errors, coercion, dates, dependency graph, spills, cycles, pivots/charts | Preserve spreadsheet formula dialect; Vibe is explicitly selected, never substituted |
| Imaging/colour | Caller-buffered tiled surfaces, per-depth/channel formats, alpha/ICC, masks, effect graph | Existing whole-layer RGBA8 model is insufficient; do not silently downsample |
| Curves/geometry | f64/unit-aware paths/CAD, booleans, snapping, constraints and display lists | f32 render projection is derived; authoritative CAD data retains precision |
| Time/animation | Exact rational ticks/frame/sample/tempo mapping and interpolation | Existing f64-seconds authoring model is insufficient for timeline semantics |
| Audio/DSP | Bounded blocks, automation/routing, resampling, delay compensation, spectral view | No allocation/logging/blocking in callback; recording/source decoding is bounded/streamed |
| Rendering/codecs | CPU oracle, GPU capability intersection, cache invalidation, codecs/interchange | Shared device, bounded jobs, preview/export agreement and honest fallback |
| Jobs/hosting | Progress, cancellation, admission, durable receipts, workers/device providers | No decorative progress/meter or unchecked host effect |

Do not put every pixel/sample into a Quin. Semantic facts use Quins; large numeric/blob state uses validated payload/tensor/asset formats. Use canonical `frame_layout` packing helpers; historical AGENTS sections disagree about old float tags, while its resolved section specifies `INLINE_TAG_FLOAT`. Recheck live definitions; do not allocate tags/opcodes here. Preserve 48-byte ABI, existing zero-heap two-tier model, 42MB pass budgets and determinism. Large media passes tile/stream/partition; app cache sizes are not authorised Sentinel exemptions. Admission accounts CPU, GPU, browser memory, queue and disk separately. Parser recursion, compressed expansion and input-derived allocation are bounded.

## ST-J. App work orders and fidelity cases

All rows require the generic app migration procedure in playbook section 18 plus the specified domain cases. No row is complete from its initial demonstration alone. Every engine command, UI-only action, gesture, format, device/plugin hook and claimed workflow from the pinned baseline must have a coverage row and disposition. Native-only/unsupported-upstream limitations are explicit and reviewed; migration-created loss is a defect, not 'unsupported'. Upstream README counts are not coverage evidence.

| Slug / packet | Source seams | Poet destination / toolbox chains | Required regression scenarios |
|---|---|---|---|
| `gridcraft` / ST-20 | `crates/{core,formula,functions,model,calc,xlsx,chart,engine}` | Virtual grid/chart/pivot containers; Cells, Formulas, Data, Styles, Charts | XLSX formula/styles/validation/chart/pivot round-trip; dynamic spill + cycle; insert row adjusts references; undo one fill; 1900/1904 dates; canonical vs locale formula syntax; unknown-part loss warning |
| `wordcraft` / ST-21 | `crates/{doc,fonts,layout,docx,docbin,formats,engine,zotero}` | Paginated/flowing doc, outline/review; Typography, Paragraphs, Styles, Tables, References, Review | IME/grapheme/bidi selection; list Enter/Tab; table pagination/header/footer; comments/track changes; DOCX and alternate format round-trip; CML anchor after edit; nonexecuting macro preservation; Zotero contract where available |
| `deckcraft` / ST-22 | `crates/{model,text,anim,media,pptx,render,engine}` | Slide/sorter/notes/presenter; Layout, Shapes, Media, Animation, Present | Master/layout/theme inheritance; unknown opaque XML; editable connectors/text; keyframes/transitions; PPTX round-trip; audio/video synchronisation; presenter state distinct from editable content |
| `pdfcraft` / ST-23 | `crates/{cos,model,render,annot,forms,redact,sign,ocr,automation,engine}` | PDF page/thumbnail/forms/review; Organise, Forms, Review, Redact, Sign | Original bytes retained on incremental save; unknown objects; threaded annotation/forms; encrypted recovery; signed-file edit warnings; full redaction removes recoverable content; PDF scripting sandbox; OCR missing model/device honest |
| `vectorcraft` / ST-25 | `crates/{doc,pathops,brush,text,effects,tools,render,svg,pdf,format,engine}` | Artboard/object/appearance; Paths, Shapes, Type, Brushes, Effects | Pen begin/preview/commit/cancel; exact path booleans; shape-builder/symbol/appearance editability; SVG/PDF/native formats; legacy DrawCraft import; text and transforms; accessible object tree |
| `photocraft` / ST-26 | `crates/{raster,color,cms,doc,compose,gpu,algo,psd,io,engine}` | Tiled image/layer/mask/histogram; Paint, Selection, Layers, Adjustments, Filters | PSD/PSB unknown block preservation; U8/U16/F32 and channels; ICC/alpha; smart objects/filters/layer styles; pressure gesture; large sparse tiles; undo brush one step; preview/export CPU/GPU differential |
| `lightcraft` / ST-27 | `crates/{raw,meta,develop,pipeline,catalog,preview,gpu,engine}` | Asset gallery/develop/comparison; Library, Metadata, RAW, Masks, Presets, Batch | Immutable RAW original; EXIF/XMP; process version pins old edits; virtual copies/albums/smart albums; catalogue torn-write recovery; masks/colour-profile comparison; missing originals relink; batch cancellation |
| `cadcraft` / ST-29 | `crates/{geom,dxf,doc,constraints,render,io,dwg,engine}` | Precision viewport/paper layout/constraints; Draw, Modify, Snap, Layers, Annotate, Plot | f64 units/coordinates; prompt/transparent commands; snapping; associative dimension after move; conflicting constraints; nested block cycles; DXF round-trip/loss report; native DWG vs browser-only DXF; no unsupported 3D-solid claim |
| `soundcraft` / ST-31 | `crates/{time,audio-io,midi,dsp,model,mix,playback,engine,clap-host,vst3-host,au-host}` | Timeline/mixer/MIDI/score/automation; Edit, Route, Mix, DSP, MIDI, Record | Sample-exact clip/fade; tempo ramps; routing + delay compensation; merged fader undo; realtime callback discipline; bounded recording; offline stems; plugin missing/state restoration and worker crash; browser worklet/capture permission |
| `filmcraft` / ST-32 | `crates/{time,frame,media,project,edit,codecs,render,gpu,export,interchange,engine,platform}` | Timeline/monitors/bins/scopes/captions; Trim, Effects, Grade, Audio, Titles, Export | Exact 23.976/29.97 frame/timecode; linked audio moves; ripple/roll/slip; keyframe and sequence nesting; GOP seek/range reads; preview/export parity; captions/interchange; cancellation + recovery; WebCodecs fallback; no UI-thread starvation |
| `artcraft` / ST-33 | Rust `crates/{desktop/artcraft,api_clients,schema/database/sqlite_tasks}`; `frontend/apps/artcraft`, state/scene libraries | Composition/scene/camera/assets/jobs; Compose, Pose, Camera, Mask, Generate, Variants | Pose/camera/mask state survives reopen; queued/running/failed/cancelled tasks; result lineage; provider-supported models only; timeout/idempotency prevents duplicate charge; no automatic remote disclosure; remove nonconsented analytics |

App source architecture documents are at `<slug>-main/docs/architecture.md` except ArtCraft, whose root AGENTS, root Cargo.toml and frontend/package.json define its mixed Rust/TypeScript architecture. Source `AGENTS.md`, licences/notices and assets lists must be read for the touched app before coding. Do not invoke proprietary reference software or copy proprietary assets in violation of those rules.

### J1. ArtCraft-specific boundaries

Production backend lives in a separate `artcraft-services` repository absent from the supplied tree. Do not claim rebuilding remote generation services from client stubs. First ship local composition, scene authoring and durable task projections; connect only provisioned, authorised official provider APIs. Provider browser-emulation/session-cookie bypass mechanisms are not migrated. Never crawl credentials. No external call, paid generation, account migration or production query is part of an ordinary test packet; synthetic provider doubles cover job semantics. Signing/payment/provider credentials require a principal-selected provisioned host provider.

### J2. UI-only coverage

Inventory app-shell commands separately from engine commands. Replace dialogs with typed panels, pointer tools with gesture adapters, screenshots/control functions with the common inspect/visual-test interface, and standalone window actions with host/window services. Fonts, keyboard layouts, localization, IME, accessibility, clipboard formats and tablet input are real migration features. Codecs copied across apps must not be consolidated merely by filename; compare codec/version/profile and oracle fixtures first.

## ST-K. Dependency gates and incremental delivery

```text
ST-00 baseline -> ST-01 local forks/rights -> ST-02 contract review (G1)
  -> host extraction ST-03/04/05
  -> package + worker ST-06/07/08 and durable state ST-09
  -> shared UI ST-10, registry/context ST-11/12
  -> menu/orb/attention ST-13/14/15/16
  -> agent policy ST-17 (G2) -> supervision UI ST-18
  -> generic app adapter ST-19 -> Grid ST-20 -> Word ST-21 (G3)
  -> Deck/PDF ST-22/23
  -> visual core ST-24 -> Vector/Photo/Light ST-25/26/27
  -> precision geometry ST-28 -> CAD ST-29
  -> time/DSP ST-30 -> Sound/Film ST-31/32
  -> Art ST-33 -> harmonisation audit ST-34
  -> cross-app ST-35 -> collaboration ST-36
  -> release ST-37 -> Desktop retirement ST-38 -> final G4 ST-39
```

Some internal tasks may run in parallel after dependencies pass; never share a dirty file or global registrar between concurrent agents. `ST-03` starts with toolkit-free supervisor extraction, not the whole desktop rewrite. `ST-20/21` are first complete vertical proofs but later ST-34 and G4 validate shared primitive consumption and all baseline coverage. Contracts/authority gates G1/G2 use independent review; G3 checks two actual app workflows across execution modes; G4 checks all app/domain coverage and retirement evidence. Gates do not reopen unrelated completed Health programmes.

## ST-L. Verification and evidence

Work packets name new tests explicitly; future test paths/filters are planned until implemented. A zero-test filter run is not a pass. Use existing native unit tests for pure code, synthetic fixtures for parser/provider behaviour and browser/native UI assertions for presentation. Record exact commands, target/features, exit status, actual tests run, artifacts and missing hardware. Never turn 'compiles' into 'works'.

At baseline, collect `cargo metadata --no-deps --format-version 1` and `cargo tree -e features` in the relevant standalone workspace; do not launch every build. Verification recipes:

- Qualia focused test: `cargo test --manifest-path C:\github\qualiaDB\Cargo.toml -p qualia-core-db --lib <filter>`; browser engine check: same manifest, package, `--target wasm32-unknown-unknown --no-default-features --features portal`. Choose a narrower actual profile for nonrenderer work.
- Poet: `cargo test --manifest-path C:\github\poet\Cargo.toml -p poet --lib <filter>`; `-p poet --test product_integrity` for changed presentation; `cargo check --manifest-path C:\github\poet\Cargo.toml -p poet --target wasm32-unknown-unknown`; `trunk build` from `C:\github\poet\wasm` only when this frontend changed.
- OS shell: `cargo check --manifest-path C:\github\webizen-os-shell\Cargo.toml` for native. The README's `--features desktop` is not declared as a package feature at planning baseline; do not use it blindly. Browser needs target-gated Qualia dependency profiles before `cargo check ... --target wasm32-unknown-unknown` can be assumed supported. `dx build --platform web` only after baseline confirms installed CLI syntax and target dependencies.
- Planned crates: run `cargo test --manifest-path <owning-root> -p <crate> --lib <packet-filter>` after the packet creates/registers them. Contracts/UI-model tests should not require graphics hardware.
- App forks: preserve existing app `cargo xtask layers`, asset/clean-room and wasm gates; run the adapter's focused tests each session. Once per completed app migration, run its existing `cargo xtask ci`; record any corpus/hardware omissions rather than changing floors/security settings.

Performance evidence is measured, host-specific and not a completion percentage: frame/input latency, callback underruns, import/render/export throughput, CPU/VRAM/worker memory, cancellation delay, storage growth and schema/payload sizes. Capture before/after under equivalent workloads and list device/browser/backend. Budget limits come from existing policy or an explicitly accepted profile, not arbitrary increased thresholds. `ST-00` records source and `ST-39` verifies enforcement. Deterministic or numerical oracle tests precede performance tuning.

UAT must exercise browser-local with daemon absent, native-backed with connection loss, native shell, GPU available/unavailable, reduced motion, keyboard/list alternative, touch/pen where available, large input, malformed input, crash/reopen, denied permission and stale target. Never take an unrelated full-desktop screenshot. Retained captures use caller-selected artifact directories with explicit byte limits; clean up only owned scratch.

## ST-M. Decisions and release criteria

Defaults specified here let implementation start without rediscovering architecture: Dioxus components, Qualia graphics, one canonical portable manifest, process-isolated native engines, worker WASM engines, revision-checked single-writer first, no automatic external effects and no legacy Desktop executable dependency. A reviewer may change a default only through a recorded contract update with affected packets/tests.

Principal-only decisions: public repository owner/names/branding; licensing/relicensing; trusted signing identity and provider credential provisioning; paid/network effects; genuine new ABI/tag/opcode; destructive migration/deletion; accepting unsupported-feature exceptions; fully webview-free native renderer; release/push/publication. Until resolved, implement local testable work and mark the affected release/external action blocked. Do not hold every unrelated packet hostage to a remote/signing decision.

Release criteria: eleven independent buildable app forks and installable compatible packages; all pinned baseline rows accounted for; actual contextual orbs/menu/toolchest/palette use one dispatcher; macro/micro navigation restores view without unintended mutation; supervised-agent boundaries proven; real durable cross-mode editable state and format fidelity; shared primitives consumed with no silent dual truth; native platform capabilities labelled; accessible CPU fallback; no Tauri/egui/React requirement in canonical presentation; useful Desktop services extracted before UI retirement; narrow Cargo dependency graph; update/rollback and user-data retention verified. Completion requires evidence and G4 review, not checked prose or renamed generic forms.

## ST-N. VibeScript as the canonical application language

### N1. Product goal and authoritative extension policy

VibeScript replaces handwritten JavaScript/TypeScript application scripting in Poet, imported apps and the Webizen environment: app orchestration, commands, reactive state, event handlers, declarative presentation, workflows and supervised-agent programmes. Rust remains the compiled implementation language for engine/host/components; Vibe scripts edit/hot-reload without rebuilding that Rust host. This is a three-layer system, not a requirement to rewrite codecs, Dioxus, operating-system drivers or GPU kernels in interpreted code.

Read [VibeScript Core](../manuals/standards/vibescript-core.md), especially sections 0, 4-6, 8, 11, 14-17, and [current host DevRel](../vibe/devrel-frozen-host.md) before extending. The current dialect may grow through versioned documented changes. The four operations parse/check/diagnose/capability.invoke are the stable facade; `vibe-host-0.1` does NOT freeze `ALL_BOUND` or the grammar. Historical freeze wording in this plan/playbook means do not break that facade or bypass review, NOT leave a necessary app capability permanently unavailable. Additive catalogue/library growth is expected. Breaking source, AST, bytecode, effect or wire semantics require explicit version negotiation and migration; never silently redefine an old version.

Full support means every inventoried required application-script behaviour has a Vibe implementation or an explicit platform adapter with conformance evidence. It does not mean ECMAScript source compatibility, JS prototype/coercion quirks, executing arbitrary npm modules, DOM access, `eval`, `new Function`, or embedding another language such as Rhai/QuickJS as a disguised Vibe engine. No JIT, raw Quin overlay or ambient credentials/network access is introduced.

Generated wasm-bindgen loader glue and required browser event/worklet shims are transport/platform artifacts, not an application-language escape hatch. Record each retained handwritten JS file/function by owner, API necessity, behaviour, budget, test and replacement route. Prefer Rust/WASM host adapters; if platform constraints require a small JS worklet/loader bridge, it contains no business rules, command decisions, authority, state truth or provider logic. Removing every platform byte of JavaScript is a separately reviewed deployment goal. Do not claim 'no JS' merely because code was generated or renamed.

### N2. Source seams and confirmed evidence boundaries

- `crates/vibe/src/{lex.rs,ast.rs,types.rs,value.rs,check.rs,effects.rs,diagnose.rs}`, directory-backed `parse/`, `eval/`, `grammar/`, `bytecode/` and `cbor_ast/{mod,codec,decode,encode}.rs`: language/runtime and wire pipeline. Resolve actual directory-backed files before editing; do not create shadow `grammar.rs` or `cbor_ast.rs`.
- `vibe/src/{catalog/,capability_schema.rs,bind/host.rs,module_system.rs,reactive_cell.rs,presentation.rs,replay_clock.rs,tick_policy.rs}`: catalogue, machine schemas, host seam, module/reactivity/presentation/time.
- `qualia-core-db/src/poet_host/invoke/{ids.rs,mod.rs,coverage.rs}` and focused family files: actual engine bindings and native/WASM coverage; public core alias is `vibe_host`, with compatibility `poet_host` module naming.
- `crates/vibe-wasm/src/lib.rs`, `crates/vibe-lsp/src/{catalog_intel,server}.rs`, `crates/poet-cli/src/` legacy reference and canonical `C:\github\poet\native/src/`: browser exports, editor support and CLI.
- Existing tests: `crates/vibe/tests/{conformance,vibe_catalog_surface,vibe_features,vibe_bind_lambda,vibe_reactive_destructure,graphics_backend_policy}.rs`; existing runtime unit tests. Existing normative conformance counts are historical evidence, not a fresh run.

The current interpreter branch `ExprKind::Await(e) => self.eval_expr(e, env)` in `eval/expr.rs` does not establish suspension/resumption; `Stmt::Yield` currently produces Flow::Next, not a complete stream/event scheduler. `module_system.rs` describes a module graph while `eval/mod.rs::populate_import_aliases` checks a fixed namespace list; test actual execution, not the existence of a ModuleResolver type. `vibe-wasm::value_to_js` preserves integer strings but has a Debug-format fallback for complex values; that is not an acceptable typed production interchange contract for all app values. These are baseline gaps/hypotheses to exercise in VB-00, not authority to replace working language features wholesale.

### N3. Required behaviour inventory and choice of extension seam

Add a Vibe coverage row for every app-script/host-adapter behaviour: original file/symbol and trigger, input/result shape, command/target/effect, existing Vibe syntax/library/capability, native/WASM execution proof, missing seam, migration packet, required assertion and residual JS reason if any. Cover collection/record/string/bytes operations, checked numeric/time/unit conversions, branches/loops/functions, error propagation, imports/version pinning, reactive dependencies, event subscription/disposal, async jobs/streams/cancel/timeouts, declarative UI, storage/assets, graphics/audio/device/network adapters, hot reload and debugging.

Use this decision order:

1. Existing language and library already expresses it: write a `.vibe` script and tests; no new keyword.
2. Domain/OS/engine operation is absent: implement the reusable Rust primitive/host adapter and expose a reviewed capability through the existing invoke seam.
3. Repeated pure operation belongs in standard library: add one bounded implementation/type signature/effect and tests, not a UI-private helper copy.
4. New source expressiveness is genuinely necessary: SPEC/D4 supplies exact grammar, typing/effect rule, runtime behaviour, diagnostics, budget/version and positive/negative examples before FE/D2 implements it.
5. Platform-only loader/worklet operation cannot run as a Vibe function: implement an explicit minimal adapter, record the boundary and conformance; do not hide business logic there.

Do not adopt automatic JS-to-Vibe transpilation as correctness proof. JS truthiness, undefined/null, floating-point coercion, prototype mutation, callback closure state and Promise timing have different meanings. Preserve the application's intended outcomes with explicit typed state/Result/effects rather than replicate accidental JS behaviour. Capturing closures are not assumed supported; prefer explicit state in event/session parameters, and extend the dialect only after a real use case and review. Language feature additions are driven by required app coverage, not a speculative ECMAScript checklist.

### N4. Extension implementation recipe

For each extension create a session-sized VB subunit. First write the exact interface and feature/version/effect contract with failing tests. Update normative grammar/dialect notes and generated grammar/schema resources, lexer, AST/parser, type/effect checker, interpreter, CBOR AST encoder/decoder, formatter/projectional/decompiler/reflection paths and diagnostics wherever the feature participates. An exhaustive match compiling is not evidence of correct semantics. AST decoder must reject unknown tags/trailing/oversize input and support declared older versions. Bytecode must implement equal semantics or explicitly refuse compilation and use a labelled AST path; no silently dropped operation/no-op opcode. LSP/CLI/Poet/WASM introspection and examples follow the same catalogue/schema.

For capability additions update the exact callable ID in Vibe catalogue and core ALL_BOUND, actual dispatch, machine schemas, effect checking, discovery and WASM feature coverage. Add approved App/Job/UI families only after binding real handlers; names in a proposal are not live exports. App calls use ST-E targets/revisions/receipts and ST-G authority; scripts cannot bypass the human/agent dispatcher. Keep small DTO adaptation explicit: Value-to-contract conversion validates all fields/types, preserves i64/u64/ticks/hashes exactly and never serialises opaque state with Debug strings. Schema declarations and real handler arguments must agree, including optional/unknown arguments.

Two different effect systems exist (language Pure/Hot/Cold/Async/External and capability Pure/Read/Write/External); define and test their mapping. A generic app execution binding is effectful even when its params name a read-only command, unless a reviewed separate snapshot-read binding guarantees purity. Declaring `using Family` requests availability; it never grants authority from a document. Dynamic capability IDs and aliases must still undergo catalogue/type/effect/lease/runtime scope checks. Native and browser can report different availability, not different hidden semantics.

Do not modify unchecked global keyword/opcode/CBOR tables or expand an oversized implementation monolith without scoped decomposition. Distinguish Vibe bytecode opcodes from Quin modality opcodes; allocate neither opportunistically. Any new persistent numeric/AST/tag semantics are reviewer-owned. Do not enlarge budgets, claim zero-heap from a function name or use JS number to repair an interop test.

### N5. Real async, event and reactive execution

Implement a bounded cooperative scheduler for Vibe app tasks, independent of the browser's JS event-loop semantics. Native and WASM use the same state machine and deterministic order; host adapters deliver completion/events. Task contains checked module digest/version, explicit frame/continuation state, captured immutable context/revision, remaining step/workspace budget, cancellation token and authority reference. Poll returns Ready(value/diagnostic), Pending(job/subscription ID) or Cancelled; waiting releases the execution turn and never blocks browser/native UI. Events are finite queues with explicit overflow/backpressure and disposable subscription handles. Disposal occurs on view close, module replacement, revoke and error; late completions cannot mutate a replacement module/document.

Host job handles are opaque; no dense media or OS objects stored in ordinary Value records. Await/resume rechecks revision/authority before any new mutation/effect. Do not hold an open graph transaction or mutable document lock across await: split into snapshot -> await external result -> validate expected revision -> commit, or reject that programme at check/runtime with an explicit diagnostic. Resume consumes the original budget; it does not reset gas. Remote timeout does not authorise replay of a paid effect. Time/entropy during replay comes from explicit inputs/receipts; Pure cells have neither.

Reactive cells use versioned dependencies and cycle detection; update only after committed source revision. Separate event time, media tick and wall clock. Hot tick callbacks cannot allocate/query live graph/network; delegate heavy work into bounded cold/async jobs. Cancellation and supervised-agent revoke converge on the same execution gate. Scheduling policy and opcode changes require D4 review; low-cost implementation follows the accepted tests/state transitions.

### N6. Vibe-authored UI without another UI kit

Use normative declarative presentation/records as input to the same shared Dioxus components and orb/menu descriptors. Scripts return a bounded keyed UI tree with validated node kinds/properties/accessibility and command IDs, not arbitrary DOM/HTML/CSS/JS functions. Rust renderer maps allowlisted presentation tokens and semantic assets to Dioxus/Qualia graphics. User text is escaped, children/depth bounded, duplicate keys refused. Events are tagged serialisable payloads (command_id/context/target/revision/input), not raw DOM events or executable callback strings. UI mutation remains an authorised effect.

Script source updates parse/check/validate in isolation, atomically replace an accepted module and reconcile keyed views without rebuilding the host. On invalid update retain last valid module/view and show precise diagnostics. Old event subscriptions/resources are released; UI editing state is preserved only through an explicit version-compatible state migration. No automatic rerun of old external effects or agent approvals on hot reload. A Vibe-authored panel, menu contribution, child orb and job-progress view must render native/WASM with the same action IDs and accessible projection.

### N7. Completion and sequencing

The dedicated queue is playbook section 19, VB-00 through VB-09. VB-00 runs after ST-00 and before closing G1; VB-01/02/03/04 can progress on fixtures after accepted contracts. VB-05 attaches to ST-10 through ST-16; VB-06 attaches to ST-05/07/08/09; VB-07 covers tooling; VB-08 runs per app alongside its a-f migration; VB-09 is part of G4. ST-19 may not claim script accessibility before VB-01 bindings are live; app f closure requires its VB-08 rows. This supplements ST packets rather than renumbering historical entries.

Completion: required handwritten app business/orchestration/UI logic can be authored and hot-edited in Vibe, all command/effect paths use shared authority, real asynchronous work/events/reactivity function, imports/versioning and errors are correct, exact-value/opaque-handle serialization survives browser/native, language and tooling contracts agree, and each retained JS bridge has an explicit minimal platform-only justification. No universal ECMAScript compatibility or zero-JavaScript distribution claim is made without its own evidence. Any missing required behaviour remains a named VB row, not a hidden JS fallback or generic 'future extension'.
