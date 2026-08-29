# POET Standalone UI Parity — Implementation Tracker

**Opened:** 2026-08-27  
**Scope:** standalone `crates/poet` browser UI parity with the Dioxus POET UI and
the requirements recorded in `poet_ui_handover.md`.  
**Completion rule:** a visible control must perform its advertised operation,
or be visibly disabled with a specific prerequisite. Placeholder notifications,
fabricated results, and silent no-ops do not count as implemented.

## Parallel programme — QApps migration into native POET format

- **Legacy QApps are being migrated into the native POET UI format as a
  separate programme.** They will not gain a parallel QApp runtime. Per ADR 0012, each
  former QApp must become a POET construct, manifold, typed container, or an
  honestly labelled Library Software stub; execution uses normal capabilities,
  COP/Semantic Library persistence, and HCF/HMC packaging. No QApp placeholder
  is counted against the present parity sprint, and this exception does not
  apply to ordinary POET containers, Tool Chest actions, daemon execution,
  persistence, search, logic, rendering, or accessibility. The normative
  migration checklist is section K. This separation prevents QApps structural
  work from blocking completion of ordinary POET application functionality; it
  does not mean the QApps have been abandoned or removed from the product.

All other registered POET surfaces are in scope. A read-only prototype or a
disabled control is not considered complete merely because it names a missing
contract. The completion target is a real local or daemon-backed operation,
persistent state where the surface edits records, and an honest result/error
state. Hardware- and credential-dependent operations may require the user to
supply the device/session, but the software path up to that boundary must be
implemented and tested.

## Status legend

- `[x]` implemented and covered by automated verification
- `[~]` implemented but still requires final browser UAT
- `[ ]` not yet implemented
- `[!]` intentionally unavailable until an external prerequisite exists; the
  UI must expose that prerequisite and disable the operation
- `[R]` completion claim withdrawn; product workflow remediation required
- `[Q]` assigned to the parallel QApps-to-POET migration in section K

## A. Interaction shell and canvas

- [x] Mount-order-safe, idempotent DOM interaction wiring.
- [x] Stable unique container and wire identities with legacy repair.
- [x] Complete canvas history for create/delete/move/resize/z-order/wires and
  semantic metadata.
- [x] Saved-manifest startup restore with default-manifold merging.
- [x] Document HTML and Tool Chest setting persistence.
- [x] New-manifold state updates after edits.
- [x] Zoom/scroll-aware drag, resize, wire drag, and wire geometry.
- [x] Exact wire path/label association and attached-wire cleanup.
- [x] Semantic wire inspector and container semantic editor.
- [x] Radial actions, clipboard staging, click-to-connect, and keyboard escape.
- [x] Fifteen Tool Chest families and corrected family IDs.
- [x] Four-way docking, responsive layouts, focus/ARIA states.
- [x] Bounded 4D playback timer emitting `poet_tick` events.
- [x] Standalone mail container dispatch.
- [~] Final Chrome interaction pass across all canvas operations. A fresh
  system-Chrome tab loaded the final build, confirmed daemon attachment and
  opened Search by keyboard. Chrome then blocked further input because another
  extension UI is open over the page; dismiss that popup and resume the gesture
  pass in the retained `POET UI parity UAT` tab.

## B. Shared execution bridge

- [x] Add real daemon `/eval` endpoint backed by `PoetSnapshot`.
- [x] Add real daemon `/invoke` endpoint backed by registered POET capability IDs.
- [x] Add real daemon `/gazetteer` endpoint backed by `nlp::analyze_document`.
- [x] Add and verify render-preview execution; unavailable hosts expose the exact
  renderer/GPU prerequisite and never emit a synthetic preview.
- [x] Align browser daemon DTOs and endpoint documentation with actual routes.
- [x] Replace the disconnected action-endpoint map in the WASM intent bus.
- [x] Enforce bounded request bodies and return structured diagnostics.
- [x] Add endpoint unit/integration tests.

## C. Tool Chest actions

- [x] Carry `ActionType`, capability ID, and availability into rendered tool views.
- [x] Replace the shared notification-only click handler with an action dispatcher.
- [x] Implement local UI actions (place, navigate, toggle, formatting, epistemic
  tagging) against the selected surface.
- [x] Route executable actions through `/invoke` with typed/default arguments.
- [x] Disable tools needing hardware, credentials, or an unavailable daemon and
  show the exact reason inline.
- [x] Audit every registered non-QApp tool: no visible notification-only action.

## D. Workbenches and containers

- [x] Logic workbench: replace every legacy mock-result path with real local or
  daemon evaluation, or an explicit unavailable state.
- [x] Search workbench: remove fabricated offline result generation; preserve
  local indexing only where it actually searches user data.
- [x] NLP/gazetteer: wire real analysis results and diagnostics.
- [x] Code/Vibe evaluation: wire real program, cell, and function execution.
- [x] 3D/render controls: wire the real renderer where supported; otherwise expose
  the GPU/runtime prerequisite and disable preview/export.
- [x] Audit generic container fallback text and replace each case with a real view
  or an intentional unavailable state.
- [x] Audit remaining legacy prototype surfaces outside QApps. Explicitly labelled
  and type-classified static telemetry containers are forced into read-only mode,
  with unavailable badges and contract-specific prerequisites.

## E. Persistence and lifecycle

