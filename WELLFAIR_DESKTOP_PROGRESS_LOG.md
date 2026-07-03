# WellFair on Webizen Desktop — running progress log

Single running log for the [wellfair-webizen-desktop](docs/plans/wellfair-webizen-desktop/README.md)
workstream (PROJECT RULE §9). Newest entry at the bottom. Honest engineering record:
regressions and gaps included. No personal circumstances.

Prior phase records (by the Grok instrument): `WELLFAIR_PHASE2_CLOSEOUT.md`,
`WELLFAIR_PHASE3_CLOSEOUT.md`, `WELLFAIR_PHASE4_SPRINT.md`, and the `coordination/NOTICES.md` feed.
This log starts when the workstream was reallocated to the Claude (Opus 4.8) instrument.

---

## 2026-07-02 — Foundation hardening (Claude / Opus 4.8) — DONE

**Phase / status:** Cross-cutting foundation fix on top of Grok's Phase 4 baseline. **Done, green.**

**Context:** Reallocated by Timothy to continue the implementation. Reviewed the tree first; it
did **not compile** and Sanctuary had two boundary leaks. Fixed both before adding features —
you cannot build features on a broken, leaky foundation.

**What was built / fixed:**
1. **Build break unblocked.** Grok's in-flight edit changed `decide_live_share_request` to take a
   4th `deny_reason` arg but left the desktop call site at 3 args (and discarded its own `reason`).
   Fixed [commands/mod.rs](crates/webizen-desktop/src/commands/mod.rs) to thread `reason.as_deref()`
   through — the deny reason now actually reaches the store instead of being dropped.
2. **Sanctuary excluded from the ordinary sync outbox** (master plan §5.2). `VaultService::commit_envelope`
   ([vault.rs](crates/qualia-client-core/src/wellfair/vault.rs)) previously enqueued **every** committed
   record — including `Classified` sanctuary notes — into the ordinary `SyncOutbox`. Now `Classified`
   records are journaled durably but never enter the ordinary routing lane.
3. **Sanctuary applied to the graph-coverage query path** ([api.rs](crates/qualia-client-core/src/wellfair/api.rs)).
   `query_graph_coverage` read the journal directly, bypassing the projection, so a locked Sanctuary's
   protected rows were visible through the Tools view. It now applies the same projection as
   `list_health_records`.

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **52 passed, 0 failed**
(+2 new boundary tests: `classified_records_excluded_from_sync_outbox`,
`locked_sanctuary_hides_protected_kinds_from_graph_coverage`). `cargo check -p webizen-desktop`
and `-p webizen-studio` both **green**.

**⚑ Where I need the human:** Sanctuary is still a **projection + PIN-hash filter**, not the
"independent key material and encrypted storage" the plan (§6) requires. The two read/write leaks are
now closed, but a full boundary needs (a) an ADR for Sanctuary key separation + at-rest encryption and
(b) a decision on decoy semantics (currently decoy == locked view of the main domain; the plan wants a
separate decoy domain that never aliases Sanctuary data). This is a threat-model decision, not just code.

**Next step:** first real feature slice — Personal Finance ledger.

---

## 2026-07-02 — Phase 5 slice: Personal Finance ledger core (Claude / Opus 4.8) — DONE (backend)

**Phase / status:** Phase 5 (FIN-01.., §17 money-safety). **Backend done, green. UI + Tauri command
wiring: not yet — listed as next.**

**What was built:** new [`wellfare-core/src/finance.rs`](crates/wellfare-core/src/finance.rs) domain module:
- `LedgerEntry` (signed minor-unit amounts, ISO-4217 currency, optional category/counterparty/project link);
- `merge_ledger` — **add-wins by stable entry id, never sum-merge**, deterministic ordering;
- `derived_balance` — a **pure derivation over the unique-id set**, so duplicate/reordered/replayed
  frames cannot move money (directly answers the §17 risk "duplicate sync creates money/obligations");
- envelope builder (Restricted / SelfReported), summary + round-trip parser.
- Host API on `WebizenHostApi`: `add_ledger_entry`, `list_ledger_entries`, `ledger_balance`;
  `wellfair-finance` added to the policy writer set; `ledger_entry` added to the journal-kind map.

