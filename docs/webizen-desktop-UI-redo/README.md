# Webizen Desktop UI redo — Gate 0

**Branch:** `0.0.40` · **Status:** shared-sense (plans + mock-ups), not shipped UI

## Order (locked)

1. **Leave Poet** — do not grow a second half-done Poet.
2. **Migrate Webizen Desktop** into an OS-like shell first (dock, windows, launcher).
3. **Then migrate Poet into it** as one launchable app (WASM is the host form, not a rival product).

## Coexistence (locked)

- The **new Desktop is the default**.
- The **existing shell stays in the tree** and remains launchable via a **flag** (or equivalent switch) until the new shell is actually better.
- **Nothing is deleted** on the way through. Retire the old path only after migration is honest and the new default wins on use.

## Hard-fails

- Life-domain chrome (selfhood / relations / memory / care / world / practice) as top-level IA
- Default UX of held / not yet / gated / barrier copy that hides live work
- MCP-only usefulness — native apps and the browser must be findable in the shell

## What Desktop becomes

An operating-system desktop environment:

- Apps launch **in-window** or in a **new window**
- WASM hosts are first-class (Poet migrates in later)
- Native capabilities (renderer, etc.) presentable on apps
- Browser is a visible app again

## Mock-ups

Open [`mockups/index.html`](mockups/index.html) in a browser (no build step).

| File | Purpose |
|------|---------|
| `01-shell-desktop.html` | Wallpaper + menu strip + dock with launchable apps |
| `02-window-chrome.html` | Window chrome; in-window vs new-window |
| `03-app-launcher.html` | Launcher listing real capabilities (Browser included) |
| `04-poet-as-app-tile.html` | Poet as a single future app tile — migrate-in later |

## Migration stub

See [`MIGRATION-NOTES.md`](MIGRATION-NOTES.md). Inventory of hidden surfaces is Marvin’s lane; architecture (window manager + WASM host + legacy flag) is Neo’s.
