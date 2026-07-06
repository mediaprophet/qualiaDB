# Master Execution Checklist — WellFair + Cooperative + Agency

**The single source of truth for what's left and what's done.** Check items off as they land
(green build + tests). Detail lives in the linked plans/ADRs; this file is the tracker.

- Plans: [remaining-work-consolidated](remaining-work-consolidated-plan.md) ·
  [cooperative-qapps](cooperative-qapps-desktop-implementation-plan.md) ·
  [wellfair-webizen-desktop](wellfair-webizen-desktop/README.md)
- **Future/parked initiatives register (so nothing is forgotten):**
  [future-work-register](future-work-register.md) — ★ 3D anatomy prioritized; native visual, native
  auditory, T3.4 Phase-7, and the cooperative WP series parked with pointers.
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
- [x] **T1.1** government-letter attachment bytes (host+cmd+bridge+panel) — **DONE/committed** (`dd931a64`;
      `add_government_letter_attachment_from_path` host + Tauri command + bridge wired).

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
- [x] Host API + Tauri commands + Studio panel(s) — **Agency surface DONE** (`59acdbf1`, local): host
      API (create/list/get/consent/revoke/evaluate/domains) persisting delegations through the signed
      journal (lossless summary, latest-version-per-id) + 6 Tauri commands + a Studio **Agency** area/panel
      (domain picker flagging consequential ⚠, grant/withdraw/revoke, "Test read/write/decide" showing the
      ABAC verdict + reason — surfaces selfhood default-deny & consequential-provenance). +2 tests.
- [x] Migration: **guardianship `Suspend` wired** (§C T1.5); **`government_letter` → authority attestation
      generalized** (wellfare-core `authority_attestation.rs` — extensible authority type + agent-in-capacity
      + jurisdiction/department + PDF|credential|both; 8 tests; host `add_authority_attestation` + journal
      kind; gov-letter is now a preset); **ZK predicate circuits landed** (core-db `zk_predicates.rs` — real
      Groth16 threshold + range via 64-bit decomposition; false statements *unprovable*; soundness +
      public-input-binding tests, incl. 2 integrator adversarial checks). Remaining: keep `wellfair` ids
      traceable when the dedicated cooperative service lands (deferred to that service).

## C. WellFair finish-out

- [x] **T1.2** OS-keychain vault wrapping — **DONE (mechanism, opt-in, off by default)**: an OS-keychain-held
      pepper folds into the Sanctuary KDF so disk + PIN alone can't open a wrapped vault; `setup_wrapped`
      returns a one-time recovery code; `unlock_with_recovery` handles keychain loss. core-db
      `sanctuary_keychain.rs` + `sanctuary_vault.rs` pepper-threading (3 hermetic tests) + host API + 3
      Tauri cmds + host_client bridges + experimental toggle in the vault panel. Unwrapped stays the
      default (unchanged). ⚑ *Turning it on by default / recovery-code UX copy = Timothy's call (T2.1).*
- [x] **T1.4** native file dialogs (`tauri-plugin-dialog`) — **DONE**: `wellfair_pick_file_path` /
      `wellfair_pick_save_path` commands + capability + host_client bridges + Browse buttons in the
      clinical Attachments section; typed paths remain the fallback. Desktop + studio (host + wasm) green.
- [x] **T1.5** guardianship M:N — **DONE**: proxy writes of protected (Restricted) records suspend into a
      persisted `GuardianshipProposal`; M-of-N guardians co-sign with immutable votes; status is a
      replay-safe *derived* projection (latest-vote-per-guardian, protective veto); on ratification the
      escrowed record commits through the signed vault path (idempotent). `wellfare-core/guardianship.rs`
      (10 tests) + policy `Suspend` production + `submit_record_guarded`/escrow/vote host API (3 host tests)
      + 3 Tauri cmds + host_client bridge + **Guardianship** Studio panel/area. 94 wellfair tests green.
- [x] Generalize `government_letter` → **authority attestation** record (ADR §2) — **DONE**: wellfare-core
      `authority_attestation.rs` is the general model (extensible authority type + agent-in-capacity +
      jurisdiction/department + PDF|credential|both), `government_letter` is a preset over it, host
      `add_authority_attestation` + journal kind wired. Agency layer has now landed too.

