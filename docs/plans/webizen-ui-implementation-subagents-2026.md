# Webizen UI — Sub-Agent Implementation Plan (2026)

**Status:** ready to execute  
**Branch:** `0.0.25` only · **Tree:** `C:\Projects\qualia-27062026` only  
**Parent plan:** [`comprehensive-ui-gui-webizen-plan-2026.md`](./comprehensive-ui-gui-webizen-plan-2026.md)  
**Desktop spine:** [`webizen-desktop-master-implementation-plan.md`](./webizen-desktop-master-implementation-plan.md)  
**Authored:** 2026-07-17  

This plan turns the comprehensive UI plan into **parallel, CLAIM-isolated sub-agent tracks**.  
**UI first.** Audio programme and non-MCP external agent integrations are **deferred** (see §0.3).

---

## 0. Principal direction (binding)

| Priority | Decision |
|----------|----------|
| **1 — UI product cut** | Ship Talk / Browser / Library / 10D / honesty / agent-tool loop so the product is usable as a client onto QualiaDB. |
| **2 — Native inference excellence** | Default and excellence path = **Local GGUF → Qualia engine** (Phase-8, Sentinel, `shared_gpu`). Live telemetry, real model activation, no cosmetic mocks. |
| **3 — Optional Ollama** | Already wired as **opt-in** backend (`InferenceBackendSettings::Ollama`, harness, Settings panel). Keep it honest; do **not** present Ollama as the Qualia engine. No new Ollama product narrative. |
| **Deferred** | Full **audio** programme (listen / speech product cut). **Non-MCP** deep integrations for external agents (Grok, Claude, etc.) beyond existing Remote-MCP / allowlist paths. **GPUI** as product shell. |

### 0.1 Immovable (from AGENTS.md / Claude.md)

- Canonical tree only; CLAIM before edit; `commands/mod.rs` **one owner at a time**.  
- One GPU story (`shared_gpu` / wgpu / Forge).  
- Rights / deontic **fail-closed** in UI.  
- Honesty labels: Present / Partial / Scaffold / Needs model / Needs consent / FeatureDisabled.  
- Runtime-proven bar for product-critical paths (compile-green ≠ done).  
- Fidelity to principal: no mock standing in for a claimed capability.

### 0.2 Definition of done (UI programme)

A reviewer answers **yes** when:

1. **Talk** streams tokens from **Local** Qualia path with **live** HUD (not static mock).  
2. **Ollama** remains selectable in Settings; probe/list/generate work; copy says optional bridge.  
3. **Browser** navigates major sites; trust + cookies panels honest; ready checklist items for B0–B3 green.  
4. **10D Browser** lists anatomy + vision recon; load + scrub; citable fails closed.  
5. **Agent** can execute **one** allowlisted MCP tool with propose → permit → result UI.  
6. **Library** shows models / ontologies / perception / computer_vision seeds with honesty labels.  
7. **Developer manuals** in §7 exist (stubs OK if linked and accurate).  
8. Progress logged; NOTICES CLAIM/RELEASE complete.

### 0.3 Explicitly out of scope for this plan

| Out | Where it lives later |
|-----|----------------------|
| Audio excellence / Listen product | `native-auditory-*`, `audio-adrs/` |
| Non-MCP “agent as peer” deep integrations | future protocol plan; not U1–U6 |
| Servo as default engine | never; experimental flag only |
| gpui-ce product shell | U7 principal-only gate |
| Full computer_vision in ontology WASM | never |

---

## 1. How to run sub-agents (orchestration rules)

### 1.1 Before any code

1. `cd C:\Projects\qualia-27062026`  
2. Read this plan + the **one** specialist plan for the track.  
3. Read tail of `coordination/NOTICES.md` for collisions.  
4. Append **CLAIM** with exclusive paths.  
5. Work only on claimed paths. **Do not** invent parallel crates or HTTP LLM servers.

### 1.2 Parallelism map (safe concurrent tracks)

