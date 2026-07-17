# Webizen UI — Progress Log

**Programme:** UI product cut via [`webizen-ui-implementation-subagents-2026.md`](./webizen-ui-implementation-subagents-2026.md)  
**Parent:** [`comprehensive-ui-gui-webizen-plan-2026.md`](./comprehensive-ui-gui-webizen-plan-2026.md)  
**Branch:** `0.0.25`

---

## 2026-07-18 — Orchestrator: full multi-agent closeout

**Status:** code complete for **U1–U6 + U2-B** (compile-verified); **runtime dogfood still human**

| Wave | Status |
|------|--------|
| U1-A telemetry · U1-B conduct · U1-C honesty · U1-D docs | done |
| U2-A browser honesty · U2-B browser agent v0 | done |
| U3-A/B MCP tool loop + allowlist | done (7 unit tests) |
| U4-A 10D · U4-B vision workbench · U4-C library perception | done |
| U5-A SPARQL · U5-B chemistry / physics / portfolio | done |
| U6 palette · event catalogue · harness notes | done |

**Measured:** `cargo check` client-core + desktop + studio **Finished OK**; `mcp_tool_loop` **7 passed**.

**⚑ Human dogfood:** rebuild desktop; Ctrl+K; Talk Permit `list_capabilities`; browser agent Permit/Deny; Local GGUF stream; 10D citable FORBID.

**Deferred by design:** audio AU waves; non-MCP external agents.

---

## 2026-07-18 — U2-B Browser agent (v0) UI shell

**Status:** done (UI shell; no new Tauri commands)

**What was built**
- **`chrome.html` Agent side panel → Browser agent (v0):**
  - Honesty chips: **Partial** + **Scaffold** (no silent tools; not full page-context agent).
  - Lists local MCP tools via `mcp_list_local_tools`; seeds safe allowlist via `mcp_ensure_safe_tool_allowlist` (roster write only, not a Permit).
  - Allowlist note for agent slug `local`; points to Talk MCP tool card for full editor.
  - **Two-step golden path:** “Run list_capabilities on permit” → propose UI → **Permit** calls `mcp_call_tool_gated` with `principal_permitted=true`, agent `local`, tool `list_capabilities`, args `{}`.
  - **Deny** clears propose state and **never** invokes MCP.
  - Deterministic page Q&A (`browser_agent_ask`) kept; separate from MCP path.
- **`browser_panes.rs` Reach parity:** “Browser agent” toggle panel with same Partial/Scaffold chips, tool list, two-step Permit/Deny, honesty footer updated.
- **Commands lock honoured** — reuses U3 `mcp_list_local_tools` / `mcp_call_tool_gated` / `mcp_ensure_safe_tool_allowlist`; no `commands/mod.rs` edits.
- Cert-override / Servo claims **unchanged** (not product-active / experimental pref only).
- Dogfood notes: brief U2-B section in [`webizen-browser-dogfood-notes.md`](./webizen-browser-dogfood-notes.md).

**Measured results**
- `cargo check -p webizen-studio` — **Finished OK** (exit 0 build; pre-existing warnings only).
- Desktop chrome is static HTML (no separate compile); asset written on desktop open via `ensure_chrome_asset`.
- Runtime dogfood of Permit/Deny on desktop **not measured** this session (human gate).

**⚑ Where I need the human**
- Rebuild desktop; open native Browser → Agent → Refresh tools → Run list_capabilities on permit → Permit (result) and Deny (no call).
- Confirm Talk allowlist still owns editing; browser shell is display + one golden path only.

**Next step**
- Optional: richer page-context tools under same gate; full agent autonomy remains out of U2-B scope.

---

## 2026-07-18 — U6-A/B/C Command palette + event catalogue + runtime harness

**Status:** done (code + docs); stream dogfood **blocked without model**

