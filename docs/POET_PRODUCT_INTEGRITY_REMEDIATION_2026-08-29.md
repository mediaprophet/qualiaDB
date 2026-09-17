# POET product-integrity and liability remediation

**Date opened:** 2026-08-29  
**Status:** Active; broad non-QApps completion claims withdrawn  
**Scope:** Standalone POET UI and the implementation/tracking process that
produced the 2026-08-27/28 parity claims

## Why this record exists

The implementation programme optimized for visible route coverage, persistence,
and honest failure states, but in many places treated a generic COP record form
as a replacement for a domain workflow. That is not functional parity and does
not deliver the intended cooperative, fiduciary, creative, clinical, semantic,
and project outcomes.

The resulting harm is loss of product utility and trust. The direct cost is the
principal's time, repeated audits, rework, and delayed delivery. Misstating that
state as “fully implemented” creates a further risk that users, contributors,
or funders rely on capabilities the interface does not actually provide.

This is a record against the implementation outcome. It does not assign a legal
conclusion or invent an actor identity. It preserves the evidence and defines
the remedy.

## Evidence recorded on 2026-08-29

- The authoritative Workstream A plan contains **545 `[todo]`**, **1 `[wip]`**,
  and only **13 `[done]`** markers.
- **115** domain view files were reduced to thin `pub use ... build_*_view`
  delegations in the current worktree.
- Across those 115 files, the working diff contains **230 additions and 30,870
  deletions**: a net removal of 30,640 lines of domain-specific UI.
- **118** view builders are concentrated in six generic persistence modules.
  Four of those new modules exceed the repository's 500-line new-file limit;
  `specialist_persist.rs` is 814 lines.
- Representative regressions include Budget (six domain tabs reduced to one
  budget-record form), project planning views, governance disputes, health
  timelines, and Studio material/audio/scene controls.
- The parity tracker nevertheless marked entire Project, Studio, Health,
  Governance, Device, Dataset/Ontology, Agreement, and Rights families complete.
- The 223 passing POET unit tests establish code-level invariants; they do not
  prove domain workflow parity or usefulness.

The removed implementations contained fabricated/static data and therefore
could not simply ship. The failure was replacing them with generic CRUD and
then closing the feature, instead of retaining their interaction design and
wiring it to real state and capabilities.

## Completion standard from now on

A surface is complete only when evidence covers all applicable dimensions:

1. **User outcome:** a plain-language job the person can finish.
2. **Domain interaction:** the appropriate board, timeline, editor, graph,
   calculator, mixer, consent flow, or decision surface—not merely raw fields.
3. **Real state:** persisted domain data with provenance, sensitivity, and
   identity/consent boundaries where applicable.
4. **Real actions:** create, edit, transition, link, calculate, validate,
   publish, export, or otherwise perform the advertised work.
5. **Decision support:** derived totals, status, conflicts, dependencies,
   history, or other domain-relevant projections are computed from real data.
6. **Failure integrity:** unavailable operations are disabled with the exact
   prerequisite, but a disabled control is not counted as delivered utility.
7. **Lifecycle:** reload, undo/redo, checkpoint, manifold switching, and
   HCF/HMC behavior are verified where relevant.
8. **Human verification:** accessibility and Chrome UAT cover the actual task,
   not only page presence.

Persistence is a foundation, not a substitute for items 1–8.

## Remediation programme

### R0 — stop further claim inflation

- [x] Withdraw broad family-level completion claims in the parity tracker.
- [x] Record the regression and cost in this document and the root changelog.
- [x] Add an automated source audit that prevents the delegation count from
  increasing and prevents generic persistence from being labelled full parity.
  `crates/poet/tests/product_integrity.rs` fixes 115 as a ceiling that must
  trend downward and requires withdrawn claims to remain visible. The Budget
  restoration reduced the measured count to 114.
- [ ] Split new persistence modules above 500 lines by domain responsibility.

### R1 — recover the product specification without principal re-authoring

- [ ] Build a machine-readable surface inventory from the Dioxus UI, the
  standalone POET sources before collapse, Workstream A, the original handover,
  registrations, and capability catalogue.
