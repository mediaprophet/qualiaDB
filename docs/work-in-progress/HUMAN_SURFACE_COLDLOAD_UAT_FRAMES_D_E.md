# Cold-load UAT — Frames D–E (motion · monet)

**Branch:** `0.0.38` · **Parity tip:** ≥ `b3b8e2e` · **Lexicon:** `541c3a6` · **D–E motion:** `f22851d`  
**Surfaces:** WASM then Desktop — same steps · same beats  
**Owners:** Capt (run + shots) · monet (score motion) · davinci (Keep discoverability chrome)

---

## Frame D — Sanctuary commit

| # | Step | Pass if | Fail if |
|---|------|---------|---------|
| D1 | Keep / open volume | Dock **arrives** soft-rise (entrance) | Hard pop / no shelter feel |
| D2 | Held / closed before open | Care look · held/not yet/closed — never broken | Red broken theatre |
| D3 | Checkpoint → real commit | Celebrate / twin beat **only** when `volume_commit` succeeds | Celebrate on deny/fault/E300/preview |
| D4 | Reduced-motion | still-arrive / still-hold named; state readable without motion | State only visible via animation |

## Frame E — Leave well

| # | Step | Pass if | Fail if |
|---|------|---------|---------|
| E1 | Dismiss / leave surface | **Leave** along same path as arrive | Unrelated fade / kill-abort primary |
| E2 | Reduced-motion leave | still-leave (shorter/crossfade) | “Animation off” as if state vanished |
| E3 | Re-enter | Arrive again — same dialect | Alternate cold-load motion dialect |
| E4 | Wheel (cross-check A4) | Same wheel dialect both surfaces | WASM missing wheel / thinner path |

---

## Scores (monet)

| Item | Desktop | WASM | Evidence |
|------|---------|------|----------|
| **A3 soft-rise feel** | — | **PASS*** | Capt Frame A WASM shots 01–02; first paint not modal slap |
| **Frame A empty-bay arrive** | — | **PASS** | Capt re-UAT `cfac542` A1/A2 — studio bay + Ask·Keep·Play live |
| **A4 / E4 wheel** | PASS* | **FAIL** | Capt re-UAT `b3b8e2e` PR #91: right-click owns menu (no OS) but **no ring paint**; stamp `0.0.37` |
| D1 Keep soft-rise | PASS* | **HELD** | Checkpoint modal only; no daemon — retake pending (shots `d1-keep-soft-rise` + `d1-volume-after`) |
| Deny care / no celebrate (DENIED·NO DAEMON) | — | **PASS*** | Checkpoint deny care look; no celebrate on DENIED·NO DAEMON (`d1-keep-soft-rise` + `d1-volume-after`) |
| Studio bay before Keep (A2 context) | — | **PASS*** | Ask·Keep·Play live; studio bay precedes Keep attempt |
| D2 / D3 Volume CLOSED care | PASS* | **PASS*** | Footer CLOSED honest; Graph live 708 quins |
| D3 commit celebrate | PASS* | **HELD** | No Keep→commit path yet |
| E1 global leave | PARTIAL | PARTIAL | Modal still-leave PASS*; uneven elsewhere |
| E2 still-leave | NEEDS_UAT | **PASS*** | e0 Reduce motion ON (`44c3ba15`); e1 Container settings open (`1c8180cb`); e2 Esc dismiss panel gone (`e3403df1`). Paths `/workspace/uat-monet-de-wasm/e0|e1|e2-*.png`. |
| GIS held/not yet (adjacent) | — | PASS* | Remap Cosmic `held / not yet` on Map (shot 02) |

**Chrome note (davinci):** Doc/LaTeX red `missing` reads harsher than held/not yet — align when empty-state lands. UI stamp on shots: `0.0.37` while branch tip is `0.0.38` — Capt/Neo confirm stamp path.

**Blocker:** Real Keep entrance + commit path (daemon) before D1/D3 retake; Desktop shots still thin.

Wave-22 stays **held** until real Keep entrance + commit path.