```text
Wave U1 (parallel OK if commands/mod.rs owned by U1-A only for telemetry appends)
  U1-A  Live telemetry + lifecycle honesty     [commands + studio HUD]
  U1-B  Conduct/deny visibility                 [studio Talk + event listeners]
  U1-C  Capability honesty strip                [studio components + registry labels]
  U1-D  Docs stubs (event catalogue, arch)      [docs/manuals only]

Wave U2 (after U1-A releases commands if U2 needs new commands)
  U2-A  Browser dogfood + checklist             [chrome.html, browser_panes, docs]
  U2-B  Browser agent v0 shell UI               [chrome + agent panel; needs allowlist API]

Wave U3 (serial with U2-B if shared agent state)
  U3-A  Tool-use loop UI (propose/permit/result)
  U3-B  MCP allowlist editor

Wave U4 (parallel with U3 if no commands collision)
  U4-A  10D Browser polish (load/scrub/citable UX)
  U4-B  Vision workbench pane (SR + MCP entry + consent)
  U4-C  Library perception seeds UI

Wave U5
  U5-A  SPARQL / Knowledge workbench
  U5-B  3 computational panes → real specialized_libs

Wave U6
  U6-A  Command palette
  U6-B  Event catalogue wiring + virtualized lists
  U6-C  UI runtime harness (scripted dogfood notes)
```

**Hard rule:** Only **one** track may append to  
`crates/webizen-desktop/src/commands/mod.rs` at a time.  
Orchestrator assigns that lock explicitly (see wave tables).

### 1.3 Spawn template (paste into sub-agent)

```text
You are implementing Webizen UI track <TRACK_ID> on branch 0.0.25.
Canonical tree: C:\Projects\qualia-27062026 only. No worktrees.

Read:
- docs/plans/webizen-ui-implementation-subagents-2026.md (§ for this track)
- docs/plans/comprehensive-ui-gui-webizen-plan-2026.md (principles + matrix)
- <SPECIALIST PLAN if any>
- coordination/NOTICES.md (CLAIM before edit)

Exclusive paths: <LIST>
Commands lock: <YES owner | NO do not touch commands/mod.rs>

Invariants:
- Native Local inference is the excellence path; Ollama is optional only.
- No second GPU adapter; no GPUI product shell.
- Fail-closed rights; honest Present/Partial/Scaffold labels.
- Prefer extend existing panes/commands over new crates.

Deliver:
1. Code + tests as specified
2. NOTICES PROGRESS then RELEASE
3. Short note in docs/plans/webizen-ui-PROGRESS-LOG.md (append)
4. Do not start next track unless orchestrator says so
```

### 1.4 Orchestrator (human or lead agent) checklist per wave

- [ ] All tracks CLAIMed with non-overlapping exclusive paths  
- [ ] Commands lock assigned  
- [ ] Disk free ≥ 5 GB before large desktop rebuilds  
- [ ] After merge: `cargo check -p webizen-desktop -p webizen-studio`  
- [ ] Targeted tests for touched crates  
- [ ] Progress log entry with measured results (or “not measured”)  

---

## 2. Wave U1 — Honesty & telemetry (do first)

**Goal:** Stop lying about the engine state; make Talk trustworthy.  
**Parallelism:** U1-A owns `commands/mod.rs`. U1-B/C/D must not add Tauri commands (or queue behind U1-A).

### U1-A — Live LLM telemetry + lifecycle honesty

| | |
|--|--|
| **Specialist** | Desktop master plan WS-A A2/A3 |
| **Exclusive** | `crates/webizen-desktop/src/commands/mod.rs` (telemetry + lifecycle only), `crates/webizen-studio/src/components/llm_harness.rs`, related telemetry UI, `qualia-client-core` only if needed for real numbers |
| **Commands lock** | **YES — owner** |
| **Do** | Replace `wellfair_get_llm_telemetry` static payload with live orchestrator / model lifecycle / VRAM / loaded model / tok-s if available. Replace or remove no-op `wellfair_force_model_lifecycle_phase`. Wire Studio HUD to show **backend** (Local / Ollama / Hybrid / Remote) honestly. |
| **Do not** | Change chat inference algorithm; no Ollama default flip. |
| **Acceptance** | With Local model Active: HUD numbers change under load or clearly report “no live counters yet” for missing fields (never fake tok/s). With Ollama: HUD says Ollama optional path + probe status. |
| **Tests** | Unit/integration where feasible; dogfood note if live model available. |

