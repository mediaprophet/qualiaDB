# Master Execution Checklist — WellFair + Cooperative + Agency

**The single source of truth for what's left and what's done.** Check items off as they land
(green build + tests). Detail lives in the linked plans/ADRs; this file is the tracker.

- Plans: [remaining-work-consolidated](remaining-work-consolidated-plan.md) ·
  [cooperative-qapps](cooperative-qapps-desktop-implementation-plan.md) ·
  [wellfair-webizen-desktop](wellfair-webizen-desktop/README.md)
- ADR: [authority-attestation-guardianship (supported agency)](adr-authority-attestation-guardianship-model.md)
- Logs: [WELLFAIR_DESKTOP_PROGRESS_LOG](../../WELLFAIR_DESKTOP_PROGRESS_LOG.md) ·
  [COOPERATIVE_QAPPS_PROGRESS_LOG](../../COOPERATIVE_QAPPS_PROGRESS_LOG.md)

**Decisions taken (Timothy "you've got the gist", 2026-07-03; overridable):**
- Naming: **"domains of agency"** (not "guardianship roles"); guardianship = one relationship pattern within it.
- Values-anchor: **required on every delegation, with a UN-HR / UNCRC default** (overridable per delegation).
- Consequential domains (declared-reliance + epistemic-horizon **mandatory**): medical, legal, financial,
  reproductive/biometric/genetic, civic. Others: recommended, not required.

---

## A. Done (shipped / committed unless noted)

- [x] WellFair MVP: 8 domains, consent/policy, receipts, Samsung import, sleep, medication, offline
- [x] Sanctuary encrypted vault (PBKDF2-310k + AES-256-GCM) + independent decoy lane; plaintext note path retired
- [x] Content-addressed blob store (credential claims + clinical attachment bytes)
- [x] Phase-5 sync-operation protocol: versioned ops, quarantined inbox, replay-safe convergence
- [x] `qualia-cooperative-core` crate + work-item Kanban (replay-safe board)
- [x] Extensible taxonomy primitives (`taxonomy.rs`) — open registry, sphere (selfhood/personhood)
- [x] `AuthorityType` **reframed for supported agency** (Modality × Trigger × Accountability + presets)
- [x] **T1.3** `aead` API modernization in `sanctuary_crypto.rs` (7/7 tests, warnings gone)
- [x] **ZK review** — `crypto/zk_proofs.rs` is REAL Groth16 (arkworks, default `zk-culling`); 7/7 incl. soundness. CLAUDE.md note corrected.
- [~] **T1.1** government-letter attachment bytes (host+cmd+bridge+panel) — code done, **pending commit** (folds into next commit)

## B. Agency layer (ADR §7–§10). New isolated files in `qualia-cooperative-core`. **60 crate tests green.**

- [x] `agency_domain.rs` — the 17 domains of agency (personhood), extensible, sphere-tagged; consequential flag
- [x] `agency_delegation.rs` — `AgencyDelegation` (principal + agent(s) + domain + `AuthorityProfile` +
      values-anchor + scope + jurisdiction + **precedence** [primary/secondary/local-temporary] + validity +
      consent + evidence-chain ref)
- [x] `Trigger` algebra — `VerifiableEvent | TemporalWindow | DeadmanSwitch | HumanConsensus{m,n,capacity}`
      composed with All/Any/Not (ADR §7.1) — externally-tagged serde
- [x] Developmental transfer schedule — monotonic `GuardianSole → CoSigned → PrincipalSole`, trigger-gated
      (ADR §7.2). *(Model done; the signed `TransferEvent` runtime flow is host-level, below.)*
- [x] `AgentType` (natural | software/AI | organization | instrument | dataset) + `RelianceDeclaration`
      (standing) + `JudgementProvenance` (`informed_by` **DAG**) + `has_undeclared_ai` (ADR §8, §9)
- [x] Epistemic horizon — content-addressed Merkle/checkpoint ref field on `JudgementProvenance` (ADR §9)
- [x] Disclosure model — `DisclosurePolicy` (subject Full / others SelectiveField default) × modality
      (full | selective field | **ZK property proof**); real Groth16 available, predicate circuits TBD (ADR §9)