**What was built**
- **U6-A Command palette (desktop shell):** `shell_html.rs` modal palette — Ctrl+K / Ctrl+P, address-bar ⌘K button, View menu + `ShellAction::OpenCommandPalette` → `shell-open-command-palette` / `window.__webizenOpenCommandPalette`. Destinations (≥5): Talk, Browser (Reach), 10D / Infosphere, Settings, Library, QApps, Keep, Logs. Filter + ↑↓/Enter/Esc; navigates via existing `navigate(qappId)`.
- **U6-A Studio:** new `command_palette.rs` (same destinations → Dioxus `Route`), wired in `AppLayout`; floating Ctrl+K affordance; document keydown on wasm. Unit tests: 4/4.
- **U6-B Event catalogue:** expanded living doc `docs/manuals/webizen-ui-event-catalogue.md` — chat-token, chat-done, conduct-violation, shell-*, hardware-telemetry vs system-telemetry polling, render/GPU/anatomy, tool-loop slot for U3.
- **U6-B Virtualized list helper:** `virtualized_list.rs` — `window_range` / height helpers + frame component; unit tests 5/5.
- **U6-C Harness:** `scripts/webizen-ui-runtime-harness.md` + `scripts/webizen-ui-runtime-harness.ps1` (compile gate + manual Local stream steps).
- **No** `commands/mod.rs` edits.

**Measured results**
- `cargo check -p webizen-studio` — Finished OK (warnings only).
- `cargo check -p webizen-desktop` — Finished OK (warnings only).
- `command_palette` tests — **4 passed**.
- `virtualized_list` tests — **5 passed**.
- Runtime stream (activate model → chat-token) — **not measured / blocked** (no GGUF dogfood this session).

**Harness result template**
```
Date: 2026-07-18
Compile check: pass (desktop+studio)
Unit (palette/virt): pass (4+5)
Model available: no
Palette keyboard: code path landed; needs human desktop dogfood
Stream tokens: blocked-no-model
Conduct visible on deny: not exercised
```

**⚑ Where I need the human**
- Desktop dogfood: Ctrl+K opens palette; open all five core destinations.
- Optional: Local GGUF path for harness §3 stream assertion.

**Next step**
- Human dogfood palette + stream when model available; U3 owns tool events in catalogue when released.

---

## 2026-07-18 — U4-B Vision workbench polish

**Status:** done (UI honesty polish only; no new Tauri commands)

**What was built**
- **`vision_workbench.rs`:** Product-grade sections without rewriting the detector path:
  - **(a) Synthetic detect demo** — existing `vision_run_synthetic_demo` + generate / Image→3D / G→S / QVWT / reject-correct; plain empty state when no overlay; humanized offline/host errors.
  - **(b) SR / device policy status** — honest Partial text (classical SR + thermal/VRAM policy in engine/MCP; Studio is status-only; learned SR Needs model).
  - **(c) 10D recon link** — `Link` to `TenDBrowserRoute` with load/scrub/citable guidance (U4-A owns the path).
  - **(d) Biosense / self-monitor** — checkbox “I consent to process my biometrics on-device”; Start **disabled until consent**; on consent click reports **Scaffold** (no host command registered) — never fakes HR/rPPG success; no camera open.
- **HonestyChip:** Partial (detect, SR status, 10D link), Scaffold (biosense), NeedsConsent (biometrics), NeedsModel (learned SR).
- Manual append: [`docs/manuals/vision-10d-ui.md`](../manuals/vision-10d-ui.md) § Vision workbench (U4-B).
- **Commands lock honoured** — no `commands/mod.rs` / client-core edits. No audio. No `ten_d_browser.rs`.

**Measured results**
- `cargo check -p webizen-studio` — **Finished OK** (exit 0; warnings only elsewhere).
- Runtime dogfood not exercised this session (needs desktop shell).

**⚑ Where I need the human**
- Desktop dogfood: Keep → Vision → synthetic demo; confirm biosense Start stays disabled until checkbox; with checkbox, Start shows Scaffold text (not a fake pulse); Open 10D Browser link lands on recon list.
- When a consent-gated biosense host command is allocated, wire it under the same checkbox (do not remove NeedsConsent).

**Next step**
- U4-C library perception (sibling track); host biosense command is a later wave (commands lock YES then).

---

## 2026-07-18 — U5-A SPARQL + U5-B three real compute panes

**Status:** done (UI only; **no** `commands/mod.rs`)

### Three panes chosen (real host invokes)

| Pane | Host command | Engine path | Honesty |
|------|--------------|-------------|---------|
| **Chemistry Modeler** | `calculate_chemistry_properties` | `organic_chemistry::parse_smiles` + `compute_descriptors` (MW, Crippen LogP) | **Partial** (structure view still Scaffold) |
| **Physics Simulator** | `certify_forge_physics` | Forge `kinematics.nbody_step` CPU oracle ± WGPU | **Partial** (viewport is procedural; probe is real) |
| **Portfolio Analyzer** | `calculate_monte_carlo_var` | `economics::run_monte_carlo_var` (10k paths) | **Partial** (ES is host 1.25× heuristic) |

