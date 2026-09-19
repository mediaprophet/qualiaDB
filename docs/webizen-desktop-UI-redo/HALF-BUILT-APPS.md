# Half-built / hidden / buried Desktop capabilities

**Branch:** `0.0.40-webizen-ui` · **Source of truth:** code on this branch (not wish-lists)  
**Scope:** Webizen Desktop + Studio shell + related host surfaces  
**Gate 0 context:** leave Poet; migrate Desktop shell first; new Desktop default; legacy behind a flag; life-domain chrome = hard-fail; HELD/not-yet as default UX = rejected

## How to read this table

| Status | Meaning |
|--------|---------|
| **Live** | Code exists and a normal user can open it today (menu, top bar, Talk home, or obvious palette entry). |
| **Live / buried** | **Built and reachable**, but only after life-domain chrome, Advanced disclosure, poet shell, or a deep hash — useless for an OS launcher until resurfaced. |
| **Present / hidden** | **Substantial code exists**, but no honest first-class app tile / dock entry / default path — omnibox, Advanced, MCP, QApp dispatcher, or native-only. |
| **Partial** | Half-built: UI or host commands exist; core path incomplete, stubbed, or honesty-chip gated. |
| **Planned** | Named in migration/mockups or shell routes with little/no product surface yet. |
| **Lost** | Referenced (menu id, route alias, palette keyword) but destination missing, remapped oddly, or dead. |

**Reach column** answers: can a non-expert find it without life-domain literacy? Prefer “buried / HELD / MCP-only / none” when not.

Honesty labels in-product that say **“held / not yet”** (`HonestyLevel::NeedsModel` / `Unavailable`) are called out as burial UX to retire for the new shell — not as proof the capability is absent.

---

## Inventory

