# Comprehensive UI / GUI / Webizen Plan (2026)

**Status:** living plan — authored 2026-07-17  
**Branch target:** `0.0.25+`  
**Canonical tree:** `C:\Projects\qualia-27062026` only  
**Audience:** principal (Timothy), UI/desktop agents, Studio agents, browser swarm, docs agents  

This document is the **unifying UI plan** for QualiaDB / Webizen. It covers:

1. **All shell surfaces** already in-tree (desktop, studio, portal, browser chrome).  
2. **All engine capabilities** that must be *reachable, honest, and governable* from UI.  
3. **Webizen-browser** as a client onto QualiaDB (not a second Chromium).  
4. **Developer documentation** required so agents and humans can implement without inventing parallel stacks.

It does **not** replace specialist plans (browser-and-trust, vision-10d, desktop master plan, anatomy, Talk UX). It **indexes** them and assigns each capability to a UI surface, tier, and honesty bar.

---

## 0. Principles (immovable)

| # | Principle | Consequence for UI |
|---|-----------|-------------------|
| 1 | **Client onto QualiaDB**, not a V8 document viewer | Chrome is one edge; graph, inference, modalities, `.10d`, rights, MCP are first-class |
| 2 | **One GPU story** | `shared_gpu` / wgpu / Forge / PortalGpu — no second adapter for product chrome |
| 3 | **Rights / deontic fail-closed** | Biosense, citable `.10d`, agent tools, vault — UI must surface Permit/Forbid and never hide gates |
| 4 | **Honesty over polish** | Present / Partial / Missing / FeatureDisabled labels in UI match registry + CAPABILITY_DESCRIPTORS |
| 5 | **Anti-monolith** | Panes and commands stay modular; no 5k-line single “UI god object” |
| 6 | **Native excellence; Ollama optional** | Default / excellence path = Local Qualia GGUF. Ollama is an **opt-in** harness (Settings) for bridge use — never marketed as the Qualia engine. Hybrid / Remote remain principal-gated. |
| 7 | **Canonical tree** | All UI work in this repo; no worktree fork of the product |
| 8 | **Runtime-proven bar** | Compile-green is not done; desktop/browser paths need dogfood notes |

---

## 1. Current product surfaces (as-built)

```text
┌─────────────────────────────────────────────────────────────────────────┐
│  webizen-desktop (Tauri 2)                                              │
│  · Shell: tabs, menu, tray, settings server                             │
│  · Talk-as-home, Wellfair, Library, QApps                               │
│  · Browser: chrome.html + browser/* (engine preference, trust, cookies) │
│  · Native GPU surface hooks (upload_gpu_10d, mesh, …)                   │
│  · 300+ Tauri commands → client-core / core-db                          │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ host API / events
┌───────────────────────────────▼─────────────────────────────────────────┐
│  webizen-studio (Dioxus)                                                │
│  · Social hub, Talk, domains, 10D browser, anatomy, QApp library        │
│  · Pane registry (SolidOS-style dispatcher)                             │
│  · Hundreds of academic *_qapp stubs + real product panes               │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│  Engine (qualia-core-db + client-core + specialized_libs + vision)      │
│  · Graph, WAL, SPARQL, modalities, MCP                                  │
│  · Inference (Phase-8, Sentinel), Forge, render/.10d                    │
│  · computer_vision, audio, geometry, privacy, …                         │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│  Portal / WASM (profiles)                                               │
│  · Ontology MCP · portal WebGPU · anatomy packs                         │
│  · Size-gated: no full computer_vision in ontology wasm                 │
└─────────────────────────────────────────────────────────────────────────┘
```

| Crate | Role | UI tech |
|-------|------|---------|
| `webizen-desktop` | OS app shell, tray, native webview browser, command registry | Tauri 2 + WebView2/WebKit |
| `webizen-studio` | Product panes, QApps, 10D browser, Talk | Dioxus (+ HTML chrome) |
| `docs/playground` | Public demos (anatomy, etc.) | HTML + WASM portal |
| `webizen-lite-wasm` / portal profiles | Browser-local kernels | WASM |

