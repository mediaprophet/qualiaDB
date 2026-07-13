# Webizen Browser — progress log

Workstream: build our own browser (native webview), our own trust store (user-controlled
trust anchors), and its own agent code. Plan: `docs/plans/webizen-browser-and-trust.md`.
Honest engineering record (CLAUDE.md §9): real results, caveats, and where the human is needed.

---

## 2026-07-09 — P0: native webview substrate — **done (compile-verified); runtime pending Timothy's test**

**Trigger.** Testing surfaced: typing `https://www.google.com` in the web-browser pane
showed a blank page. Diagnosis (confirmed, not a wiring bug): the pane renders in an
`<iframe>`, and Google — like most major sites — sends `X-Frame-Options: SAMEORIGIN`, which
forbids iframe embedding. `submit_omnibox_query` **is** registered and there is **no**
app-side CSP; the remote site is refusing, so no iframe-path change can fix it.

**What was built.**
- `crates/webizen-desktop/src/commands/mod.rs` — new Tauri command `open_web_url(app, url)`:
  parses the URL and opens it in a **single reusable native webview window** labelled
  `webizen-browser` (reuse + `WebviewWindow::navigate` if it already exists, else
  `WebviewWindowBuilder::new(.., WebviewUrl::External(url))`). A native webview does a
  *top-level* navigation, not subject to framing policies, so real sites load. Registered in
  `tauri::generate_handler!`.
- `crates/webizen-studio/src/components/browser_panes.rs` — omnibox `submit_query` now routes:
  `http(s)://` → `invoke("open_web_url")` (native window); `qualia://`/`webizen://` → the
  in-pane semantic iframe (unchanged). The iframe is now the *semantic-web preview*; the
  native window is the real browser.
- Announced in `coordination/NOTICES.md` (CLAIM); full architecture in
  `docs/plans/webizen-browser-and-trust.md`.

**Measured results.**
- `cargo check -p webizen-desktop` → **0 errors** (16.88s) — confirms `open_web_url`,
  `WebviewWindow::navigate` (`&self`), and the handler registration.
- `cargo check -p webizen-studio --bins --target wasm32-unknown-unknown` → **0 errors**
  (17.39s) — confirms the omnibox routing compiles in the wasm UI bin (the target that
  gives false-green on `--lib`).
- Full app rebuild **completed clean**: `dx build --release --web` exit 0 (wasm `[229/229]`),
  stage exit 0, `cargo build -p webizen-desktop --release` exit 0 (5m 22s). Exe rebuilt
  **72.9 MB at 14:00** (`target/release/webizen-desktop.exe`). **Runtime behaviour (does
  google.com actually load in the native window) = still not verified — Timothy to test in the
  windowed app.**

**⚑ Where I need the human.**
- **Runtime confirmation:** after the build lands, type `https://www.google.com` in the
  browser pane and confirm a native Webizen browser window opens and loads Google.
- **Three design decisions** captured in the plan §6: (1) repo home — does the standalone
  browser eventually live in `C:\Projects\webizen-browser` or stay inside `webizen-desktop`?
  (2) the *default suggested* trust set / any community-jurisdictional bundle we ship (a
  curation call that is yours); (3) how far P4 goes (per-webview cert hook vs. full local
  TLS-terminating proxy).

**Next step.** P0.1 — give the `webizen-browser` window our own chrome (omnibox, back/forward,
reload, trust-indicator slot, agent-sidebar slot) so it's recognisably Webizen's, not a bare
OS frame. Then P1 (own trust store).