**Measured results:** `cargo test -p wellfare-core finance::` → **5 passed** (merge idempotency,
order-independence, replay-safety, per-currency balance, summary round-trip).
`cargo test -p qualia-client-core wellfair::` → **53 passed** (+1 host round-trip test
`ledger_entries_commit_and_balance_by_currency`). Desktop check green.

**⚑ Where I need the human:** none for this slice. For the *full* Phase 5 (projects, contributions,
derived obligations, cross-node sync convergence) the plan requires the signed operation/merge contract
(ADR 8–9) to be frozen first — that is a contract decision to confirm before fanning out.

**Next step:** wire the finance Tauri command + Studio `finance_panel`, then either (a) continue Phase 5
Cooperative Projects, or (b) run a read-only audit swarm to produce the prioritized executable backlog
for the remaining phases. Awaiting Timothy's steer on ordering.

---

## 2026-07-02 — Phase 5 Projects + Phase 3 Credentials (Claude / Opus 4.8, swarm-authored) — DONE (backend)

**Phase / status:** Phase 5 Cooperative Projects (COP-01..19) + Phase 3 Credentials (CRE-01..09).
**Backend + host API done, green. Tauri commands + Studio panels: not yet — listed as next.**

**How:** ran a 2-agent workflow (author + adversarial review per module) to produce two **isolated
new domain modules** in parallel (distinct files, no shared-file writes, no worktrees). I then did the
shared-file integration (journal kinds, policy writers, host API, host tests) and the green-build
verification myself — no unverified landings (§14).

**What was built:**
- [`wellfare-core/src/projects.rs`](crates/wellfare-core/src/projects.rs): `Project`, `ProjectMembership`
  (Steward/Contributor/Observer), `Contribution` (author chain via predecessor), and **derived**
  `Obligation`. `merge_contributions` (add-wins by id) + `derive_obligations` (pure over the unique-id
  set) — the same money-safety discipline as finance, so replayed contributions can't double-count effort.
- [`wellfare-core/src/credentials.rs`](crates/wellfare-core/src/credentials.rs): `CredentialRecord`,
  `VerificationState` (Unverified→SchemaValid→IssuerTrusted, plus terminal Revoked/Expired), `evaluate_state`
  (a **status cache**, explicitly not signature/proof verification), and `build_presentation` returning a
  `FieldSelectedPresentation` that is **plain field selection, honestly NOT cryptographic selective
  disclosure** — documented in the type name, module header, and the stored summary tag
  `"disclosure_kind":"field_selection_not_zk"` (plan §Q11 demanded this distinction).
- Host API on `WebizenHostApi`: `add_project` / `add_project_membership` / `add_contribution` /
  `list_contributions` / `project_obligations`; `add_credential` / `list_credentials`. Journal kinds
  `project` / `project_membership` / `contribution` / `credential` and policy writers
  `wellfair-projects` / `wellfair-credentials` added.

**Measured results:** `cargo test -p wellfare-core` → **59 passed** (finance 5, projects 8, credentials 12,
+ existing). `cargo test -p qualia-client-core wellfair::` → **55 passed** (+2 host round-trip tests:
`contributions_commit_and_obligations_derive_through_journal`, `credential_commits_with_credential_kind`).
`cargo check -p webizen-desktop` → green.

**Honest note on the swarm:** the review agent claimed both files "compile clean" by inspection, but
`credentials.rs` had a `u32` literal overflow (`9_999_999_999` in a test) that only the compiler caught.
Fixed on integration. Lesson reinforced: inspection ≠ compilation; the integrator must build.

**⚑ Where I need the human:**
- **Credential claims are not durably retrievable yet.** The vault journal stores an envelope + a
  *summary* (claim_count, not the claims). Full credential claims — needed to build a real presentation
  later — belong in a content-addressed blob (plan §5). That blob path is not built; `build_presentation`
  currently operates on an in-memory `CredentialRecord` the UI holds during a session. Decision needed:
  wire the blob store now, or defer presentation persistence to Phase 7.
- **Real credential trust/status** (issuer signature, revocation registry) is Phase 7 and needs the
  identity/key-vault layer + a trust-policy ADR — `evaluate_state` is deliberately only a local cache.

