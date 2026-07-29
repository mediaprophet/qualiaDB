# Webizen UI Event Catalogue

**Status:** living catalogue (U1-D stub → U6-B filled)  
**Surfaces:** Tauri host (`webizen-desktop`) → Studio / shell webviews  
**Rule:** Prefer typed, documented events over ad-hoc string emits. Deny/Forbid always surface (banner or toast). Do **not** invent a second HTTP LLM event bus.

Polling fallback where noted: Studio may call `wellfair_get_llm_telemetry` ~every 500ms for harness HUD when a stream is absent.

---

## 1. Chat / inference stream

| Event | Direction | Payload (sketch) | Purpose | Emit site (approx.) | Listen site |
|-------|-----------|------------------|---------|---------------------|-------------|
| **`chat-token`** | host → UI | `{ session_id: string, delta: string }` | Stream generation tokens into the in-progress agent bubble | `stream_chat_inference` (`commands/mod.rs`) per-token callback | `connect_chat.rs` |
| **`chat-done`** | host → UI | `{ session_id, committed: bool, result: ChatInferenceResult }` | End of turn; may carry `block_reason` / `shield_alert` inside `result` | `stream_chat_inference` (local + remote agent paths) | `connect_chat.rs` → `conduct_banner` parsers |
| **`conduct-violation`** | host → UI (optional) | `{ reason: string, summary?: string }` or nested intent summary | Explicit deontic / gate deny that **must not be silent** | Host emit **optional** today (UI ready); WAL may still record | `connect_chat.rs` + `conduct_banner::notice_from_conduct_violation` |

**Conduct note:** Even without a `conduct-violation` emit, `chat-done.result.block_reason` / `shield_alert` and invoke errors feed `ConductBanner`. Silent failure is forbidden.

**Cancel:** `cancel_chat_inference` is a command (not an event); cooperative flag checked mid-decode.

---

## 2. Shell routing & chrome

| Event | Direction | Payload | Purpose | Emit | Listen |
|-------|-----------|---------|---------|------|--------|
| **`shell-navigate`** | host → UI | `qapp_id: string` (e.g. `talk`, `browser`, `10d-browser`) | Open/switch product surface | `menu.rs` `navigate_main_to` | `shell_html.js` + Studio `AppLayout` |
| **`shell-open-command-palette`** | host → UI | `()` | Open U6 command palette | `ShellAction::OpenCommandPalette` | `shell_html.js` |
| **`shell-new-tab`** | host → UI | `()` | New Talk tab | (emitters as added) | `shell_html.js` |
| **`shell-close-tab`** | host → UI | `()` | Close active tab | | `shell_html.js` |
| **`shell-nav-back`** | host → UI | `()` | History back | menu dispatch | shell + Studio history |
| **`shell-nav-forward`** | host → UI | `()` | History forward | menu | shell + Studio |
| **`shell-nav-reload`** | host → UI | `()` | Reload content | menu | shell + Studio |
| **`shell-toggle-gpu`** | host → UI | `()` | Toggle GPU overlay | | `shell_html.js` |
| **`shell-import-samsung`** | host → UI | `()` | Import flow handoff | menu | Studio |
| **`shell-view-logs`** | host → UI | `()` | Open logs | | Studio |
| **`shell-zoom-in` / `shell-zoom-out` / `shell-reset-zoom`** | host → UI | `()` | Zoom (shell may also `eval` CSS zoom) | menu | Studio (debug) |
| **`open-settings`** | host → UI | `()` | Jump Settings | tray/menu | Studio → `SettingsRoute` |
| **`open-backup`** | host → UI | `()` | Backup UI | menu | Studio |
| **`open-sanctuary-unlock` / `open-sanctuary-status`** | host → UI | `()` | Vault UX | tray | Studio |
| **`sanctuary-locked`** | host → UI | `()` | Lock confirmation | commands path | Studio |
| **`open-med-reminders`** | host → UI | `()` | Health reminders | tray | Studio → Health |
| **`open-sync-inbox`** | host → UI | `()` | Sync inbox | tray | Studio |
| **`diagnostics-result`** | host → UI | diagnostics JSON | Menu diagnostics completed | `OpenDiagnostics` | Studio → Tools |

