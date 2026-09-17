# WD-01 — Control-plane information architecture

**Date:** 2026-09-06  
**Packet:** `WD-01` (Wave 5 — Webizen Desktop control plane)  
**Lane:** Poet Gate B swarm Lane C  
**Branch:** `0.0.36-dev`  
**Status:** Documentation / IA map only — **no shell rewrite in this packet**  
**Authority:** Playbook §14 `WD-01`; programme §7 (`docs/POET_WEBIZEN_HEALTH_PLATFORM_2026-09-04.md`); naming via ADR 0013 (`APP-01`). Desktop lifecycle implementation starts at `WD-02+`.

---

## 1. Purpose

Replace the current **POET / QApp-centric** native navigation story with a general multi-app **control plane**:

| Target section | Short purpose |
|----------------|---------------|
| **Apps** | Install, inspect, launch, stop, update portable apps (POET first; Health proof later). |
| **Node** | Real daemon / process / hardware / Sentinel / log status — never simulated. |
| **Identity & Permissions** | DIDs, vault/sanctuary, app grants, revocation — no private key material in UI. |
| **Assets** | Q42 asset inventory, licences, digests, quarantine, dependency-safe removal. |
| **Connections** | Peers, connectors, transports, sync/relay status — honest unavailable states. |
| **Recovery** | Backups, migrations, failed jobs, quarantined imports, auditable retry. |

Acceptance (playbook): **POET appears under Apps**; **domain logic does not move into Desktop**; **old routes remain reachable during migration**; **no static fake daemon status**.

---

## 2. Current IA inventory (read from tree, 2026-09-06)

Sources (read-only): `crates/webizen-desktop/src/shell/{menu,action,tabs,shell_html}.rs`, tray block in `main.rs`, settings portal routes in `settings_server.rs`.

### 2.1 Native app menu (`shell::menu::build_app_menu`)

| Menu | Item id | Label | Dispatches to |
|------|---------|-------|---------------|
| **File** | `new_window` | New Window | new studio window |
| | `quit_app` | Quit Webizen | quit |
| **View** | `nav_back` / `nav_forward` / `nav_reload` | Back / Forward / Reload | chrome nav |
| | `open_command_palette` | Command Palette… | palette |
| | `toggle_gpu` | Toggle GPU Surface | navigate `gpu-viewport` |
| | `toggle_ambient` | Toggle Ambient Visualizations | ambient |
| | `zoom_in` / `zoom_out` / `reset_zoom` | Zoom… | zoom |
| | `shell_classic` / `shell_poet` | Shell: Classic / Poet | **presentation chrome only** (`SetShellKind`) — not app launch |
| **QApps** | `open_talk` | Talk | navigate `talk` |
| | `open_wellfair` | WellFair | navigate `wellfair` |
| | `open_chora` | Chora | navigate `chora` |
| | `open_browser` | Web Browser | navigate `browser` |
| | `open_10d` | 10D Browser | navigate `10d-browser` |
| | `open_qapp_studio` | QApp Studio | navigate `qapp-studio` |
| | `open_qapp_manager` | Manage QApps… | navigate `qapps` |
| **Tools** | `open_settings` | Settings… | navigate `settings` |
| | `open_diagnostics` | Diagnostics | `OpenDiagnostics` |
| | `open_library` | Hypermedia Library | navigate `library` |
| | `open_wallet` | Wallet | navigate `wallet` → route `/identity` |
| | `open_poet` | **Poet Harness** | navigate `poet` → `/poet` |
| | `import_samsung` | Import Samsung Health… | domain import action |
| | `sync_relay` | Sync with Relay | sync action |
| | `backup` | Backup… | backup action |
| **Help** | `help_about` | About Webizen | about |
| | `help_update` | Check for Updates… | updater |
| | `help_logs` | View Logs | navigate `logs` |
| | `help_portal` | Open Settings Portal | external `http://127.0.0.1:{settings_port}/` |

**Observation:** POET is buried under **Tools** as “Poet Harness”. Primary product surfaces sit under a **QApps** submenu. That is the qApp-centric IA this packet replaces in naming/structure (implementation in later WD packets).