**Specialist plans (do not fork):**

| Plan | Owns |
|------|------|
| `webizen-desktop-master-implementation-plan.md` | Desktop “getting it all done” spine |
| `webizen-browser-and-trust.md` + advisor briefing + swarms | Browser / trust / agent |
| `vision-10d-browser-excellence-programme-2026.md` | Vision → GPU → `.10d` → browse |
| `agentic-chat-workspace.md` / Talk UX | Chat, agents, CML |
| `audio-algorithms-catalogue-gap-plan-2026.md` | Audio MIR/generative algorithm gaps (execute after UI waves) |
| `wellfair-webizen-desktop/*` | Wellfair / vault / QApp policy |
| `first-run-setup-and-inforg-onboarding.md` | First-run |
| `servo-experimental.md` | Optional engine preference (not product default) |

---

## 2. Product shell (locked) and UX improvements on that shell

**Product shell:** Tauri 2 + Dioxus + native webview (browser) + engine `shared_gpu` / wgpu / Forge. Do not replace this stack with a second UI framework or second GPU adapter.

Qualia product UX improvements (on the existing shell):

1. **Unified action / command palette** — every shell action, QApp open, browser command, MCP tool (allowlisted) discoverable.  
2. **Typed app event bus** — replace ad-hoc string emits with a documented event catalogue.  
3. **Frame / latency budget for Studio panes** — especially 10D scrub and Talk stream.  
4. **Dense virtualized lists** — agent logs, SPARQL results, WAL browser, vision detections.  
5. **UI runtime harness** — scripted: launch → activate model → prompt → assert tokens (desktop master plan §2).  
6. **Keyboard-first paths** for Talk, Library, 10D browser.

---

## 3. Engine capability → UI incorporation matrix

Every major engine area must appear in at least one **primary** surface and one **honesty** path (status strip / Library / MCP list).

### 3.1 Identity, vault, first-run

| Capability | Engine locus | Primary UI | Secondary | Status (honest) |
|------------|--------------|------------|-----------|-----------------|
| DID / profiles | identity, key vault | First-run + Settings | Reception | Partial–Present |
| Sanctuary / vault | sanctuary crypto | Wellfair / Sanctuary CTAs | Settings | Partial |
| Onboarding inforg | first-run plan | Wizard | docs | In progress |
| Guardianship / multi-party | deontic, suspended tx | Care / legal QApps (when real) | Graph | Engine ahead of UI |

### 3.2 Talk, agents, inference

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| Local LLM decode | llm_agent, gguf, Forge | Talk | Model Hub | Real, runtime dogfood still required — **excellence path** |
| Ollama (optional harness) | ollama_harness, InferenceBackendSettings | Settings (opt-in) | Talk when selected | Present as optional bridge; not the engine |
| Hybrid / Remote backends | AgentBackend | Settings + agent defaults | — | Partial |
| Model lifecycle / thermal | orchestrator | Telemetry HUD (must be **live**, not static) | Dev console | HUD often mock — **fix required** |
| Sentinel / DenyRollback | Phase-8 rings | Status / conduct log | — | Under-surfaced |
| Agent roster | agent_registry | People / agents | MCP allowlist | Partial |
| Agent tool loop over MCP | MCP + allowlist | Talk tool use UI | — | **Missing** |
| CML context | cml_context | Talk context panel | Library | Partial |
| Chat jobs / schedule | local_job_scheduler | Talk / Projects | — | Partial |