**Next step:** Tauri commands + Studio panels for finance/projects/credentials to make these usable in
the desktop UI, OR continue Phase 5 sync-operation protocol. Awaiting Timothy's steer.

---

## 2026-07-02 — UI wiring: finance / projects / credentials desktop panels (Claude / Opus 4.8) — DONE (host green; wasm-app blocked by pre-existing unrelated break)

**Phase / status:** Desktop UI surface for the three new domains. **Tauri commands + host_client
bridges + Studio panels + shell nav done; host builds green; Studio wasm *lib* green. The Studio wasm
*app* build is blocked by a pre-existing break unrelated to WellFair — see below.**

**What was built:**
- Six Tauri commands in [commands/mod.rs](crates/webizen-desktop/src/commands/mod.rs):
  `wellfair_add_ledger_entry`, `wellfair_ledger_balance`, `wellfair_add_project`,
  `wellfair_add_contribution`, `wellfair_project_obligations`, `wellfair_add_credential`
  (all registered in the invoke handler).
- Host-client bridges + DTOs in [host_client.rs](crates/webizen-studio/src/components/wellfair/host_client.rs)
  (`add_ledger_entry` / `fetch_ledger_balance` / `add_project` / `add_contribution` /
  `fetch_project_obligations` / `add_credential`, with `BalanceReportDto` / `CurrencyBalanceDto` / `ObligationDto`).
- Three Studio panels: [finance_panel.rs](crates/webizen-studio/src/components/wellfair/finance_panel.rs)
  (ledger entry + derived per-currency balance), [projects_panel.rs](crates/webizen-studio/src/components/wellfair/projects_panel.rs)
  (project + contribution + derived effort obligations), [credentials_panel.rs](crates/webizen-studio/src/components/wellfair/credentials_panel.rs)
  (import + list, with the field-selection-not-ZK honesty note in the UI copy).
- Wired into the panel registry and shell nav (new "Finance" and "Credentials" areas; "Projects" area
  now has its panel).

**Measured results:** `cargo check -p webizen-desktop` → green. `cargo check -p webizen-studio` (host)
→ green. `cargo check -p webizen-studio --target wasm32-unknown-unknown` → the **library** compiles
(all WellFair panels + wasm `tauri_invoke` bridges included), confirming the WellFair wasm path is sound.

**⚑ Where I need the human — pre-existing wasm-app build break (NOT WellFair):**
The Studio **wasm application** build fails on pre-existing errors in non-WellFair code:
- `render/spatial_bridge.rs` — borrow-after-move on wasm (the `move` closure owned `page_for_effect`,
  then it was used again). **I fixed this** (one-line clone-shadow mirroring the existing non-wasm path)
  because it blocked the lib from compiling and was an unambiguous bug. Flagged here as out-of-lane.
- `components/physics_simulator.rs` / app-loop `running` / `forge_running` — `cannot borrow … as mutable`
  (missing `let mut`). **Not fixed** — clearly outside the WellFair lane and possibly another instrument's
  active render/physics WIP (an untracked `render/studio_preview.rs` is in the tree). The desktop UI as a
  whole cannot build to the webview until these are resolved. **Decision for Timothy:** allocate the
  Studio wasm-build repair (physics/forge mutability) to an instrument, or confirm I may fix them.

**Honest scope statement:** the WellFair desktop wiring is complete and compiles; I have NOT been able to
run the assembled desktop app end-to-end because the Studio wasm build is broken by unrelated pre-existing
code. That is a real limit, stated plainly — not something my changes caused.

**Next step:** unblock the Studio wasm app build (needs the allocation call above), then a live end-to-end
pass of the three new panels; or continue Phase 5 backend (sync-operation protocol).

---

## 2026-07-02 — Studio wasm build fully green + Phase 5 sync protocol + clinical/welfare-support (Claude / Opus 4.8) — DONE

**Phase / status:** Timothy removed the lane restriction ("you're the only agent"). Cleared the wasm
blocker, built the Phase 5 sync-operation protocol, and added two more domain modules via swarm.
**All green.**