### U1-B — Conduct / deny visibility

| | |
|--|--|
| **Exclusive** | `connect_chat.rs` (deny UI only), new small component e.g. `conduct_banner.rs`, event listeners for `conduct-violation` if host already emits; else host **queue** for U1-A or U3 |
| **Commands lock** | NO (unless U1-A already released and orchestrator assigns a tiny emit) |
| **Do** | When intent/output gate denies: visible banner/toast + short reason; never silent failure. Link to audit/conduct if command exists. |
| **Acceptance** | Simulated or real Deny shows in UI within one turn. |

### U1-C — Capability honesty strip

| | |
|--|--|
| **Exclusive** | Shared honesty component; Talk/Browser/10D entry chrome; `pane_registry` honesty fields if needed |
| **Commands lock** | NO |
| **Do** | Reusable status chip (Ready / Partial / Scaffold / Needs model / Needs consent / Unavailable). Attach to Talk header, Browser chrome banner area (without fighting U2), 10D browser header. Source: capability registries + existing status commands. |
| **Acceptance** | Scaffold QApps and experimental Servo never read as “Ready” by default. |

### U1-D — Docs stubs

| | |
|--|--|
| **Exclusive** | `docs/manuals/webizen-ui-architecture.md`, `webizen-ui-event-catalogue.md`, `talk-and-agents-ui.md` (stubs) |
| **Commands lock** | NO |
| **Do** | Architecture diagram (desktop/studio/engine), event table from comprehensive plan §5.2, Talk backend honesty (Local primary, Ollama optional). |
| **Acceptance** | Files exist, link from comprehensive plan §10. |

**Wave U1 exit:** U1-A RELEASE + progress log; at least U1-B or U1-C landed; docs stubs optional but preferred same wave.

---

## 3. Wave U2 — Browser product cut

**Goal:** Browser is a usable client edge, not a half-panel.  
**Specialist:** `webizen-browser-and-trust.md`, `webizen-browser-ready-checklist.md`, servo-experimental (honesty only).

### U2-A — Dogfood + trust/cookies polish

| | |
|--|--|
| **Exclusive** | `crates/webizen-desktop/src/browser/*` (trust/cookies/chrome only), `chrome.html` / browser static assets, checklist docs |
| **Commands lock** | Only if new thin commands needed — claim then release same session |
| **Do** | Walk ready checklist B0–B3; fix gaps; ensure Servo banner remains honest; trust suggested import + cookies side panel complete dogfood notes. |
| **Acceptance** | Checklist items for B0–B3 checked with evidence in dogfood notes. Cert override **not** claimed active. |

### U2-B — Browser agent v0 (UI shell)

| | |
|--|--|
| **Depends** | Prefer after U3 tool-loop design; can land **UI shell + allowlist display** first |
| **Exclusive** | Browser agent panel files; do not rewrite Talk |
| **Commands lock** | Coordinated with U3 |
| **Do** | Side panel: agent status, allowed tools list, “run on page context” button that routes through same permit path as Talk tools (or stub with Scaffold until U3). |
| **Acceptance** | No silent tool execution; Forbid/Deny visible. |

---

## 4. Wave U3 — Agent tool loop UI (MCP, not external agents)

**Goal:** Agents can use **in-tree MCP tools** under principal permit.  
**Not in scope:** Grok/Claude native SDKs, non-MCP agent peer protocol.

### U3-A — Propose → permit → result

