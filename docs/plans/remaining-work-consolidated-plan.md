# Remaining Work — Consolidated Plan (WellFair + Cooperative)

**Status:** Active execution backlog
**Prepared:** 2026-07-03
**Supersedes nothing** — this folds the outstanding items from both existing plans into one
prioritized, honestly-tracked backlog:

- [wellfair-webizen-desktop](wellfair-webizen-desktop/README.md) — the WellFair desktop plan (Phases 0–7)
- [cooperative-qapps-desktop-implementation-plan](cooperative-qapps-desktop-implementation-plan.md) — WP0–WP11 / Releases A–F

Progress logs: [WELLFAIR_DESKTOP_PROGRESS_LOG.md](../../WELLFAIR_DESKTOP_PROGRESS_LOG.md),
[COOPERATIVE_QAPPS_PROGRESS_LOG.md](../../COOPERATIVE_QAPPS_PROGRESS_LOG.md).

## What is already done (do not rebuild)

WellFair MVP core (8 domains), encrypted Sanctuary vault + decoy lane, content-addressed blob store
(credential claims + clinical attachment bytes), Phase-5 sync-operation protocol with replay-safe
convergence, and the shared `qualia-cooperative-core` crate + work-item Kanban. All committed on `0.0.24`.

## The consolidated backlog (prioritized)

Legend: **size** S(mall)/M(edium)/L(arge); **gate** = needs a human decision only Timothy can make.

### Semantic reframing (2026-07-03, Timothy)

`government_letter` / `clinical_report` / `credential` / guardianship are not flat leaf records — they are
**authority attestations** (authorizing entity + agent-in-capacity + jurisdiction/department + a
representation that may be a document, a credential, or a document-with-baked-in-credential). Guardianship
is a family of role/purpose/agent-scoped authorizations, not a single relation. See
[adr-authority-attestation-guardianship-model.md](adr-authority-attestation-guardianship-model.md). This
reshapes **T1.1** (government letters → attestation `DocumentBlob` representation) and **T1.5**
(guardianship → role/purpose/basis delegation). Implementation is gated on Timothy settling the ADR §5
vocabulary; the shipped flat records keep working meanwhile.

### Tier 1 — WellFair finish-out (clear these before large new efforts)

| # | Item | Size | Gate | Notes |
|---|---|---|---|---|
| T1.1 | **Government-letter attachment bytes** → generalize to **authority-attestation** | S→M | — | Bytes path built (blob representation). Generalizing the record to the attestation model is ADR-gated. |
| T1.2 | **OS-keychain key-wrapping for the Sanctuary vault** | M | ⚑ | Mix an OS-keyring per-vault pepper into the KDF (via `keyring = "3.1"`) so the vault file is useless without the keychain entry. **RECOVERY GATE:** keychain loss (OS reinstall, machine change) then = permanent vault loss. Needs a recovery-model decision (opt-in? one-time recovery code? keychain-optional?) folded into T2.1 before shipping enabled. Mechanism can be built opt-in/off-by-default now; enabling it is Timothy's call. |
| T1.3 | **`aead` API modernization** in `qualia-core-db::crypto::sanctuary_crypto` | S/M | — | Replace deprecated `AeadInPlace`/`Array::from_slice` with `AeadInOut`/`TryFrom` (§13). Keep the 8 sanctuary_crypto tests green. |
| T1.4 | **Native file dialogs** for attach/export | S/M | — | `tauri-plugin-dialog` file picker feeding the existing path-based commands; typed paths remain a fallback. |
| T1.5 | **Guardianship M:N approval** wiring | M | — | `Suspend`/`SuspendedTransactionQueue` types exist but the policy never emits `Suspend` and no request→suspend→approve→resume flow is wired. Build the flow + a Consent/guardian UI. |

### Tier 2 — Human-gated (Timothy decides; then implementable)

| # | Item | Size | Gate | Notes |
|---|---|---|---|---|
| T2.1 | **Sanctuary threat-model ADR** | — | ✅ | Confirm/adjust: KDF (PBKDF2-310k vs Argon2id), AEAD (AES-256-GCM), decoy-domain semantics, OS-keychain layering (T1.2). Defensible defaults shipped; needs your sign-off. |
| T2.2 | **Mental-wellbeing assessment instruments** (DASS-21, PHQ-9, GAD-7, K10, BDI-II) | M | ✅ | Per-instrument: version, licence, exact items, scoring, interpretation, repeatability, disclaimer (plan §Q8). Blocked until you supply/approve the licensing + scoring for each. |

### Tier 3 — Large new efforts (staged; multi-session each)

| # | Item | Size | Gate | Notes |
|---|---|---|---|---|
| T3.1 | **Real network transport for sync** (libp2p / WSS) | L | — | The sync-operation protocol + quarantined inbox exist; this adds an authenticated transport that drains the outbox and feeds the inbox. Cooperative plan WP7 / §15.2. Hostile-peer + two-node convergence tests. |
| T3.2 | **Companion PWA + secure-origin pairing** (HTTPS/WSS or WebRTC) | L | — | Replace the plain LAN-WS companion gateway with a release-secure origin + signed PWA bundle. WellFair Phase 4 / plan §9. (Originally Grok's lane.) |
| T3.3 | **Phase 6 release hardening** | L | partial | Reproducible builds, installers + signed updates, SBOM/provenance, backup/restore/migration/rollback, corrupt-volume recovery, accessibility audit, 42 MB Sentinel + zero-alloc checks, privacy-safe diagnostics. |
| T3.4 | **Phase 7 optional** | L | partial | 3D anatomy, studies/rules, authenticated Solid Pod sync, model-assisted extraction, wallet/private transport, distributed analytics, native mobile peer. |

### Cooperative plan work packages (parallel initiative)

WP1 Qapp token v2 + isolation (release gate) · WP2 Studio Package & Publish · WP4 standalone Cooperative
Qapp · WP9 Development Cooperative dogfood · WP5/6/7/8/10/11 as staged. These proceed on the cooperative
track; **T3.1 (sync transport) is shared** with cooperative WP7.

## Execution order

1. **Now:** T1.1, T1.3 (parallel: sub-agent), T1.2 — clear the cheap WellFair gaps + one hardening item.
2. **Next:** T1.4 (dialogs), T1.5 (guardianship) — finish WellFair Tier 1.
3. **Then:** cooperative WP1 (token v2) — the security release gate that unblocks installed Qapps.
4. **Then, by size and your steer:** T3.1 sync transport (shared win for WellFair + cooperative), WP2/WP4,
   T3.2 companion PWA, WP9 dogfood.
5. **Gated on you:** T2.1 (Sanctuary ADR sign-off), T2.2 (assessment instrument licensing).

Each item lands green (tests + `cargo check` desktop/studio host+wasm) and is logged before the next.