- [x] Persist specialist container state, not only outer HTML/tool settings.
- [x] Restore state through reload, manifold switching, save/load, and undo/redo.
- [x] Replace hard-coded manifest creation timestamps with real creation times.
- [x] Add round-trip tests for the generic persisted state adapter and legacy defaults.

## F. Accessibility, honesty, and verification

- [x] Ensure disabled actions use semantic disabled/ARIA state and expose a reason.
- [x] Ensure async operations expose running, success, empty, and error states.
- [x] Ensure no result labelled live is mock/generated fallback data; daemon
  connectivity alone no longer elevates a surface's honesty label.
- [x] `cargo test -p qualia-core-db` relevant service/POET tests.
- [x] `cargo test -p poet --lib` (223 passed on the 2026-08-28 merged-state pass).
- [x] `cargo check -p poet --target wasm32-unknown-unknown`.
- [x] `trunk build`.
- [x] `git diff --check`.
- [~] Final Chrome UAT is pending the completed non-QApps build. Chrome control
  loaded the final build and verified live startup/search. Drag, resize, wire,
  specialist placement, and undo/redo remain blocked by Chrome's open-extension
  UI guard, not by an application or build failure.

## G. Continued implementation — standalone render service

- [x] Define a renderer-provider boundary in `qualia-core-db` so the loopback
  router does not depend directly on `webizen-render`.
- [x] Register a real `webizen-render` provider from `qualia-cli`.
- [x] Add bounded `/render/preview` request/response contracts and tests.
- [x] Wire standalone map/media/3D and submanifold surfaces to live PNG previews.
- [x] Verify adapter-unavailable, empty-scene, oversized-request, and successful
  render outcomes without synthetic fallback frames.

## H. Continued implementation — Semantic Library parity

- [x] Replace the standalone bundled/sample shelf with live
  `qualia-client-core::wellfair::HypermediaStore` data.
- [x] Add a host-injected semantic-library provider to the loopback daemon without
  introducing a `qualia-core-db`/`qualia-client-core` dependency cycle.
- [x] Expose bounded stats, faceted query, and text-document ingest endpoints.
- [x] Wire section tabs, free-text search, facet counts, sorting, empty/error
  states, and text ingestion to those endpoints.
- [x] Verify persistence by ingesting through the daemon, restarting it, and
  finding the same semantic entry after restart.
- [x] Cover provider-unavailable, malformed/oversized input, query, facet, and
  ingestion paths with automated tests.

## I. Continued implementation — ordinary container truthfulness

- [x] Implement local LaTeX source editing/snippet insertion and local slide
  editing/layout/transition/presentation controls in focused modules.
- [x] Remove fabricated live status from unbound health, anatomy, finance,
  WebRTC, vision, listen, triad, portal, Aura, and embedded webview surfaces.
- [x] Give every unavailable specialist surface a contract-specific prerequisite
  and semantic disabled state until its backend/session is registered.
- [x] Re-run the non-QApp visible-control audit after the fail-closed pass,
  including social, relationship, policy, reputation, capability, settings,
  channel, and presence surfaces that previously exposed static status.

## J. Non-QApps completion programme — reopened by product-integrity audit

The fail-closed work in section I was an honesty prerequisite, not final
functional completion. The following work supersedes any earlier statement
that static/read-only specialist surfaces were complete.

- [x] Bind every Logic Workbench primary and secondary operation to a real
  native capability, including N3/RDF-Star/ontology, legal/governance,
  advanced logic, scientific/domain, and infrastructure panels.
- [R] Replace static Agreement and Rights data with persistent COP-backed
  authoring, evaluation, contribution, licensing, and obligation records.
- [R] Replace static Project views with persistent project records and working
  lifecycle, planning, documentation, resource, governance, issue, asset,
  event, reporting, import, and export operations.
  - [~] Project economics remediation: the Budget route now separates live
    plan, actual, funding, royalty, and tax ledgers; requires amount unit,
    lifecycle, effective date, actor, provenance, and sensitivity; derives
    unit-separated variance/funding position using exact fixed-decimal
    arithmetic; and exports an audit JSON bundle. Compensation/obligation
    linkage, authority transitions, and system-Chrome task UAT remain, so the
    Project family stays reopened.
- [R] Replace static Dataset/Ontology views with persistent registries,
  ingestion, annotation, lineage, validation, mapping, presentation/media,
  CAD, and render operations backed by registered DAT/graph capabilities.
- [R] Replace static Studio views with live Audio and Scene capability
  sessions, including transport, routing, meters, automation, scene graph,
  material/light/camera controls, animation, GIS, and export paths.
- [R] Replace static Health views with consent-gated persistent health records,
  real clinical calculations, provenance, disclosure, authority, and safeguard
  operations; no medical value may be fabricated.