| | |
|--|--|
| **Specialist** | Desktop master WS agent tool loop; `agent_registry`, MCP dispatch |
| **Exclusive** | Client-core tool-loop if missing; desktop stream/tool events; Studio tool cards in Talk |
| **Commands lock** | **YES for this wave** (serial after U1-A) |
| **Do** | Minimal loop: model or agent proposes `tool_name` + args → UI card → principal Permit/Deny → call existing MCP dispatch → result card in thread. One golden path: e.g. `list_capabilities` or `computer_vision` status (safe). |
| **Acceptance** | End-to-end one tool with Permit; Deny does not call tool; audit line if available. |

### U3-B — MCP allowlist editor

| | |
|--|--|
| **Exclusive** | Agent settings / People agent pane; persist via existing agent_registry APIs |
| **Commands lock** | Share with U3-A or run after U3-A |
| **Do** | Edit `allowed_mcp_tools` per agent; default deny-all or tight allowlist for new agents. |
| **Acceptance** | Tool not on list cannot execute from UI path. |

---

## 5. Wave U4 — Vision / 10D product cut

**Goal:** Perception assets reachable without CLI. Engine already strong (vision-10d + computer_vision).

### U4-A — 10D Browser polish

| | |
|--|--|
| **Exclusive** | `ten_d_browser.rs` and related components |
| **Commands lock** | NO if commands already exist (`browse_vision_10d`, `load_vision_10d`, `scrub_vision_10d_paint`) |
| **Do** | Clear empty states; load/scrub UX; citable failure message; category labels anatomy vs vision. |
| **Acceptance** | List → load → scrub path dogfoodable; citable Forbid shows error not blank. |

### U4-B — Vision workbench pane

| | |
|--|--|
| **Exclusive** | New studio component under `components/`; pane registry entry |
| **Commands lock** | Only if new host commands required |
| **Do** | Entry points: SR policy status, mesh/10d handoff summary, link to MCP `computer_vision`, biosense **consent-first** CTA (no silent biometrics). |
| **Acceptance** | Honesty labels; consent gate for biosense. |

### U4-C — Library perception UI

| | |
|--|--|
| **Exclusive** | Library / perception catalog UI surfaces |
| **Do** | Surface `seed_perception_library` / computer_vision rows; models + ontologies remain. |
| **Acceptance** | User can see computer_vision library rows after seed. |

---

## 6. Wave U5 — Knowledge & compute

### U5-A — SPARQL / Knowledge workbench

| | |
|--|--|
| **Exclusive** | Knowledge / SPARQL pane components; use existing graph/SPARQL host commands |
| **Do** | Query box, results table, error honesty; default examples against local graph. |
| **Acceptance** | One sample query returns rows or clear empty. |

### U5-B — Three real computational panes

| | |
|--|--|
| **Exclusive** | Three chosen `*_qapp` or computational panes + host wrappers |
| **Do** | Wire **three** panes to real `specialized_libs` (e.g. stats, linear algebra privacy status, geometry bounded analysis) — not mock solvers. Leave rest Scaffold-labelled. |
| **Acceptance** | Three panes produce real numbers; stubs remain labelled Scaffold. |

---

## 7. Wave U6 — Unification (GPUI-inspired, not GPUI)

### U6-A — Command palette

| | |
|--|--|
| **Exclusive** | Shell palette + action registry (desktop menu map + studio open-pane) |
| **Do** | Ctrl/Cmd+K style: open Talk, Browser, 10D, Settings, QApps by name. |
| **Acceptance** | Keyboard path opens ≥5 destinations. |

### U6-B — Event catalogue + dense lists

| | |
|--|--|
| **Exclusive** | docs event catalogue updates; virtualized list component for logs/SPARQL/detections |
| **Do** | Implement catalogue as living doc; use virtualization where lists can be large. |

### U6-C — UI runtime harness notes

| | |
|--|--|
| **Exclusive** | `docs/plans/webizen-ui-PROGRESS-LOG.md` + optional script under `scripts/` |
| **Do** | Scripted steps: launch → Settings backend Local → activate model → prompt → assert stream (when model available). Without model: document blocked + what was compile-verified. |