| App / surface | Code path(s) | Status | How user reaches it today | Notes |
|---------------|--------------|--------|---------------------------|-------|
| **Talk (home)** | `webizen-studio/src/main.rs` `TalkRoute` → `components/relations/`; shell `open_talk` | Live | Default `/`; top bar Relations; menu Talk (Ctrl+0); palette | Desktop home. Empty hash = Talk. |
| **Mail (daily inbox)** | `TalkMailRoute` `/talk/mail`, `/mail`; `relations/mail*.rs`; shell `open_mail` | Live / buried | Talk → Mail tab; menu Mail; palette “Mail”; not a top-level dock app | Real inbox UI under Relations life-domain. Must become a first-class Mail app tile. |
| **Directory / People** | `TalkDirectoryRoute`, `TalkPeopleRoute`; `directory_pane.rs`; shell `open_directory` | Live / buried | Talk → Directory/People; menu Directory; palette | Humans-first address book exists but buried under Relations. |
| **Relations shell (tabs)** | `components/relations/` | Live / buried | Top bar “Relations” / Talk | Life-domain chrome wraps Mail, Directory, chat, projects. Hard-fail IA for redo. |
| **Selfhood / Identity** | `IdentityRoute` `/identity`; `wellfair/personal_panel`, `social_book_panel`, `consent_panel`; menu Wallet → `/identity` | Live / buried | Top bar Selfhood; sidebar; Wallet menu remaps here | Profile/consent live; labeled as life domain. Wallet is not a separate wallet app. |
| **Lived Memory / Library** | `LibraryRoute` `/library`; `wellfair/semantic_library`, `library_panel`; shell `open_library` | Live / buried | Top bar Memory; menu Library; palette | Simple vs Advanced Technical split. Advanced holds CML/export — Present but mode-gated. |
| **Care / WellFair shell** | `WellfairRoute` `/wellfair`; `wellfair/shell.rs` + panels; shell `open_wellfair` | Live / buried | Top bar Care; menu WellFair (Ctrl+1) | Full care stack; life-domain header. Pairing QR lives inside this shell. |
| **Practice / Work** | `WorkRoute` `/work`; projects/work_board/finance/credentials panels | Live / buried | Top bar Practice; sidebar | DomainRouteHeader “Life domain”. Projects board is real code, buried. |
| **Instruments / Tools** | `ToolsRoute` `/tools`; `ModelSetupPanel`, `WellfairToolsPanel`, sync/backup/audit | Live / buried | Top bar Instruments; sidebar | Models + companion ingest + sync. Labeled Instruments life domain. |
| **Settings (in-app)** | `SettingsRoute` `/settings`; `components/settings_page`, `settings/` | Live | Top bar gear; menu Settings (Ctrl+,); palette | Classic shell forced on open from native menu. |
| **Settings portal (loopback)** | `webizen-desktop/src/settings_server.rs`; `static/portal/`; Help → Open Settings Portal | Present / hidden | Help menu portal; tray-oriented; not the in-app Settings surface | Second settings surface on `127.0.0.1` — easy to miss vs in-app Settings. |
| **Browser (Reach / World)** | `BrowserRoute` `/browser`; `browser_panes/`; desktop `browser/`; shell `open_browser` | Live (desktop) / Present-hidden (wasm) | Top bar World **only if** `supports_browser_pane()` (DesktopWebview); menu Web Browser (Ctrl+3); palette | **Built but gated off public wasm** (`BrowserUnavailable`). Migration must make Browser a visible app again on Desktop. |
| **10D / Infosphere browser** | `TenDBrowserRoute` `/10d-browser`; `ten_d_browser.rs`; shell `open_10d` | Live / buried | Menu 10D Browser (Ctrl+4); palette; Advanced/sidebar Chora-adjacent | Real `.10d` inspect UI; not in life-domain top tabs. Good OS-app candidate. |
| **Chora / Universe** | `ChoraRoute` `/chora`; `wellfair/chora_panel.rs`; shell `open_chora` | Live / buried | Menu Chora (Ctrl+2); sidebar In domain | Spatial/universe panel; Advanced experience toggles density. |
| **Keep (vault hub)** | `KeepRoute` `/keep`; `keep_hub.rs`; `keep_volume.rs`; palette/shell “keep” | Live / buried | Sidebar Advanced → Legacy hub (Keep); omnibox `keep`/`vault`; shell palette Keep | Volume browse + Poet volume ops. Default UX leans **held / not yet** when volume missing — burial pattern. |
| **QApps catalog** | `QAppsRoute` `/qapps`; `components/qapps/`; shell `open_qapp_manager` | Live / buried | Menu QApp Manager; Advanced sidebar; palette “QApps (Advanced)” | ~334 catalog entries (`279 Active / 49 Beta / 6 Soon`). Academic flood is Present; many dispatcher arms are thin scaffolds. Opt-in “academic / Soon” toggle. |
| **QApp Studio (layout builder)** | `StudioRoute` `/qapp-studio`; `studio_canvas` / `DynamicPage`; shell `open_qapp_studio` | Live / buried | Menu QApp Studio; catalog routes | Point-grid editor Present; NodeRelational/Spatial still fall back (PAGES.md). |
| **Context Studio** | `ContextStudioRoute` `/context-studio`; `contextual_workspace.rs`; catalog platform entry | Present / hidden | QApps catalog / deep link `#/context-studio` | Catalog marks Active; not on life-domain top bar. |
| **Knowledge Nexus** | `NexusRoute` `/nexus`; `nexus.rs` | Present / hidden | Advanced sidebar; omnibox `nexus` | Research/claims UI Present; buried under Advanced. |
| **Health vault** | `HealthRoute` `/health`; health/wellbeing/sleep/med panels | Live / buried | Sidebar In domain Health; omnibox | Life-domain Care fragment as its own route. |
| **Anatomy** | `AnatomyRoute` `/anatomy`; anatomy 3D/scorecard/comorbidity; shell anatomy | Live / buried | Sidebar Anatomy; menu via qapp_route | 3D + panels Present; not a dock app. |
| **Clinical** | `ClinicalRoute` `/clinical` | Present / hidden | Omnibox / deep hash only (no top-tab) | Panels Present; reachability weak. |
| **Agency** | `AgencyRoute` `/agency`; guardianship/agency/accountability/safeguards | Present / hidden | Advanced sidebar | Built panels; Advanced-only. |
| **Sanctuary** | `SanctuaryRoute` `/sanctuary`; sanctuary panels; shell sanctuary_* | Live / buried | Sidebar Sanctuary; native sanctuary menu actions | Lock/unlock host commands Live; chrome still domain-framed. |
| **Vision workbench** | `VisionRoute` `/vision`; `vision_workbench.rs`; desktop `vision_audio` cmds | Partial + buried | Sidebar Vision; Instruments omnibox aliases | Detect path Partial on desktop; SR/biosense honesty = Partial / NeedsModel (**held / not yet** label) / NeedsConsent. |
| **Listen workbench** | `ListenRoute` `/listen`; `listen_workbench.rs` | Partial + buried | Sidebar Listen | Maps host `audio_capabilities` honesty; Missing → Scaffold, NeedsWeights → held label. |
| **Communications (live-share)** | `CommunicationsRoute` `/communications`; `WellfairCommunicationsPanel` | Present / hidden | Advanced → Live-share requests | Not chat/mail; companion consent requests. Easy to confuse with Talk. |
| **Jobs / Job centre** | `JobsRoute` `/jobs`; `job_center.rs`; social job cmds | Present / hidden | Advanced sidebar; omnibox `jobs`/`queue` | Background jobs UI Present; not dock-visible. |
| **Desktop logs** | `LogsRoute` `/logs`; `desktop_log`; Help → View Logs | Present / hidden | Advanced; Help menu | Host log stream. |
| **Agent QA** | `AgentQaRoute` `/agent-qa`; `agent_qa_panel`; desktop `commands/agent_qa.rs` | Present / hidden | Advanced sidebar | Diagnostics for agents/models. |
| **Supervisor / Problems** | `SupervisorRoute` `/supervisor`; `problems_pane.rs` | Present / hidden | Deep link / omnibox only | Problems pane Present; no top chrome. |
| **About** | `AboutRoute` `/about`; `about_page.rs`; Help About | Live | Help → About | Includes wallet status snapshot via Tauri. |
| **Dashboard / Overview (ops)** | `DashboardRoute` `/home`; `dashboard.rs` | Present / hidden | Advanced → Overview (ops) | Ops dashboard; Talk replaced it as home. |
| **GPU viewport (WGPU surface)** | `GpuViewportRoute` `/gpu-viewport`; `native_gpu_viewport.rs`; `mount_gpu_surface`; shell `toggle_gpu` | Present / hidden | Shell GPU toggle / navigate `gpu-viewport`; not life-domain tab | Native HWND+wgpu Present on desktop host. Classic shell HTML also toggles GPU overlay. **Capability buried** — migration should attach renderer to an app window. |
| **Render preview** | `RenderPreviewRoute` `/render-preview`; `render_preview.rs`; shell qapp_route | Present / hidden | Deep link / shell route id | Preview size fixed 800×600 in route. |
| **Scene interaction** | `SceneInteractionRoute` `/scene-interaction` | Present / hidden | Deep link only | Component Present; no menu entry found. |
| **Anatomy test** | `AnatomyTestRoute` `/anatomy-test` | Present / hidden | Deep link only | Explicit test route. |
| **Poet Harness (Studio embed)** | `PoetRoute` `/poet` (+ `/poet/catalog`, `/poet/instruments`); `components/poet/` HyperCanvas; `poet_harness.rs` re-export | Live / dual | Dedicated window `open_poet_window` → `#/poet`; poet shell chrome; Tools→Poet; palette Catalog·Lexicon | **Leave alone this Gate.** Full-bleed when on Poet routes. Catalog palette copy still says “held / not yet”. |
| **Poet window (Desktop)** | `shell/menu.rs` `open_poet_window`; loads same Studio `index.html#/poet` | Live / dual | File → New Poet Window; Tools → Poet Harness; tray | Same Studio Poet UI in a second webview — not `crates/poet` WASM product shell. |
| **crates/poet (WASM / tool-chest)** | `crates/poet/` (`tool_chest`, `vibe_host`, `browser`, `keep_volume`) | Present / parallel | Built as crate; Desktop commands + Studio keep/poet paths call related ops | **Dual Poet:** library/WASM host form vs Studio HyperCanvas. Migration later: one launchable app; fold duplicate half. |
| **Desktop Poet commands** | `commands/poet.rs`, `poet_daemon.rs`, `poet_render.rs` | Present / hidden | Invoked from Poet UI / daemon `:4242` | Daemon probe honesty string `"held"` when unreachable — wait-honest, but product chrome still HELD-flavoured. |
| **Vibe / VibeScript host** | `commands/vibe_host.rs`; `crates/poet/vibe_host.rs`; `crates/vibe*`; Poet `vibe_console.rs` | Present / buried | Inside Poet; Tauri vibe_* commands; no standalone Vibe app tile | Frozen vibe-host-0.1 ops Present. Shell aliases `vibe` → Poet. OS shell should expose VibeScript as app later with Poet. |
| **Dual Studio (Vibe + GPU)** | `components/dual_studio.rs` | Present / hidden | Component Present; no first-class Route enum entry | Shared-WASM editor+viewport — buried component, not an app. |
| **Phone remote / companion pairing** | `wellfair/pairing_panel.rs`; `companion_gateway.rs` (WS `:8080`); Tools companion ingest | Live / buried | Inside WellFair shell (“Pair your phone”); Instruments tools paste-ingest | **Built LAN QR + WS ingest**, not a Phone app. PWA publish panel notes phone install = next stage. |
| **Companion PWA publish** | `wellfair/qapp_publish_panel.rs` | Partial | Inside WellFair | Scaffold generator Present; secure-origin phone install Planned. |
| **MCP server (Desktop)** | `webizen-desktop/src/mcp_server.rs` (`127.0.0.1:4245`); social `mcp_*` cmds | Present / MCP-only | TCP MCP for agents; not a user app | Hard-fail risk: usefulness MCP-only. Must surface native/browser apps in shell. |
| **MCP Inspector UI** | `mcp_inspector.rs`; qapp id `mcp-inspector` | Partial / hidden | QApp dispatcher only | **Hardcoded mock UI** (GitHub/Jira rows) — not wired to live Desktop MCP. |
| **Solid LDP browser** | `solid_ldp_browser.rs`; qapp dispatcher; pane_registry | Present / hidden | QApp / pane id only | Present component; no Route / menu. |
| **Wallet (menu)** | shell `open_wallet` → navigate `wallet` → `/identity` | Lost (as app) / remapped | Tools → Wallet | **No `/wallet` route.** Alias to Identity. Lightning/ILP status only on About snapshot + identity finance-ish panels. |
| **Files (OS files app)** | — (Keep is vault volumes, not a file manager) | Planned / Lost as named app | none | Migration notes name “Files/Keep”. Keep ≠ general Files. No `Files` route found. |
| **Native shell HTML (tabs/GPU/palette)** | `shell/shell_html.rs`, `tabs.rs`, `action.rs`, `menu.rs` | Live (host chrome) | OS menu bar + embedded shell script | Classic host chrome around Studio webview. `shell_classic` / `shell_poet` presentation switch. **Legacy path to keep behind flag** per Gate 0. |
| **Command palette (Studio)** | `components/command_palette.rs` | Live / buried IA | Ctrl+K / Ctrl+P | Destinations are **life-domain named** (Selfhood, Relations, Care…). Catalog entry explicitly “held / not yet”. Redo: app names, not domains. |
| **Command palette (shell HTML)** | `shell_html.rs` `PALETTE_ITEMS` | Live | Shell Ctrl+K | Parallel list (Talk, Directory, Mail, Browser, Keep, Poet-as-Catalog…). Slightly different from Studio palette — dual chrome. |
| **Omnibox router** | `route_from_omnibox` in `main.rs`; `shell_dest.rs` | Live / expert | Top omnibox | Honest aliases (mail≠poet). Still the escape hatch for buried routes. |
| **Experience mode (Simple/Advanced)** | `experience_mode.rs` | Live (gate) | Top bar switch | **Buries** engineering: Advanced sidebar (Keep, Nexus, QApps, Jobs, Agent QA…), library technical shelf, Relations advanced tabs. |
| **Life-domain top chrome** | `AppLayout` top tabs + sidebar “Life domains”; `DomainChrome`; `DomainRouteHeader` | Live (hard-fail IA) | Default Studio layout | Selfhood / Relations / Memory / Care / World / Practice / Instruments — **reject as default UX** for new OS shell. |
| **Honesty chip “held / not yet”** | `honesty_chip.rs` | Live (burial UX) | Scattered Partial surfaces | NeedsModel/Unavailable **label** is “held / not yet”. Product rule for redo: live vs planned — never HELD as surface. |
| **Academic / scaffold QApps (bulk)** | `components/*_qapp.rs` (~270+); `qapp_dispatcher.rs`; catalog academic_* | Present / hidden (many Partial) | QApps catalog (academic opt-in) | Massive Present code. Library panel text: “many are stubs”. Treat as inventory mass, not OS dock apps. |
| **WGPU diffusion runtime** | `webizen-desktop/src/runtime.rs`; system cmds | Present / hidden | Background host service | Optional; deferred on first-run to avoid driver faults. Not user-facing app. |
| **Mesh / P2P** | `commands/mesh.rs`; `p2p_dashboard.rs` | Present / hidden | Component/cmd level | Start/stop/status Present; findability unverified as first-class route (mark draft if no Route). |
| **Updater** | `updater_service.rs`; Help → Check for Updates | Live | Help menu | Host updater path. |
| **Med reminders notifier** | `med_reminder_notifier.rs`; shell `health_med_reminders` | Present / hidden | Native shell action | Opens med reminders path; not a dock app. |
| **WebRTC manager** | `webrtc_manager.rs` | Present / hidden | Host-only | No Studio Route found. |
| **New OS Desktop shell (dock/WM)** | `docs/webizen-desktop-UI-redo/mockups/*` | Planned | Mock-ups only (`mockups/index.html`) | Gate 0 shared-sense. Not in `crates/webizen-desktop` as default shell yet. |
| **`--legacy-desktop` flag** | Named in `MIGRATION-NOTES.md` | Planned | none yet | Coexistence lock: keep old shell launchable; name TBD by Neo. |

