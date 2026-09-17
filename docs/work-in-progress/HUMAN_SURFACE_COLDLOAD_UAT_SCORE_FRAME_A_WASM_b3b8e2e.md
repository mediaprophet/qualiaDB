# Cold-load UAT score — Frame A WASM (re-UAT after PR #91)

**Tip:** `b3b8e2e` · **Surface:** WASM · **Date:** 2026-09-11 · **Runner:** Capt

| # | Result | Notes |
|---|--------|-------|
| A1 | **PASS*** | Studio bay first paint (prior `cfac542`; still present) |
| A2 | **PASS*** | Ask · Keep · Play soft-rise card live |
| A3 | **PASS*** | Soft-rise OK |
| A4 | **FAIL** | Right-click main canvas → **no** 8-sector wheel, **no** OS menu (gesture swallowed, ring not painted). Dual Studio N/A. Soft-rise still up. |
| A5 | **PASS*** | Methods not primary |

**Sayables-first:** PASS*  
**Wheel:** FAIL (residual after PR #91 — own-contextmenu works; ring paint missing)  
**UI stamp:** pill shows `0.0.37` despite HEAD `b3b8e2e` / branch `0.0.38`

**Screenshots:** `/workspace/uat-a4-b3b8e2e/01-studio-before.png` · `02-after-rightclick.png`

**Next:** davinci residual chrome (ring paint after preventDefault; version pill stamp) · Desktop A deferred · Wave-22 held.