### 3.3 Graph, SPARQL, modalities, governance

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| Graph query / SPARQL | sparql_library, daemon 4242 | Knowledge pane / Studio | Dev tools | Partial |
| N3 / SHACL | n3_parser, shacl_compiler | Forms (SHACL), ontology workbench | MCP | Partial |
| Deontic / epistemic / LTL / ASP | modalities | Governance / rights panes | Conduct log | Engine strong; UI thin |
| Rights ontology / intent gate | orchestrate_inference | Always-on indicator when Deny | Audit | Must be visible |
| WAL / provenance browser | wal, provenance | Library + timeline | — | Partial |
| Six Vectors cost (when wired) | intent logging | Session cost strip | — | Spec’d, partial product |

### 3.4 Browser / trust / network

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| Navigate URL | Tauri webview | Browser chrome omnibox | — | Present (P0+) |
| Engine preference (WebView / Servo flag) | browser/engine | Engine select + honest banner | — | Present experimental |
| Trust store / roots | trust plans | Trust panel + suggested import | — | Partial |
| Cookies transparency | cookie design | Cookies side panel | Graph summary | Partial |
| Cert override | spike | **Must not over-claim** | — | Spike only |
| Browser agent | browser plan | Agent side-panel | Talk | Missing–Partial |
| `window.webizen.*` capability API | design in browser-and-trust | Page + docs | — | **Missing** |
| eBPF / platform filter | ebpf_filter | Advanced network settings | — | Engine; UI absent |
| P2P / WebTorrent | p2p, seeder | Library / seed UI | — | Partial |

### 3.5 Render, 10D, vision, audio

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| `.10d` browse / inspect | container_10d, commands | **10D Browser** (Studio) | Desktop commands | Present + vision path |
| Vision recon list/load/scrub/citable | vision_10d_*, barrier | 10D Browser vision toggles | MCP computer_vision | Present |
| Volumetric / PortalGpu | render | Anatomy / native surface | Playground | Present |
| σ spectral + acoustic | spectral, acoustic | 10D paint + future hear | — | Partial UI |
| Computer vision classical / SR | specialized_libs::computer_vision | Vision workbench (needed) | MCP | Engine Present; workbench Partial |
| Biosense / PAD / rPPG | qualia-vision product | Wellfair self-monitor (consent-first) | — | Engine strong; UX gated |
| Audio / speech | qualia-audio | Listen / Talk audio | — | Partial |
| Anatomy packs | anatomy_pack | Anatomy QApp + playground | Release assets | Present |

### 3.6 Domain libraries & MCP

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| CAPABILITY_DESCRIPTORS catalogue | lib.rs | Library → Software seed | `list_capabilities` MCP | Present backend |
| MCP tools (60+) | mcp_server | Agent allowlist + dev MCP console | — | Tools real; **agent loop UI missing** |
| computer_vision MCP | mcp tool_impls/vision | Dev + agent tools | — | Present |
| Geometry / solvers / finance / medical | specialized_libs | Computational pane category | QApps | Engine > UI |
| Academic QApp stubs | `*_qapp.rs` (hundreds) | QApp manager / palette | — | Catalogued; most **scaffold** — honesty required |

### 3.7 Storage, platform, packaging

| Capability | Engine locus | Primary UI | Secondary | Status |
|------------|--------------|------------|-----------|--------|
| Local storage / Index | storage_driver | Settings paths | — | Partial |
| Bundled ontologies | bundled_ontologies | Library / Reception | — | Present |
| Updates | Tauri updater | Settings | — | Partial |
| Telemetry file logging | system_telemetry | Dev settings | — | Present |

---

## 4. Target information architecture

### 4.1 User-facing IA (human principal)

