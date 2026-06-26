# Dependency Modernization — execution wrapper

**Canonical inventory (the "what"):** `20260626_dependencyLeverageAudit.md` (repo root, Timothy's draft —
items **A–K**; A–J are work, **K = already-modernized/verified-good**). **Version-bump tracker:**
`20260626-eval-to-do-list.md` (12 categories — the *crates* are bumped/evaluated, but the *code* still
needs the A–J modernizations). Both are **drafts, possibly incomplete** (Timothy maintains them).

This file is the **execution wrapper (the "how")**: working the audit's items under
[`CLAUDE.md`](CLAUDE.md) **§13** (modernize to the new API **and capabilities**, fix problems along the
way) and **§14** (spawn sub-agents, each in its own worktree) — plus **lane cautions** and one correction.

> ⚑ These two source drafts are currently **untracked at repo root** — Timothy may want them committed so
> the worktrees/sub-agents see them; until then, read them by repo-root path.

## ⚠ Correction to verify (don't trust the draft blindly — the compiler is the authority)
Audit **item E** states "`wgpu::PollType` is entirely removed … replaced by `wgpu::Maintain::Wait`."
**That is backwards for wgpu 29.** The observed build error is `cannot find Maintain in wgpu` at
`lora/webgpu_lora.rs:187` → in wgpu 29 **`Maintain` is the *removed* symbol; the current poll API is
`PollType`**. Confirm against the wgpu 29.0.3 docs at execution time. (Flagged to Timothy to fix the draft;
do not propagate either claim without checking the actual 29.0 API.)

## Items × disposition (full detail + locations in the audit)
| Id | Modernization (location) | Lane disposition |
|----|----|----|
| A | stream HTTP body → `tokio::fs` (kill `Vec<u8>` + blocking `std::fs::write`) — `daemon/webizen_server.rs` | services/daemon — also the `uuid` build-break site; own worktree |
| B | warp → axum harmonize — `qualia-solid-bridge` (`solid_proxy.rs`, `oidc_micro_idp.rs`) | solid-bridge (ex-OIDC area) |
| C | tiny_http → axum + `tower-http::ServeDir` — `qualia-client-core/qapps_protocol.rs` | ⚠ **qualia-client-core = Gemini's live lane — coordinate, DO NOT spawn** |
| D | raw TCP websocket → axum `WebSocketUpgrade` — `qualia-cli/telemetry_server.rs` | qualia-cli |
| E | wgpu instance/poll → wgpu 29 (**see correction above**) — `lora/webgpu_lora.rs` | ⚠ **LLM/lora lane — coordinate** |
| F | WGSL → naga 29 caps (`f16`, matrix) for LoRA matmul — `lora/webgpu_lora.rs` | ⚠ **LLM/lora lane — coordinate**; perf-relevant |
| G | arkworks 0.6 serialization (compressed / length-checked) — `specialized_libs/cryptographic_library` | crypto |
| H | reqwest blocking → async `bytes_stream()` — `qualia-client-core` + `qualia-semantic-library` | ⚠ **client-core = Gemini's lane — coordinate**; semantic-library is separate |
| I | `#[wasm_bindgen]` → `.wit` component model — `wellfare-core`, `webizen-web` | wasm |
| J | `windows::core::Result` → `windows-result 0.4` — `inference/directml_bridge.rs` | inference (DirectML) |
| K | already-modernized — base64/secp256k1/chrono/ash/clap/thiserror/dioxus/sysinfo/oxigraph | none (verified good) |

**Method (§13/§14):** own worktree per item; update to the new API **and adopt its new capabilities**
(not just "make it compile"); green build + targeted tests pasted; per-step log + one `NOTICES.md` line;
behaviour preserved (or the change *is* the fix, stated plainly).

**Parallelization (sub-agents):** good independent, non-lane-conflicting units → **A, B, D, G, J**.
**Coordinate, never spawn into:** **C, H** (Gemini's `qualia-client-core`), **E, F** (LLM/lora lane —
and they pair with the substrate GPU-backend work, so sequence them with that). Announce every sub-agent
in `NOTICES.md`.