**Not chosen (mock / no host API):** `statistical_analysis.rs`, `matrix_lab.rs`, `ode_solver.rs` — left alone (hardcoded numbers or no invoke).

### U5-A SPARQL Explorer (`sparql_explorer.rs`)

- Honest **error banner** on invoke fail / bad payload (no fake S/P/O error rows).
- **Empty success** message (0 bindings ≠ error).
- Loading + idle placeholders; sample query kept.
- **Presets:** SELECT LIMIT 10, LIMIT 5, `rdf:type` pattern.
- HonestyChip **Partial** — local `execute_sparql_query` → hex slot dump, not full IRI SPARQL.

### U5-B polish details

- **Chemistry:** phase machine Idle/Loading/Ready/Error; never shows zeros on host fail; SMILES presets (ethanol/benzene/aspirin).
- **Physics:** HonestyChip + sample particle position metrics from certification result (existing Forge path retained).
- **Portfolio:** removed mock `execute_computational_vm` payload; **Run Monte Carlo VaR** → real host numbers; local weighted return/σ/Sharpe labelled as local.

### Measured results

- `cargo check -p webizen-studio` — **Finished OK** (exit 0; warnings only; no errors in U5 files).
- Runtime desktop dogfood of the three invokes not exercised this session.

### ⚑ Where I need the human

- Desktop: open SPARQL → empty graph empty-state; Chemistry CCO → non-zero MW; Portfolio → Run MC → VaR dollars; Physics → Certify step → backend/error metrics.

### Next step

- U6 palette/docs (other track); human dogfood closes U5 acceptance.

---

## 2026-07-18 — U4-C Library perception UI

**Status:** done (UI only; existing host commands; no `commands/mod.rs`)

**What was built**
- **`library_panel.rs`:** **Seed perception library** button + status text; prefers host `library_seed_perception_assets`, falls back to `wellfair_seed_perception_library`.
- After seed: switches to Software + **Perception** filter; lists model/ontology/`computer_vision` rows from host JSON (honest empty state if none).
- Compact **computer_vision** summary strip when Perception filter is on.
- **Honesty chips** (Catalogue strip): models · ontologies · perception/computer_vision — Ready (Present) vs Partial vs Unavailable derived from actual shelf rows (seed-reference → Partial; CV specialized-lib rows → Ready; ontologies catalogued → Partial).
- Perception / computer_vision counts in stats chips; existing Library features (ingest, vault secret, QApps, legislation, commons) preserved.
- **`host_client.rs`:** `seed_perception_library()` wrapper only (Library sibling path).
- **Commands lock honoured** — no new Tauri commands.

**Measured results**
- `cargo check -p webizen-studio` — **Finished OK** (warnings only).
- Runtime seed not exercised this session (needs desktop + storage/vault).

**⚑ Where I need the human**
- Desktop dogfood: Seed perception library → expect computer_vision rows under Perception filter; chips show models Partial / ontologies Partial / perception Ready when CV rows present.

**Next step**
- Human dogfood closes U4-C acceptance; U4-B vision workbench is a separate track.

**Note:** Minimal format-string compile fix in `portfolio_analyzer.rs` (`:,.2` invalid in Rust/rsx) so studio check could finish — owned by U5-B; not a U4-C feature change.

---

## 2026-07-18 — U3-A + U3-B agent MCP tool loop + allowlist

**Status:** done (code + unit tests); runtime desktop dogfood = human gate

**What was built**
- **`qualia-client-core/src/mcp_tool_loop.rs`:** principal-gated MCP path — pure `evaluate_tool_gate`, `mcp_list_local_tools` (`tools/list`), `mcp_call_tool_gated` / `_for_agent` (Deny / allowlist reject never dispatch), `agent_set_allowed_mcp_tools`, `ensure_safe_tool_allowlist` seeds `list_capabilities` + `computer_vision` when empty. In-process only via `handle_jsonrpc_message`.
- **API + Tauri:** `mcp_list_local_tools`, `mcp_call_tool_gated`, `agent_set_allowed_mcp_tools`, `mcp_ensure_safe_tool_allowlist` in `api.rs` + `commands/mod.rs` (handler registration).
- **Talk UI:** `tool_use_card.rs` — tool dropdown, JSON args (`{}` / `{"op":"list"}` for CV), Propose → **Permit** / **Deny**, result panel. Deny never invokes. Embedded in `connect_chat` sidebar under active agent.
- **Allowlist editor:** `agent_config.rs` rewritten from mock sliders → roster agent select + multi-select tools + save + seed-safe; honesty chip Partial.
- **Tests:** gate deny principal, allowlist reject, allowlist permit + real `list_capabilities` dispatch, tools/list includes golden tools, seed/set allowlist roundtrip (tempdir).

