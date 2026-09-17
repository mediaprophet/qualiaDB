# WIP — Vibe REPL in Poet (useful UI, not a toy console)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev` · **Reviewed tip:** `5c758e63` · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8`
**Owner:** Neo (seams/IDE host) · Vibe (dialect/DevRel) · davinci/monet (chrome/motion) · Capt. (gate)
**Priority:** Poet first; Webizen Desktop later (reuse contract).

## What already landed (tip `5c758e63`)
- Poet IDE bottom drawer **Vibe REPL** (`crates/poet/src/browser/ide.rs`): history, `eval_repl`, `::`→`.` normalize, sequential prior-line eval into env, gas note, Problems tab sibling.
- Code actions: diagnose / format / outline / eval via `spec_tools/code_actions/vibe.rs` (`vibe::diagnose`, script eval).
- Capability bridge / operational suites in same tip — REPL can already probe e.g. `Animation.orbit_spin(...)`.

## Gap (why Timothy’s ask still matters)
A REPL that only evals arithmetic is insufficient. It must be the **hot-edit surface** onto live QualiaDB capabilities: diagnose → fix → invoke → show result, with gated honesty when daemon/wasm/E300 applies — without forcing host rebuilds.

## Product goals
1. **Human loop:** type vibe · see diagnose glow · accept suggested_fix · run · see value/error.
2. **Agent loop:** same four-ops; Capability.method strings first-class; fixtures replayable.
3. **Discovery:** list/search live `ALL_BOUND` families from REPL (CapabilityDiscovery / catalog) — no ghost APIs.
4. **Sanctuary:** volume open/commit recipes; never fake durable on fail-closed/E300.
5. **Copy:** SHACL-first language — never call persons/living “things” in REPL errors.

## Language accept (Vibe)
- **suggested_fix must stay catalog-honest** — only cite live `ALL_BOUND` / `Capability.method` (no ghost APIs in DevRel or diagnose copy).
- UAT fail rows triage to deltas **before** deepening the REPL drawer.
- Expose-not-rebuild; diagnose-first loop is the DevRel surface for dark/catalog-only families.

## Chrome accept (davinci)
1. **REPL is a studio drawer, not a terminal** — prompt + output + gas as secondary; primary loop is diagnose → suggested_fix → run → result (Problems tab is peer, not afterthought).
2. **Catalog tab** = discovery only of live `ALL_BOUND` (filter by family); click inserts invoke stub — no ghost APIs; progressive disclosure so ~887 methods don’t dump the Stage.
3. **Safe mode default** (diagnose-before-run); Invoke/Sanctuary modes explicit — sanctuary recipes never celebrate fake save on E300/fail-closed.
4. **Copy** follows SHACL-first list in REPL errors too (“person/living/country” vs “tool/volume/file”).
5. **UAT still before more chrome** — prove office:graph + volume, then deepen REPL drawer.

## Motion accept (monet)
1. Drawer open/close = entrance·exit (soft rise / z-dissolve); not a terminal blink.
2. Diagnose → run → result = dwell breath on active track; error glow on cell/token span (never fake precision if span coarse).
3. Catalog browse = quiet dwell; inserting an invoke stub = short entrance on the cell — no Stage dump of ~887 methods.
4. Safe-mode default: no celebratory commit beat on run; sanctuary recipes only light commit on real `volume_commit` success (E300/fail-closed gated).
5. Reduced-motion: same beats, shorter/crossfade; state never motion-only.
6. Copy in glow/toasts: person/living/country vs tool/volume/file.

## Design — Poet UI (davinci/monet implement later when Capt. unlocks chrome code)
### Layout
- Keep Zone D drawer: tabs **REPL** · **Problems** · (optional) **Catalog**.
- REPL: prompt · output · gas/cost · “Insert into cell” / “Copy invoke”.
- Problems: diagnose list with span → click highlights editor (token fidelity).
- Catalog: filterable Capability.method; click inserts invoke stub.

### Motion / state
- entrance/dwell/exit on run; error = glow not celebration; reduced-motion variants (monet notes).

### Modes
- **Safe:** diagnose-only until check passes.
- **Invoke:** allow capability_invoke / eval with effect classes visible.
- **Sanctuary:** volume recipes; fail-closed states explicit.

## Design — Engine / seams (Neo)
1. Pin all REPL paths through `poet::vibe_host` four-ops + daemon invoke — **no Host trait bleed**.
2. Shared env across REPL history (already sketched); document reset / fork semantics.
3. Map REPL errors to diagnose JSON shape (`valid`, `error_code`, `span`, `suggested_fix`).
4. Playbook snippets (markdown → later fixtures under `crates/vibe/fixtures`):
   - graph ASK/SELECT via GraphDatabase.sparql
   - volume_open → edit → volume_commit
   - SHACL.validate · N3Logic.evaluate
   - Cosmic.* / Position-on-cell (post G-COORD bind)
   - Animation/HID smoke
5. `::` vs `.` normalization: document as UX sugar only; canonical dialect uses vibe grammar.
6. Gas/budget display honesty — don’t invent meters without live budget hooks.

## Stage plan
| Stage | Deliverable | Accept |
|-------|-------------|--------|
| 0 | This WIP + link from WIP README | on remote |
| 1 | DevRel one-pager: REPL recipes for exposed ids only | Vibe accept |
| 2 | Problems↔editor span jump + diagnose-before-run default | UAT row |
| 3 | Catalog tab fed by live discovery list | no dark ids |
| 4 | Fixture pack replay in REPL | hot-edit, no rebuild |
| 5 | Webizen Desktop reuse checklist | later gate |

## Relation to core-db uplift audit
REPL is a **primary exposure channel** for catalog-only families (Econ/Stats/ML/…) until toolchains exist — playbooks beat silent methods. See sibling `qualia-core-db-uplift-audit.md`.

## Out of scope now
Webizen Desktop implementation · Solid IdP · Host widen · reimplementing engines already in core-db · QDNF bleed into REPL copy
