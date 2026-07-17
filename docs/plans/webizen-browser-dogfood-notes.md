# Webizen Browser — Dogfood Notes (U2-A)

**Date:** 2026-07-18  
**Branch:** `0.0.25`  
**Track:** U2-A (browser dogfood + trust/cookies honesty polish)  
**Tree:** `C:\Projects\qualia-27062026`  
**Method:** code-level walk of B0–B3 + honesty labels. **Runtime rebuild / live site dogfood remains a human gate.**

---

## B0–B3 checklist (code evidence)

| Phase | Deliverable | Status | Evidence |
|-------|-------------|--------|----------|
| **B0** | Native WebView navigation (no iframe for main web) | **Present** (code) · **runtime unproven this session** | `browser/mod.rs` multi-webview shell; `browser_navigate_content` / content child; Reach opens native window (`browser_panes.rs` honesty copy). |
| **B1** | Chrome: omnibox, back/forward/reload, engine banner | **Present** (chrome UI) · **Partial** (tabs live mainly in Reach mirror) | `browser/chrome.html` toolbar + history; `engine.rs` Servo banner: *“Servo experimental — not linked…; WebView remains active”*; `servo_renders_pages: false` always. |
| **B2** | Trust panel + suggested roots (honest empty) | **Present** | Trust store list/add DID/PEM; suggested catalog `browser_trust_list_suggested` with honest empty (no invented PEMs); Import/Enable; cert-override **not claimed product-active** (status line + copy). |
| **B3** | Cookies side panel | **Present** (UI + jar bridge) · **Partial** (not Chromium parity) | Side panel (not alert): first-party / third-party / coverage_note; Refresh → `browser_cookies_refresh` + graph summary fallback; local schemes honest N/A. |

### Ready checklist (`webizen-browser-ready-checklist.md`) mapping

| # | Item | Code | Human runtime |
|---|------|------|---------------|
| 1 | Rebuild desktop + chrome asset | `ensure_chrome_asset()` writes `webizen-studio/dist/browser-chrome.html` | ⚑ Rebuild on machine |
| 2 | Navigate DuckDuckGo / Google | Commands + content WebView path present | ⚑ Confirm ≥1 major site |
| 3 | Go / back / forward / reload; resize under toolbar | chrome.html + `relayout_browser` | ⚑ Resize + history dogfood |
| 4 | 🔖 → Reach Bookmarks / Library filter | `save_qlink` + Reach bookmarks UI | ⚑ Vault unlock + list |
| 5 | Trust DID → badge; remove reverts | Store + `browser_trust_verdict` | ⚑ Add/remove DID dogfood |
| 6 | Agent summarise / trusted / privacy | `browser_agent_ask` deterministic path | ⚑ Optional; U2-B/U3 for tools |
| 7 | Cookie summary or honest empty | Cookies panel + coverage_note | ⚑ Visit https site then Refresh |

**Honesty (binding):** OS validates ordinary WebView TLS. Custom PEM/DID = agent/policy + badge. **Cert-override is not claimed product-active** as full TLS control (Windows error-path hook / escape hatch may exist; default deny; never auto-allow).

---

## Honesty labels (U2-A polish)

| Surface | Claim allowed | Claim forbidden |
|---------|---------------|-----------------|
| Engine default | OS WebView | Servo as Ready / rendering pages |
| Servo select | Experimental preference; banner when preferred | “Servo browser” product cut |
| Trust store | Your policy + agent HTTPS + badge levels | “WebView uses your PEMs for all TLS” |
| Suggested catalog | Empty until principal curates | Invented PEMs / fake community roots |
| Cert-override | Status: unavailable / disabled / hook path; Partial | “Active product TLS” without principal verification |
| Cookies | Best-effort jar + graph; coverage_note | Complete Chromium cookie parity |

---

## What was polished this track (2026-07-18)

1. **`chrome.html`:** Trust honesty note; live `browser_cert_override_status` line that always says **not claimed product-active**; empty trust store copy; cookies empty-state + local-scheme coverage; status bar Servo-as-preference wording.  
2. **`browser_panes.rs`:** Removed “unless override is active” ambiguity; footer Present/Partial/Experimental strip; cookies default empty + coverage.  
3. **`cookies.rs`:** Local `qualia://` / `webizen://` returns empty cookies arrays + explicit N/A coverage note.  
4. Docs: this file rewritten for U2-A; progress log entry; operator stub.

**No** `commands/mod.rs` edits. **No** PEMs invented. **No** Servo as default.

---

## Human dogfood needs (⚑)

1. Rebuild `webizen-desktop` on the dogfood machine; open Reach → Browser / native window.  
2. Navigate `https://duckduckgo.com/` and one other major site; confirm page paints under chrome.  
3. Open **Trust**: empty suggested catalog text; add a DID you control; badge on matching URL; remove.  
4. Confirm **Cert-override** status line never reads as “full TLS is ours.”  
5. Open **Cookies** after site visit → Refresh; note first/third party or honest none + coverage.  
6. Toggle Engine → Servo: yellow banner; page still WebView; switch back.  
7. Optional: Agent “Is this trusted?” on current URL.

---

## Pass / fail for U2-A

| Criterion | Result |
|-----------|--------|
| B0–B3 checked with evidence in dogfood notes | **Yes** (code-level; runtime = human) |
| Servo banner honesty | **Yes** (`SERVO_NOT_LINKED_BANNER`, `servo_renders_pages: false`) |
| Cert override **not** claimed active as product TLS | **Yes** (UI copy + status line) |
| Suggested import honesty / empty catalog | **Yes** |
| Cookies side panel honesty | **Yes** |

**Not done this session:** live Windows rebuild, real site load, vault bookmark e2e (need principal machine time).

---

## U2-B — Browser agent (v0) shell (2026-07-18)

| Item | Status | Notes |
|------|--------|-------|
| Side panel honesty Partial + Scaffold | **Present** (code) | chrome Agent drawer + Reach “Browser agent” panel |
| List local tools | **Present** | `mcp_list_local_tools` on open / Refresh |
| Allowlist display | **Partial** | Shows seeded/roster allowlist for slug `local`; full editor remains Talk tool card |
| `list_capabilities` two-step Permit | **Present** | Propose → Permit → `mcp_call_tool_gated` (`principalPermitted: true`, args `{}`) |
| Deny never calls | **Present** | Deny clears propose; no MCP invoke |
| No silent tool execution | **Yes** | No auto-call on panel open |
| Cert / Servo claims | **Unchanged** | Still not product-active / experimental pref only |

**Human dogfood (⚑):** rebuild desktop → Agent → Refresh tools → Run list_capabilities on permit → Permit (expect capability list or allowlist error if roster empty) → repeat and Deny (status says no call).