### 2.2 Tray menu (`main.rs` setup)

| Item id | Label | Notes |
|---------|-------|-------|
| `show` | Open Webizen Studio | → navigate `talk` (legacy home) |
| **Sanctuary** | `sanctuary_lock` / `sanctuary_unlock` / `sanctuary_status` | vault |
| **Daemon** | `daemon_status` | label **"Daemon: starting…"** (static initial text) |
| | `daemon_restart` / `daemon_stop` | process actions |
| **Health** | `health_med_reminders` / `health_backup` / `health_diagnostics` | domain admin shortcuts |
| **Sync** | `sync_relay` / `sync_inbox` | connections-adjacent |
| `settings` | Settings | → `settings` |
| `toggle_ambient` | Toggle Ambient Visualization | |
| `revoke` | Revoke Sessions | identity/permissions |
| Help items | same ids as Help menu | |
| `quit` | Quit | |

### 2.3 Shell destination ids → routes (`qapp_route` / `tabs::qapp_url`)

Canonical navigate ids used by `shell-navigate` / `ShellAction::Navigate`:

| Destination id | `qapp_route` path | Studio hash (`qapp_url`) where defined | Title / note |
|----------------|-------------------|----------------------------------------|--------------|
| `talk` / `dashboard` / `home` | `/` | `/studio/#/` | Talk (home); dashboard/home are **legacy aliases** |
| `wellfair` | `/wellfair` | `/studio/#/wellfair` | WellFair |
| `chora` | `/chora` | `/studio/#/chora` | Chora |
| `browser` | `/browser` | `/studio/#/browser` | Browser / Reach |
| `10d-browser` | `/10d-browser` | `/studio/#/10d-browser` | 10D Browser |
| `settings` | `/settings` | `/studio/#/settings` | Settings |
| `library` / `memory` | `/library` | (custom hash path) | Library; `memory` alias |
| `wallet` | `/identity` | (custom) | Wallet → identity path |
| `qapp-studio` | `/qapp-studio` | `/studio/#/qapp-studio` | QApp Studio |
| `qapps` | `/qapps` | `/studio/#/qapps` | QApps catalog/manager |
| `render-preview` | `/render-preview` | `/studio/#/render-preview` | |
| `anatomy` | `/anatomy` | | |
| `health` | `/health` | | Health surface route exists |
| `tools` | `/tools` | | |
| `sanctuary` | `/sanctuary` | | |
| `logs` | `/logs` | | Desktop logs |
| `poet` / `vibe` | `/poet` | custom `#/poet` via catch-all | POET; `vibe` alias |
| `gpu-viewport` | `/gpu-viewport` | `/studio/#/gpu-viewport` | |
| `nexus` | (fallback `/` in `qapp_route`) | `/studio/#/nexus` | **Mismatch:** `tabs::qapp_url` knows `nexus`; `qapp_route` falls through to `/` |
| `about` | (fallback `/`) | `/studio/#/about` | same class of mismatch |
| `anatomy-test` | (fallback `/`) | `/studio/#/anatomy-test` | test surface |
| `keep` | (fallback `/`) | used in palette | palette destination; route via catch-all studio path |

**Unknown / honest gaps:**

- `qapp_route` unknown ids silently map to `/` — easy to hide broken destinations.
- Menu test `every_native_destination_has_an_explicit_route` does **not** assert `poet` / `vibe` even though `qapp_route` defines them.
- Palette omits `poet`, `wallet`/`identity`, `wellfair`, `chora`, daemon/node entries.

### 2.4 Command palette (`shell_html.rs` `PALETTE_ITEMS`)

| id | label |
|----|-------|
| `talk` | Talk |
| `browser` | Browser (Reach) |
| `10d-browser` | 10D / Infosphere |
| `settings` | Settings |
| `library` | Library |
| `qapps` | QApps |
| `keep` | Keep |
| `logs` | Desktop logs |

### 2.5 Settings portal HTTP routes (`settings_server.rs` excerpt)