**1. Studio wasm app build — now GREEN.** Fixed the pre-existing, non-WellFair wasm-only errors that
blocked the whole UI from building to the webview:
- `render/spatial_bridge.rs` — borrow-after-move (clone-shadow, mirroring the non-wasm path).
- `components/benchmark_harness.rs` (×2) and `components/physics_simulator.rs` — Dioxus signals whose
  `.set()` was called before the `let mut` shadow (reordered / added `let mut`). Only bit on wasm because
  the calls sit inside `#[cfg(target_arch = "wasm32")]` blocks.
- `cargo check -p webizen-studio --target wasm32-unknown-unknown` → **green**. The entire desktop UI,
  including the three new WellFair panels, now builds to wasm.

**2. Phase 5 sync-operation protocol (SyncService §4.2 / §9.5 / §17).** New
[`wellfair/sync_protocol.rs`](crates/qualia-client-core/src/wellfair/sync_protocol.rs):
- `SyncOperation` — versioned (protocol + schema), content-hashed wire DTO with Lamport clock,
  routing lane, and detached signature.
- `validate_operation` — fail-closed quarantine: rejects wrong-version, oversized, unsigned,
  wrong-hash, and **Classified/Sanctuary** frames; flags replays as `Duplicate`.
- `SyncInbox` — durable quarantined jsonl inbox; **idempotent admission** (replayed id never applied twice).
- `merge_operations` — add-wins by op id, order-independent → deterministic convergence.
- Host API: `build_outbound_operation` (real ed25519 signature over the bound payload; **returns None
  for Classified** so Sanctuary never enters the lane), `admit_sync_operation`, `validated_sync_operations`.
- Tests include a **two-node partition→rejoin convergence** check and full replay/malformed/lane rejection.

**3. Two more domain modules (swarm-authored, self-integrated).**
- [`clinical.rs`](crates/wellfare-core/src/clinical.rs) (CLI-01..13) — manual clinical reports +
  attachment metadata (content-addressed, no fake parsing) + an honest claim lifecycle: only
  `ClinicianConfirmed` maps to `ClinicianObserved` evidence; a self-report is never laundered into
  clinician-grade evidence.
- [`welfare_support.rs`](crates/wellfare-core/src/welfare_support.rs) (LIF-08..14) — assistance needs,
  welfare streams, government letters.
- Integrated: journal kinds added; both modules declared in `lib.rs`.

**Measured results:** `cargo test -p wellfare-core` → **76 passed**. `cargo test -p qualia-client-core
wellfair::` → **69 passed**. `cargo check -p webizen-desktop` → green. `cargo check -p webizen-studio
--target wasm32-unknown-unknown` → green.

**⚑ Where I need the human:** none blocking. Note (not a blocker): `clinical`/`welfare_support` are
domain + `wellfare-core` layers only — not yet surfaced via Tauri commands or Studio panels; and the sync
protocol has host API but no Tauri command/UI yet. All are complete, tested units ready to wire next.

**Next step:** wire clinical/welfare-support host API + panels and sync-inbox status into the UI; or
continue Phase 5 (apply validated inbound ops into the local vault) and the credential-blob persistence
flagged earlier.

---

## 2026-07-02 — Clinical + Welfare + Sync-inbox UI, wired end-to-end (Claude / Opus 4.8) — DONE

**Phase / status:** Full-stack wiring of the clinical, welfare-support, and sync-inbox surfaces.
**Host API → Tauri → Studio panels; all layers compile (host + wasm).**

**What was built:**
- **Host API** (`wellfair/api.rs`): `add_clinical_report` / `list_clinical_reports`;
  `add_assistance_need` / `add_welfare_stream` / `add_government_letter` / `list_welfare_records`;
  policy writers `wellfair-clinical` + `wellfair-welfare`. Clinical epistemics stay honest (claim
  status drives evidence/epistemic type in `clinical.rs`).
- **Tauri commands**: `wellfair_add_clinical_report` (observed=0 ⇒ host stamps now),
  `wellfair_add_assistance_need`, `wellfair_add_welfare_stream`, `wellfair_add_government_letter`,
  `wellfair_list_sync_inbox`; all registered. String→enum parse helpers for report type / urgency / stream status.
