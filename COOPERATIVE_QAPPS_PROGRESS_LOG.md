# Cooperative Qapps & Qualia Desktop — progress log

Running log for the [cooperative-qapps-desktop-implementation-plan](docs/plans/cooperative-qapps-desktop-implementation-plan.md)
(PROJECT RULE §9). Newest entry at the bottom. Honest engineering record.

This is a **large, multi-release initiative** (the plan defines WP0–WP11 and Releases A–F). It is
explicitly *not* a single-session task. The plan's own §5.2 notes that much of the foundation already
exists from the WellFair work — this log tracks what's genuinely new on top.

## Crossover with the shipped WellFair work (what is reused vs new)

**Already built (reused, not rebuilt):**
- `wellfare-core::projects` — projects, memberships, immutable contributions, replay-safe obligation
  derivation. `wellfare-core::finance` — signed minor-unit ledger, project links, multi-currency balances.
- `qualia-client-core::wellfair::api` — signed journal commit path, policy gate, receipts,
  `synced_project_obligations` (validated inbound sync folded into obligations).
- `wellfair::sync_protocol` — versioned content-hashed ops, quarantined inbox, add-wins convergence.
- `wellfair::blob_store` — content-addressed, integrity-verified, path-traversal-safe.
- `wellfair::sanctuary_vault` — real encrypted-at-rest boundary.

**Genuinely new per the plan (what this workstream adds):** the shared `qualia-cooperative-core` crate,
work items / Kanban, phases/roadmaps, budgets, agreements; Qapp token v2 + per-app isolation (WP1); the
Studio Package & Publish pipeline (WP2); the standalone **Cooperative Qapp** (WP4); and the **QualiaDB
Development Cooperative** dogfood (WP9).

---

## 2026-07-03 — WP3 foundation: shared cooperative-core crate + work-item domain (Claude / Opus 4.8) — DONE (backend + desktop)

**Phase / status:** First slice of the cooperative plan (§8 domain model, §24 first slice). Establishes
the shared cooperative crate and the first genuinely-new domain the plan flags as "Build" (work items).
All green.

**What was built:**
- New workspace crate [`qualia-cooperative-core`](crates/qualia-cooperative-core/) — the transport-neutral
  cooperative domain home (plan §8.1), so cooperative work is not a health-vault feature. It re-exports the
  existing `record` / `projects` / `finance` from `wellfare-core` for a single import surface.
- [`work_item.rs`](crates/qualia-cooperative-core/src/work_item.rs): `WorkItem` (Task/Issue/Milestone,
  immutable core) + `WorkItemStatusEvent` (immutable transitions). The current status is a **derived
  projection** (`derive_board`) — the latest event per item, pure over the unique-event-id set — so the
  Kanban board is invariant under duplicated/reordered/replayed transitions (plan §8.3/§8.4).
- Host API on `WebizenHostApi`: `add_work_item`, `add_work_item_status`, `list_work_items`,
  `work_item_board(project_id)`; journal kinds `work_item` / `work_item_status`; policy writer
  `qualia-cooperative`. Persists through the existing signed journal (a dedicated cooperative service may
  take over persistence later; the domain + derivations already live in the shared crate).
- Tauri commands (`wellfair_add_work_item` / `_status` / `_board`) + host_client bridges + a new
  **Work board** Studio panel (Kanban columns, add item, move card = append transition) in the Projects area.

**Measured results:** `cargo test -p qualia-cooperative-core` → **6 passed** (board invariance under
duplicate/reorder, latest-status, canonical column order, summary round-trip). `cargo test -p
qualia-client-core wellfair::` → **87 passed** (+1 host board round-trip). `cargo test -p wellfare-core`
→ 76. `cargo check` green for `webizen-desktop`, `webizen-studio` (host + wasm32).

**⚑ Where I need the human / decisions made:**
- **Architecture default (overridable):** `qualia-cooperative-core` currently *depends on* `wellfare-core`
  for the shared `record` base (rather than the plan's eventual direction of moving record + projects +
  finance *down* into cooperative-core with wellfare-core re-exporting). This is the low-risk way to
  establish the crate without a big-bang move; the full extraction (record base → a foundation crate) is a
  later, compiler-checked refactor. If you'd rather I do the full move now, say so.
- Work items persist via the WellFair journal/policy for now (shared vault). The plan's dedicated
  `qualia-client-core/src/cooperative/` service (§8.1) is a later step.

**Next step (per plan priority):** the big-ticket items are **WP1 Qapp token v2 + per-app isolation**
(a release gate for any restricted-data Qapp) and **WP2 Studio Package & Publish** (so users can create
least-privilege Qapps) — these unblock the standalone Cooperative Qapp (WP4). The **WP9 Development
Cooperative** (bind this repo read-only, backlog/claims/changes) is the dogfood gate. My recommendation:
tackle WP1 next (security foundation), then WP2, then stand up the Cooperative Qapp shell.

---

## 2026-07-03 — Supported-agency layer: domains, triggers, provenance DAG, delegation + ABAC (Claude / Opus 4.8, swarm-authored) — DONE (crate)

**Phase / status:** The load-bearing semantic core (ADR §7–§10, "supported agency" reframe). Built the
three leaf modules via a 3-agent workflow, integrated the delegation core myself. **60 crate tests green.**

**What was built (all in `qualia-cooperative-core`):**
- `agency_domain.rs` — the 17 domains of agency (welfare/socio-economic/technological/civic), extensible,
  sphere-tagged (only reproductive/biometric/genetic is Selfhood), consequential flags on
  medical/legal/financial/reproductive-biometric-genetic/civic.
- `trigger.rs` — the `Trigger` algebra: `VerifiableEvent | TemporalWindow | DeadmanSwitch |
  HumanConsensus{m-of-n, capacity} | All | Any | Not`, `evaluate(...)`. **Crypto + subjective human
  consensus, composed** (2 doctors attesting incapacity = a trigger primitive).
- `provenance.rs` — `AgentType` (natural/software/org/instrument/dataset), `AgentRef`, dual-timed
  `InputVeracity`, `Reliance` + `JudgementProvenance` as a real **DAG** (`Option<Box<..>>`),
  `RelianceDeclaration` (standing toolchain disclosure), epistemic-horizon field, `DisclosurePolicy`
  (subject Full / others SelectiveField), and `has_undeclared_ai` (integrity-breach detector).
- `agency_delegation.rs` (integrator) — `AgencyDelegation` (principal + agents + domain + authority
  profile + trigger + **required values-anchor** + scope + jurisdiction + **precedence** + validity +
  consent + evidence-ref + developmental transfer schedule) and `delegation_permits(...)` — fail-closed
  ABAC with **selfhood default-deny**, consequential-domain **provenance+horizon required**, trigger
  gating, and jurisdiction match (the backpacker case).

**Bug caught by integration (not by agent inspection):** `trigger.rs` used internally-tagged serde
(`#[serde(tag="type")]`) on a recursive enum with sequence-wrapping newtype variants — invalid for serde
and caused an unbounded serializer trait-resolution overflow (compile hung at recursion_limit 2048).
Fixed to the default externally-tagged representation; crate compiles instantly. Reinforces: agents
verify by inspection, the integrator verifies by compiling.

**Measured:** `cargo test -p qualia-cooperative-core` → **60 passed**. `cargo check -p qualia-client-core`
→ green.

**Next:** host API + Tauri commands + a Social Book / Agency Studio panel; wire guardianship `Suspend`;
generalize `government_letter` → authority attestation; author predicate circuits so ZK property-proofs
back the disclosure modality with the (now real) Groth16.
