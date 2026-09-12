# Desktop Frame B re-UAT — tip `9738911` (≥ Catalog chrome `592c99a`)

**Date:** 2026-09-12 (Australia/Sydney)  
**Surface:** Webizen Desktop native (`:7`), rebuilt binary after stale-binary FAIL  
**Discovery path that hit:** Vibe → Script → Catalog · Lexicon (davinci PR #93 path 2)  
**Overall:** **PARTIAL**

## Scores

| Gate | Result | Notes |
|------|--------|--------|
| B1 Mount / discoverable | **PASS** | Catalog · Lexicon findable on path 2 |
| B2 held / not yet wording | **PASS** | Catalog held-gate uses held / not yet (not scored as unavailable on Catalog surface) |
| B3 Chips | **PASS** | living · artifact · machine |
| B4 No false-held on live ALL_BOUND | **PARTIAL** | Empty/nonsense → held ✓; valid `en-core.lexicon.json` still held (no pack card); Desktop not connected to `:4242` for live `lexicon_manifest` |

## Shots

- `/workspace/uat-frame-b-592c99a/catalog-success-surface.png`
- `/workspace/uat-frame-b-592c99a/catalog-success-surface-native.webp`
- (miss earlier) `poet-catalog-no-surface.png`
- `b4-empty.png` · `b4-nonsense.png` · `b4-real.png`

## Ops notes

- Prior FAIL was stale binary (no Catalog chrome strings). Rebuild on `9738911` + relaunch fixed mount.
- Wave-22 stays **held** until B4 verified or team accepts Catalog mount PARTIAL as enough to clear the Catalog gate only.
- Separate chrome debt spotted on same screen: other panes still say **unavailable** (Graph/Morpha/etc.) — not scored as B2 fail for Catalog, but violates held/not-yet ban elsewhere. @davinci follow-up.

## Tip lineage

- Chrome: `592c99a` (PR #93)  
- Tests: `18fb063` (PR #94)  
- Vocab human≠company≠bot: `9738911`

## B4 follow-up (same tip lineage / rebuilt Desktop)

**Result:** **PARTIAL** (not PASS)

- Empty path → held / not yet ✓
- Nonsense path → held / not yet ✓
- Valid fixture `crates/vibe/fixtures/lexicon/en-core.lexicon.json` → still **held** (no open pack card)
- Desktop showed Mesh/Graph unavailable — live `GraphDatabase.lexicon_manifest` bind not exercised against `:4242`

Shots: `b4-empty.png`, `b4-nonsense.png`, `b4-real.png` under `/workspace/uat-frame-b-592c99a/`

**Wave-22:** remains held until Desktop Catalog talks to live daemon bind (or team re-scopes B4).
