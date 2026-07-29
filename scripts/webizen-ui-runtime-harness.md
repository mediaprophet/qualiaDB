# Webizen UI Runtime Harness (U6-C)

**Purpose:** Scripted dogfood path for the product cut: launch → Local backend → activate model → prompt → assert stream.  
**Branch:** `0.0.25` · tree `C:\Projects\qualia-27062026`  
**Companion checklist script:** [`webizen-ui-runtime-harness.ps1`](./webizen-ui-runtime-harness.ps1) (compile-verified + manual steps)

This is a **harness notes + checklist** document. Full automation of GPU model load and token stream assertion requires a machine with a GGUF, VRAM, and a built desktop package — when those are missing, mark **Blocked** and still record compile-verified items from U1–U6.

---

## 0. Preconditions

| Check | How |
|-------|-----|
| Canonical tree | `cd C:\Projects\qualia-27062026` |
| Disk free | ≥ 5 GB before desktop rebuild |
| Optional GGUF | Principal-chosen Local model path (not invent one) |
| Commands lock | Do not edit `commands/mod.rs` while another agent owns it |

---

## 1. Compile-verified gate (always run)

```powershell
cd C:\Projects\qualia-27062026
cargo check -p webizen-desktop -p webizen-studio
cargo test -p webizen-studio --bin webizen-studio command_palette virtualized_list conduct_banner -- --nocapture
```

Record: pass / fail + date. **Compile-green ≠ runtime-done.**

### U1–U6 compile / unit surface (what this wave expects green)

| Track | What to verify without a model |
|-------|--------------------------------|
| **U1-A** | `wellfair_get_llm_telemetry` compiles; harness UI builds; no static fake tok/s in source claims |
| **U1-B** | `conduct_banner` unit tests; Talk mounts banner |
| **U1-C** | `honesty_chip` present on Talk / 10D headers |
| **U1-D** | Manuals exist: architecture, event catalogue, talk-and-agents |
| **U2-A** | Browser chrome/dogfood notes; honesty on trust/cookies |
| **U3** | Tool loop — owned elsewhere; skip if not released |
| **U4-A** | 10D empty states + citable copy compile |
| **U4-B/C** | Vision / library — if released |
| **U5** | SPARQL / compute panes — if released |
| **U6-A** | Command palette destinations ≥5; Ctrl+K path in shell + studio |
| **U6-B** | Event catalogue filled; `virtualized_list::window_range` tests |
| **U6-C** | This harness + progress log entry |

---

## 2. Launch path (manual / semi-auto)

1. Build or run desktop (when packaging is available):  
   `cargo run -p webizen-desktop` **or** packaged `webizen` exe from dist.  
2. Confirm shell loads Talk as home (`qualia://talk` / studio `/`).  
3. **Command palette smoke:**  
   - Press **Ctrl+K** (or **Ctrl+P**, or View → Command Palette…).  
   - Open each of: Talk, Browser, 10D / Infosphere, Settings, Library (or QApps).  
   - Esc closes; filter “settings” jumps Settings.  
4. Studio-only: same hotkey inside the studio iframe / webview if focus is in Studio.

---

## 3. Local backend + model activation (needs model)

| Step | Action | Assert |
|------|--------|--------|
| 3.1 | Settings → inference backend **Local** (not Ollama default) | HUD / settings show Local |
| 3.2 | Model Hub / discover models → select GGUF | List non-empty or honest empty |
| 3.3 | Activate model | Lifecycle → Active (or honest error); telemetry VRAM changes or “no live counters yet” |
| 3.4 | Talk → new session → prompt e.g. “Reply with one short sentence.” | Status not silent fail |
| 3.5 | Watch stream | UI receives **`chat-token`** deltas; bubble grows |
| 3.6 | End of turn | **`chat-done`** fires; committed reply or **visible** `block_reason` / ConductBanner |
| 3.7 | Optional deny dogfood | Force gate deny if available → banner, not silent |

**Without model:** mark steps 3.2–3.6 **Blocked — Needs model**. Do not fake stream success.

---

## 4. Optional Ollama (honesty only)

- Settings: Ollama URL + probe.  
- Copy must say optional bridge, **not** the Qualia engine.  
- If used: harness records backend label = Ollama; does not replace Local excellence path.

---

## 5. Event catalogue smoke (devtools)

When desktop is running with DevTools:

1. Open Talk; send a turn with model Active.  
2. Confirm `chat-token` then `chat-done` (or only `chat-done` on remote/agent path).  
3. Deny path: banner from `block_reason` even if `conduct-violation` is not emitted.  
4. Shell: menu Navigate Talk → Browser → `shell-navigate` handled.  
5. Telemetry: polling `wellfair_get_llm_telemetry` returns backend + lifecycle; tok/s only after measured turn.

See [`docs/manuals/webizen-ui-event-catalogue.md`](../docs/manuals/webizen-ui-event-catalogue.md).

---

## 6. Result template (paste into progress log)

```text
Date:
Compile check: pass/fail
Unit (palette/virt/conduct): pass/fail
Model available: yes/no (path if yes)
Palette keyboard: pass/fail
Stream tokens: pass / blocked-no-model / fail
Conduct visible on deny: pass / not exercised / fail
Notes:
```

---

## 7. Out of scope

- Full Playwright e2e against every QApp  
- Inventing GGUF downloads without principal consent  
- Servo as default engine  
- GPUI product shell  
- Editing `commands/mod.rs` from U6