- [ ] For every surface, record the intended user job, preserved interaction
  design, current implementation, backend contract, missing behavior, and UAT.
- [ ] Reconcile contradictory trackers; the most conservative evidence-backed
  status wins.
- [ ] Define explicit deletion decisions for features that do not serve POET's
  intended purpose rather than silently replacing them with technical demos.

### R2 — restore high-impact workflows

- [ ] Project economics: budget versus actuals, funding, compensation,
  obligations, royalties/tax routing, variance, and auditable exports.
  - [x] Replace the single generic budget form with distinct live plan,
    actual, funding, royalty, and tax ledgers.
  - [x] Compute unit-separated plan/actual variance and funding position using
    fixed six-decimal arithmetic; exclude draft, observed-only, settled,
    cancelled, and malformed rows from live totals.
  - [x] Export a current-daemon audit JSON bundle with an explicit warning that
    lifecycle evidence is not proof of payment or legal settlement.
  - [ ] Integrate compensation and obligation records, add approval authority
    transitions, and complete system-Chrome task UAT.
- [ ] Project delivery: interactive Kanban/task transitions, dependencies,
  milestones, roadmap/Gantt/calendar, risks, deliverables, and backlinks.
- [ ] Agreement/Rights/Governance: lifecycle, parties, consent, evidence,
  disputes, decisions, obligations, remedies, and provenance.
- [ ] Health: person-controlled timelines, measurements, records, disclosures,
  safeguards, and consent—not a generic mini-EHR or sample clinical values.
- [ ] Studio/Media: preserve purpose-built creative controls and bind them to
  real Scene/Audio/Render sessions instead of session-record forms.
- [ ] Dataset/Ontology/Knowledge: restore visual lineage, mapping, validation,
  semantic editing, and Library integration around the real graph backend.
- [ ] Device/Social/Communications: implement the human workflow up to genuine
  permission/session boundaries; do not count saved configuration as operation.

### R3 — economic and fiduciary safeguards

- [ ] Add provenance receipts for transformations that affect budgets,
  compensation, licensing, rights, obligations, or publication.
- [x] Require explicit actor, currency/unit, sensitivity, and effective date on
  economically consequential records; reject ambiguous amounts.
- [x] Separate planning, approval, commitment, settlement, and observation so a
  saved amount cannot be presented as paid or legally effective.
- [ ] Add conflict/consent/authority gates before irreversible or externally
  consequential actions.
- [ ] Provide append-only audit/export views suitable for independent review.
- [ ] Prohibit ROI, risk, clinical, legal, or governance claims derived from
  demo/default payloads from receiving a `live` badge.

### R4 — QApps-to-POET migration (active parallel programme)

The QApps refactor is being carried out as migration into native POET format,
and it must use the completion standard
above. A migrated QApp is not complete because it became a manifold/container;
its actual user job must survive. Follow section K of the parity tracker after
the R1 inventory establishes the baseline.

### R5 — restore POET as a usable human application platform

This work is broader than repairing isolated specialist forms. POET must expose
the ordinary application capabilities people expect, while preserving its
semantic, local-first, consent-aware architecture.

- [~] Local LLM and agent use: a first-class Local AI workspace now selects a
  real GGUF/P64 model, optionally retrieves grounding from the Semantic
  Library, runs the existing guarded local inference runtime, renders the
  answer and evidence without unsafe HTML, and persists a bounded turn record.
  Bounded resident/configured model discovery and persisted-turn restoration
  are now included, and named conversations supply up to eight prior persisted
  turns as bounded context for follow-up prompts. Decode now runs as a bounded
  server-side job with sequenced token SSE, cooperative cancellation in native
  and fallback loops, retained terminal events, and honest partial-output
  handling. Completed assertions enter a human approval/rejection queue with a
  reviewer DID; cancelled output is not stored as a completed turn. Model
  activation/eviction controls now operate on the real process-wide resident
  model slot and refuse to replace memory while a decode job is active. Each
  request has an explicit 1–256 token budget, and completed/cancelled/failed
  runs persist separately as terminal operational receipts. Model installation,
  saved agent profiles now enforce the controlling principal DID, selected
  model, per-run token ceiling, local-inference capability, and an exact
  `project:<tag>`/all/none Semantic Library scope at the daemon boundary.
  Connector/tool execution grants, aggregate resource budgets, and browser UAT
  remain. The cancellation
  lifecycle was extracted to `inference_agent/control.rs`; the pre-existing
  1,000+ line decoder receives only cooperative checks pending its tracked
  decomposition.
