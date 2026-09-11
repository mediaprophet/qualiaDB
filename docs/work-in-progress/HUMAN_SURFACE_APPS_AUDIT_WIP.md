# Human-surface apps audit

**Status:** work-in-progress · Capt ops · **Not a release gate**  
**Branch:** `0.0.38` · **Tip:** `1e2605c` (fold) · prior product-cut tip `b07ddef` · **Repo:** `mediaprophet/qualiaDB`  
**Workspace:** `/workspace/qualiaDB`  
**Date:** 2026-09-12 (AEST) · **Runner:** Capt (ops scan)  
**Product context:** One Poet · human-alone · cherry-pick not parity · reveal live `ALL_BOUND` · **top-level IA names wait on Timothy (or BRICS)** · Ask · Keep · Talk cited below as **teachable loops only**, not locked nav chrome.

---

## Method / tip

1. Scanned `crates/webizen-desktop` (shell menu/routes, Tauri commands), `crates/webizen-studio` (Dioxus `Route` + panels), `crates/poet` (manifolds, container views, lexicon bay), and Capt/UAT docs under `docs/work-in-progress/`.
2. Cross-checked live Host catalog: `crates/qualia-core-db/src/poet_host/invoke/ids.rs` → `ALL_BOUND` ≈ **1121** const entries (~**1116** resolved `Family.method` strings; 5 const names unresolved in the same file at tip). **No Host ids invented.**
3. Chrome status from code + prior Capt circuits: Desktop apps walkthrough tip `d7f0bdc`; Frame B tip `d08d1c9` folded at `1e2605c`; One Poet cut `2532cde` / `b07ddef`.
4. Scores: **WORKING** | **PARTIAL** | **HELD** | **MISSING** | **BURIED** (live bind/UI exists but not surfaced for human-alone).

### Tip lineage (cite only)

| Tip | Role |
|-----|------|
| `1e2605c` | Docs fold: Frame B — WASM **PASS**, Desktop **HELD** (Catalog) |
| `b07ddef` | One Poet cut — cherry-pick, drop parity theatre |
| `d08d1c9` | Capt Frame B UAT score |
| `d7f0bdc` | Desktop apps walkthrough |
| `8f5b7d6` | Ask · Keep · Talk IA amend (cited by Frame B / lexicon) |

---

## Scoreboard table

