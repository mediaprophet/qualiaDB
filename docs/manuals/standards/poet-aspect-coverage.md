# Poet surface aspects (Layout · Stage · Timeline)

**Packet:** davinci Stage 7 · **Term:** **aspects** — not twins, not a credential “digital twin”.

Every shipped Poet surface has three 1:1 **aspects**:

| Aspect | Role | Beat |
|--------|------|------|
| Layout | 2D structure | — |
| Stage | depth / z / camera | entrance = soft rise |
| Timeline | named time | dwell · exit |

Machine tokens: `layout` · `stage` · `timeline`. Human labels are UTF-8. Named beats only (`entrance` · `dwell` · `exit`). Not legal `FormationStage`.

Chrome: `data-aspect-surface`, `data-aspect-*`, `.aspect-chip`. Helper: `browser::surface_aspects`.

## Coverage (regression = missing Stage or Timeline)

| Shell | Marked |
|-------|--------|
| `.app` | yes |
| `.main-workspace` | yes |
| `.canvas-viewport-container` | yes |
| `.toolbox-dock` | yes |
| `.right-dock` | yes |
| `.bottom-statusbar` | yes |
| `.dock-panel` | yes |
| `.vibe-console` | yes |
| `.contextual-instrument-panel` | yes |
| `.construct-shelf` | yes |
| `.g-coord-map` | yes |
| `q-cell` | yes |
| `.native-render-preview` | yes |
| `.canvas-container-node` | yes |
| `.cmd-palette-panel` | yes |

## Webizen Desktop extract (later gate)

Reuse the same attributes, CSS beats, and aspect chips. Do not invent a second motion system. No Solid IdP chrome. QDNF is not this surface.