```text
Talk (home)
  ├─ Chat / agents / tools (gated)
  ├─ People / social
  ├─ Projects / work board
  └─ Mail (local) / Reception

Library
  ├─ Software (models, specialized libs, QApps catalogue)
  ├─ Ontologies
  ├─ Media / perception assets
  └─ .10d / geometry assets

Browser
  ├─ Omnibox + tabs (native webview)
  ├─ Trust / cookies / engine banner
  └─ Browser agent (governed)

Create / Studio
  ├─ QApp Studio canvas + pane palette
  ├─ 10D Browser (anatomy + vision recon)
  ├─ Knowledge (SPARQL / graph)
  └─ Computational / scientific panes

Wellfair / Self
  ├─ Vault / Sanctuary
  ├─ Self-monitor (biosense, consent)
  └─ Care / rights (when ready)

Settings / Advanced
  ├─ Models / inference mode
  ├─ Network / identity / domains
  └─ Developer (MCP, WAL, honesty flags)
```

### 4.2 Developer-facing IA

```text
docs/
  manuals/     — operator + capability manuals (existing)
  plans/       — this plan + specialist plans
  playground/  — public demos

crates/
  webizen-desktop/   — shell commands, browser, native surfaces
  webizen-studio/    — Dioxus panes / QApps
  qualia-client-core/ — host APIs used by UI
  qualia-core-db/     — engine (no UI chrome)
```

---

## 5. Surface contracts (how UI must talk to the engine)

### 5.1 Command / invoke layer

- **Desktop:** Tauri `#[tauri::command]` in `commands/mod.rs` (append carefully; one owner per swarm).  
- **Studio:** `invoke_json("command_name", json)` — command must exist on host.  
- **Never** invent a second HTTP LLM API. Inference goes through existing client-core paths.

### 5.2 Event layer (typed, documented catalogue)

Document a **stable event catalogue** (names + payload schemas), e.g.:

| Event | Direction | Purpose |
|-------|-----------|---------|
| `chat-token` / `chat-done` | host → UI | Stream tokens |
| `shell-navigate` | host → UI | Route change |
| `render-preview-ready` | host → UI | GPU preview |
| `system-telemetry` | host → UI | Live VRAM/thermal |
| `conduct-violation` | host → UI | Deontic deny (must not be silent) |

**Deliverable:** `docs/manuals/webizen-ui-event-catalogue.md` (to write).

### 5.3 Honesty / status component

Every product surface that claims a capability must show one of:

- **Ready** (Present + tested path)  
- **Partial** (works with caveats)  
- **Needs model/weights** (WeightAbsent / AdapterMissing)  
- **Needs consent** (biosense)  
- **Unavailable on this profile** (WASM / FeatureDisabled)  
- **Scaffold** (academic QApp stub)

Source of truth: capability registries + CAPABILITY_DESCRIPTORS + vision D1–D9 registry.

### 5.4 Rights barrier pattern (mandatory)

For biosense, citable `.10d`, vault, agent tools:

```text
UI request → host command → evaluate_processing_act / vision_10d_barrier / vault gate
         → Permit: execute
         → Forbid: structured error + audit line (no silent no-op)
```

---

## 6. Webizen-browser programme (within this UI plan)

Aligned with `webizen-browser-and-trust.md` §0.5:

