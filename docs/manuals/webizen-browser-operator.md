# Webizen Browser — Operator Stub

**Status:** stub (accurate to code as of U2-A, 2026-07-18)  
**Branch:** `0.0.28`  
**Detail plans:** [`webizen-browser-and-trust.md`](../plans/webizen-browser-and-trust.md), [`webizen-browser-dogfood-notes.md`](../plans/webizen-browser-dogfood-notes.md), [`servo-experimental.md`](../plans/servo-experimental.md)

---

## What you open

| Surface | Role |
|---------|------|
| **Native browser window** (`webizen-browser`) | In-window chrome + OS WebView content child. Real https pages load here. |
| **Reach → Browser** (Studio) | Mirror: tabs, omnibox, Focus, Trust, Cookies, Bookmarks. Not an iframe for external sites. |

Default home: **Chora universe** (`qualia://chora/universe`), not a search engine.

---

## Engine

| Preference | Reality |
|------------|---------|
| **OS WebView (default)** | Product path. WebView2 / WKWebView / WebKitGTK via Tauri. |
| **Servo (experimental)** | Preference + banner only in this build. **Does not render pages.** WebView stays active. |

---

## Trust store

- Path: `{storage}/webizen/trust_store.json`
- Add **DID** or **PEM** (you supply material — software invents none).
- **Suggested** section: empty until principal curates; Import (disabled) / Enable (import + on).
- Badge levels (policy + agent): `os_default`, `did_match`, `local_scheme`, `custom_root_available`, `untrusted_policy`, `unknown`.
- **Ordinary WebView TLS** still uses the **OS** trust store.
- **Cert-override:** not claimed as product-active full TLS control. On Windows, an error-path hook may attach (`ServerCertificateErrorDetected`); default deny; never auto-allow. Query `browser_cert_override_status` or the Trust panel status line.

---

## Cookies

- Side panel (chrome + Reach): first-party, third-party, **coverage_note**.
- Refresh pulls WebView jar via Tauri `cookies_for_url` into a local transparency graph.
- **Best-effort** — not complete Chromium parity.
- Local `qualia://` / `webizen://`: no HTTP cookies (honest N/A).

---

## Agent (thin + U2-B MCP shell)

Chrome **Agent** drawer:

1. **Page Q&A** — `browser_agent_ask` (deterministic / grounded path; not MCP).
2. **Browser agent (v0) MCP shell** — lists local tools (`mcp_list_local_tools`); golden path **Run list_capabilities on permit** is two-step Propose → **Permit** / **Deny**. Permit alone calls `mcp_call_tool_gated` (`agent=local`, `list_capabilities`, `{}`). Deny never calls. Full allowlist editor remains **Talk → MCP tools card** (U3).

---

## Dogfood

Use [`webizen-browser-ready-checklist.md`](../plans/webizen-browser-ready-checklist.md) and record results in dogfood notes. Compile-green ≠ runtime-proven.
