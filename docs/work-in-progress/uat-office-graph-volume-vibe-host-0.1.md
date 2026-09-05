# WIP — Browser UAT: office:graph + sanctuary volume

**Status:** work-in-progress (not standards) · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8` · **Sync tip:** `d2b79211` / INDEX tip-lock · **Branch:** `0.0.36-dev`
**Owner:** Vibe (diagnose/DevRel accept) · Chrome: davinci/monet · Seam: Neo · Shapes: Marvin
**Scope:** prove live binds — no new code from this crew while G-COORD advances elsewhere.

## Rules under test
- Thin four-ops only (parse/check/diagnose/invoke)
- Live ids: `GraphDatabase.sparql` · `volume_open` · `volume_commit`
- Unbound/daemon/wasm E300 = gated, never stub-broken or fake-durable
- Diagnose copy: never call persons/living “things” (SHACL-first vs OWL-ok list)

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

## D — Pass / fail log (fill in session)
| ID | Step | Pass? | Tip SHA | Notes / delta row |
|----|------|-------|---------|-------------------|
| A1–A5 | | | | |
| B1–B5 | | | | |
| C1–C4 | | | | |

## Out of scope this UAT
G-COORD bind · next toolchain · Solid IdP · QDNF · new Host methods