Portal serves studio SPA under many paths still named for legacy catalogue surfaces, including: `/`, `/dashboard`, `/qapps`, `/library`, `/tools`, `/nexus`, `/communications`, `/identity`, `/agency`, `/sanctuary`, `/work`, `/anatomy`, `/clinical`, `/chora`, `/qapp-studio`, `/10d-browser`, `/gpu-viewport`, `/render-preview`, `/settings`, `/health`, `/logs`, `/desktop-logs`, `/shell`, `/about`, plus APIs `/api/status`, `/api/logs`, `/api/health`.

These remain **reachable during migration**; control-plane IA does not delete them.

### 2.6 Fake / proxy daemon status (honesty finding)

In `shell_html.rs` `updateStatus()`:

- On **any** successful `invoke('get_hardware_status')`, the chrome sets **`Daemon: running`**.
- On invoke failure, **`Daemon: off`**.

That equates hardware-status invoke success with daemon liveness — **not** a real daemon supervisor probe. Tray also seeds `daemon_status` as **"Daemon: starting…"**.  
**WD-01 rule for implementers:** treat this as a known violation of “no static / fake daemon status”; **WD-04** must replace it with real process state or an explicit unavailable label — never keep this proxy.

---

## 3. Target IA — six control-plane sections

Aligned with programme architecture §7 and playbook §14. Desktop remains a **host + admin plane**; domain workflows stay in portable apps / core capabilities.

### 3.1 Apps

**Purpose:** Registry and lifecycle of portable applications (ADR 0013 / `APP-02+`).  
**Contains:** installed/bundled manifests; integrity/compatibility summary; launch/stop/update/uninstall (via `WD-03`); permission-intent preview (read-only until `WD-05`); **POET** as first-class entry; later **Health** focused app (`WD-08` / `APP-04`).  
**Does not contain:** POET authoring/manifolds, clinical workflows, or health domain UI (those stay in the app projection).

### 3.2 Node

**Purpose:** Local Qualia node operations.  
**Contains:** daemon start/stop/restart; real running/crashed/offline; bounded redacted logs; CPU/RAM/GPU/Sentinel/thermal **only where real providers exist** (`WD-04`).  
**Does not contain:** fabricated gauges, sampled “live” tiles, or hardware-success → “daemon running” shortcuts.

### 3.3 Identity & Permissions

**Purpose:** Principal and app authority.  
**Contains:** sanctuary/vault status; session revoke; wallet/identity surfaces; app grants/denials/expiry/revocation UI (`WD-05`).  
**Does not contain:** private key export/import unless a separate reviewed packet says so; presentation hints must never grant authority (APP-03).

### 3.4 Assets

**Purpose:** Governed Q42 / package assets on disk.  
**Contains:** versions, sizes, digests, licence obligations, validation/quarantine, dependent apps, safe-removal eligibility (`WD-06`).  
**Does not contain:** automatic remote download; labelling unknown licence as unrestricted.

### 3.5 Connections

**Purpose:** Transports and peers.  
**Contains:** sync/relay, sync inbox, connector/peer status, honest “unavailable” (`WD-07`).  
**Does not contain:** invented transport health when the backend has no probe.

### 3.6 Recovery

**Purpose:** Repair and continuity.  
**Contains:** backup, migrations already supported by backend, failed jobs, quarantined imports, auditable retry (`WD-07`).  
**Does not contain:** silent side-effect retries for unknown operations.

### Chrome that stays chrome (not a sixth “product” section)

File / View navigation, zoom, ambient, command palette, Help About/Updates, and **Shell: Classic|Poet** remain host chrome. `shell_poet` ≠ launching the POET app under Apps.

---

## 4. Mapping table — old → new (migration-reachable)

**Convention:** During migration, keep existing menu ids, navigate destination ids, and HTTP paths working. New section labels wrap or re-home them; do not break `ShellAction::from_id` or `qapp_route` aliases until a dedicated cleanup packet.