**Measured results**
- Unit tests: `cargo test -p qualia-client-core --lib mcp_tool_loop` → **7 passed**, 0 failed (0.04s).
- `cargo check -p qualia-client-core -p webizen-desktop -p webizen-studio` → **Finished OK** (EXIT=0; warnings only, none blocking).

**Dogfood (Permit `list_capabilities`)**
1. Rebuild desktop; open Talk → Chat.
2. Sidebar **MCP tools** card: seed runs on open (empty allowlist → safe tools).
3. Tool = `list_capabilities`, args = `{}` → **Propose tool** → **Permit**.
4. Result card should show capability catalogue JSON text.
5. **Deny** on a second propose: status “Denied… was not invoked”, no result body change from MCP.
6. Agent Config pane: uncheck tools / save → Permit of unlisted tool → `not on allowlist`.

**⚑ Where I need the human**
- Desktop rebuild + one Permit of `list_capabilities` on real host.
- Decide whether default local agent should ship with safe tools pre-seeded in `default_local_agent` (currently empty + seed-on-first-tool-UI-open).

**Next step**
- Optional: model-proposed tool cards (same gate); U2-B browser agent after dogfood.

---

## 2026-07-18 — U4-A 10D Browser polish

**Status:** done (UI polish only; no new Tauri commands)

**What was built**
- **`ten_d_browser.rs`:** Clear empty states for (a) no containers after scan, (b) vision recon filter empty, (c) not-yet-scanned / loading; each with plain language + next actions.
- Load / temporal scrub: parse failures and host `Err` surface in **banner + inline** vision panel; citable Forbid humanized (`missing_provenance` → explicit FORBID copy). On deny, clear prior load/scrub success (no silent success).
- Category list UI: **ANATOMY** vs **VISION** vs LIBRARY/OTHER badges + accent colours; vision entries without provenance show `NO PROV`.
- Honesty chips retained/improved: Partial list/load/scrub; Ready citable fails closed.
- Auto-scan on mount; citable banner when toggle on; “No provenance sidecar” callout on inspect.
- Manual stub: [`docs/manuals/vision-10d-ui.md`](../manuals/vision-10d-ui.md).
- **Commands lock honoured** — no `commands/mod.rs` edits.

**Measured results**
- `cargo check -p webizen-studio` — **Finished OK** (warnings only; no errors in `ten_d_browser`).
- Runtime dogfood of citable Forbid not exercised this session (needs desktop + unattested recon file).

**⚑ Where I need the human**
- Desktop dogfood: empty storage empty-state; vision filter; load unattested with Citable on → must show FORBID text; load attested → “Loaded (policy passed)”.

**Next step**
- U4-B vision workbench (separate track); human dogfood closes U4-A acceptance.

---

## 2026-07-18 — U2-A Browser dogfood + trust/cookies honesty

**Status:** done (code + docs honesty polish); runtime dogfood = human gate

**What was built**
- Walked B0–B3 against code; rewrote [`webizen-browser-dogfood-notes.md`](./webizen-browser-dogfood-notes.md) with Present/Partial + human dogfood list.
- **`chrome.html`:** Trust honesty + live `browser_cert_override_status` line (**not claimed product-active**); cookies empty-state; Servo preference wording on status bar.
- **`browser_panes.rs`:** Removed “unless override is active” ambiguity; Present/Partial/Experimental footer; cookies default empty + coverage.
- **`cookies.rs`:** Local scheme honest N/A coverage + empty arrays.
- Operator stub: [`docs/manuals/webizen-browser-operator.md`](../manuals/webizen-browser-operator.md).
- No `commands/mod.rs` edits; no PEMs invented; WebView remains default.

**Measured results**
- Not runtime-measured (no desktop rebuild/site load this session).
- `cargo check -p webizen-desktop -p webizen-studio` — **Finished OK** (warnings only; no errors).

