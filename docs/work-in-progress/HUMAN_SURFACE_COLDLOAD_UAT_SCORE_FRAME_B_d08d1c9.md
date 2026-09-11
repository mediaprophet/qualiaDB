# Cold-load UAT score — Frame B (held / not yet)

**Tip:** `d08d1c9` · **Date:** 2026-09-12 · **Runner:** Capt  
**IA context:** Ask · Keep · Talk (tip `8f5b7d6` / product cut `2532cde`); cherry-pick not “parity.”

## Desktop
**Frame B: HELD** — Catalog · Lexicon not mounted this circuit; shell steady, no panic-red.
| # | Result | Notes |
|---|--------|-------|
| B1 | **HELD** | Catalog/Lexicon surface absent |
| B2 | **HELD** | Chips not observable without Catalog |
| B3 | **PASS*** | Steady shell; no panic red |
| B4 | **PARTIAL** | Unavailable panels elsewhere; Catalog path missing |

## WASM
**Frame B: PASS**
| # | Result | Notes |
|---|--------|-------|
| B1 | **PASS** | held / not yet — open lexicon pack |
| B2 | **PASS** | living · artifact · machine chips |
| B3 | **PASS** | Steady hold |
| B4 | **PASS** | held/unavailable preference OK |

**Screenshots:** `/workspace/uat-frame-b-d08d1c9/`  
**Wave-22:** still **held** until Desktop B clears (Catalog mount or honest held path).  
**Next:** davinci surface Catalog under Ask · Keep · Talk on Desktop; cherry-pick Desktop wins into WASM; three human loops.

## Follow-up (agent, 2026-09-11)

**Root cause (confirmed):** Desktop `webizen-studio` Dioxus Poet never mounted `lexicon_bay`. WASM `poet-ui` did.

**Fix landed** on branch `cursor/desktop-catalog-lexicon-57c7`: Catalog · Lexicon studio bay on Desktop Poet + Script/Zone D tabs + `GraphDatabase.lexicon_manifest` bind. Mesh/Aura/SHACL/Pulse/Job wait-honest → held / not yet.

**Ask Capt:** re-UAT Desktop Frame B (B1–B4) on the PR tip. WASM Frame B remains PASS.