- [R] Replace static Governance, Device, Social, presence/channel, finance,
  Aura, WebRTC, vision/listen, triad, portal, and embedded-web controls with
  typed live/local contracts and persisted session state.
  - [~] Social now has DID-attributed persistent threads, filtered message
    history, composition, refresh, and a separate Pulse publication notice.
    This does not yet establish delivery, signature, membership enforcement,
    moderation, attachment, encryption, or multi-host transport.
  - [~] Local AI is exposed as an ordinary Tool Chest application. It runs the
    real local GGUF/P64 agent runtime, optionally grounds from the live Semantic
    Library, presents evidence/verification state, and records bounded turns.
    It now discovers bounded resident/configured local models and restores
    previous persisted turns. Named conversations now supply up to eight prior
    turns to follow-up prompts. Runs now stream sequenced token events, can be
    cooperatively cancelled, retain partial output without recording it as a
    completed turn, and place completed assertions in a reviewer-DID approval/
    rejection queue. Catalogue models can now be activated into or evicted from
    the real resident-model slot, with active jobs blocking replacement. Runs
    have a 1–256 token budget and persist completed/cancelled/failed terminal
    receipts separately from conversation turns. Bounded model installation,
    Saved profiles now enforce controlling principal DID, permitted model,
    token ceiling, local-inference capability, and exact project-tag/all/none
    Semantic Library scope before decode. Connector/tool grants beyond local
    inference and aggregate budget management remain.
  - [~] Connectors are exposed as an ordinary Tool Chest application. It
    discovers and invokes negotiated host capabilities and persists semantic
    connector contracts with interface/input/output IRIs, transport,
    sensitivity, status, and capability/endpoint binding. Schema-derived
    authoring, auth, mapping, and external adapters remain.
    Saved contracts are now reconciled against negotiated capabilities and can
    run a stored JSON probe; external endpoints stay configured rather than
    being mislabelled connected until a host adapter probes them. Their JSON-LD
    semantic contracts can be published to the persistent Semantic Library.
    Negotiation now exposes machine argument/return schemas, family, effect
    class and honesty, and capability selection pre-fills the runnable semantic
    descriptor. Invocations persist bounded run receipts; a failed run is
    retryable through attempt three only when its machine contract declares the
    operation Pure.
  - [~] Social channel creation now carries semantic topic, visibility and
    membership policy. Relationship requests have explicit pending → accepted
    or denied transitions, with the backend requiring the receiving DID as the
    transition actor. Open-channel membership is persisted, restricted-channel
    posting is membership-gated, request-policy channels support creator-decided
    join requests, invitation-only channels support creator-issued/recipient-
    decided invitations, and voluntary presence is scoped and expiring. The
    inbox exposes incoming decisions and recent persisted Pulse activity;
    request parties and scope cannot be rewritten during transition.
    Messages now support validated same-thread replies and at most 16 DID
    mentions. Mentioned recipients receive local unread/read receipts whose
    principals/source are immutable and whose read transition is recipient-only.
    Channel creators can change participant roles; creators and active
    moderators can hide a message using an attributed receipt while preserving
    source evidence and reply topology. Cryptographic signing, attachments,
    blocked-relationship enforcement and remote delivery remain.
- [x] Add shared bounded daemon contracts and browser adapters rather than
  duplicating transport/state logic in each view.
- [R] Add automated contract, persistence, malformed-input, unavailable-host,
  and cross-restart tests for every family above.
- [x] Wire remaining specialist surfaces (governance, device, social, pulse,
  health ClinicalRisk, anatomy HU demo, rights/agreement deontic) to live
  host invoke IDs; merge COP form fields into those calls.
- [x] Pulse invoke publishes on `pulse_transport` and persists `pulse_event`
  COP records (allowlist-prefixed `poet/` topics). Canned pulse/ontology
  trees removed.
- [x] Remaining large backend work recorded in
  `docs/POET_UI_BACKEND_GAPS_2026-08-28.md` (PDF/OCR, RTC signaling, mic/cam,
  ILP/Lightning, DICOM store, DID signing, mixnet delivery). Not mocked.
- [ ] Complete system-Chrome gesture and specialist-surface UAT after the fresh
  build is running. (Requires the principal to launch poet-ui against a local
  daemon; not a code gap.)

### J.2 Construct authoring (2026-08-28)

- [x] Vibe 0.1 lockstep: `Poet.manifold_create`, `Poet.container_place`,
  `Poet.nested_link`, `Poet.subject_declare` (host receipts + catalog).
- [x] HyperCanvas applies those receipts locally (no DOM in qualia-core-db).
- [x] Authored lenses persist via `save_all_manifolds` + construct extras.
- [x] Thin `SubjectSeed` registry (not a construct; not a canned world).
- [x] Nested-manifold breadcrumb pop (`Up` + clickable crumbs).
- [x] Observer DID from `Identity.current_user` after daemon attach (bound when the probe succeeds).
- [x] Social manifolds: `ManifoldSociality`, participants, `Poet.participant_invite`, Projects construct. Projects/social/communications are social; health/anatomy stay personal.
- [x] HCF/HMC export of an authored construct, including construct metadata,
  its authored manifold set, observer attribution, checksummed CBOR envelopes,
  import verification, and Construct Shelf downloads.

### J.1 Completed foundations

- [x] Expand `/vibe/capabilities` from nine hand-picked entries to the complete
  native `ALL_BOUND` invoke catalogue and cache it in the standalone browser.
- [x] Gate workbench controls from negotiated capability IDs/prefixes instead of
  the previous hard-coded seven-button allowlist.
- [x] Complete Clinical Risk Workbench bindings for Framingham,
  CHA2DS2-VASc, SCORE2, drug interactions, contraindications, and real FHIR
  Observation validation with bounded typed input parsing.
- [x] Complete all nineteen Chemistry Workbench operations through the native
  organic-chemistry engine: SMILES, descriptors, drug-likeness, functional
  groups, pKa/chirality/fingerprints, thermochemistry, and green metrics.