- **Studio host_client** bridges + DTOs for all of the above, incl. `SyncInboxRecordDto`.
- **Three new panels**: [clinical_panel.rs](crates/webizen-studio/src/components/wellfair/clinical_panel.rs)
  (records-first, no-parsing note), [welfare_panel.rs](crates/webizen-studio/src/components/wellfair/welfare_panel.rs)
  (needs / streams / letters), [sync_panel.rs](crates/webizen-studio/src/components/wellfair/sync_panel.rs)
  (read-only quarantined-inbox status with per-op outcome colouring).
- **Shell nav**: new "Clinical" area; welfare panel folded into "Life"; sync panel into "Tools".

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **71 passed** (+2 host tests:
`clinical_report_commits_with_clinical_kind`, `welfare_records_commit_and_list`).
`cargo check -p webizen-desktop` → green. `cargo check -p webizen-studio` (host) → green.
`cargo check -p webizen-studio --target wasm32-unknown-unknown` → green.

**⚑ Where I need the human:** none blocking. Remaining Phase-5 depth items (unchanged): apply validated
inbound sync ops into local derived views; credential-claim blob persistence; and the Sanctuary
independent-key encryption ADR. All are scoped and flagged, none block the current usable product.

**Next step:** the inbound-apply step (fold validated inbox ops into `ledger_balance` /
`project_obligations` so cross-node convergence shows in the UI), then credential-blob persistence.

**Addendum — inbound-apply for obligations DONE.** `synced_project_obligations` folds validated
inbound `contribution` sync operations into the local obligation derivation (pure over the unique
record-id set, so replayed remote ops never double-count). The `wellfair_project_obligations` command
now returns this converged view, so the Projects panel reflects accepted remote contributions. New test
`synced_obligations_fold_in_validated_remote_contributions_replay_safe` builds a signed op on peer A,
admits it on peer B, and asserts both contributors' effort appears and a replay leaves it unchanged.
`cargo test -p qualia-client-core wellfair::` → **72 passed**; desktop green. (Personal-finance ledger
left local-only by design — it is a personal ledger, not a shared one; revisit if a shared-ledger use
case appears.)

---

## 2026-07-02 — Sanctuary encrypted vault: real at-rest boundary + decoy lane (Claude / Opus 4.8) — DONE

**Phase / status:** The highest-integrity gap. Sanctuary upgraded from a read-projection filter to a
**real encrypted-at-rest store with an independent decoy lane**, wired end-to-end. All green.

**What was built:**
- New [`wellfair/sanctuary_vault.rs`](crates/qualia-client-core/src/wellfair/sanctuary_vault.rs)
  (native-only), built on the repo's existing tested `qualia_core_db::crypto::sanctuary_crypto`
  (PBKDF2-HMAC-SHA256 @ 310k iterations + AES-256-GCM, key material zeroized):
  - Two independent lanes (**real** + **decoy**), each with its own random salt-derived key. Notes live
    only as AEAD ciphertext in `sanctuary_vault.json`; nothing is readable without the PIN.
  - **PIN never stored** (not even hashed) — a per-lane encrypted *verifier* recognises which lane a PIN
    opens. Duress/decoy PIN operates only on the decoy lane and never aliases real data.
  - Monotonic per-lane nonce counter (no nonce reuse under a key); AAD binds ciphertext to its lane.
  - No destructive "nuke PIN" (plan §6).
  - Enabled the `sanctuary-crypto` feature on the `qualia-core-db` dep for this crate.
- Host API (native-only): `sanctuary_vault_configured` / `setup_sanctuary_vault` /
  `sanctuary_vault_resolve_lane` / `add_sanctuary_vault_note` / `list_sanctuary_vault_notes`.
- Tauri commands (4) + host_client bridges + a new **Encrypted vault** section in the Sanctuary panel
  (create → open with PIN → shows real/decoy lane badge → add/list decrypted notes → close clears them).

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **78 passed** (+6 vault tests:
lane isolation, decoy-never-sees-real, **plaintext-never-on-disk**, tamper rejection, wrong-PIN
rejection, survives-reopen). Tests parameterise PBKDF2 iterations (1k in tests, 310k in production) so
the suite runs in <1s. `cargo check` green for `webizen-desktop`, `webizen-studio` (host), and
`webizen-studio --target wasm32-unknown-unknown`.