## C2. Sanctuary vault v2 — CBOR-LD + coercion-response decoy (active)

Design + full task breakdown: [adr-sanctuary-vault-v2-cbor-decoy-mirroring](adr-sanctuary-vault-v2-cbor-decoy-mirroring.md)
(§12 = slices + swarm curation). Decisions locked: Option A (functional decoy), §3 asymmetric crypto,
CBOR-LD, retention = both modes as a real-session toggle (default auto). **Nothing pushed without Timothy.**

- [x] **Argon2id + PIN policy + constant-work unlock** landed (`a60671a1`).
- [x] **S0** audit primitives — `core-db/crypto/sanctuary_audit.rs` (X25519 sealed box + key-wrap +
      hash-chain), 8 tests (`2d69a21f`, **local, not pushed**).
- [x] **A** (swarm) audit DAG + retention domain — `sanctuary_audit_dag.rs` (AuditRecord content-address,
      verify_chain, derive_sessions [entry-point sessions, not a headcount], route). 21 tests green.
- [x] **B** (swarm) retention-toggle Studio panel (§8 copy, default auto) — with a hard gate that renders
      nothing in a decoy session. Host + wasm green.
- [x] **C** (swarm) adversarial primitive suite — 14 tests, 360+ byte-flip assertions, all fail-closed.
      ⚑ **Finding:** the hash-chain is tamper-*evident*, not a MAC → **S5 must anchor each branch head**
      (ADR §10 integrity boundary).
- [x] **S3** (integrator) n-layer CBOR container `wellfair/vault_container.rs` — CBOR-native, constant-shape,
      4 tests. **No JSON, no migration** (no deployed vault; Timothy 2026-07-03).