- [x] Complete all nine Bioinformatics Workbench operations through the native
  alignment, k-mer, FASTA, gene-expression, fingerprint, MinHash, and bounded
  UPGMA implementations.
- [x] Complete the deterministic GBM/VaR panel with a bounded seeded price path,
  bounded Monte Carlo paths, selectable confidence, and portfolio-scaled VaR.
- [x] Complete all fourteen Physics Simulator modes plus the standalone ODE
  panel through a bounded native contract: seeded Metropolis sampling, coupled
  RK4, Thomas-Fermi LDA, bounded embedded PINN inference, Gibbs energy,
  battery-pack, MPPT solar, heat-loss, phase-change, and efficiency models.
- [x] Split browser request construction into focused parsing, physics, and
  domain adapters so each new module remains below 500 lines.
- [x] Bind the Comorbidity Analyzer to the existing zero-allocation,
  paraconsistent clinical evaluator with patient/organ filtering and bounded
  compounding edges.
- [x] Bind the lightweight DICOM panel to the real bounded CT Hounsfield-unit
  window/level kernel with dimension, payload, and finite-value validation.
- [x] Bind the Diffusion Controller to the existing native 1D heat-diffusion
  solver and persist revised bounded configuration in browser storage.
- [x] Complete every Calculus panel mode through a bounded native adapter:
  symbolic RK4, Simpson-Kahan, trapezoidal SIMD, adaptive Simpson, bounded
  large-grid integration, and host SIMD-width reporting. The misleading
  unimplemented GPU label was replaced with its real large-grid execution path.
- [x] Complete N3 Studio parse/evaluate through the native streaming parser,
  N3 compiler, and Webizen rule engine; parse-only diagnostics and browser-
  persisted rulesets are now real operations.
- [x] Complete all eight Advanced Logic panels through the bounded
  `AdvancedLogic.compute` contract: Bayesian abduction, eight fuzzy
  t-norm/t-conorm operations, Bayesian evidence updates, bounded graph
  topology, all 13 Allen interval relations plus interval sums, native 10D
  quaternion projection, epistemic safety/referral gates, and K/T/D/B/S4/S5
  Kripke-frame evaluation. Browser request parsing lives in its own focused
  module and the backend adapter remains below the 500-line source limit.
- [x] Complete CTL, Defeasible, Linear, and Dialectical panels through the
  bounded `FormalLogic.compute` contract: all eight CTL operators, explicit
  rule kinds/superiority/ambiguity plus grounded justification, tensor
  consumption and structural-rule licensing, contradictory synthesis/IBIS
  scoring, and the secondary causal-counterfactual action.
- [x] Bind RDF-Star Resolve and Extract plus Ontology Compile and structural
  Validate to semantic backends. `GraphAuthoring.process` uses the bounded
  RDF/RDF-Star collector and emits a real base64 CBOR-LD payload; extraction
  delegates to `NLP.relation_extract`. Domain-ontology import now reads bounded
  local Turtle/N3/RDF/OWL files into the authoring editor before the same native
  compile/validate path; remote URL fetching is deliberately not implied.
- [x] Complete the remaining LTL, ASP, paraconsistency-saturation, and bounded
  symbolic-inference actions. LTL now evaluates explicit G/F/X/U/R traces and
  reports the first streaming-safety violation; ASP parses a bounded normal
  program, returns real stable/brave/cautious results, and selects a genuine
  weak-constraint optimum; paraconsistent routing reports measured global
  saturation; N3 inference can return matched-rule derivation evidence.
- [x] Replace Deontic Compile with real norm-Quin compilation and replace the
  misleading universal free-form modality evaluator with a navigator into each
  modality's typed editor/contract. The workbench footer now states the actual
  negotiated/fail-closed behavior instead of describing results as mocks.
- [x] Complete all ten Legal Logic panels through `LegalLogic.compute`:
  live-graph Hohfeld correlativity, Chellas/deliberative STIT and joint
  liability, bounded causal-root/overdetermination tracing, allegation and
  accountability gates, capacity/duress/standing, attenuated delegation and
  revocation, capacity-gated contract formation, BFT/partition consensus,
  provenance-anchored breach records with explicit external-signature
  requirements, and grounded/preferred/stable/complete Dung extensions plus
  graph data. Every former legal secondary notification now returns native data.
- [x] Complete all six P1 governance panels through the bounded
  `GovernanceLogic.compute` contract: Permissive Commons cost/discharge/E-ROI
  plus royalty trees, deontic-verdict `PolicyMode` mapping with emergency
  override, k-of-n identity-fabric survival, required-vs-held capability gap
  with optional learning-path cost, selective disclosure/Curation Directive/
  instrument anchoring plus ZK eligibility, and mens rea plus locative
  obligation/trust-gate composition. Secondary royalty, survival, ZK, and
  mens-rea actions return native data.
- [x] Complete Allen/RCC8 and Manifold Logic through `SpatialLogic.compute`:
  13 Allen relations, region and zero-heap point RCC8, AABB spatial-index
  query, Minkowski interval plus causal connectability, one heat-equation
  FTCS step, and wave-eval / absolute integration / continuous-to-fact.
