# Master Execution Checklist — WellFair + Cooperative + Agency (+ project-wide index)

**The single source of truth for what's left and what's done.** Check items off as they land
(green build + tests). Detail lives in the linked plans/ADRs; this file is the tracker.
**[§0 below](#0-project-wide-workstream-index-all-lanes--the-log-against-list) is the ALL-LANES index** —
every workstream across the project (not just WellFair), with status + pointer + which lane is active — so
this one file is the document to log updates against. §A–§H are the detailed WellFair/Cooperative/Agency
tracker.

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

## 0. Project-wide workstream index (ALL lanes — the log-against list)

**Purpose:** one place to see *everything left across the whole project*, not just WellFair. §A–§H below are
the detailed WellFair/Cooperative/Agency tracker; this index also captures the other lanes and links their
plans. Tags: `[x]` done (local unless noted) · `[~]` partial · `[ ]` not started · **🔒** needs Timothy
(sign-off / clinical / legal / infra keys) · **🚧** blocked by an active external lane. Effort S/M/L.

**Parallel work — operational heads-up, NOT an ownership map:**
- The **computational-geometry** modules under `qualia-core-db/.../computational_geometry/` are being actively
  built out (P12/P13/P14). Practical effect: they **intermittently break the shared `core-db` compile** — if a
  build fails there, it's usually that, not you; verify in a green window and don't edit CG files while it's churning.
- **Inference-engine optimisation** (`inference/`, `gguf_*`, decode shaders) is a separate track that may be active.
- **`coordination/NOTICES.md` is the live source for who is actually working now — and it goes stale.** Verify a
  CLAIM is *current* before treating anything as off-limits; **a stale tag is not a constraint** (per
  [[feedback-no-lane-excuses-verify-first]]). §10 lane-avoidance applies ONLY to genuine, *currently-active*
  shared-file collisions — it is not a default blocker and must never be used to defer work.

**A · WellFair, accountability & selfhood-config (§C–§H detail below)**
- [x] §H accountability fabric COMPLETE (consent credentials, ledger, envelope encryption, dead-man/incapacity
  switches, disclosure-trace, duty-of-inquiry, key-release-on-enact, Shamir social-recovery, peer-resolved keys).
- [x] Score-card **weight model person-authored** (principle: means, not definitions).
- [~] **Extend "person authors it"** — score-card **aspects + band thresholds** editable (S); fabric-wide
  "every default person-owned + opt-out" sweep (M). → [[principle-software-provides-means-not-definitions]]
- [~] **Epistemic reasoning / investigative pathways** — VOI slice done; compose core-db abductive/argumentation/
  probabilistic + step library (M, 🔒 clinician). `epistemic-reasoning-and-investigative-pathways.md`

**A2 · Hypermedia semantic library → personal SEMANTIC ASSET SYSTEM (NEW — Timothy, 2026-07-07; "increasingly important") — FOUNDATIONAL**
- Find your files by **meaning / time / place / project / content — never by folder.** Use cases (Timothy):
  a meme by its topic or *what the image depicts*; files *about* a topic (biology/engineering/policy/software/law);
  a day or period as a **timeline** or on a **map** (travel/locations/photos); collect+process a **project**;
  the documents that **support a tax return / expenses claim**. **Ingest derives searchability** (a doc goes in
  → processors make the searchable derived files). Plan: `docs/plans/hypermedia-semantic-library.md`.
  → [[hypermedia-semantic-library]]
- [x] **P0 — container + edge-graph + query** (`qualia-core-db/src/hypermedia.rs`, 3 tests): `HypermediaContainer`
  = primary asset ⊕ related-by-role ⊕ analytics, bound as **NQuin edges** (`bundledWith`/`hasRole`/
  `prov:wasDerivedFrom`/`hasProvenance`/`analysisTarget`); `container_to_nquins` + query-by-lineage helpers;
  subjects share `render/assets.rs`'s identity space. **VERIFIED GREEN.**
- [x] **P0.5 — descriptors + search-by-facet + `Flag`** (+2 tests → **5/5**): `Descriptors`
  (topics/depicts/occurredAt+interval/place/projects/documentType/purposes) + `descriptors_to_nquins` +
  `by_topic`/`by_depiction`/`by_place`/`in_project`/`for_purpose`/`in_time_range` (timeline) + `Flag`
  (severity-graded, `flags_on`/`flag_severity`) for the guardian-notify path. **VERIFIED GREEN.**
  §11: **DONE** — `hypermedia.rs` split into `hypermedia/mod.rs` + `hypermedia/processors/{mod,image,audio}.rs`
  (the new content-processors are their own submodule; the public path `crate::hypermedia::*` is preserved).
- [x] **P2 — anatomy on it (VERIFIED GREEN):** `wellfair/anatomy_body.rs` `organ_container` (an organ = sealed
  `.10d` mesh primary ⊕ source-GLB with CCF/HRA provenance+licence ⊕ topic/depicts/system descriptors, joined
  to its geometry manifest — **findable by system**) + `body_container` (bundles organs as one semantic unit).
  +2 tests → anatomy_body 4/4. *("advance anatomy so it works properly", on the container.)*
- [x] **P3 — ingest derives searchability + guardian-notify (VERIFIED GREEN):** `hypermedia`: `Processor`
  trait + `TextProcessor` (model-free: text/markdown → topics {biology/engineering/policy/software/law/finance}
  + a searchable text derivation + watch-word flags) + `ingest_with` (folds output into a container so the
  original is findable) — hypermedia 7/7. `wellfair/ingest_guardian.rs` `guardian_notifications` (flags ≥ a
  severity → `GuardianNotification`s; 2 tests) + host `record_guardian_notifications` (records each in the
  **tamper-evident ledger** — a flagged ingest is an auditable event). Heavy image/audio processors compose
  the parked `qualia-vision`/`qualia-audio` (not rebuilt).
- [x] **P4a — usable ingest → search → browse (VERIFIED GREEN):** `wellfair/hypermedia_store.rs`
  (`LibraryEntry` carries the container's NQuin edge-graph; **search runs real graph queries** over the union
  of all entries' quins — `by_topic`/`depicts`/`place`/`project`/`purpose`/timeline; test: 3 docs findable by
  biology/law/finance) + host `ingest_document` (TextProcessor → topics + searchable text → container →
  persist; **guardianship flag → notify + ledger record**) / `search_library` / `search_library_time` /
  `list_library` + 4 Tauri commands + 3 bridges + a Studio **Library** tab (ingest a doc; find by facet).
  hypermedia_store 1/1; desktop + studio host + studio wasm green.
- [x] **P1 — provenance sidecar PHYSICALLY in `.10d` (VERIFIED GREEN).** NEW `container_10d/provenance_section.rs`
  (source bytes + licence + optional VC in-envelope; encode/decode + **validate-before-use gate** — carried
  source must hash to the declared digest AND a licence must be present, else it's a stripped context). Flipped
  the reserved `SpecReservedProvenanceSidecar` → honest **`ProvenanceSidecar`** (now `is_implemented`, encodable);
  `render/portal/mod.rs`'s existing `has_attestation` governance gate now lights up. Producer
  `render/compile_10d.rs::compile_mesh_to_10d_with_provenance` bundles it into a compiled organ `.10d`.
  **provenance_section 7/7; compile_10d 11/11** (incl. an end-to-end round-trip through the real section table +
  a bundled-`.10d` that still decodes the mesh AND validates the sidecar). *(P2 records provenance semantically;
  P1 makes the source bytes byte-inseparable.)*
- [x] **P4b (processors + views) — heavy content processors + timeline/map (core VERIFIED GREEN; UI wired).**
  `hypermedia/processors/`: **`ImageProcessor`** — REAL model-free EXIF/PNG → a photo's capture-time (**timeline**)
  + GPS (**map**); **`WavProcessor`** — composes the project's own STFT → duration + dominant-frequency;
  `processor_for` media-type dispatch. **hypermedia 14/14** (EXIF Paris fixture + 1000 Hz tone verified). Studio
  Library panel gained **List / Timeline / Map** views + person-authored date/place facets; host `ingest_bytes`/
  `ingest_file_hex` (photo) + `ingest_document_annotated` + `wellfair_ingest_file_hex` command + bridges.
  **⚑ honest gaps (NOT faked):** *depicted-subject recognition / OCR* and *speech transcript* need a **vision /
  ASR model** — they are `Processor` plug-in points (the dispatcher already routes by media type), the one true
  out-of-band dependency. Also remaining: a native **file-picker** in the Studio to feed `ingest_file_hex` bytes
  (the host path is ready + tested); fold the library quins into the core graph store / daemon `/query`. L.

**B · Anatomy / health (§G — the flagged priority; "advance so it works properly", Timothy 2026-07-07)**
- [ ] **Make the anatomy asset pipeline real, on the hypermedia container (A2).** Canonical GLB→Q42 honouring
  glTF accessor layout (normals/UV/indexed prims/node transforms/materials) — not the BIN-offset assumption;
  retire the desktop `glb_ingest` parser (Tensor10D-axis overloading) → thin client of the core compiler; an
  organ = a **hypermedia container** (mesh ⊕ systemic-burden analytics ⊕ source-GLB/LOD/provenance), valid
  parity, compiled-geometry sidecar. M–L. `future-work-register.md` ★ + `three-d-anatomy-qapp.md`
- [ ] **Render surface** — Studio WebGPU body view (orbit + `load_10d_colored`/organ). M–L. (stale "Grok" tag.)
- [ ] **Anatomy P6** — cycle/pregnancy factors → score-card via `bridge.rs`. S–M.
- [ ] **`.10d` node-`t`-field write** (native 4-D). Coordinate if the `.10d`/CG code is churning; else mine. M.
- 🔒 Reproductive-continuum tail (later fetal period; real-scale registration) — curation.

**C · Selfhood & dignity plans (design-of-record)**
- [ ] Human-centric care relationships (M) · [ ] Human-centric identity & biometric rights (M) ·
  [ ] Selfhood-container provable ownership (M) · [ ] Selfhood/personhood content taxonomy (S–M) ·
  [ ] Post-death continuity & self-definition — *representation/vellum* piece; dead-man+Shamir done (M) ·
  [ ] Social-fabric distributed memory custody — *replication/custody* layer; commons+Shamir done (M) ·
  [ ] Interactive biology learning environment / OSCE (L). *(each has a plan in `docs/plans/`)*

**D · Cooperative Qapps (§F)**
- [~] **WP1** installed-Qapp **token v2 + loopback CSP/origin isolation** — P0 restricted-data release gate (L, security).
- [ ] **WP4** Cooperative Qapp shell · [ ] **WP9** QualiaDB Development Cooperative · [ ] **WP5/6/8/10/11**
  finance-receipts / agreements / advanced-economics / forge-CI / release-hardening. `cooperative-qapps-desktop-implementation-plan.md`

**E · Companion PWA / mobile (§E T3.2)**
- [~] P0 done → **P1 WebRTC LAN** (decided) → P2 pairing/install → P3 wasm build pipeline → P4 token-v2. L. `companion-pwa-installable-qapps.md`

**F · Sync · networking · discovery · social**
- [~] **T3.1 sync** — thin remainder: blocking client-core `SyncTransport` adapter over the libp2p node. S–M.
- [~] Social network / discovery (`connection_identifier` P0; rendezvous / mutual-auth / semantic-DNSSEC / WG mesh). L. `social-network-plan.md`
- [~] Personal platform provider & networking — config model + host API; tiered rendezvous (email/**Cloudflare Tunnel**/own-edge/Nym/mDNS). M–L. `personal-platform-provider-and-networking.md`
- [~] Rights-aware peer-agreement addressbook — unify Party store → M:N sigs → wire downstream. M. `rights-aware-peer-agreement-addressbook.md`
- [~] Socially-defined comms & telehealth — expose engine → invites↔peering → boringtun → WebRTC → governed share. L. `socially-defined-communication-and-telehealth.md`
- [~] Solid Chat interop — lossless mapping done; **LDP transport (PUT/PATCH) + live wiring** parked. M. `solid-chat-interop.md`

**G · Vault · release · assessment (finish-outs, several 🔒)**
- [x] Sanctuary vault v2 (CBOR+decoy). 🔒 **T2.1** threat-model ADR sign-off (6 decisions + `open_lane` timing
  side-channel). 🔒 **T2.2** more instruments (DASS-21/K10 free; BDI-II licensed). 🔒 **T3.3** release hardening
  tail (reproducible builds, signed installers, SBOM, a11y audit, 42MB Sentinel).

**H · Wallet / economy** — [ ] Multi-chain **BIP-39 HD multisig** wallet (BTC/Lightning/eCash-XEC/Nym; **SLP+ALP**
tokens; **semantic tokens** RDF/JSON-LD on XEC — Timothy's prior art); scaffolding exists; **real crypto only**. L.
This *is* the concrete selfhood-crypto multisig/social-recovery instantiation. `future-work-register.md`

**I · Larger substrates (parked, L each)** — [ ] Chora `qualia-ste` spatio-temporal 10d-browser ·
[ ] Native visual intelligence & generative 3D (`qualia-vision`) · [ ] Native auditory/language/music
(`qualia-audio`, consent/cultural) · [ ] Agent/qApp enablement (webcivics fork) · [ ] Architectural-enhancement
specs (O1 Memory-CRDTs, Intermittent Computing, NVMe storage-pushdown, Formal Verification, Spatial-Web Anchoring).

**J · Engine / infra hygiene** — [~] Inference W3–W10 (a separate active track may be on this — check NOTICES, don't assume) · [ ] Dependency
modernization (wgpu/naga 29, arkworks 0.6, reqwest 0.13 — `DEPENDENCY_MODERNIZATION.md`) · [ ] Honesty quick-win:
make C2PA/SPARQL-MM verification status honest (`sparql_mm.rs`), like the `sparql_did` fail-closed precedent. S.

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
- [x] **Reproductive-continuum P1** — DONE (2026-07-06, `wellfare-core/src/anatomy/physiology.rs`, 9
      tests). `PhysiologicalState`/`ReproductiveState` continuum state machine (menarche → cyclical phases
      → conception → trimesters → the fourth trimester → lactation → perimenopause → menopause) with
      validated transitions; `StateModulator` (whole-body sibling of `EnvironmentModulator`, scales an
      *external* load per-system by state engagement — pregnancy makes a nephrotoxic med a bigger ask on
      the kidneys — **without pathologising the healthy state**); `whole_body_profile` (heightened
      *engagement*, not adverse burden); `as_environment_modulators` (drop a state onto a `Timeline`).
      Coarse, integer-only, illustrative seed; **authoritative milestone content + dignity framing deferred
      to Timothy/clinician (§9).** Baseline-not-deviation. Next: **P2** maternal–fetal dyad (domain buildable now).
- [x] **Score-card (accumulative, traceable interpretation)** — DONE (2026-07-06,
      `wellfare-core/src/anatomy/scorecard.rs`, 7 tests, Timothy's idea). `ScoreCard` across 6 `Aspect`s
      (SystemicLoad / Stress / Resilience / Convergence / InteractionLoad / PhysiologicalDemand); every
      `AspectScore` carries `Contribution` **linkages** to its underlying considerations (system/factor/
      interaction/state + weight + evidence tier) + coarse `ScoreBand`, always `Hypothesis`. `WeightModel` =
      an explicit, editable `(system,aspect)→weight` table (the "human weights model", **not** a black box).
      Scores over state-modulated burdens (integrates P1). **Anti-reductive / anti-rating-weapon by design:**
      multi-aspect, fully traceable, scores load/stress/resilience **NOT illness/disease** (that's diagnosis).
      **Selfhood-classified (Timothy, 2026-07-06):** `forum_class()→Internum` + `sensitivity_class()→Sanctuary`
      intrinsic to the type — forum-internum, most-restrictive, non-default-disclosed.
      **PERSON-AUTHORED WEIGHT MODEL (2026-07-07 — principle: software provides means, not definitions):** the
      `WeightModel` is now **the person's own to set**, not a fixed lens the software imposes. `wellfair/
      scorecard_prefs.rs` (persist/clear the person's model; test) + host `get`/`set`/`reset_weight_model` +
      `seed_weight_model` (the *suggestion*, shown separately) + `compute_scorecard` reads **their** model +
      3 commands + 3 bridges + an in-panel editor ("How your body is read — your weight model": edit each
      weight / save mine / reset to suggestion). The seed is explicitly a **starting suggestion, never a
      verdict.** Verified green (backend test + desktop + studio host + wasm). **⚑ Corrected:** the old ⚑
      "aspects/weights/bands = curation-grade *Timothy/expert*" was itself the anti-pattern (routing the
      person's self-definition through a third party). Weights are now the **person's**; a curated *seed
      suggestion* + the aspect/band *scaffolding* are the only expert-informed pieces, and both are the
      person's to override. Remaining means-to-build: make aspects + band thresholds equally person-editable
      (same pattern). See [[principle-software-provides-means-not-definitions]].
- [x] **Reproductive-continuum P2 — maternal–fetal dyad DOMAIN model (on the science)** — DONE (2026-07-06,
      `wellfare-core/src/anatomy/dyad.rs`, 10 tests). `MaternalFetalDyad` = two coupled principals, modelled on
      biology: **two genetic `Progenitor`s** (ovum+sperm; `Known|Donor|Unknown` — a father may be unknown/
      unaware, gamete donation representable), with **genetic ≠ gestational ≠ social ≠ guardian** (surrogacy
      modelled — carrier can be neither genetic parent). **Guardian during gestation = the gestational mother.**
      **Social/legal personhood accrues at/after *birth*** (`RightsStage` + deferred `SocialRightsThreshold`) —
      during gestation the entity is *stewarded*, NOT a competing legal person (the mother's autonomy stays
      paramount). `validate()` invariants: child never collapsed into any known adult, gestational mother present,
      dyad only in pregnancy/gestation-stage. forum-internum/Sanctuary; `considerations()` = structural/science/
      rights **proposals** (Hypothesis) — medical correlation content deliberately NOT seeded. ⚑ social-rights
      threshold + guardianship-transfer = deferred values calls (Timothy, §9.4); correlations + clinician sign-off (§9.3).
- [x] **Birth transition + Digital Birth Record domain model** — DONE (2026-07-06,
      `wellfare-core/src/anatomy/birth.rs`, 5 tests) — aligned to the **DigitalBirthRecord draft standard**
      (`docs/manuals/standards/init-draft-standards-wip-main/DigitalBirthRecord`). `MaternalFetalDyad::
      give_birth() → DigitalBirthRecord`: born child = subject who **owns** the inalienable, biometric-extended
      record. **NOT self-sovereign — stewardship via a permissive commons** (Timothy: the infant case falsifies
      "self-sovereign"): `Guardianship` = a `Steward` commons (distributed `StewardRole`s that different
      guardians may hold; default biological-parents subject to official credentials — surrogacy order →
      intended parents not the surrogate); **agency is a `AgencyStage` GRADIENT** (Neonate→…→Adult→
      SupportedAdult; monotone `self_determination()`; SupportedAdult = full agency *with* supports, CRPD
      not-substituted) — a shape, not a sovereign switch. Biometrics by **class** (datum Sanctuary-held,
      referenced not inlined); non-collapse + never-unstewarded invariants; forum-internum/Sanctuary. Domain
      model — RDF `br:` vocab / VC / biometric wire-formats / wallet-`did:q42` = identity-layer + standard
      (coordinate). ⚑ draft prose says "sovereignty" — flagged; stage boundaries + legal mapping + values (§9.4).
- [x] **Score-card surfaced in webizen-desktop** — **DONE end-to-end (2026-07-06).** Data path:
      `wellfair/anatomy_view.rs` `WellbeingScorecardReport` + `build_scorecard_report[_from_journal]`
      (score_card + hypotheses over the health journal, forum-internum/Sanctuary) →
      `WebizenHostApi::compute_scorecard` → `wellfair_compute_scorecard` Tauri command (anatomy_view 9/9).
      **Studio panel DONE:** `scorecard_panel.rs` `WellfairScorecardPanel` — one card per aspect (label +
      plain-wording + coloured **band** + score bar; `Resilience` reads "higher is supportive"), each with its
      **contribution linkages** (the traceability) and the investigable **hypotheses** as pathway-starts; the
      forum-internum disclosure is always shown. `compute_scorecard` host_client bridge + wired into the
      **Anatomy** tab (above the anatomy view). Studio host + wasm green.
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
- [~] **Epistemic reasoning & investigative pathways** — plan; abduction + probabilistic + argumentation +
      deontic (mostly composition) + the one new primitive = **clinical value-of-information**. **FIRST SLICE
      DONE (2026-07-06 — "the point of Qualia"):** `wellfare-core/src/anatomy/pathway.rs` (6 tests) — the
      **value-of-information primitive** + investigative-pathway machinery over the anatomy hypotheses:
      hypothesis → VOI-ranked `InvestigativeStep`s (`Question/Observation/Test/LifestyleLever/SpecialistInput`,
      traceable `bears_on`); enables never directs/diagnoses (every step a `Hypothesis`; specialist → the
      person's *chosen* clinician). Remaining: compose the core-db abductive/argumentation/probabilistic
      engine; the step library (curation/clinician content, §9).
- [ ] **Human-centric care relationships** — plan; consented scoped care relationships + counter-record
      legibility (NOT a public rating weapon).
- [ ] **Social workers — supported to help, fairly accountable** — plan of record (2026-07-06,
      `docs/plans/social-worker-support-and-accountability.md`). Support the worker (consented need = the
      person's score-card + pathway; means to act + escalation-as-a-recordable-act) AND fair accountability:
      the record distinguishes **unable/no-fault (system failed → worker exonerated)** vs **negligence** vs
      **malfeasance (court/political/gov — tamper-evident admissible evidence)**, via the **Six Vectors of
      Transparency applied to a human worker** (Cost vector separates fault/no-fault). Consensual vs statutory
      authority (proportionately-required-by-law, accountable to person + democratic-legal order — not
      self-sovereignty). NOT a rating weapon; locus = a `Hypothesis` proposal over evidence, never an
      automated verdict. Mostly composition. ⚑ statutory model + evidentiary standard + locus criteria (legal).
- [ ] **Selfhood-container provable ownership** — design note; encrypted payload + **pseudo-anonymous,
      court-resolvable** ownership marker → violation is a provable, attributable crime.
- [ ] **Selfhood/personhood content taxonomy** — design note; `forum internum` (near-absolute, non-derogable)
      vs `forum externum` (proportionately regulable); *"they're also human."*
- [ ] **Post-death continuity & self-definition** — plan (on Timothy's 2016 Vint-Cerf **digital-vellum** work);
      mandate, trusteeship, erasure-prevention (murder → right to truth), reversibility, the self-defined
      representation (grounded-or-refused, ≠ the self).
- [ ] **Social-fabric distributed memory custody** — design note; blind encrypted copies held by *chosen
      people* until the last lets go.
- [~] **Selfhood cryptography fabric** — direction (define after the model): multi-sig/threshold + **Shamir
      social recovery** + **dead-man switch** (paired with reversibility); real primitives only. **DOMAIN
      MODELS BUILT (2026-07-06, client-core):** `consent_credential.rs` (12 tests — revocable-access +
      crypto-shred + court/authority + **multi-sig** party-instigation) + `dead_mans_switch.rs` (7 — gamified
      liveness+quorum trigger, `Disposition`, **reversible** un-fire) + `incapacity_switch.rs` (6) +
      `disclosure_trace.rs` (6 — betrayal knowable/attributable incl. staff) + `duty_of_inquiry.rs` (5) + the
      encrypted permissive-commons payload; **ADR 0011** records the whole fabric.
      **REAL-CRYPTO + DESKTOP WIRING LANDED (2026-07-06, all local, verified green):**
      (1) **tamper-evident store ✓** — `accountability_ledger.rs` (5) + `accountability_store.rs` (5): real
      `sha2`+`ed25519`, append-only hash-chained signed ledger; `verify()` names any content-edit/deletion/
      forgery. (2) **real envelope encryption / key hierarchy ✓** — `envelope_encryption.rs` (7): random DEK +
      XChaCha20-Poly1305 payload AEAD, DEK **sealed to a recipient X25519 key** (real sealed box, via
      `core-db::crypto::sanctuary_audit` + a new `AuditKeypair::from_secret`) = the credential's real
      `wrapped_key`; revoke destroys it ⇒ crypto-shred; owner keypair **derived** from the signing-key seed
      (nothing secret at rest). (3) **wired to the desktop ✓** — 11 host-API methods + 11 `wellfair_*` Tauri
      commands + a Studio **Accountability** panel/tab (grant/seal-&-grant/record-conduct/revoke/audit/open/
      ledger-integrity). Verified: store+envelope+ledger tests green; desktop build; studio host + studio wasm.
      (4) **whole accountability fabric WIRED to the desktop (2026-07-06/07) — all four dormant models now
      reachable:** `dead_mans_switch` + `incapacity_switch` (arm/alive/attest/enact + activate/regain,
      reversible) **and** `disclosure_trace` (record-cc / record-disclosure / chain / actors-with-access /
      trace-leak → attributes a staff leak) + `duty_of_inquiry` (assess → Diligent/NoFault/UncheckedNoHarm/
      Negligent), every stateful action owner-**signed into the tamper-evident ledger** (+4 store tests → 8/8).
      Host: 27 accountability methods total. Desktop: 27 `wellfair_*` commands. Studio: a **Safeguards** tab
      (switches) + **Accountability** tab now carries the consent-credential panel + a **Disclosure & inquiry**
      panel. **Verified green: store 8/8; desktop + studio host + studio wasm all exit 0.**
      (5) **key-release-on-enact ✓ (VERIFIED GREEN 2026-07-07):** `enact_dead_mans_release` re-seals the
      payload DEK to each `ReleaseTo` disposition party's X25519 key + grants them a credential — a fired
      switch **actually hands over access** (store 9/9; a trustee with no prior access decrypts after enact).
      Host + command + bridge + Safeguards "Enact & release" control; **desktop + studio host + studio wasm all
      exit 0** (confirmed in a green-core-db window after the CG-lane build-out settled — not mine, §10).
      (6) **Shamir social-recovery ✓ (VERIFIED GREEN 2026-07-07):** NEW `shamir_recovery.rs` — real Shamir
      Secret Sharing over GF(2⁸) (AES field; Horner eval + Lagrange interpolation at 0; 6 tests incl. field
      axioms, any-k-of-n, k-1-reveals-nothing). Store `reconstruct_and_release` recovers a payload DEK from a
      quorum of friends' shares and releases **without the owner key** (store 10/10; a trustee decrypts from
      2-of-3 friend shares alone). Host `split_dek_recovery` / `reconstruct_and_release` + 2 commands + 2
      bridges + a Safeguards "Social recovery" panel section. Desktop + studio host + studio wasm all exit 0.
      (7) **remote-agent X25519 key distribution ✓ (VERIFIED GREEN 2026-07-07):** `SocialPeer` gained
      `envelope_pubkey_hex` + `set_peer_envelope_key` + a pure `resolve_envelope_keys` (peer test 4/4); host
      `set_peer_envelope_key` + `enact_dead_mans_release_via_peers` (resolves the disposition parties' keys
      from the peer store, reports any still-missing) + 2 commands + 2 bridges + a Safeguards "Enact & release
      (via peers)" button. Desktop + studio host + studio wasm exit 0. So a worker/trustee's key is now
      **resolved from their peer record**, not pasted by hand.
      **§H accountability fabric is COMPLETE + desktop-wired end-to-end:** consent credentials (revocable /
      court-authority / multi-sig), tamper-evident ledger, real envelope encryption, score-card, all 4
      switch/trace models, key-release-on-enact, Shamir social-recovery, remote-key distribution. Full surface
      ~35 host methods + ~35 `wellfair_*` commands + 6 Studio panels.
      **Only deferred (explicit values/connection calls, NOT missing crypto):** `MakePublic` key-release
      (irreversible — a values call, §H.⚑); auto-populating the peer envelope key through the connection
      handshake (a connection-layer nicety — the field + setter + resolver exist). ⚑ reversibility limits,
      trigger criteria, key-mgmt-after-death.
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
