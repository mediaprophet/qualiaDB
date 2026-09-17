# Poet surface aspects (Layout · Stage · Timeline)

**Packet:** davinci Stage 7–8 · **Term:** **aspects**.

Every shipped Poet surface has three **aspects** — readings of one surface, not copies of it:

| Aspect | Reading | Beat |
|--------|---------|------|
| Layout | 2D structure | — |
| Stage | depth / z / camera | entrance = soft rise |
| Timeline | named time | dwell · exit |

Do **not** call them twins: twin infers identical, which is almost always misleading. Do **not** call them planes (acoustic-plane and network control/data plane are other vocabularies). Not a credential digital twin. Not legal `FormationStage`.

Machine tokens: `layout` · `stage` · `timeline`. Human labels are UTF-8. Named beats only (`entrance` · `dwell` · `exit`).

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
| `.top-menubar` | yes |
| `.canvas-control-bar` | yes |
| `.toolbox-flyout` | yes |
| `#save-mode-dialog` | yes |

## Webizen Desktop extract

Reuse the same attributes, CSS beats, and aspect chips: [`webizen-chrome-aspect-extract.md`](webizen-chrome-aspect-extract.md). Do not invent a second motion system. No Solid IdP chrome. QDNF is not this surface.