- [x] Complete all nine P2 infrastructure inspectors through `InfraLogic.compute`:
  N-Triples→WebizenVM bytecode compile/trace, 42MB Sentinel arena occupancy,
  CPU GEMM/top-k/FFT/roofline, live GPU adapter profile, calibrated DP
  Laplace/Gaussian plus aggregation quorum, ModelLifecycle transitions,
  default byte-level tokenizer encode, and P64 magic/layout inspection.
  LLM telemetry and BFV ciphertext ops fail closed with the exact session
  prerequisite. Forge certify/autotune/Naga validate require a live GPU
  Forge session.
- [x] Complete all ten P2 infrastructure-extension panels through
  `InfraExtLogic.compute`: LWW CRDT plus delegation window, author-scoped
  Merkle root, 8-slot HE key-metadata vault, clearance-vs-sensitivity policy,
  scoped consent in-force evaluation, BLAKE3 carrier binding, conservative
  PID, ordinal likeliness meet, QUBO compile+classical solve, and bounded
  OWL 2 RL materialization.
- [x] Add the shared `/records` COP ledger on the loopback daemon
  (`query`/`upsert`/`delete`, family-keyed, optional `kind` filter,
  file-backed under the daemon storage path) and a single browser adapter.
  Agreement builder, contribution ledger, license builder, obligation
  tracker, compensation, Rights tabs, every Project view, and Dataset/
  Ontology registries persist through that contract. QApps remain excluded.
  Wallet mint, DID signing, consensus tallies, and specialist-agent answers
  remain unbound and are labelled as such; they are not fabricated. Natural
  persons are modelled with RDFS + SHACL/ShEx; `owl:Thing` is a forbidden
  type for persons and is used only as a SHACL `sh:not` guard target.

## K. Parallel programme — migrate legacy QApps into POET UI

This is a structural migration, not implementation of a second application
runtime. The target model is fixed by
`docs/manuals/adr/0012-construct-is-the-distributable-composition.md`:

`legacy QApp → ConstructSeed | ManifoldSeed | SeedContainer | Library Software stub`

The existing POET interaction shell, pager, Tool Chest, command palette,
capability negotiation, COP ledger, Semantic Library, checkpoints, and HCF/HMC
packages are the only destination architecture.

### K.0 Completed foundations

- [x] Accept ADR 0012: QApp is not a runtime type and `qapp.json` is not a
  second package ABI.
- [x] Add typed constructs, authored/nested/social manifolds, subjects,
  participants, construct shelf/portals, checkpoint snapshots, and checksummed
  HCF/HMC construct import/export.
- [x] Migrate Anatomy into manifold `anatomy` on the Health/default POET
  constructs and absorb clinical/scientific workbenches into Domain Lab.
- [x] Establish honest Library Software stubs for catalogue entries that do not
  yet have real manifolds or capabilities.

### K.1 Inventory and migration map — first implementation step

- [Q] Enumerate every legacy QApp catalogue row, route, `qapp.json`, source
  module, saved-state key, command, and documentation entry; assign an owner
  and disposition to every item.
- [Q] Classify each item as: observer-scope construct, manifold/lens, typed
  container, existing POET capability/surface, Library Software stub, or delete
  as a duplicate/obsolete wrapper.
- [Q] Identify functionality already delivered by POET (Anatomy, Domain Lab,
  Knowledge/Ontology, Studio, Communications/Vibe, Projects, Health) so the
  migration reuses it rather than recreating nested applications.
- [Q] Publish a machine-readable migration manifest and a human review table;
  fail CI when a discovered legacy QApp has no explicit disposition.

### K.2 Structural conversion

- [Q] Replace first-class QApp routes and launchers with POET construct,
  manifold, nested-manifold, portal, or container navigation.
- [Q] Convert useful legacy layouts and metadata into `ConstructSeed`,
  `ManifoldSeed`, `SeedContainer`, Tool Chest registration, and command-palette
  placement records.
- [Q] Refactor QApp Studio into native construct/manifold authoring: compose
  lenses and containers, validate them, preview them on HyperCanvas, then
  export HCF or archive HMC.
- [Q] Route all executable behavior through negotiated POET capability IDs;
  move durable records to the bounded COP ledger and semantic assets to the
  Semantic Library. No QApp-specific transport or persistence store survives.
- [Q] Preserve observer DID, sensitivity, provenance, consent, sociality,
  subject/project links, and unavailable prerequisites during conversion.

### K.3 Catalogue and saved-state migration

- [Q] Convert the remaining genuinely implemented bundled QApps in dependency
  order, beginning with the former first-class routes, and add parity tests for
  each converted surface.
- [Q] Keep the academic catalogue as searchable Library Software stubs until a
  real manifold seed and capabilities exist; do not promote catalogue metadata
  to a live UI claim.
- [Q] Add a bounded, one-shot importer for supported legacy QApp manifests and
  saved state. It must produce normal POET seeds/COP/Semantic Library records,
  record a migration receipt, and leave unsupported input unchanged with a
  precise diagnostic.
- [Q] Provide an explicit archive/export path before removing any legacy state;
  migration must be reversible until the converted HCF/HMC validates.

### K.4 Removal and completion gates

- [Q] Remove legacy QApp runtime types, routes, loaders, stores, styling, and
  user-facing “launch QApp” language after their mapped replacements pass.
- [Q] Retain the term only in migration tooling, historical documentation, and
  regression fixtures; update active specifications to the POET vocabulary.