---

## 8. Inference backend honesty (all waves)

| Backend | UI treatment |
|---------|----------------|
| **Local** | Primary. Model Hub / activation / live HUD. Excellence path. |
| **Ollama** | Settings: optional harness (URL, models, probe, save). Talk works when selected. Copy: “Optional local server — not the Qualia engine.” |
| **Hybrid** | Local-first; consent before remote; document gaps honestly. |
| **Remote** | Principal-gated; VC / consent as engine requires. |

**Do not** remove Ollama. **Do not** make Ollama the default. **Do not** spend UI wave capacity on new external agent SDKs.

---

## 9. Progress log

Create/append: **`docs/plans/webizen-ui-PROGRESS-LOG.md`**

Each track entry:

1. Track ID + status  
2. What was built (files, mechanism)  
3. Measured results or “not measured”  
4. ⚑ Human asks (model choice, bandwidth for download, dogfood machine)  
5. Next track  

---

## 10. Collision matrix (quick)

| Path | Owner waves |
|------|-------------|
| `commands/mod.rs` | U1-A → then U3-A → then any late command needs |
| `connect_chat.rs` | U1-B then U3-A (coordinate) |
| `chrome.html` / browser/* | U2-A / U2-B |
| `ten_d_browser.rs` | U4-A |
| `pane_registry.rs` | U1-C, U4-B, U5 (merge carefully) |
| `llm_harness.rs` | U1-A |
| docs/manuals | U1-D, U6-B |
| Inference engine core (`llm_agent`, `gguf_bridge`) | **Not UI wave** — separate excellence track if needed |
| Audio crates | **Off-limits this plan** |
| `specialized_libs/computer_vision` kernels | Engine done; UI only in U4 |

---

## 11. Suggested first spawn (orchestrator)

**Immediate (max parallel without commands fight):**

1. **U1-A** — commands lock (telemetry)  
2. **U1-C** — honesty strip (no commands)  
3. **U1-D** — docs stubs (no commands)  

**Then:**

4. **U1-B** after chat events known  
5. **U2-A** browser dogfood  
6. **U3-A** tool loop (commands lock)  
7. **U4-A/B/C** vision product cut  

**Hold:** U5–U6 until U1–U4 acceptance green enough for dogfood.  
**Hold:** Audio, non-MCP agent integrations, U7 GPUI spike.

---

## 12. Session spawn prompts (copy-ready)

### Spawn U1-A

```text
Track U1-A Live LLM telemetry. Plan: docs/plans/webizen-ui-implementation-subagents-2026.md §2 U1-A.
Exclusive: webizen-desktop commands telemetry/lifecycle, webizen-studio llm_harness.rs.
Commands lock: YES.
Replace static wellfair_get_llm_telemetry; honest backend Local|Ollama|Hybrid|Remote.
Do not change default to Ollama. CLAIM NOTICES first. Append webizen-ui-PROGRESS-LOG.md.
```

### Spawn U1-C

```text
Track U1-C capability honesty strip. Plan §2 U1-C.
Exclusive: shared honesty chip component + Talk/10D headers. No commands/mod.rs.
Labels: Ready|Partial|Scaffold|Needs model|Needs consent|Unavailable.
CLAIM NOTICES. Append progress log.
```

### Spawn U1-D

```text
Track U1-D docs stubs. Plan §2 U1-D.
Exclusive: docs/manuals/webizen-ui-architecture.md, webizen-ui-event-catalogue.md, talk-and-agents-ui.md.
No code crates. Ollama = optional; Local = primary excellence. CLAIM docs only.
```

---

## 13. Change log

| Date | Note |
|------|------|
| 2026-07-17 | Initial sub-agent plan: UI-first waves U1–U6; Ollama optional; audio + non-MCP agents deferred; native inference excellence; CLAIM isolation |

---

*Execute U1 first. Excellence for on-device inference is a separate continuous track; this plan makes the UI stop hiding the engine and start governing it.*