- [~] Social communication: a first-class Social workspace now provides
  DID-attributed persistent threads, channel filtering, composition, refresh,
  and live Pulse publication notices. It also supports semantically described
  channel creation and explicit
  relationship request acceptance/denial by the receiving DID. The inbox now
  exposes incoming decisions, recent persisted Pulse activity, and creator-only
  invitation issuance for invitation-only channels; acceptance activates the
  invitee's participant record. Request principals/scope are immutable after
  creation and a terminal decision cannot be rewritten. Cryptographic receipts,
  attachments, encryption, blocked-relationship message enforcement, and
  multi-host transport remain. Creator-only participant role administration
  and creator/active-moderator non-destructive hide receipts are implemented;
  originals and reply topology remain preserved.
  Same-thread replies are validated against real message IDs. Bounded DID
  mentions create local recipient receipts with immutable principals/source;
  only the recipient can irreversibly mark a receipt read. Open-channel membership is explicit and
  persistent; restricted channel posting requires an active participant record.
  Voluntary presence is scoped, expiring, updateable, and separately announced
  over Pulse.
- [~] Semantic connectors: a first-class Connectors workspace now discovers
  negotiated native capabilities, shows their semantics/transport/mode, runs
  selected capabilities with explicit JSON arguments, and persists connector
  contracts containing interface, input-class, and output-class IRIs.
  Native connector records are reconciled against the negotiated catalogue and
  can run stored JSON probes; unprobed external transports cannot be marked
  enabled. A connector contract can publish its interface, input/output
  classes, transport, capability, auth mode, sensitivity and status as JSON-LD
  in the persistent Semantic Library for discovery by people and agents.
  Native negotiation now also carries the existing machine argument/return
  schemas, semantic family, effect class, and honesty classification; selecting
  a capability pre-fills its runner and semantic descriptor from that contract.
  Every runner/saved-probe invocation now persists a bounded execution receipt.
  Failed operations can be retried only when negotiation declares them `Pure`,
  and only through attempt three; Cold/unknown-effect operations require a new
  explicit run. Endpoint adapters, authentication ceremonies, schema-derived
  forms, semantic mapping transformations, and external connector UAT remain.
- [ ] Identity and relationship lifecycle: profiles, contacts, relationship
  requests, consent, delegation, blocking, groups, roles, and verifiable
  provenance must become coherent user workflows across Social and Projects.
- [ ] Communications: notifications, mail/activity inboxes, mentions, replies,
  attachments, calls/sessions, presence, and channel administration must use
  real delivery/session contracts and honest receipt states.
- [~] Agent operations: reusable agents, scoped tools/connectors, memory and
  aggregate budgets,
  evidence, and human approval gates must be manageable from the UI.
  Streaming cancellation, per-run token budgets, resident model activation/
  eviction, durable terminal run history, and the first human assertion-review
  queue are now implemented; approval records review only and cannot trigger a
  tool effect. Profiled agents now enforce principal, model, token and exact
  Semantic Library project authority before decoding; connector/tool grants
  beyond read-only local inference remain.
- [ ] Connector semantics: connector descriptions must be graph-queryable and
  reusable by agents and workflows, with typed inputs/outputs, sensitivity,
  authority, provenance, and transport status represented separately.

The three `[~]` entries identify useful implemented slices, not completion of
their families.

## Delivery discipline

- Delivery is counted in completed user jobs, not routes, files, lines changed,
  tests launched, screenshots, or narrated implementation activity.
- Work proceeds in coherent multi-outcome batches. A narrow patch is not
  presented as the completion of a product family.