- [Q] Add CI invariants: no second package ABI, no live QApp runtime registry,
  no QApp-only persistence/transport, every migrated control performs work or
  is semantically disabled with its exact prerequisite.
- [Q] Verify HCF/HMC round-trip, legacy import, reload, checkpoint restore,
  manifold navigation, accessibility, and system-Chrome interaction for every
  migrated first-class surface.
- [Q] Close the QApps refactor only when the migration manifest has no
  unclassified rows and no active runtime references remain.

### K.5 Immediate order of work

1. Execute `docs/POET_PRODUCT_INTEGRITY_REMEDIATION_2026-08-29.md`; broad
   non-QApps completion is reopened.
2. Finish the retained system-Chrome interaction UAT as evidence for the shell,
   without treating it as evidence for the reopened domain workflows.
3. Generate the unified surface and legacy-QApp inventory/migration manifest.
4. Restore product workflows, then convert first-class QApp routes and QApp
   Studio using the K.2 architecture.
5. Migrate useful bundled implementations, remove the legacy runtime, and run
   both the product-integrity and K.4 completion gates.

## Evidence and audit notes

- **2026-08-29 product-integrity correction:** broad family completion claims
  are withdrawn. The audit found 115 domain view files collapsed to thin
  delegations (230 additions versus 30,870 deletions), 118 builders concentrated
  in six generic persistence modules, and 545 `[todo]` entries still present in
  the authoritative Workstream A plan. Persistence/honest disabling are useful
  foundations but are not domain workflow parity. The binding remediation plan
  is `docs/POET_PRODUCT_INTEGRITY_REMEDIATION_2026-08-29.md`.
- **2026-08-29 first corrective slice:** added a product-integrity regression
  gate and restored Project Budget as a domain workflow backed by five bounded
  economic record families. Automated evidence covers unit/lifecycle
  validation, exact multi-currency aggregation, pending-state exclusion,
  persistence round-trip, route selection, and retention of reopened claims.
  This is a verified slice, not a restored Project-family completion claim.
  The generic delegation count moved from 115 to 114. Host tests and the WASM
  compile pass. System Chrome confirmed the rebuilt app and live daemon load,
  but click dispatch failed at the Chrome-control boundary after reload, so the
  Project Budget task UAT remains explicitly open.

- The initial audit found 23 registered `RunAction` tools sharing a
  notification-only handler.
- The logic workbench contained 72 mock-result call sites.
- The search workbench produced mock results when the daemon was offline.
- Roughly 130 standalone browser source files contained a mock/pending/backend
  integration marker; each non-QApp occurrence must be classified and resolved.
- The 2026-08-28 re-audit found that specialist containers had been implemented
  but roughly 120 command-palette entries still fell through to a notification-
  only handler. They now place their corresponding live or fail-closed
  specialist container; unknown catalogue drift fails closed.
- [x] Checkpoints now retain their own bounded seed snapshot; the reachable
  tray selects and restores an exact checkpoint and exports that snapshot as a
  construct HMC. Branch stays semantically disabled until a DAG exists.
- [x] The reachable publication panel creates real snapshot checkpoints and
  exports current HCF/HMC construct packages. Pruning, credits, signed Q42, and
  metadata stripping are semantically disabled with exact missing contracts.
- [x] Credential, provenance, context-markup, constituency, and consent panels
  now use the shared bounded COP ledger. Consent is never inferred: absent
  records remain visibly absent, and capability availability is negotiated
  separately through `CapabilityDiscovery.list`.
- The browser advertised `/eval`, `/gazetteer`, and `/render/preview`, while the
  native daemon did not expose those routes. The core `PoetSnapshot` and invoke
  registry already provide the underlying evaluation capabilities, so the bridge
  is implementation work rather than a reason to fabricate results.
- The standalone loopback router now owns a host-injected render-provider
  boundary, avoiding the `qualia-core-db`/`webizen-render` dependency cycle.
  `qualia-cli` registers the real provider at cold startup. A live daemon smoke
  test returned a genuine 320x180 PNG through Vulkan on an NVIDIA RTX A2000,
  with 6 nodes, 5 edges, and 1 face reported by the typed scene contract.
- Legacy domain prototype views are retained as read-only UI/spec references.
  Making them operational requires the typed commands named in their own
  notices (for example COP, DAT, AUD, R3D, and consent-gated health contracts),
  not additional click handlers.

## Change log

- **2026-08-27:** Replaced the earlier completion-style report with this tracker;
  recorded the QApps exclusion and the strict honesty/completion criteria.
- **2026-08-27:** Completed the interaction-shell items listed in section A;
  automated Rust/WASM/package checks passed before this tracker revision.
- **2026-08-27:** Added bounded `/eval`, `/invoke`, `/gazetteer`, and `/intent`
  routes; smoke-tested live evaluation (`1 + 2 = 3`), Sentinel invocation,
  gazetteer extraction, and SPARQL against a running `qualia-cli` daemon.
- **2026-08-27:** Replaced Tool Chest and contextual-ribbon notification stubs
  with real dispatch policies; unsupported tools now fail closed with explicit
  prerequisites.
- **2026-08-27:** Added specialist state persistence and fail-closed read-only
  treatment for explicitly labelled prototype surfaces. QApps remain deferred.
- **2026-08-27:** System Chrome loaded the packaged app and verified its initial
  accessible structure. The first automation session later became stale; a new
  Chrome session has since connected successfully and is retained for final UAT.