| Phase | UI deliverable | Engine |
|-------|----------------|--------|
| **B0** | Native webview navigation works (no iframe for main web) | Tauri webview |
| **B1** | Chrome: omnibox, tabs, back/forward, engine banner | browser/* |
| **B2** | Trust panel + suggested roots (honest empty) | trust store |
| **B3** | Cookies side panel | cookie jar |
| **B4** | Browser agent (governed tools only) | agent + MCP allowlist |
| **B5** | `window.webizen.*` tiered API design + docs | capability API |
| **B6** | Semantic overlay (graph highlights, provenance) | sparql + render |
| **B7** | Dogfood checklist green | `webizen-browser-ready-checklist.md` |

**Servo:** optional experimental preference only (`servo-experimental.md`) — never break WebView default.

---

## 7. Vision / 10D UI programme (within this UI plan)

Aligned with vision-10d excellence programme:

| UI item | Command / pane | Notes |
|---------|----------------|-------|
| List anatomy + other `.10d` | `browse_10d_containers` | Category labels |
| List vision recon | `browse_vision_10d` | `vision_geometry/` |
| Load + paint summary | `load_vision_10d` | citable flag |
| Temporal scrub | `scrub_vision_10d_paint` | node `t` |
| Native GPU paint | `upload_gpu_10d` / load surface | shared_gpu |
| Vision workbench (needed) | Studio pane + MCP | SR, mesh, biosense entry |
| Library seed | `seed_perception_library` | includes computer_vision rows |

---

## 8. Talk / agent UI programme

| UI item | Priority | Note |
|---------|----------|------|
| Live model telemetry | **P0** | Replace static mock HUD |
| Tool-use UI (propose → permit → result) | **P0** | Unblocks MCP from agent |
| Conduct / deny visibility | **P0** | Fidelity to principal |
| Agent roster + allowlists | P1 | |
| Jobs board | P1 | |
| CML context inspector | P1 | |

---

## 9. Studio / QApp programme

| UI item | Priority | Note |
|---------|----------|------|
| Pane registry honesty | P0 | Scaffold QApps labelled |
| Computational panes wired to specialized_libs | P1 | Not mock solvers |
| Knowledge pane (SPARQL) | P1 | |
| QApp Studio canvas | P1 | |
| Academic catalogue browse | P2 | Search + “stub” filter |

---

## 10. Developer documentation plan

### 10.1 Documents to maintain (create if missing)

| Doc | Purpose |
|-----|---------|
| **This plan** | Unifying UI architecture |
| `docs/manuals/webizen-ui-architecture.md` | Surfaces, data flow, non-goals |
| `docs/manuals/webizen-ui-event-catalogue.md` | Events + payloads |
| `docs/manuals/webizen-command-index.md` | Generated or curated list of Tauri commands by domain |
| `docs/manuals/webizen-pane-catalogue.md` | Pane registry + honesty |
| `docs/manuals/webizen-browser-operator.md` | Trust, cookies, engine, dogfood |
| `docs/manuals/vision-10d-ui.md` | Load/scrub/citable + GPU paint |
| `docs/manuals/talk-and-agents-ui.md` | Inference, tools, consent |
| `docs/manuals/capability-to-ui-map.md` | Auto-derived from CAPABILITY_DESCRIPTORS when possible |
| `docs/plans/*` specialist plans | Depth; linked from this doc only |

### 10.2 Agent rules (for implementers)

1. Read this plan + the **one** specialist plan for your lane.  
2. CLAIM exclusive paths in `coordination/NOTICES.md`.  
3. Prefer **extend** Studio pane / desktop command over new crates.  
4. No second GPU adapter for product chrome.  
5. Ollama stays **optional** (Settings); never default; never documented as the Qualia engine.  
6. Every new user-facing capability updates honesty labels + progress log.  
7. Runtime dogfood note when path is product-critical.

### 10.3 Doc generation (optional automation)

- Script: extract `#[tauri::command]` names → markdown table.  
- Script: extract `builtin_pane_definitions` → pane catalogue.  
- Script: CAPABILITY_DESCRIPTORS → “MCP tools / UI surfaces” matrix.

---

## 11. Phased roadmap (UI-centric)

### Phase U0 — Orientation (done when this doc is accepted)

- [x] This plan written  
- [x] Sub-agent implementation plan: [`webizen-ui-implementation-subagents-2026.md`](./webizen-ui-implementation-subagents-2026.md)  
- [ ] Principal skim + any IA corrections  

### Phase U1 — Honesty & telemetry (1–2 sessions)

- Live LLM HUD (no static mock)  
- Conduct/deny toast or panel  
- Capability honesty strip on Browser / 10D / Talk  

### Phase U2 — Browser product cut (per browser plan)

- Trust + cookies complete dogfood  
- Ready checklist green  
- Browser agent v0 (allowlisted tools only)  

### Phase U3 — Agent tool loop UI

- Propose tool call → principal permit → result card  
- MCP allowlist editor  

### Phase U4 — Vision / 10D product cut

- Vision workbench pane (SR, mesh handoff entry)  
- 10D scrub + load polished  
- Library shows vision + computer_vision seeds  

### Phase U5 — Knowledge & compute

- SPARQL workbench usable  
- 3–5 computational panes wired to real specialized_libs (not stubs)  

### Phase U6 — Unification polish (palette, events, density)

- Command palette  
- Event catalogue implemented  
- Virtualized dense lists  
- UI runtime harness  

---

## 12. Acceptance criteria (programme-level)

A reviewer answers **yes** when:

1. **Talk** streams real tokens from a local model with live telemetry.  
2. **Browser** loads major sites in native webview; trust/cookies honest.  
3. **10D Browser** lists anatomy + vision recon; load + scrub work; citable fails closed.  
4. **Library** seeds models, ontologies, computer_vision lib rows.  
5. **Agent** can run at least one allowlisted MCP tool with visible permit path.  
6. **No** product claim of Servo as default (WebView remains default).  
7. **Developer docs** listed in §10 exist at least as stubs with links to this plan.  
8. **Registries** match UI labels (no false Present).  

---

## 13. Workstreams & ownership (swarm-friendly)

| Stream | Primary paths | Collides with |
|--------|---------------|---------------|
| **S-Shell** | webizen-desktop shell, menu, tabs | — |
| **S-Browser** | desktop browser/*, chrome.html, trust/cookies | S-Shell commands appends |
| **S-Talk** | studio social/talk, chat_inference | S-Agent |
| **S-Agent** | agent UI, MCP allowlist, tool cards | S-Talk |
| **S-10D** | ten_d_browser, native_surface GPU | vision-10d engine |
| **S-Vision-UI** | vision workbench pane | computer_vision, biosense consent |
| **S-Library** | perception_catalog, Library UI | — |
| **S-Knowledge** | SPARQL / ontology panes | graph engine |
| **S-Docs** | docs/manuals webizen-ui-* | — |

CLAIM rules: one stream owns `commands/mod.rs` appends at a time.

---

## 14. Explicit non-goals

- Replacing Tauri 2 + Dioxus as the product shell.  
- Full Chromium/Servo as the only engine (WebView remains default).  
- Shipping hundreds of academic QApps as “complete” without honesty.  
- Putting full computer_vision into ontology WASM.  
- Silent biometrics or silent agent tool execution.  
- Second GPU adapter for UI chrome.  
- **Audio product cut** in this UI wave (deferred; see auditory plans).  
- **Non-MCP deep integrations** for external agents (Grok/Claude/etc.) — later; MCP allowlist + Remote-MCP remain the in-scope agent path.  

---

## 15. Next concrete sessions (suggested order)

Execute via **[`webizen-ui-implementation-subagents-2026.md`](./webizen-ui-implementation-subagents-2026.md)**:

1. **U1** parallel: live telemetry (commands lock) + honesty strip + docs stubs.  
2. **U2** browser dogfood against ready checklist.  
3. **U3** agent tool card + one MCP tool end-to-end (not external agent SDKs).  
4. **U4** vision/10D product cut.  
5. **U5–U6** knowledge/compute + palette/harness.  
6. **Hold:** audio programme; non-MCP external agent integrations.

---

## 16. Change log

| Date | Note |
|------|------|
| 2026-07-17 | Initial comprehensive UI plan; capability→UI matrix; doc + phase plan |
| 2026-07-17 | Ollama principle corrected (optional harness); sub-agent plan linked; audio + non-MCP agents deferred from UI waves |
| 2026-07-18 | Removed third-party UI-framework comparison material; product shell = Tauri/Dioxus only |

---

*End of plan. Excellence = every Qualia capability is reachable, rights-gated, and honestly labelled in the Webizen UI — without abandoning the Tauri/Dioxus/wgpu architecture that already exists.*
