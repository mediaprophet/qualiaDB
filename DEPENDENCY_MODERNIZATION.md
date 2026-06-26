# Dependency Modernization — execution wrapper

## ⚠ Source reliability — read this first
The two source files — `20260626_dependencyLeverageAudit.md` (items **A–K**) and
`20260626-eval-to-do-list.md` (12 version categories) — were produced by **a bot that is not reliable**
(Timothy, 2026-06-26). They are **illustrative of the *shape* of what needs to happen — NOT an exacting
account of what is actually in the code.** Treat them as **a hypothesis list, not facts.**

**Verify everything before acting** against the real sources of truth — the **codebase, the compiler, and
the actual dependency docs**:
- Locations and **line numbers** may be wrong or stale.
- **Version-direction claims** may be reversed (see the confirmed example below).
- The eval-list `[x]` marks and the audit's **K ("already-modernized")** are **UNVERIFIED claims** — do
  **not** skip an item because the draft says it's done; check it.
- Items may be **missing**, or describe code that no longer exists.

So this file is the **execution wrapper (the "how")**: work each *candidate* area under
[`CLAUDE.md`](CLAUDE.md) **§13** (modernize to the new API **and capabilities**) and **§14** (sub-agents,
own worktrees), **verifying the claim first**, with the lane cautions below. These two drafts are also
**untracked at repo root** — read by path; Timothy may want them committed.

## ⚠ Confirmed error (proof the source is unreliable)
Audit **item E** states "`wgpu::PollType` is entirely removed … replaced by `wgpu::Maintain::Wait`."
**Backwards for wgpu 29:** the observed build error is `cannot find Maintain in wgpu` at
`lora/webgpu_lora.rs:187` → in wgpu 29 **`Maintain` is the *removed* symbol; the current poll API is
`PollType`**. The compiler is the authority. **Assume other such errors exist in the drafts** — verify each.

## Candidate items × disposition (claims to VERIFY, not facts — detail in the audit)
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
| K | *claimed* already-modernized — base64/secp256k1/chrono/ash/clap/thiserror/dioxus/sysinfo/oxigraph | **UNVERIFIED** — don't skip on the draft's say-so; spot-check before trusting |

**Method (§13/§14):** own worktree per item; update to the new API **and adopt its new capabilities**
(not just "make it compile"); green build + targeted tests pasted; per-step log + one `NOTICES.md` line;
behaviour preserved (or the change *is* the fix, stated plainly).

**Parallelization (sub-agents):** good independent, non-lane-conflicting units → **A, B, D, G, J**.
**Coordinate, never spawn into:** **C, H** (Gemini's `qualia-client-core`), **E, F** (LLM/lora lane —
and they pair with the substrate GPU-backend work, so sequence them with that). Announce every sub-agent
in `NOTICES.md`.