| App | Purpose | Status | Desktop | WASM | Evidence | Next |
|-----|---------|--------|---------|------|----------|------|
| **Talk / Chat + LLM** | Relations habitat: people, conversation, local inference | **WORKING** (D) · **PARTIAL** (W) | Mounted: QApps → Talk (`Ctrl+0`); shell home = Talk; `TalkRoute` → `RelationsShell` → Inbox `ConnectChat` (`stream_chat_inference`) | Studio WASM needs Native host for Tauri invoke; Poet communications manifold containers mostly `held` | `crates/webizen-desktop/src/shell/menu.rs` `qapp_route("talk")`→`/`; `shell_html.rs` PALETTE + normalize home→talk; `webizen-studio/src/main.rs` `TalkRoute`; `components/connect_chat.rs`; Capt `d7f0bdc` Talk **PASS**. Live binds (tip): `ChatGraph.validate_fragment` · `ChatGraph.link_reply` · `ChatGraph.session_summary` in `ALL_BOUND` (`ids.rs`). Older `EXP_B0` (no ChatGraph family) is **stale vs tip** — cite tip, do not invent further ids. | Human-alone Talk that talks without agent; WASM host honesty; cherry-pick Desktop chat UX |
| **Keep / Sanctuary** | Save/reopen volumes; vault; care hub into life domains | **WORKING** (hub) · **PARTIAL** (human browse) | `/keep` KeepHub; `/sanctuary` Sanctuary panels; menu Sanctuary lock events; Capt Keep/Sanctuary **PASS** | Poet `sanctuary` manifold: Vault/Pulse `present`, Health/Anatomy `held`; Frame D Keep path still path-field heavy | `main.rs` `KeepRoute`/`SanctuaryRoute`/`KeepHub`; `manifolds/sanctuary.rs`; Capt walkthrough; parity audit Frame D | Keep without absolute paths (recent/sayable browse); celebrate only on real `GraphDatabase.volume_commit` |
| **Ask (graph / studio bay)** | Usable answer from graph/room — primary sayable | **PARTIAL** | Studio bay / Poet harness via Tools → Poet; Dual Studio under Media | Studio bay first-arrive Frame A **PASS** both (`d7f0bdc`/`f712e97`); live `GraphDatabase.sparql` · `stats` · `volume_open` · `volume_commit` · `lexicon_manifest` | `manifolds/studio_bay.rs`; `ids.rs` GraphDatabase.*; One Poet cut | Mute Capability.method on cold-load; make Ask discoverable without agent |
| **Browser (Reach / World)** | Native / in-shell web; pages into same session as Memory | **WORKING** | QApps → Web Browser (`Ctrl+3`); `/browser`; Tauri `browser_*` cmds | Studio `BrowserRoute` → `WebBrowserPane`; Poet `webview` container `held` | `menu.rs`; `commands/browser.rs`; `main.rs` `BrowserRoute`; Capt Browser **PASS** | Keep cherry-pick; avoid second browser chrome track |
| **10D / Infosphere Browser** | Anatomy & vision `.10d` | **PARTIAL** | Menu `Ctrl+4` → `/10d-browser` | `TenDBrowserRoute` → `TenDBrowser` | `menu.rs`; `main.rs` | Human discoverability under World / Keep, not Capability soup |
| **Hypermedia Library** | Local lived-memory shelf (docs, models, knowledge) | **PARTIAL** · chrome **BURIED**/bounce | Tools → Hypermedia Library → `/library`; wellfair `library_*` / `wellfair_*_library` commands **live**; Capt: entry **bounced to home** | Studio `LibraryRoute` (Route enum `/` still named Library — shell remaps empty hash to Talk); Poet `build_library_view` (placeholder stats — not daemon-backed UI) | `menu.rs`; `commands/wellfair/library.rs`; `container_views_ext/library.rs`; Capt `d7f0bdc` Library **PARTIAL**; product cut: Library secondary/held | Reveal real library under Keep/Memory without false-held; wire Poet view to live library cmds or honest held |
| **Directory / address book** | AD-like categorised addressbook + agreement slots | **BURIED** (live) | Tauri: `list_directory`, `search_directory`, categories (`personal_directory.rs`); UI: Relations → People → “Open personal directory” (`DirectoryPane`) — **not** top-level QApp/menu | Pane invokes need Native host; no Poet manifold seed named directory | `commands/personal_directory.rs`; `directory_pane.rs`; `relations/people.rs`; plan cite in pane header | Surface under Talk/People for human-alone; agreement slot honesty already empty-until-P1 |
| **Ontology workbench** | Visual ontology authoring, vocab map, SHACL, N3 | **PARTIAL** · **BURIED** in Poet | No dedicated Desktop menu/`qapp_route` for ontology; knowledge packs via portal jobs | Poet `ontology` manifold seed honesty=`present` for graph/library/mapper/relation/SHACL/N3; `ontology_views/*` builders wired in `body_ontology.rs`; Knowledge manifold Ontology Browser `partial` | `manifolds/ontology.rs`; `browser/ontology_views/`; `containers/body_ontology.rs` | Reveal under Ask/Studio craft — not top-level peer until Timothy names IA; no Host invent for Ontology.* (none in `ALL_BOUND` families named Ontology) |
| **Catalog · Lexicon** | Lexicon packs; living/artifact/machine chips; held-gate | **HELD** (D) · **WORKING** held-gate (W) | Catalog/Lexicon **not mounted** this tip (Frame B Desktop **HELD**) | Lexicon bay chrome; bind `GraphDatabase.lexicon_manifest`; held string `held / not yet — open lexicon pack` | `poet/.../lexicon_bay/`; Frame B `d08d1c9` / fold `1e2605c`; parity audit | davinci: mount Catalog under Ask·Keep·Talk on Desktop **or** honest held path — Wave-22 held until Desktop B clears |
| **Poet (shell + sought function)** | One human product dialect inside Desktop shell; toolchest, manifolds, Dual Studio, radial wheel, honesty chrome | **WORKING** (arrive) · **PARTIAL** (sought depth) | Tools → Poet Harness `/poet`; View Shell: Poet; Capt Studio/Poet **PASS** | Trunk/WASM Poet: studio bay, docks, 17 manifold seeds, Dual Studio `live`, ontology/library/comm views | `poet_harness`; `manifolds/mod.rs` `all_seeds()`; `IMPLEMENTATION_TRACKER.md`; Frame A PASS both | Cherry-pick great Desktop↔WASM; drop parity theatre; reveal buried live capability |
| **Dual Studio** | VibeScript + GPU viewport on Media manifold (not nested DCC) | **WORKING** | Capt Dual Studio under Media **PASS** | `media` seed `dual_studio` honesty=`live`; `build_dual_studio_view`; studio Dioxus `dual_studio.rs` | `manifolds/media.rs`; `studio_views/dual_studio`; Capt `d7f0bdc` | Keep as Media craft — not nav peer |
| **Sanctuary (vault UI)** | Vault lock/unlock, protected spaces | **WORKING** | `/sanctuary`; Wellfair sanctuary panels; shell SanctuaryLock | Poet sanctuary manifold Vault `present` | `main.rs` SanctuaryRoute; `wellfair/sanctuary_*`; Capt Sanctuary **PASS** | Continuity copy handle≠who still PARTIAL |
| **WellFair** | Care shell: body, rights, welfare, labour | **WORKING** | QApps → WellFair `Ctrl+1` → `/wellfair` | `WellfairShell` route | `menu.rs`; `WellfairRoute`; Capt QApps **PASS** | Keep as Keep/Care destination |
| **Chora** | Spatio-temporal commons manifold | **WORKING** | QApps → Chora `Ctrl+2` → `/chora` | `WellfairChoraPanel`; browser default `qualia://chora/universe` | `menu.rs`; `ChoraRoute`; Capt | Situate under World, not Capability soup |
| **QApps catalog / manager** | Browse & manage QApps; QApp Studio | **WORKING** | QApps menu + Manage + QApp Studio | `/qapps`, `/qapp-studio` | `menu.rs`; `QAppsRoute`/`StudioRoute`; Capt | Secondary under shell — not peer to Ask·Keep·Talk |
| **Settings** | Backend & preferences | **PARTIAL** | Tools → Settings (`Ctrl+,`) → `/settings`; also Settings Portal (external); Capt: **listed, no visible Settings surface** | `SettingsRoute` → `SettingsPage` → `SettingsShell` | `menu.rs`; `settings_page.rs`; Capt Settings **PARTIAL** | Fix Desktop chrome path so Settings actually paints; portal vs in-shell clarity |
| **Mesh** | P2P mesh start/status among peers | **BURIED** / chrome **HELD** | Tauri `mesh_start`/`mesh_stop`/`mesh_status` live; UI buried in Connect / social_hub people | No `Mesh.*` family in `ALL_BOUND`; Poet WebRTC view still demo/held language; topbar mesh “Unavailable…” | `commands/mesh.rs`; `connect_pane.rs`; `social_hub/people.rs`; Capt gaps Mesh unavailable | Honest **held / not yet** chrome (ban “unavailable”); reveal when human needs Connect |
| **Pulse / Aura / Job** | Event stream / aura panel / background jobs | **HELD** / false-unavailable risk | Jobs route `/jobs`; portal job form | Pulse.* **10** ids live in `ALL_BOUND`; panels often labelled unavailable in UAT | `ids.rs` Pulse.*; parity audit Frame B gap; Capt gaps | davinci: unavailable → held/not yet |
| **Wallet / Identity** | Profile, social book, consent (identifiers ≠ who) | **PARTIAL** | Tools → Wallet → `/identity` | `IdentityRoute` Wellfair personal/social/consent | `menu.rs` wallet→`/identity`; `IdentityRoute` | Continuity / handle≠who copy |
| **Nexus** | Research / claims epistemic threads | **PARTIAL** | `/nexus` route present | `Nexus` component | `main.rs` NexusRoute | Secondary knowledge craft |
| **Admin Mission Control (7 hubs)** | Daemon/fleet/admin launcher | **HELD** / prototype | — | `app_launcher.rs` AdminOperatorHub UI (status strings look aspirational) | `poet/src/browser/app_launcher.rs` | Do not present as human first-session |