| Old surface (id / label / path) | Target section | Migration note |
|---------------------------------|----------------|----------------|
| **QApps** submenu | **Apps** | Rename IA to Apps; keep ids `open_*` until WD-02/03 rewire |
| `open_qapp_manager` / `qapps` / `/qapps` | **Apps** | Becomes installed-app list / inspect (`WD-02`) |
| `open_qapp_studio` / `qapp-studio` | **Apps** (legacy authoring) or deprecate label | Reachable; prefer portable-app inspect over “QApp Studio” naming (ADR 0013) |
| `open_talk` / `talk` / `/` | **Apps** (Talk app) *or* home chrome | Keep as default home tab; not control-plane core |
| `open_wellfair` / `wellfair` | **Apps** | Hosted app entry, not Desktop domain logic |
| `open_chora` / `chora` | **Apps** | Same |
| `open_browser` / `browser` | **Apps** or embedded browser chrome | Spec still allows embedded browser; do not fold browser admin into domain apps |
| `open_10d` / `10d-browser` | **Apps** | Same |
| `open_poet` / `poet` / `vibe` / `/poet` / “Poet Harness” | **Apps → POET** | **Primary relocate:** label “POET”; leave under Apps, not Tools |
| `shell_poet` / Shell: Poet | *(chrome)* | Unchanged meaning: presentation shell kind |
| Palette `qapps` | **Apps** | Relabel to Apps; add POET palette row in WD-02+ UI pass |
| Tray **Daemon** / `daemon_*` | **Node** | Real status in WD-04; remove proxy/fake labels |
| `help_logs` / `logs` / `/logs` / `/desktop-logs` | **Node** | Node workspace log view |
| `open_diagnostics` / `health_diagnostics` | **Node** (host) vs Health app | Split: host diagnostics → Node; clinical diagnostics stay out of Desktop domain |
| `open_settings` / `settings` / Help portal | Split: Node prefs / Identity / Connections as they land | Portal URLs remain reachable |
| `open_wallet` / `wallet` → `/identity` | **Identity & Permissions** | Relabel away from “Wallet” over time |
| Sanctuary tray items | **Identity & Permissions** | |
| `revoke` | **Identity & Permissions** | |
| `/agency`, `/sanctuary` portal paths | **Identity & Permissions** | Reachable |
| `open_library` / `library` / `/library` | **Assets** (hypermedia/models) *interim* | Full asset manager is WD-06; library may remain an Apps or Assets child |
| `import_samsung` | **Recovery** (import) + **Health app** (domain) | Desktop may *trigger* import job; clinical UX stays in Health app |
| Tray **Health** med reminders | **Apps → Health** (later) | Do not expand health domain into Desktop shell |
| `backup` / `health_backup` | **Recovery** | |
| `sync_relay` / `sync_inbox` | **Connections** | |
| `/communications`, `/nexus` | **Connections** (interim) | Reachable; exact peer UI TBD WD-07 |
| `/clinical`, `/health`, `/anatomy` | **Apps → Health** (later proof) | Routes stay; Desktop does not own domain logic |
| `render-preview`, `gpu-viewport`, `tools` | Host/dev chrome or Apps tooling | Keep reachable; not core six unless they become real admin tools |

---

## 5. Where POET sits

| Question | Answer |
|----------|--------|
| Target home | **Apps → POET** (bundled portable app; first registry fixture per `WD-02`) |
| Today | Tools → `open_poet` (“Poet Harness”); also `shell_poet` chrome; route `/poet` |
| Launch path (future) | Same registry/lifecycle as any app (`WD-03`, `WD-08`) — **no POET-only launch branch** |
| Authoring | Remains inside POET (manifolds/containers/toolbox) — not moved into Desktop |
| Health proof | Later under **Apps → Health** (`WD-08` / `APP-04`); not a Desktop “Health product menu” expansion |

ADR 0013: Desktop is a **launch/inspect projection** of the portable application contract; POET is one projection among peers.

---

## 6. Non-goals (this packet and IA constraints)

1. **No product Rust changes** in WD-01 — no menu rewrite, no route deletes, no Host widen.
2. **No domain logic in Desktop** — Health clinical flows, POET authoring, econ/bio workflows stay in apps/core; Desktop hosts and administers.
3. **No static fake daemon status** — do not ship new chrome that paints “running” without a real supervisor; retire the hardware→daemon proxy in WD-04.
4. **No Host widen / invented `Family.method` IDs** (Gate B freeze).
5. **No new competing portable-app ABI** — APP-01/0013 naming; ABI is `APP-02+`.
6. **No full ADR** for this IA map — APP-01 already covers naming; this WIP is the navigation plan.
7. **Do not break old routes** during migration — aliases (`dashboard`/`home`→`talk`, `vibe`→`poet`, `memory`→`library`) stay until an explicit deprecation packet.