---

## Built but useless until resurfaced (migration priority)

These are the highest-value **Present/Live but buried** items for the OS-shell pass (not stubs):

1. **Browser** — Live on Desktop, hidden on wasm, easy to miss behind World life-domain gate.
2. **Mail / Directory** — Live under Talk; need first-class app tiles.
3. **Settings portal vs Settings app** — two surfaces; portal is Help/tray-shaped.
4. **Keep / Jobs / Logs / Nexus / QApps / Agent QA** — Present, Advanced-or-deep only.
5. **GPU / Render / 10D** — native capabilities Present; not dock apps.
6. **Phone companion pairing** — Live QR/WS inside WellFair; not a Phone app.
7. **MCP** — Live server; Inspector UI is mock; risk of MCP-only usefulness.
8. **Vibe host + Dual Studio** — Present under Poet/components; no standalone launcher.
9. **Wallet menu** — Lost as product; remapped to Identity.
10. **Files** — Named in migration; **no code app** (Keep ≠ Files).

Poet (Studio HyperCanvas + dedicated window + `crates/poet`) is **dual/Present** — **do not grow**; migrate Desktop shell first, then fold Poet in as one app.

---

## Route enum snapshot (Studio)

All routes declared in `crates/webizen-studio/src/main.rs` `Route` (code-backed):