- **2026-08-27:** Added the injected standalone render-provider boundary,
  registered `webizen-render` from `qualia-cli`, wired map/media/3D/submanifold
  previews, and smoke-tested a real GPU-rendered PNG. Added tests for bounded
  requests, missing providers, empty scenes, and unavailable results without
  synthetic fallback frames.
- **2026-08-27:** Replaced the standalone Library sample shelf with the persistent
  Semantic Library backend. Added injected stats/query/ingest routes, faceted
  live DOM rendering, sort/section/search controls, and bounded text ingestion.
  An isolated daemon smoke test ingested a Work entry, derived 13 semantic Quins,
  restarted the process, and found the same URI by semantic text search; the
  isolated test store was then removed.
- **2026-08-27:** Completed the second ordinary-container honesty pass. Static
  specialist telemetry and peer/session samples now render as unavailable,
  disable every control, replace any optimistic header badge, and name the exact
  backend/session contract required to activate the surface.
- **2026-08-27:** Reopened non-QApps completion after clarifying that fail-closed
  prototypes are not the functional endpoint. Added section J as the normative
  remaining-work programme. QApps remain the only deliberately deferred area.
- **2026-08-27:** Re-established system-Chrome control. Existing POET tabs were
  discoverable but could not be reclaimed from an older automation session; a
  fresh controllable Chrome tab was created and retained for final UAT.
- **2026-08-27:** Expanded daemon capability negotiation to the complete native
  invoke catalogue and added browser capability gating. Exposed the engine's
  FHIR Observation validator as `ClinicalRisk.fhir_observation` and completed
  all six Clinical Risk Workbench request bindings with validated panel input.
- **2026-08-27:** Added the bounded `OrganicChemistry.compute` invoke contract
  and connected every Chemistry Workbench mode to the existing native molecular,
  thermochemistry, and green-chemistry implementations.
- **2026-08-27:** Added the bounded `Bioinformatics.compute` invoke contract and
  connected all nine workbench modes to real sequence, expression, similarity,
  sketching, and phylogenetic-tree implementations.
- **2026-08-27:** Added `FinancialModeling.gbm_var`, composing the existing
  caller-buffered GBM and seeded Monte Carlo kernels into the workbench's real
  path-plus-risk result.
- **2026-08-28:** Bound all Physics Simulator and standalone ODE controls to a
  typed bounded native contract, replacing the advertised mock execution paths.
- **2026-08-28:** Added live comorbidity and medical-imaging contracts for the
  corresponding workbench panels; both reuse existing tested domain kernels.
- **2026-08-28:** Replaced both Diffusion Controller mock actions with native
  execution and persistent local configuration.
- **2026-08-28:** Connected all Calculus panel modes to symbolic and continuous-
  grid solver kernels with bounded panels/evaluations and explicit CPU/SIMD truthfulness.
- **2026-08-28:** Replaced N3 Studio parse, evaluation, and ruleset-save mocks
  with native rule-engine execution and browser persistence.
- **2026-08-28:** Replaced every Advanced Logic primary placeholder with a
  negotiated native capability, narrowed each panel's wording to its actual
  executable contract, and added malformed-input plus backend evaluator tests.
- **2026-08-28:** Replaced CTL, Defeasible, Linear, Dialectical, and the
  Dialectical Counterfactual placeholders with bounded native evaluations and
  aligned each editor's example syntax with its validated request contract.
- **2026-08-28:** Connected RDF-Star parsing/resolution, native relation
  extraction, ontology CBOR-LD compilation, and bounded RDF/OWL structural
  validation to the semantic graph stack; no synthetic validation result is
  emitted.
- **2026-08-28:** Replaced the LTL Safety, ASP Optimal Model,
  Paraconsistent Saturation, and Symbolic Explain secondary placeholders with
  native results. The LTL and ASP editors now document the exact bounded syntax
  accepted by their engines, and unsupported symbolic inference modes were
  removed instead of being advertised without implementations.
- **2026-08-28:** Replaced the Ontology Import pending notification with a
  bounded local-file picker. Imported ontology text is handed to the existing
  native compilation/validation workflow, with a 256 KiB browser-side limit.
- **2026-08-28:** Connected Deontic Compile to the native 48-byte Quin layout
  and converted the catch-all modality surface into typed-workbench navigation,
  removing the fabricated universal satisfiability verdict and stale mock footer.
- **2026-08-28:** Added the bounded `LegalLogic.compute` adapter and rewired all
  Legal Logic primary and secondary actions. Panel examples now use the exact
  validated syntax consumed by the backend; unsupported bipolar/VAF selector
  promises were removed from the Dung panel pending their own typed UI contract.
- **2026-08-28:** Added `GovernanceLogic.compute` and `SpatialLogic.compute`.
  All six P1 governance panels and the remaining Allen/RCC8 plus Manifold Logic
  extras now call native kernels. QApps remain excluded. Remaining Logic
  Workbench work is the P2 infrastructure inspector family.
- **2026-08-28:** Bound every remaining Logic Workbench panel. `InfraLogic.compute`
  and `InfraExtLogic.compute` cover the P2 infrastructure inspectors. QApps
  remain excluded. Next non-QApps work is Agreement/Rights, Project, Dataset,
  Studio, Health, and the other specialist surfaces in section J.