---

## 7. Ordered implementation notes — WD-02 … WD-08

Dependency order for implementers (docs → code in later lanes):

| Packet | Depends on | IA obligation |
|--------|------------|---------------|
| **WD-02** Installed-app registry | APP-02/03 contracts as available; this IA | Expose read-only Apps list; **POET first bundled fixture**; quarantine malformed packages; do not execute while inspecting. Menu can still say “QApps” temporarily if registry backend is real. |
| **WD-03** App lifecycle supervisor | WD-02 | Install/launch/stop/update/uninstall with receipts; allowlisted launch; **POET launch goes through supervisor**, not a special Tools path forever. |
| **WD-04** Node workspace | Independent of Apps UI, but should land before claiming control-plane honesty | Replace tray/chrome daemon labels with real process state; remove `get_hardware_status`→“Daemon: running”; unsupported telemetry = absent/unavailable. |
| **WD-05** Identity & permissions | WD-02 (app ids/versions) | Grants bound to app ID/version/principal/scope; relocate Wallet/Sanctuary/Revoke under Identity IA; no key material. |
| **WD-06** Asset manager | Registry + Q42 asset projections | Library/assets under Assets section; licence honesty; no auto-download. |
| **WD-07** Connections + Recovery | Node honesty helpful | Relocate Sync/Backup/import job history; honest transports; auditable retry. |
| **WD-08** POET + Health hosting proof | WD-02–05 (+ APP Health packaging) | Both apps under Apps with real lifecycle + permission state; **no POET-only branch**; Health proof is Apps entry, not Desktop Health domain expansion. |

**Suggested UI migration sequence (still not this packet):**

1. Add empty/honest section shells (Apps, Node, …) that deep-link existing routes.  
2. Move **POET** menu entry under Apps (keep `open_poet` id).  
3. Relabel QApps → Apps when WD-02 list is real.  
4. Node honesty pass (WD-04) before marketing “control plane complete.”  
5. Fold tray Health/Sync into Apps/Connections/Recovery without deleting ids.  
6. Palette: add POET + section jump targets; keep old ids.

---

## 8. Acceptance checklist (playbook `WD-01`)

- [x] Target navigation is **Apps, Node, Identity & Permissions, Assets, Connections, Recovery** (documented).
- [x] **POET appears under Apps** in the target IA (explicit §5); current Tools placement recorded for migration.
- [x] **Domain logic does not move into Desktop** (non-goals + Health/POET authoring stay in apps).
- [x] **Old routes remain reachable during migration** (mapping table + portal/route inventory).
- [x] **No static fake daemon status** called out; WD-04 owns the fix; WD-01 does not invent a fake replacement.
- [x] Concrete menu/command/destination ids cited from `webizen-desktop` tree.
- [x] No shell/menu product code rewritten in this lane.
- [ ] *Deferred to WD-02+ / UAT:* shell/menu tests + visual UAT after implementation (playbook Verify line) — **not claimed done here**.

---

## 9. References

- Playbook: `docs/POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md` §14 WD-*  
- Programme: `docs/POET_WEBIZEN_HEALTH_PLATFORM_2026-09-04.md` §7  
- Desktop spec: `docs/poet/08_DESKTOP_ADMIN_HOST_SPEC.md`  
- ADR 0013 / APP-01: `docs/manuals/adr/0013-portable-application-manifest-reconciliation.md`  
- Swarm: `docs/work-in-progress/POET_GATE_B_SWARM_2026-09-06.md`  
- Tree: `crates/webizen-desktop/src/shell/*`, `main.rs` tray, `settings_server.rs`

---

## 10. Session note

Lane C deliverable is this IA map only. Parent integrates register/ledger. **Do not commit** from this lane unless the owner asks.
