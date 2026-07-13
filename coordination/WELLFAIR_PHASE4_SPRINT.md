# WellFair Phase 4 Sprint — Live share consent (2026-07-02)

**Canonical repo:** `C:\Projects\qualia-27062026` | **Branch:** `0.0.24`  
**Human rights principle:** explicit owner approval before companion receives any health projection — minimum disclosure, fail-closed on Sanctuary kinds.

## Swarm lanes (parallel → orchestrator merge)

| Lane | Worker | Owns | Gate |
|------|--------|------|------|
| **A** | Domain | `wellfare-core/live_share.rs` wire types | 4 unit tests |
| **B** | Host API | `qualia-client-core/wellfair/live_share.rs`, `api.rs`, `phase4_tests.rs` | 3 integration tests |
| **C** | UI | `communications_panel.rs`, `host_client`, `shell.rs` Communications tab | `cargo check -p webizen-studio` |
| **Orchestrator** | Grok | `commands/mod.rs`, `companion_gateway.rs` WS handlers | full stack green |

## Delivered

| Capability | Module |
|------------|--------|
| Usage agreement wire (COM-02) | `wellfare_core::live_share::UsageAgreement` |
| Live section request/decision (COM-03) | `LiveSectionRequest`, `LiveSectionDecision` |
| Pending request store | `LiveShareStore` jsonl |
| Owner approve/deny API | `WebizenHostApi::{submit,decide,list}_live_share*` |
| Sanctuary fail-closed on classified projection | `validate_live_share_decision` |
| Companion WS ingest | `USAGE_AGREEMENT`, `LIVE_SECTION_REQUEST` → ACK |
| Desktop UI | **Communications** tab: pending queue + approve/deny + pairing QR |
| Tauri commands | `wellfair_list_pending_live_shares`, `wellfair_decide_live_share` |

## Deferred (Phase 4 remainder)

- Signed PWA bundle export (M2)
- Push decision back to companion over WS (companion polls / future event subscribe)
- Caller gate + signed usage agreement UI on mobile harness
- WebRTC / media (COM layer 5–6)

## Verification

```powershell
cd C:\Projects\qualia-27062026
cargo test -p qualia-client-core phase4 --lib
cargo test -p wellfare-core live_share --lib
cargo check -p webizen-studio -p webizen-desktop
```