---

## Buried live (reveal FAIL)

Live capability or UI that a person cannot find without an agent — chrome FAIL under One Poet cut (“if on live `ALL_BOUND` and a person can use it, chrome **shows** it”).

| Surface | Why buried | Live evidence |
|---------|------------|---------------|
| **Directory / address book** | Toggle inside Relations → People; no menu/QApp/palette entry | `search_directory` et al. Tauri cmds; `DirectoryPane` |
| **Hypermedia Library (Poet view)** | Desktop menu bounces home (Capt); Poet library view uses placeholder stats | `wellfair`/`library_*` cmds; `build_library_view` |
| **Ontology workbench** | Seeded + views built in Poet; no Desktop route/menu; not cold-load | `ontology` manifold `present`; `ontology_views` |
| **Catalog · Lexicon (Desktop)** | WASM bay exists; Desktop not mounted → Frame B **HELD** | `GraphDatabase.lexicon_manifest`; lexicon_bay |
| **Mesh controls** | Inside Connect / social hubs; chrome says Unavailable | `mesh_start`/`status`; no Mesh.* ALL_BOUND family |
| **ChatGraph.* (tip)** | Three ids now in `ALL_BOUND`; Talk UI still primarily FRB/`stream_chat_inference` — ribbon may not expose ChatGraph | `ChatGraph.validate_fragment` · `link_reply` · `session_summary` |
| **Pulse.*** | Ten live binds; UAT still sees unavailable panels | `Pulse.publish*` · `open_channel` · … |
| **Communications manifold** | Seeded; all containers honesty=`held` | `manifolds/communications.rs` |

