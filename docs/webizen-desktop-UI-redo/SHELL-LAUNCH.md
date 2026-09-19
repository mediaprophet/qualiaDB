# Shell launch — new OS shell (default) vs legacy

**Branch:** `0.0.40-webizen-ui` · **Crate:** `webizen-desktop`  
**Constraint docs:** [`DESIGN-CONSTRAINTS.md`](DESIGN-CONSTRAINTS.md) · [`MIGRATION-NOTES.md`](MIGRATION-NOTES.md) · [`DESKTOP_SHELL_ONTOLOGY_PASS.md`](DESKTOP_SHELL_ONTOLOGY_PASS.md)

## Default (new OS shell)

Cold start loads the **spatial OS-shell scaffolding** (not the Studio life-domain chrome):

- URL: `http://127.0.0.1:{settings_port}/os-shell`
- Code: `crates/webizen-desktop/src/shell/os_shell.rs` (`OS_SHELL_HTML`)
- Mode resolve: `crates/webizen-desktop/src/shell/launch.rs`

### Default favorites (Live / findable — no agent required)

**Talk · Mail · Directory · Browser · Keep · Library · Instruments · Settings · Admin** (Gate 1 spatial frame · monet tip `6fff620`)

- Browser → `/browser`
- Keep → `/keep`
- Library → `/library` (hypermedia)
- Instruments → `/tools` (models / LLM)
- Settings → `/settings`
- Admin → `/admin` (ops; humans without an agent)


**QApps are parked** — not on the default rail or launcher primary list. Poet stays a Planned tile (migrate-in later); not deleted.

GPU viewport region is a hook toward WGPU/native attach, not classic dock identity. Live vs Planned honesty only.

```bash
cargo run -p webizen-desktop
```

## Legacy Studio shell

The existing bundled Tauri Studio shell stays **in-tree** and launchable until the new shell is actually better. Nothing is deleted.

Either:

```bash
WEBIZEN_LEGACY_SHELL=1 cargo run -p webizen-desktop
```

or:

```bash
cargo run -p webizen-desktop -- --legacy-shell
```

`--legacy-desktop` is accepted as an alias. Env truthy: `1`, `true`, `yes`, `on` (case-insensitive).

## What each path shows

| Mode | Main window | How |
|------|-------------|-----|
| **Default / OS shell** | Spatial scaffolding at `/os-shell` | `window.location.replace` after settings port is known |
| **Legacy** | Bundled Studio (`frontendDist` → `webizen-studio/dist`) | No replace; Studio stays on `main` |

The tabbed `/shell` HTML (`shell_html.rs`) is unchanged — it is **not** the legacy flag target. Legacy = Studio life-domain chrome.

## Continuity

- handle ≠ human  
- chatbot = tool  
- No held / not-yet theatre as the product surface  
- MCP/agent-only usefulness = FAIL — humans must open Desktop and use apps without an agent  
- Leave Poet as a separate product this Gate; tile only  

## Gate 0 honesty

This tip is **scaffolding**: chrome + launch switch + findable Live app stage. Full window manager and WGPU attach remain Planned — labelled as such in the shell.