- **2026-08-28:** Replaced static Agreement and Rights sample tables with
  persistent COP records. Shared daemon routes `/records/query|upsert|delete`
  write `{storage}/poet-cop-records.json`; restart round-trip is tested.
  The same ledger already accepts `project` records for the next family.
- **2026-08-28:** Replaced every Project mock table with COP-backed records
  (`project_*` families, optional `kind` filter). Dashboard/analytics/resource
  report are live counts; Kanban groups live `project_task` rows by status;
  bulk import accepts JSON Lines and exports the selected family as JSON.
  Wallet mint, DID signing, consensus tallies, and agent answers stay unbound
  and labelled. Vibe remains 0.1; QApps remain excluded. Next: Dataset/Ontology.
- **2026-08-28:** Replaced Dataset/Ontology mock tables with COP-backed
  registries. RDF/N3/JSON-LD import uses Semantic Library ingest; N3 parse and
  graph validate call `N3Logic.evaluate` / `GraphAuthoring.process`. Natural
  persons are `rdfs:Class` + SHACL/ShEx; `owl:Thing`/`owl:Class` on a person is
  rejected in the relation builder, N3 sample, and native authoring path.
  CAD/video/super-resolve/render sessions stay unbound and labelled. Vibe 0.1;
  QApps excluded. Next: Studio as Scene/Audio sessions.
- **2026-08-28:** Vibe 0.1 catalog synced to host `ALL_BOUND` (no version bump):
  `N3Logic.evaluate`, `GraphAuthoring.process`, workbench `*.compute` IDs, and a
  lockstep test. GraphAuthoring/N3 hover states that persons are not owl:Thing.
- **2026-08-28:** Studio views are Scene/Audio/Animation sessions. Dual Studio
  remains the live VibeScript+GPU viewport. Transport calls `Audio.transport`;
  synth calls `Audio.oscillator`; scene create calls `Scene.create`. Mixer/DCC
  chrome is not a nested DAW. QApps excluded. Next: Health.
- **2026-08-28:** Absorbed Studio into POET structure. The Studio manifold now
  seeds Dual Studio + Scene session + Audio session only. Dual Studio also
  lives on Vibe and Media. Spatial toolbox places Dual Studio / Scene session;
  Audio toolbox places Audio session. Channel strip / routing / meters remain
  inspectable records, not a nested DAW layout. Next: Health.
- **2026-08-28:** Health is COP records + Semantic Library (classified/secret)
  + permissive share to a named clinician DID + nlp.analyze/gazetteer/Document.ingest
  on extracted PDF/report text. Binary PDF decode remains unbound (documented).
  Manifold seeds overview, documents, share, conditions — not a nested EHR.
  No fabricated clinical values. Next: Governance/Device/Social.
- **2026-08-28:** Remaining specialist surfaces persist as COP session records
  (social, presence, channel, finance, wallet, aura, webrtc, vision, listen,
  triad, portal, webview, governance, device). Nested-object fields and
  unknown families fail closed; unconfigured store is unavailable. Chrome UAT
  still needs the principal to launch poet-ui against a local daemon.
- **2026-08-28:** Re-audit after concurrent progress fixed the process-global
  COP test race, wired specialist command-palette entries to their implemented
  containers, added complete construct HCF/HMC package export/import, mirrored
  authored subjects to COP when connected, and surfaced the existing Pulse SSE
  stream in Pulse containers. Checkpoint snapshots now restore/export exactly;
  the reachable publication panel performs only real HCF/HMC/snapshot work and
  disables unsupported stages.
- **2026-08-28:** Reconciled concurrent implementation work without replacing
  it. Credential, provenance, context-markup, constituency, capability-grant,
  and explicit-consent surfaces now use allowlisted COP families; added a
  persistence/query regression test. The merged POET library suite passes all
  223 tests. Final system-Chrome UAT remains open.
- **2026-08-28:** Built and served the final WASM app on `127.0.0.1:8080`
  against the dev daemon on `127.0.0.1:4242`. System Chrome verified the
  `0.0.35` UI, native connection (708 Quins), persisted Semantic Library, and
  Search keyboard shortcut. Chrome's automation guard then detected another
  extension popup; the retained POET tab is ready to resume after that popup is
  dismissed.
- **2026-08-28:** Promoted the deferred QApps work into the next normative
  programme (section K). Legacy QApps will be classified and converted into
  native POET constructs/manifolds/containers/Library entries, then the old
  runtime and package path will be removed. No parallel QApp runtime is planned.
- **2026-08-29:** Withdrew the 2026-08-28 broad completion claims after finding
  that generic record persistence had replaced substantial domain interaction
  design. Recorded the evidence, harm/cost, revised completion standard, and
  remediation programme. QApps-to-POET migration now preserves user jobs, not
  merely structural placement.
- **2026-08-29:** Continued the non-QApps product restoration with sequenced
  Local AI token streaming, cooperative cancellation, reviewer-DID assertion
  decisions, terminal agent-run receipts, per-run token ceilings, safe resident
  model activate/evict controls, and daemon-enforced profiled-agent authority.
  Added creator-issued invitation-only channel membership and an incoming
  social/Pulse inbox with immutable request principals. Connector invocations
  now persist execution receipts and permit at most three attempts only for
  capabilities whose negotiated effect class is Pure. Native core and POET
  WASM checks pass. QApps remain an active parallel migration into native POET
  constructs/manifolds rather than a second runtime.