**⚑ Where I need the human / honest scope:**
- **Defensible defaults made (overridable by ADR):** PBKDF2-HMAC-SHA256 @ 310k as the KDF; AES-256-GCM
  as the AEAD; decoy = a separate empty encrypted lane the owner can populate. If you want Argon2id, a
  different decoy-seeding policy, or OS-keychain-wrapped keys, that's the ADR to write — say the word.
- **Coexistence:** the older projection filter + `add_sanctuary_note` (Classified journal row) remain as
  defense-in-depth for other Classified journal kinds (therapy_note, welfare_case). The encrypted vault
  is now the real store for free-text sensitive notes. Fully migrating `add_sanctuary_note` into the
  vault (removing the journal path) is a clean follow-up once you're happy with the vault UX.
- **Pre-existing (not mine):** `sanctuary_crypto` uses the deprecated `aead::AeadInPlace` API (compile
  warnings from `qualia-core-db`) — a §13 modernization follow-up in that crate, not blocking.

**Next step:** optionally migrate `add_sanctuary_note` fully into the vault; credential-claim blob
persistence; or the OS-keychain key-wrapping hardening.

**Addendum — `add_sanctuary_note` journal path RETIRED.** Free-text sanctuary notes no longer touch the
plaintext journal at all — they exist only in the encrypted vault. Removed `WebizenHostApi::add_sanctuary_note`,
the `SanctuaryNote`/`build_sanctuary_note_envelope`/`sanctuary_note_summary` types, the
`wellfair_add_sanctuary_note` Tauri command + its host_client bridge, and the plaintext "Save sanctuary
note" section in the projection panel (the Encrypted-vault section replaces it). The projection filter +
`sanctuary_note` protected-kind constant remain as defense-in-depth for any legacy rows and for the other
Classified journal kinds (therapy_note, welfare_case). Two tests that used the old note as a Classified
fixture now use `add_therapy_note` (also Classified + protected). `wellfare-core` 76 + `wellfair` 78 tests
pass; `webizen-desktop` + `webizen-studio` (host + wasm) all green. This closes the last plaintext-at-rest
path for sanctuary notes.

---

## 2026-07-02 — Credential-claim blob persistence + real presentation flow (Claude / Opus 4.8) — DONE

**Phase / status:** Closed the credential-blob gap flagged after the encrypted-vault work: credential
claims are now durably stored and a field-selected presentation works end-to-end. All green.

**What was built:**
- New content-addressed [`wellfair/blob_store.rs`](crates/qualia-client-core/src/wellfair/blob_store.rs)
  (plan §5): SHA-256-keyed blobs under `wellfair/blobs/`; idempotent `put`, integrity-verifying `get`
  (rejects a content-hash mismatch), path-traversal-safe (only 64-char hex handles resolve), atomic
  temp+rename writes. Reusable for clinical/letter attachments too.
- `add_credential` now **persists the full credential (incl. claims) as a blob** — the envelope
  `blob_hash` (previously a dangling hash) is that blob's content address.
- Host API: `get_credential(record_id)` loads the credential back from its blob; `present_credential`
  builds a `FieldSelectedPresentation` over stored claims — still honestly plain field selection, not ZK.
- Tauri commands `wellfair_get_credential` / `wellfair_present_credential` + host_client bridges + DTOs.
- Credentials panel: each held credential gets a **Present** button → loads its claims → checkbox list
  of claim keys to disclose → **Build presentation** → shows the disclosed subset (with the
  field-selection-not-ZK disclaimer in the UI).

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **85 passed** (+6 blob-store tests
incl. tamper + path-traversal rejection, +1 credential persist/present round-trip). `cargo check` green
for `webizen-desktop`, `webizen-studio` (host + wasm32).

**⚑ Where I need the human:** none blocking. The blob store is now available to give clinical/government-letter
attachments real byte storage (currently metadata-only) — a natural next use. Remaining flagged items:
OS-keychain key-wrapping for the Sanctuary vault; `aead` API modernization in `qualia-core-db`.