- Verification is proportional to risk and normally runs once after a batch;
  build/test activity is evidence, never the delivered outcome.
- Infrastructure exposure, saved configuration, disabled controls, and nominal
  CRUD are not accepted as substitutes for a functioning workflow.
- Status updates lead with new usable behavior and remaining user-visible gaps.
  They do not use ceremony to inflate the apparent amount of work.

## Immediate next slice

1. [x] Add the source/claim integrity guard.
2. [~] Restore the Project Budget surface as the first economically consequential
   exemplar: real plan/actual/funding/royalty/tax records, derived variance,
   explicit currency and lifecycle semantics, and auditable export. The five
   ledgers, derived summary, validation, and export are implemented; authority
   transitions, compensation/obligation linkage, and browser UAT remain.
3. [~] Deliver the first whole-platform application batch: persistent social
   threads, Semantic-Library-grounded local model use, and semantically
   described runnable connectors. Core paths are implemented; the remaining
   lifecycle items are enumerated in R5 rather than hidden behind a family
   completion claim.
4. [~] Implement conversation history/model management, social membership and
   receipts, and connector schema/auth/health workflows. This batch added
   bounded model discovery, turn restoration, semantic channels, receiving-DID
   request transitions, open/request-policy channel membership, expiring
   presence, connector catalogue reconciliation, machine schemas, saved probes,
   and Library JSON-LD publication.
   Streaming/cancellation and assertion review are now implemented. The same
   batch now includes invitation-only channel administration, an incoming
   activity/decision inbox, immutable request principals, connector execution
   receipts, and bounded Pure-only retry. Cryptographic receipts, moderation,
   schema-derived forms, authentication ceremonies, and remote transports remain.
5. [~] Complete model lifecycle/agent authority and communications delivery:
   activation/eviction, active-job safety, per-run token budgets and durable
   agent-run browsing are implemented. Profiled agents now enforce principal,
   model, token ceiling and exact Semantic Library project scope. Add bounded
   model installation, scoped connector/tool grants, aggregate resource budgets, and
   implement attachments, moderation, encryption and real
   multi-host receipt contracts.
6. Apply the same product standard to project delivery and Agreement/Rights
   before resuming catalogue-scale QApp migration.

## Verification record

- `cargo test -p poet --test product_integrity`: 3 passed.
- `cargo test -p poet --lib budget_model`: 2 passed.
- `cargo test -p qualia-core-db --lib services::poet_record_api`: 9 passed.
- `cargo check -p poet --target wasm32-unknown-unknown`: passed.
- `cargo check -p qualia-core-db --lib`: passed after adding the bounded local
  LLM endpoint, streamed/cancellable job service, social transition guards,
  connector receipt validation, and semantic connector validation.
- `cargo check -p poet --target wasm32-unknown-unknown`: passed after wiring
  streamed Local AI, assertion review, the social inbox/invitations, and
  connector run history/retry into the browser application; passed again after
  resident model controls, token budgets, and terminal agent-run history.
- System Chrome reloaded the live POET build and visibly exposed `💬 Social`,
  `🤖 Local AI`, and `🔌 Connectors` as Tool Chest applications. Chrome's DOM
  and screenshot channel can read the page, but both semantic and DOM click
  dispatch still time out before reaching the page; task-level interaction UAT
  for these three workspaces therefore remains open.
- System Chrome loaded the rebuilt app at `127.0.0.1:8080` and displayed a
  live daemon connection on port 4242 with 708 graph Quins. DOM and screenshot
  inspection worked. Chrome-control click dispatch timed out before execution,
  including after reload; therefore the Budget task UAT remains open and is not
  counted as passed. The captured console errors came from the Adobe Acrobat
  Chrome extension, not from the POET origin.

## Closure rule

This remediation cannot be closed by test count, route count, persistence
coverage, or an apology. It closes only when the surface inventory is complete,
each claimed workflow has task-level evidence, the reopened tracker items are
resolved, and the principal no longer has to reconstruct omitted requirements
for implementation agents.