`/` Talk · `/talk/mail` · `/mail` · `/talk/directory` · `/talk/people` · `/talk` · `/home` · `/dashboard` · `/keep` · `/anatomy-test` · `/qapps` · `/browser` · `/reach` · `/settings` · `/logs` · `/jobs` · `/agent-qa` · `/poet` · `/poet/catalog` · `/poet/instruments` · `/about` · `/context-studio` · `/qapp-studio` · `/qapp-studio/:id` · `/render-preview` · `/scene-interaction` · `/nexus` · `/library` · `/vision` · `/listen` · `/communications` · `/health` · `/anatomy` · `/clinical` · `/identity` · `/agency` · `/sanctuary` · `/work` · `/tools` · `/wellfair` · `/chora` · `/supervisor` · `/10d-browser` · `/gpu-viewport` · `/:..path` DynamicPage

---

## Evidence notes / draft-quality caveats

- **Verified on branch** `0.0.40-webizen-ui` at `/workspace/qualiaDB` (local tree; monet Gate 0 mockups already present under `docs/webizen-desktop-UI-redo/`).
- QApp catalog counts (`279/49/6`) from `Stat::` grep across `components/qapps/catalog/`.
- Academic QApp file count is large (~270 `*_qapp.rs`); individual readiness not exhaustively run — mass marked Present/Partial per catalog + library_panel stub note.
- `p2p_dashboard` / WebRTC / some pane_registry entries: **Present in tree**; first-class reachability not fully traced — treat row notes as best-effort, not invented features.
- No commit/push performed for this inventory write.

---

## Context locks (do not violate in follow-on work)

- Leave Poet; Desktop shell first; Poet migrates in later.
- New Desktop default; legacy shell remains behind a flag; do not delete old.
- No life-domain chrome as top-level IA.
- No HELD/not-yet as default product surface.
