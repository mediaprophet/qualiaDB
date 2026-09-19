# Command wheel chrome (Timothy lock — 2026-09-19)

## Decision

Bring forward Poet’s **8-sector radial command wheel** (`crates/poet` `radial_menu` / `radial_gesture`, POET-SPEC-011):

- **Right-click** (mouse) or **long-press** (touch/pen) opens the wheel at the cursor.
- Use the same pattern for **volume / window chrome** so we **do not** ship Apple-like red · yellow · green traffic lights as product identity.

## Why

- DESIGN-CONSTRAINTS already forbid classic OSX/Win/Linux chrome as Desktop identity.
- Gate 0 mock `02-window-chrome.html` still showed traffic lights as scaffolding — **retire that**.
- Poet already proved the wheel as sayable, spatial, and accessible (keyboard sectors + Escape).

## Volume sectors (v1)

| Sector | Role |
|--------|------|
| Close | Dismiss volume |
| Soften | Was minimize — soft shrink into orbit, not OS “minimize” theatre |
| Full | Expand into stage |
| Float | Detach / layer |
| Keep | Park in Keep |
| Share | Human share path |
| Inspect | Continuity / who·claim·handle·tool |
| More | Overflow |

Hub glyph **◉** on the titlebar is the discoverable affordance; right-click on the volume still works.

## Implementation notes (Neo)

- Tauri: `decorations: false` (or per-window) so OS traffic lights never paint.
- Shell volumes under `/volumes/*` host the hub + wheel; Desktop native close/soften/full map to window APIs.
- Reuse Poet SVG sector math; don’t invent a second menu language.
- Soft-rise still held for Capt UAT of elevated `/shell` + bare volumes — this chrome lock can land as mock + wire in parallel.

## Mock

`mockups/spatial/04-command-wheel-chrome.html` — interactive.