**Command palette destinations (shell + studio):** Talk, Browser (Reach), 10D / Infosphere, Settings, Library, QApps (+ Keep, Logs). Hotkeys: **Ctrl+K** / **Ctrl+P** (Cmd on macOS). Menu: View → Command Palette…

---

## 3. Telemetry & runtime

| Event | Direction | Payload (sketch) | Purpose | Notes |
|-------|-----------|------------------|---------|-------|
| **`hardware-telemetry`** | host → UI | GPU / adapter / thermal sketch | Live hardware HUD | Emitted from desktop `main.rs` telemetry bridge |
| **`system-telemetry`** | in-process / CLI | `SystemTelemetryEvent` (lifecycle, VRAM, activation) | Activation HUD during model load | Bus in `qualia_client_core::system_telemetry`; **not** a default Tauri string event today. Studio HUD uses **`wellfair_get_llm_telemetry`** polling for backend / lifecycle / last measured tok/s |
| **`webizen-runtime-ready`** | host → UI | `()` | Runtime init OK | desktop `main.rs` |
| **`webizen-runtime-failed`** | host → UI | error string | Runtime init failed | desktop `main.rs` |
| **`webizen-host-api-ready`** | host → UI | `()` | Host API ready | desktop `main.rs` |
| **`updater-progress`** | host → UI | download progress fields | Update UI | commands updater path; `updater_panel.rs` |

---

## 4. Render / GPU / anatomy

| Event | Direction | Payload | Purpose |
|-------|-----------|---------|---------|
| **`render-preview-ready`** | host → UI | path / digest optional | GPU / 10D preview ready |
| **`gpu-surface-mounted` / `gpu-surface-unmounted` / `gpu-surface-stopped`** | host → UI | `()` | Native surface lifecycle |
| **`anatomy-body-ready`** | host → UI | `()` | Body pack ready |
| **`anatomy-acquire-progress`** | host → UI | progress struct | Pack acquire |
| **`anatomy-acquire-done`** | host → UI | report | Pack acquire finished |
| **`diffusion-epoch-ready`** | host → UI | epoch record | Diffusion visualizer |

---

## 5. Agent / tool events (U3)

| Event / channel | Direction | Payload (sketch) | Purpose | Status |
|-----------------|-----------|------------------|---------|--------|
| Tool propose / permit / result | UI state + commands | tool name, args, allowlist decision, result card | MCP allowlisted tool loop | Owned by **U3** — names stabilize in that track; do not invent parallel event bus here |
| MCP inspector invoke | command | tool + args | Debug pane | `mcp_inspector` |

When U3 lands fixed event names, append them here without renaming `chat-token` / `chat-done`.

---

## 6. Rules

1. **Prefer documented events** — new host→UI names go in this table the same session.  
2. **Deny/Forbid never silent** — banner or toast (`conduct_banner` / Talk status).  
3. **No second LLM HTTP event bus** — Qualia Local streams via Tauri emits; Ollama is Settings opt-in only.  
4. **Honesty:** missing live counters → “no live counters yet”, never fake tok/s.  
5. **Shell vs Studio:** outer chrome is `shell_html`; product routes are Studio. `shell-navigate` bridges both.

---

## 7. Related manuals

- [UI architecture](./webizen-ui-architecture.md)  
- [Talk and agents](./talk-and-agents-ui.md)  
- [Browser operator](./webizen-browser-operator.md)  
- [Vision 10D UI](./vision-10d-ui.md)  
- Runtime harness: [`scripts/webizen-ui-runtime-harness.md`](../../scripts/webizen-ui-runtime-harness.md)
