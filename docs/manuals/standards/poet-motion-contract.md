# Poet motion contract (v0)

**Packet:** W10 / DES-02 · **Beats only:** entrance · dwell · exit  
**No free tweens.** Unbound chrome looks gated, never broken.

## Beats

| Beat | Look | When |
|------|------|------|
| entrance | soft rise + light fade (Stage depth cue) | surface / dock / flyout opens |
| dwell | steady focus + quiet breath | SPARQL running, inference in flight, volume open |
| exit | dissolve along the same z-path | close, dismiss, successful commit |

Reduced-motion (`prefers-reduced-motion: reduce`): same named beats, shorter or crossfade only. State must never depend on motion alone.

## Gated ≠ broken

| State | Visual | Never |
|-------|--------|-------|
| live | full contrast, dwell allowed | — |
| local | present/partial token | imply daemon durability |
| unavailable | muted icon + reason | greyed mystery disable |
| denied / fault | error glow on diagnose byte span | celebratory success motion |
| committed | exit dissolve after real `volume_commit` | celebrate wasm E300 |

## Volume dock (`q42:state`)

closed · open · committed · denied · fault — distinct token each. Commit beat only on successful `GraphDatabase.volume_commit`.

## Aspects (not twins)

Layout · Stage · Timeline are **aspects** of a surface — three 1:1 readings, not a credential “digital twin”. Chrome: `data-aspect-surface`, `.aspect-chip`. Coverage: `poet-aspect-coverage.md`.

## Diagnose glow

`vibe::diagnose` spans are UTF-8 byte `[start, end]`. Error glow lights that token, not the whole panel.

**Chrome (2026-09-05):** `.diag-glow-token` + `diag_glow::wrap_byte_spans` on the VibeScript console Diagnose action (in-process four-ops; no daemon). Volume chip `q42:state` on the save dialog and status bar. Still/clip/scene handle tabs on the live Render preview dock.

## Map / G-COORD

Geo and non-geo realm skins stay **gated** until a live bind is selected (W5). Do not ship a fake CRS Host.

Network naming/routing without DNS/IP is **QDNF**, not G-COORD. Map chrome must not pretend a CoordinateSystem is an IP or DNS replacement.

## Container · Manifold · Link

Distinct human chrome; machine ids under the hood. See `poet-container-manifold-link-shapes.md`.