**Next step:** wire real attachment bytes for clinical reports / government letters through the blob
store; or the OS-keychain vault hardening.

---

## 2026-07-02 — Clinical attachment bytes via the blob store (Claude / Opus 4.8) — DONE

**Phase / status:** The next use of the blob store — clinical attachments now carry **real file bytes**,
not just metadata. Byte handling stays native (desktop reads/writes files; the wasm UI passes paths),
avoiding browser FileReader/base64 plumbing. All green.

**What was built:**
- Host API: `add_clinical_attachment(filename, media_type, bytes)` stores the bytes as a content-addressed
  blob and commits the `clinical_attachment` metadata record (filename/size/hash); `list_clinical_attachments`;
  `attachment_bytes(record_id)` reads them back (integrity-verified via the blob store).
- Desktop commands: `wellfair_add_clinical_attachment_from_path` (native file read, media-type inferred from
  extension when omitted) and `wellfair_export_attachment` (native write of the blob bytes to a chosen path).
- host_client bridges + a clinical-panel **Attachments** section: attach a file by path, list attachments
  (filename/size/hash), and export any attachment to a destination path.

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **86 passed** (+1
`clinical_attachment_stores_and_retrieves_bytes` round-trip). `cargo check` green for `webizen-desktop`,
`webizen-studio` (host + wasm32).

**⚑ Where I need the human:** none blocking. Government-letter attachments can reuse the exact same path
(the record already has an `attachment_blob_hash` field) — a small follow-up. Native file *dialogs* (vs
typed paths) would need the Tauri dialog plugin — a UX nicety, not required for function. Remaining flagged:
OS-keychain vault key-wrapping; `aead` API modernization in `qualia-core-db`.

**Next step:** government-letter attachment bytes (same pattern); OS-keychain vault hardening; or a native
file-dialog nicety.

---

## 2026-07-03 — T1.5 guardianship M:N: proxy-write approval escrow, wired end-to-end (Claude / Opus 4.8) — DONE

**Phase / status:** WellFair finish-out §C T1.5 — the real Phase-3 guardianship gap. The `Suspend`
policy variant and `SuspendedTransactionQueue` existed but were **never wired**: policy never emitted
`Suspend`, and the submit path lumped it with `Prompt` and returned a flat error. Now it is a real
approval escrow, proven end-to-end. All green.

**What was built (supported agency, not warden control):**
- `wellfare-core/guardianship.rs` — `GuardianshipProposal` (escrows the serialized `RecordEnvelope` +
  threshold + reason) and immutable `GuardianshipVote`; `derive_status` is a **replay-safe projection**
  (latest-vote-per-guardian; duplicated/reordered co-signatures converge; a standing objection halts the
  escrow as a protective veto). 10 tests.
- `wellfair/policy.rs` — `evaluate_access` gains `is_proxy_action`; a **proxy** write of a **Restricted**
  record now returns `Suspend { required_approvals: 2 }` instead of silently committing. Non-proxy writes
  are unchanged (every existing test still green); Classified retains its fail-closed allowlist. +3 tests.
- `wellfair/api.rs` — refactored the submit path into `submit_record_guarded → SubmitOutcome`
  (`Committed`/`Suspended`); `submit_record_with_summary` is now a thin back-compat wrapper. On `Suspend`
  the write is escrowed as a proposal record; `list_guardianship_proposals` (derived status + committed
  flag), `vote_guardianship_proposal` (append vote → re-derive → **on ratification commit the escrowed
  record through the signed vault path, idempotently**), and a supporter entry point
  `propose_proxy_condition`. +3 host tests (suspend→2 approvals→commit→replay-safe; objection denies+blocks;
  non-proxy unaffected).
- Desktop: 3 Tauri commands (`wellfair_propose_proxy_condition`, `_list_guardianship_proposals`,
  `_vote_guardianship_proposal`) registered; host_client bridge (+ `GuardianshipProposalDto`); a new
  **Guardianship** Studio area/panel — record-on-behalf form + approval tray with Approve / Object.

**Measured results:** `cargo test -p wellfare-core` → **86 passed** (+10). `cargo test -p
qualia-client-core wellfair::` → **94 passed** (+7). `cargo test -p qualia-cooperative-core` → 60.
`cargo check` green: `webizen-desktop`, `webizen-studio` (host **and** `wasm32-unknown-unknown`).