- [x] Veracity/characteristics substrate — `InputVeracity` dual-timed (at-time vs determined), instrument
      characteristics via `AgentRef` version/capacity (ADR §10). *(Paraconsistent adjudication = host/engine, humans decide.)*
- [x] ABAC evaluation — `delegation_permits(...)` with **selfhood default-deny** + consequential-domain
      provenance-required + trigger-gating + jurisdiction match
- [ ] Host API + Tauri commands + Studio panel(s) — Social Book / Agency surface
- [ ] Migration: keep `wellfair` project/finance/contribution ids traceable; wire guardianship `Suspend`;
      generalize `government_letter` → authority attestation; predicate circuits for ZK property-proofs

## C. WellFair finish-out

- [ ] **T1.2** OS-keychain vault wrapping — **⚑ recovery-model gate** (keychain loss = vault loss); build
      opt-in/off-by-default; enable only after Timothy's recovery-code decision
- [ ] **T1.4** native file dialogs (`tauri-plugin-dialog`) for attach/export; typed paths remain fallback
- [ ] **T1.5** guardianship M:N — folded into the agency layer (§B); wire `PolicyService::Suspend`
- [ ] Generalize `government_letter` → **authority attestation** record (ADR §2) once agency layer lands

## D. Human-gated (Timothy decides, then implementable)

- [ ] **T2.1** Sanctuary threat-model ADR — KDF/AEAD/decoy/keychain-recovery sign-off
- [ ] **T2.2** Mental-wellbeing assessment instruments (DASS-21/PHQ-9/GAD-7/K10/BDI-II) — per-instrument
      licence/scoring/interpretation/disclaimer

## E. Large efforts (staged; multi-session)

- [ ] **T3.1** real sync transport (libp2p / WSS) — drains outbox, feeds inbox; hostile-peer + convergence tests
      (shared with cooperative WP7)
- [ ] **T3.2** companion PWA + secure-origin pairing (HTTPS/WSS or WebRTC) — replaces plain LAN-WS gateway
- [ ] **T3.3** Phase-6 release hardening (reproducible builds, installers/signed updates, SBOM, backup/restore,
      accessibility audit, 42 MB Sentinel, diagnostics)
- [ ] **T3.4** Phase-7 optional (anatomy, studies/rules, authenticated Solid Pod sync, model-assisted
      extraction, wallet, distributed analytics, native mobile)

## F. Cooperative Qapp plan work packages (parallel initiative)

- [ ] **WP1** Qapp token v2 + per-app isolation + CSP (release gate for restricted-data Qapps)
- [ ] **WP2** Studio Package & Publish (create least-privilege Qapps without hand-editing JSON)
- [ ] **WP4** standalone Cooperative Qapp shell
- [ ] **WP9** QualiaDB Development Cooperative (bind repo read-only; backlog/claims/changes/reviews)
- [ ] **WP5/6/8/10/11** finance-receipts / agreements / advanced-economics / forge-CI / release-hardening

---

## Sub-agent orchestration plan

The agency layer (§B) fans out cleanly because its modules are **new, isolated files** — no shared-file
collisions (no worktrees, per repo policy §0). Approach:

1. **Parallel author (workflow):** one sub-agent per new file — `agency_domain.rs`, `Trigger` model,
   `AgentType`+provenance-DAG, `AuthorityProfile`-consuming delegation core — each self-contained with its
   own `#[cfg(test)]`, given the ADR + `taxonomy.rs`/`authority_type.rs` as the contract. Adversarial
   review stage per module (compile/convention/selfhood-default-deny/honesty checks).
2. **I integrate serially:** wire `lib.rs`, resolve cross-module types, run the full `-p
   qualia-cooperative-core` + downstream `-p qualia-client-core` build, fix, and only then check items off.
3. **Never fire-and-forget:** every sub-agent output is compiled + tested by me before it counts as done.

Independent WellFair items (T1.4 dialogs; a T1.2 opt-in stub) can run as separate agents in parallel with
the agency fan-out since they touch different crates.