---

## Honest held

Wait-honest gaps (bind or mount really missing / not ready) — say **held / not yet**, never broken/unavailable.

| Item | Why honest held | Cite |
|------|-----------------|------|
| Desktop **Catalog · Lexicon** mount | Not surfaced this circuit; shell steady, no panic-red | Frame B Desktop `d08d1c9` / fold `1e2605c` |
| Poet **communications** containers | Seed honesty `held` (conversations, channels, presence, WebRTC, webview) | `manifolds/communications.rs` |
| Sanctuary Health / Anatomy containers | Seed `held` | `manifolds/sanctuary.rs` |
| Media 3D / Vision / Listen / Triad | Seed `held` (Dual Studio alone `live`) | `manifolds/media.rs` |
| Knowledge Ontology Browser | Seed `partial` | `manifolds/knowledge.rs` |
| Research Document / LaTeX / Slides | Seed `held`; tests forbid `missing` | `manifolds/mod.rs` tests |
| Mesh as Capability family | No `Mesh.*` in `ALL_BOUND` — desktop mesh is Tauri/session, not invent Host | `ids.rs` scan; `commands/mesh.rs` |
| Ontology.* / Library.* Host families | No such ALL_BOUND family names — UI must not invent | `ids.rs` family scan |
| Wave-22 / first-session gate | Held until Desktop Frame B clears + real Keep entrance | parity audit; Frame B score |

---

## Priority order to make useful (fun, not hostile)