**⚑ Where I need the human (one values call):** the escrow's veto semantics — I implemented **"any
guardian objection halts the request"** (fail-safe / protective of the principal against an erring proxy).
The alternative is "advisory objection; ratify once approvals ≥ threshold regardless." I chose the
protective default; if you want threshold-wins-over-objection (or a configurable per-delegation policy),
that's a one-line change in `derive_status` + a flag on the delegation. Also: the suspension trigger is
currently "proxy + Restricted"; extending it to consequential-domain gating from the agency layer
(`agency_domain::is_consequential`) is the tracked Migration follow-up, not a gap in this mechanism.

**Next step:** T1.4 native file dialogs (Tauri dialog plugin) or T1.2 OS-keychain wrapping (recovery-gated,
off by default) — both remaining §C finish-out items; then the ZK predicate circuits so property-proofs
back the disclosure modality with the now-real Groth16.

---

## 2026-07-03 — T1.4 native file dialogs + T1.2 OS-keychain vault wrapping (Claude / Opus 4.8, 2 sub-agents) — DONE

**Phase / status:** Two §C finish-out closers, done in parallel (one sub-agent for the isolated desktop
dialog lane; I did the security-sensitive keychain lane myself) and integrated. All targets green.

**T1.4 — native file dialogs (sub-agent, integrated):** added `tauri-plugin-dialog`, registered it,
authored a `capabilities/default.json` (re-declaring the bootstrap defaults + `dialog:*`), added
`wellfair_pick_file_path` / `wellfair_pick_save_path` commands (blocking `DialogExt`), host_client bridges,
and **Browse…** buttons next to the clinical Attachments attach/export path inputs. Typed paths remain the
fallback.

**T1.2 — OS-keychain vault wrapping (opt-in, off by default; recovery-gated):**
- `qualia-core-db::crypto::sanctuary_keychain` — thin OS-keychain I/O (Windows Credential Manager / macOS
  Keychain / Secret Service) for a 32-byte pepper: generate / store / get / delete.
- `wellfair::sanctuary_vault` — an optional pepper folds into the PBKDF2 input
  (`effective_secret = SHA256(domain ‖ pepper ‖ pin)`), so a wrapped vault needs **disk + PIN + this
  device's keychain**. `keychain_wrapped`/`vault_id` added to the vault meta with serde defaults, so
  existing unwrapped vaults are byte-compatible and derive exactly as before. `setup_wrapped` returns a
  one-time hex **recovery code**; `unlock_with_recovery` reopens a wrapped vault on a device whose keychain
  entry was lost and re-seats the pepper. 3 hermetic tests (no real keychain I/O) prove the pepper binds
  the key (PIN-alone and wrong-pepper both fail) and that the unwrapped path is unaffected.
- Host API (`sanctuary_vault_is_keychain_wrapped`, `setup_sanctuary_vault_wrapped`,
  `sanctuary_vault_unlock_with_recovery`) + 3 Tauri commands + host_client bridges + an **experimental**
  "bind to this device's keychain" checkbox in the vault setup panel that surfaces the recovery code once,
  with a blunt unrecoverable-if-lost warning.

**Measured results:** `cargo test -p qualia-client-core wellfair::` → **97 passed** (+3 keychain-pepper).
`cargo test -p wellfare-core` → 86. `cargo check` green: `qualia-core-db` (sanctuary-crypto),
`webizen-desktop`, `webizen-studio` (host + `wasm32`). (Two pre-existing, unrelated client-core test
failures — `qpu_oracle::commitment_text_verifies` and a `chat_session` parallelism flake — are in modules
this work never touched; confirmed independent.)

**⚑ Where I need the human:** T1.2's *mechanism* is done and off by default. Whether to make wrapping the
default, and the exact recovery-code UX/backup guidance, is the recovery-model decision folded into the
Sanctuary threat-model ADR (T2.1) — your call. The experimental toggle exists so it can be exercised now.

**Next step:** the companion PWA workstream (now elevated — see the cooperative log + plan) and the ZK
predicate circuits.