**⚑ Where I need the human**
- Rebuild desktop; navigate DuckDuckGo/Google; Trust DID badge round-trip; Cookies Refresh after https; Servo banner still WebView-active.

**Next step**
- U2-B browser agent shell UI (after U3 tool-loop design preferred); human dogfood closes B0–B3 runtime boxes.

---

## 2026-07-17 — Plan authored (orchestrator)

**Status:** done (planning only)

**What was built**
- Sub-agent implementation plan with waves U1–U6, exclusive paths, commands lock rules, spawn prompts.
- Comprehensive UI plan updated: Ollama optional honesty; audio + non-MCP external agents deferred.
- This progress log created.

**Measured results**
- Not measured (no code yet).

**⚑ Where I need the human**
- Confirm spawn order (recommended: U1-A + U1-C + U1-D in parallel).
- For runtime dogfood later: preferred Local GGUF path/size and whether download_model is OK on this machine.

**Next step**
- CLAIM + spawn Wave U1 tracks when principal says execute.

---

## 2026-07-17 — U1-A/C/D + audio catalogue plan

**Status:** done (code + docs); compile verification in progress

**What was built**
- **Audio plan:** `docs/plans/audio-algorithms-catalogue-gap-plan-2026.md` — Essentia-class domains + 2026 generative/AQA taxonomy, gap-mapped to `qualia-audio` / core `audio/`, phases AU0–AU10 (UI still first).
- **U1-A:** `wellfair_get_llm_telemetry` uses live engine VRAM/lifecycle + backend label; `tokens_per_sec` only from last measured turn (`record_last_decode_tok_s` in model_lifecycle + finalize_success_result). Force lifecycle phase returns honest Err. `llm_harness` shows backend/lifecycle/source.
- **U1-C:** `honesty_chip.rs` + Talk header + 10D browser chips.
- **U1-D:** `docs/manuals/webizen-ui-architecture.md`, `webizen-ui-event-catalogue.md`, `talk-and-agents-ui.md`.

**Measured results**
- tok/s: process-local after a real chat turn only (not measured in this session without a live model).
- Compile: `cargo check -p webizen-studio -p webizen-desktop` — Finished OK (studio + desktop).

**⚑ Where I need the human**
- Preferred Local GGUF for dogfood when ready.
- When to open **AU1** (streaming DSP) after UI waves.

**Next step**
- U1-B conduct/deny banner; U2 browser dogfood; then U3 tool loop.

---

## 2026-07-18 — U1-B Conduct / deny visibility

**Status:** done

**What was built**
- **New** `crates/webizen-studio/src/components/conduct_banner.rs`:
  - `ConductKind` { GateDeny, ShieldAlert, InferenceBlock } with distinct red/amber colours.
  - `ConductNotice` + parsers: `notice_from_chat_result`, `notice_from_chat_done`, `notice_from_conduct_violation`.
  - `ConductBanner` component: full-width `role="alert"` strip, reason text, Dismiss (reappears on next deny).
  - Unit tests for shield priority, gate language, silent-fail closed, clean commit → None, nested chat-done, conduct-violation payload.
- **Wire** `connect_chat.rs`:
  - `conduct: Signal<Option<ConductNotice>>`.
  - `send_chat_turn` sets banner from invoke `block_reason` / `shield_alert` / uncommitted fallback; also on invoke/send errors.
  - `chat-done` listener surfaces nested `result` fields (not only clear stream).
  - Optional `conduct-violation` listener (host may not emit yet; UI ready).
  - Banner mounted under header; status line still mirrors short “No reply: …”.
- **Register** `pub mod conduct_banner` in `components/mod.rs`.
- **No** edits to `commands/mod.rs` (existing fields only).

**Measured results**
- Runtime dogfood (real Deny / Shield): **not measured** (no live gate exercise this session).
- `cargo test -p webizen-studio --bin webizen-studio conduct_banner` → **6 passed**, 0 failed.
- `cargo check -p webizen-studio` → **Finished** OK (PowerShell may exit 1 on stderr warnings).

**⚑ Where I need the human**
- none this step.
- Optional later: host emit of `conduct-violation` event (queued for commands owner if needed).

**Next step**
- U2 browser dogfood / U3 tool loop; dogfood a real block_reason when a Local model + gate path is available.