1. **Talk that talks** — Inbox/`ConnectChat` human-alone with local model; honest held when model missing (never agent-remote Matrix).  
2. **Keep that reopens** — volume picker/recent/sayables; commit celebrate only on real write (`GraphDatabase.volume_commit`).  
3. **Ask that answers** — studio bay sayables; mute Capability.method; reveal `GraphDatabase.sparql` under Ask craft.  
4. **Desktop Catalog · Lexicon** — mount or honest held path (clears Frame B / Wave-22 blocker).  
5. **Reveal Directory** under Talk/People (already live) — fun addressbook, not AD cosplay.  
6. **Library that opens** — fix Desktop bounce; wire or honest-held Poet library vs live `library_*` cmds.  
7. **Settings that paints** — fix Capt PARTIAL (route exists, chrome miss).  
8. **Unavailable → held/not yet** sweep (Mesh, Pulse, Aura, Job).  
9. **Ontology workbench** reveal as Studio craft under Ask — cherry-pick, not nav peer.  
10. **Mesh/Connect** only when human needs peers — buried→discoverable without Capability soup.

---

## Open questions for Timothy

1. **Top-level IA names** — confirm Ask · Keep · Talk as the only first-session trio; confirm Catalog / Mesh / Library / Ontology stay secondary or held (product cut already says so — lock chrome?).  
2. **Studio Route `/`** still documents Library as default in `Route` enum while shell remaps empty → Talk — intentional dual meaning or debt to fold?  
3. **ChatGraph.*** now in tip `ALL_BOUND` (3 ids) vs older EXP-B0 “do not add” — keep desktop FRB primary, or deliberately surface ChatGraph in Poet ribbon?  
4. **Directory** — promote to palette/QApp, or remain under Relations → People?  
5. **Settings** — in-shell `SettingsShell` vs external Settings Portal: which is human-primary?  
6. **Ontology workbench** — when (if ever) a named top-level surface vs manifold-only craft?  
7. **Wave-22** — unblock on Desktop Catalog mount alone, or also require Keep path picker?

---

## ALL_BOUND snapshot (tip `1e2605c`, cite only)

- Source: `crates/qualia-core-db/src/poet_host/invoke/ids.rs` `pub const ALL_BOUND`
- ~**1121** array entries · ~**1116** resolved strings · ~**104** families
- Sample human-surface-relevant (not exhaustive): `GraphDatabase.*` (5) · `Pulse.*` (10) · `ChatGraph.*` (3) · `NLP.*` (18) · `Render.*` (53) · `Econ.*` (106)
- Explicitly **absent** as families: `Mesh.*`, `Ontology.*`, `Library.*`, `Sanctuary.*`, `Chat.*` (non-Graph)

---

## Related docs

- [`ONE_POET_PRODUCT_CUT_WIP.md`](./ONE_POET_PRODUCT_CUT_WIP.md)  
- [`HUMAN_ONBOARDING_LEXICON_WIP.md`](./HUMAN_ONBOARDING_LEXICON_WIP.md)  
- [`HUMAN_SURFACE_PARITY_AUDIT_WIP.md`](./HUMAN_SURFACE_PARITY_AUDIT_WIP.md)  
- [`HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md`](./HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md)  
- [`HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_B_d08d1c9.md`](./HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_B_d08d1c9.md)  
- [`EXP_B0_CHAT_GRAPH_SEAM_DECISION.md`](./EXP_B0_CHAT_GRAPH_SEAM_DECISION.md) (stale vs tip ChatGraph binds — diagnose, don’t invent)

---

## One-breath

> Desktop already runs Talk, Browser, Dual Studio, Sanctuary, WellFair, Chora, Keep.  
> WASM already teaches held/not yet for Catalog.  
> Buried live (Directory, Library bounce, Ontology, Mesh, Pulse) is the chrome FAIL.  
> Cherry-pick joy under Ask · Keep · Talk — no parity theatre, no Host invent.

## Related

- [`HUMAN_SURFACE_VOCAB_FOR_CHROME_WIP.md`](./HUMAN_SURFACE_VOCAB_FOR_CHROME_WIP.md) — Marvin plane vocab for chrome
