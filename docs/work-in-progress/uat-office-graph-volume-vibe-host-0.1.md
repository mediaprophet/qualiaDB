# WIP — Browser UAT: office:graph + sanctuary volume + lexicon held-gate

**Status:** work-in-progress (not standards) · **Freeze:** `vibe-host-0.1` · **Sync tip:** `641c2460` · **Branch:** `0.0.36-dev`
**Owner:** Vibe (diagnose/DevRel accept) · Chrome: davinci/monet · Seam: Neo · Shapes: Marvin
**Scope:** prove live binds after **G-LEXICON-0** first slice accept. G-COORD held until this UAT passes.

## Rules under test
- Thin four-ops only (parse/check/diagnose/invoke)
- Live ids: `GraphDatabase.sparql` · `volume_open` · `volume_commit` · `lexicon_manifest`
- Unbound/daemon/wasm/missing-pack E300 = **held / not yet** (gated), never stub-broken or fake-durable
- Diagnose copy: never call persons/living “things” (SHACL-first vs OWL-ok list)
- Shape: `docs/manuals/standards/lexicon-pack-shape-G-LEXICON-0.md`
- Bay chrome: `docs/work-in-progress/g-lexicon-0-bay-chrome.md`

## A — office:graph / sparql
1. Toolbar launches query without reading Capability strings
2. Happy path: results settle; dwell/exit beats feel human
3. Bad query: diagnose JSON has code · span · suggested_fix; monet glow hits cell/token
4. Daemon-gated / unbound: chrome explains, looks gated not broken
5. Hot-edit script reload needs no host rebuild

## B — sanctuary volume
1. `volume_open` (path/handle) → open state visible
2. `volume_commit` success → commit beat only then; state committed
3. Fail-closed deny → denied/fault; no “saved” celebration
4. Wasm/E300 path gated honest
5. Volume dock states match `q42:state` (closed·open·committed·denied·fault)

## C — Language / diagnose accept
1. `diagnose` before invoke catches effect/type issues on sample cells
2. suggested_fix actionable for agent loop
3. Spans stable enough for error glow (note if coarse → B row)
4. No Host/AST bleed into Poet chrome during UAT
5. `lexicon:` pin fixtures (Vibe) — missing pack → held / not yet + open-pack suggested_fix; OK pin recorded; alias row round-trip; living framing never rewritten as artifact

## D — Lexicon held-gate / bay (G-LEXICON-0)
1. Call `GraphDatabase.lexicon_manifest` with missing path → **held / not yet** + “open lexicon pack” (never broken)
2. Valid `*.lexicon.json` (example under `docs/manuals/standards/lexicon-pack-manifest-example.json`) → arrive card shows `packSemVer` + framing chip
3. `.q42` without sidecar / bad volume → held / not yet
4. Framing chips: living warm · artifact crisp · machine muted; **mixed** splits; no Thing-wash
5. Catalog peer / Zone D Catalog tab surfaces gate + chips (listen-only this slice)
6. Upgrade recipe beats: arrive on open · hold on breaking-id list · leave on dismiss · **commit** only on real pack write (N/A this slice if listen-only)

## E — Pass / fail log (fill in session)
| ID | Step | Pass? | Tip SHA | Notes / delta row |
|----|------|-------|---------|-------------------|
| A1–A5 | | | | |
| B1–B5 | | | | |
| C1–C5 | | | | |
| D1–D6 | | | | |

## Out of scope this UAT
G-COORD bind · full WordNet engine · locale packs beyond en · next toolchain · Solid IdP · QDNF · new Host methods