- [x] **S5** (integrator) vault surgery **COMPLETE** — 3 local commits (**not pushed**): `ffc4dda3` (S5a:
      `VaultMeta`→`VaultContainerV2`, CBOR-native, **serde_json / JSON path removed**, `.cbor` file),
      `940eeac6` (S5b: real→decoy one-way key hierarchy + `real_curate_decoy_add_note`), `ba415e80` (S5c:
      blind sealed audit append on decoy writes, `review_decoy_activity`, **real-lane head anchor** flagging
      truncated/forged witnessed prefix — C's finding). 33 vault tests green.
- [x] **S6** (integrator) host API + Tauri + bridges + nav **COMPLETE** — 3 local commits (**not pushed**):
      `e5933a8f` (S6a host API), `00448f50` (S6b real-session-only retention persistence; 30 vault tests),
      `c4e29a5b` (Tauri commands `wellfair_{review_decoy_activity,curate_decoy_note,get/set_decoy_retention_mode,
      sanctuary_vault_add_note_in_session}` + studio bridges + a "Cover space activity" review/curate/retention
      section in the vault's **real-lane** view; retention panel B now takes the real PIN as a prop). host +
      wasm + desktop checks clean. **C2 vault v2 workstream is fully done.**

**Standing rule (Timothy 2026-07-03):** vault serialization is **CBOR / CBOR-LD only — no JSON path** may
exist; code depending on JSON gets refactored, not migrated. (Broader codebase-wide JSON→CBOR is a separate
sweep — flag to Timothy before undertaking.)

## D. Human-gated (Timothy decides, then implementable)

- [~] **T2.1** Sanctuary threat-model ADR — **DRAFTED, awaiting Timothy's sign-off**:
      [adr-sanctuary-threat-model](adr-sanctuary-threat-model.md). 6 decisions surfaced (KDF PBKDF2→Argon2id?,
      keychain-wrapping default, recovery-code UX, decoy semantics, PIN policy/lockout, at-rest for the
      health journal) + 5 code-level findings incl. a timing side-channel in `open_lane` (real PIN = 1
      PBKDF2 pass, decoy = 2 → observable) and the decoy lane being structurally visible on disk. **⚑ Your call on the 6 decisions.**
- [~] **T2.2** Mental-wellbeing assessment instruments — **engine + PHQ-9 + GAD-7 DONE** (`75bc2334`,
      local): data-driven scoring engine (fail-closed, exhaustive band coverage, safety flags) + the two
      canonical free instruments end-to-end (wellfare-core `assessment.rs`, host API, 3 Tauri cmds, a
      "Wellbeing check-in" Studio panel with disclaimer + flag alerts + history). 9+1 tests.
      ⚑ **Timothy's call (the human-gated remainder):** which further instruments to ship + clinical
      sign-off on interpretation copy; **BDI-II** is Pearson-licensed; **DASS-21 / K10** are free and drop
      in as data once approved.

## E. Large efforts (staged; multi-session)

- [~] **T3.1** real sync transport — **transport + acceptance criteria DONE** (`a9a257b6`, local): a
      `SyncTransport` trait (dumb pipe; all trust stays in the fail-closed inbox) with an in-memory relay
      (reference) **and** a real `HttpRelayTransport` (reqwest) + a `tiny_http` **relay server**; host
      orchestration `sync_push_via` (drain Queued outbox → signed ops → mark Sent) / `sync_pull_via`
      (pull → admit) + `sync_with_http_relay` + a Tauri command. **Drains outbox, feeds inbox, with
      hostile-peer + convergence + partition-rejoin + real-HTTP-round-trip tests** (26 sync tests).
      A **Studio "Sync & Backup" panel** (`b57bb89f`) now exposes "Sync now" against a relay URL.
      **libp2p backend — SCOPE CORRECTED + first slice landed (2026-07-05, local):** the existing p2p
      (`core-db/p2p/{protocol,swarm}.rs` + the daemon swarm loop) is real but is an **authorisation
      handshake** (`QualiaRequest::Sync` = hop-count/gatekeeper/target-shapes → `SyncAck{blocks_sent:42}`
      *mock*), NOT op transfer; and the daemon matches `QualiaRequest` **exhaustively** (extending it touches
      that shared service). **Built the missing op-transfer wire:** NEW `p2p/sync_ops.rs` (native) —
      `SyncOpRequest{Publish{op_frames}|PullSince{cursor}}` / `SyncOpResponse{Published|Pulled}` carrying
      **opaque signed-op frames** (dumb pipe; trust stays in the fail-closed inbox), a `SyncOpRelay`
      reference responder store (dedup + cursor), and a `SyncOpCodec` (length-prefixed CBOR, mirrors
      `QualiaSyncCodec`). Then the **swarm-driving node** — NEW `p2p/sync_node.rs` (native): a standalone
      libp2p request-response node (own swarm, event-loop actor over command/oneshot channels) serving a
      `SyncOpRelay` + async `publish`/`pull`. **Two libp2p nodes exchange ops over real noise-encrypted
      localhost TCP — `two_nodes_exchange_ops_over_libp2p` GREEN** (B publishes 2 → A stores 2 → B pulls
      them back, cursor 2). sync_ops 3/3 + node 1/1. No daemon change; noise-encrypted (vs the plaintext
      HTTP relay). **Remaining (thin):** the blocking client-core `SyncTransport` adapter — a self-driving
      node (dedicated thread) + `SyncOperation`↔CBOR-frame serialization; the async transport itself is
      proven. Not blocking the acceptance criteria.
- [~] **T3.2 → elevated to a first-class workstream: [companion-pwa-installable-qapps](companion-pwa-installable-qapps.md)**
      "author a qapp → installable wasm app on your phone" (Timothy: key feature to build upon).
      **P0 foundation DONE**: `qualia-cooperative-core::qapp_package` — `QappManifest` (extensible kinds +
      least-privilege capabilities + content-addressed `WasmRef`) + `generate_pwa` (webmanifest + service
      worker + loader), 21 tests. **P1 secure-origin DECIDED (Timothy, 2026-07-03): WebRTC data channel to a
      local origin, both devices on the same network** (LAN-only, no cloud relay; same-network + an out-of-band
      pairing secret = the trust model; the loopback secure-context *bootstrap* on mobile is the remaining P1
      design work). Remaining: P1 WebRTC delivery (supersedes the LAN-WS companion gateway — **Grok's lane,
      coordinate**), P2 pairing/install, P3 wasm build pipeline, P4 token-v2 isolation (WP1), P5 Package&Publish
      (WP2 done), P6 Cooperative Qapp (WP4).
- [~] **T3.3** Phase-6 release hardening — **backup/restore + diagnostics DONE** (`e9f13c29`, `b57bb89f`,
      diagnostics commit; local): portable, path-traversal-safe `lz4(cbor)` archive of the `wellfair/` data
      subtree (vault stays encrypted) + a **node diagnostics** report (records, sync-queue depths, data
      footprint, Sanctuary state, build version), both surfaced in a Studio **Sync & Backup** panel;
      `wellfair_{export,import}_backup` + `wellfair_diagnostics` Tauri commands; 7 tests. **Remaining (needs
      infra/keys — Timothy):** reproducible builds, signed installers/updates, SBOM, accessibility audit,
      42 MB Sentinel check.
- [~] **T3.4** Phase-7 — **anatomy = §G below, substantially built + the current live priority**; still open:
      studies/rules, authenticated Solid Pod sync, model-assisted extraction, wallet, distributed analytics,
      native mobile. Plus the cross-cutting **selfhood/dignity governance threads = §H**.

## F. Cooperative Qapp plan work packages (parallel initiative)

- [~] **WP1** Qapp token v2 + per-app isolation + CSP — **CORRECTION (honesty):** what landed (`e9fdede7`) is
      only the **generated companion-PWA CSP** (capability-derived: default-deny; own-origin scripts +
      `wasm-unsafe-eval`, no `unsafe-inline`; `connect-src 'none'` unless the qapp requests `Sync`; loader
      externalised), i.e. a slice of companion-PWA **P4**. The plan's real **WP1 / §7 is much larger and NOT
      done:** the **installed-Qapp token v2** (version, qapp_did, package hash, session_id, issued/expires,
      audience, allowed shape+capability hashes, max sensitivity, nonce), verify-on-every-intent, one-time
      bootstrap (scrub token from URL, memory-only), MCP/WS session binding, **per-app origin isolation +
      CSP/security headers on the loopback asset server**, and offline/CSP package lint. This is the P0
      **release gate** for restricted-data Qapps (plan §7). 27 qapp_package tests cover only the generated-PWA
      CSP piece.
- [x] **WP2** Studio Package & Publish — **DONE**: host `write_pwa_bundle` (reuses P0 `generate_pwa`,
      path-traversal-safe, 4 tests) + `publish_qapp_pwa` + native folder-picker + a **Qapps** Studio area
      (manifest fields → least-privilege capabilities → generate installable PWA scaffold to a folder). No
      hand-edited JSON. (Serving over a secure origin so a phone installs = P1, needs Timothy's fork.)
- [ ] **WP4** standalone Cooperative Qapp shell
- [ ] **WP9** QualiaDB Development Cooperative (bind repo read-only; backlog/claims/changes/reviews)
- [ ] **WP5/6/8/10/11** finance-receipts / agreements / advanced-economics / forge-CI / release-hardening

## G. Phase-7 anatomy — the WellFair 3D body (T3.4 anatomy; the current live priority)

Detail: [three-d-anatomy-qapp-progress-log](three-d-anatomy-qapp-progress-log.md) (S1–S7b) +
[reproductive-continuum](reproductive-continuum-and-maternal-fetal-dyad.md). Substance-first; render surface
last. All items below are **local, tested, not pushed.**

- [x] **Anatomy engine** (wellfare-core) — 17-system taxonomy, factor/burden accumulation, temporal kinetics,
      systemic-implication **proposals** (`EpistemicStatus::Hypothesis`), interactions, evidence-tiered
      provenance-tagged knowledge base. Two lenses (person / clinician).
- [x] **σ modality-first percepts** — burden → **σ (EMF)** → BOTH colour (`render::spectral`) and pitch
      (`render::acoustic`); `SystemPercept` / `paint_organs` (discrete) / `overlay_percepts` (ECS/ENS/glymphatic).
- [x] **Both HRA adult bodies compile from live SPARQL** — `ccf_resolver` (lod.humanatlas.io → cdn GLBs,
      CC-BY) + `compile_body`: **Male 26/26, Female 33/33** organs → sealed `.10d`; all 17 systems accounted
      (14 discrete-organ + 3 distributed-overlay).
- [x] **Karyotype (XY/XX) → model (M/F)** selection; organ→system map; `geo:bodySystem` / `geo:anatomyModel`
      manifest facts; `compile_organ_asset`.
- [x] **`.10d` producer + validation** — `compile_10d` (sealed QuantizedMesh + whole-file CRC),
      `geometry_asset_shacl` (bbox/index-in-bounds/`compiledDigest`==CRC/sensitivity), `load_10d_colored`
      (wasm GPU paint, compile-checked).
- [x] **Fetal/embryonic assets** — NIH 3D **Carnegie series (CC-BY, stages 12–23)** fetched + compiled to
      `.10d`, **6/6**; `fetal_stages.rs`. **First `.10d` `t`-axis use.**
- [x] **Developmental `t`-coordinate + maternal–fetal DYAD** — `compile_developmental_asset`
      (`geo:gestationalAgeDays` / `carnegieStage`); `anatomy_dyad.rs::place_within` — embryo seated in the
      real HRA **female uterus** (illustrative scale, on real data).
- [ ] **Reproductive-continuum P1** — `PhysiologicalState`/`ReproductiveState` machine + whole-body
      `StateModulator` (cycle → gestation → the fourth trimester → menopause as **whole-body states**).
      *In-lane, mesh-independent, NO external gate — the next build ("dignity of people born female").*
- [ ] **Render surface** — Studio WebGPU body view (orbit camera + `load_10d_colored` per organ). *Grok's lane.*
- [ ] **`.10d` node-`t`-field write** (natively 4-D container, not just annotated). *Coordinate with CG/.10d lane.*
- ⚑ **Timothy:** later fetal period (9 wk→birth, no clean CC-BY 3D); real-world-scale registration for the dyad;
      curriculum/clinical sign-off; runtime-query-vs-federate for the asset graph.

## H. Selfhood & dignity governance (new threads, 2026-07-05; plans of record)

Cross-cutting rights/dignity layer under the whole platform. Mostly design-of-record + a few **encoded**
pieces; the buildable spine everywhere = **the mandate + the (care/custodian/trustee) grant role** (extends
`consent_store`/`guardianship`/authority-attestation). Grounded in the human-rights instruments (UDHR/ICCPR/
CRPD); **not legal advice.**

- [x] **Terminology harm-fix** — removed "self-sovereign"/"sovereignty" (identity sense) repo-wide → human-
      rights language; renamed the `sovereign` store → **`selfhood`** (daemon one-time migration + Sanctuary
      fail-closed lock; `qualia-core-db --lib` green). **Preserved** legitimate uses (UDHR text, UN Res 1803,
      indigenous land-sovereignty). Standing rule: use human-rights-instrument terms, never "self-sovereign".
- [x] **Selfhood-guardian / personhood model encoded** — the self is NOT in the machine; the store holds
      records that *pertain to* self; **personhood** (outward law/agency) vs **selfhood** (inward mind/secrets).
- [ ] **Reproductive continuum & maternal-fetal dyad** — plan; P1 = the state machine (see §G).
- [ ] **Interactive biology learning environment** — plan; education as emergent *lenses* over the model; OSCE.
- [ ] **Epistemic reasoning & investigative pathways** — plan; abduction + probabilistic + argumentation +
      deontic (mostly composition) + the one new primitive = **clinical value-of-information**.
- [ ] **Human-centric care relationships** — plan; consented scoped care relationships + counter-record
      legibility (NOT a public rating weapon).
- [ ] **Selfhood-container provable ownership** — design note; encrypted payload + **pseudo-anonymous,
      court-resolvable** ownership marker → violation is a provable, attributable crime.
- [ ] **Selfhood/personhood content taxonomy** — design note; `forum internum` (near-absolute, non-derogable)
      vs `forum externum` (proportionately regulable); *"they're also human."*
- [ ] **Post-death continuity & self-definition** — plan (on Timothy's 2016 Vint-Cerf **digital-vellum** work);
      mandate, trusteeship, erasure-prevention (murder → right to truth), reversibility, the self-defined
      representation (grounded-or-refused, ≠ the self).
- [ ] **Social-fabric distributed memory custody** — design note; blind encrypted copies held by *chosen
      people* until the last lets go.
- [ ] **Selfhood cryptography fabric** — direction (DEFERRED — define after the model): multi-sig/threshold +
      **Shamir social recovery** + **dead-man switch** (paired with reversibility); real primitives only.
- ⚑ **Timothy** (throughout): the mandate / death-verification / pseudonym-resolution mechanisms, key-
      management-after-death, the legibility line, reserved vocabulary, clinical/legal sign-off.

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
