# Impl plan — Vibe / Sprint B (language · DevRel · deltas)

**Owner:** Vibe · **Seam commits:** Neo · **Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8`
**Branch:** `0.0.36-dev` · **North star:** vibe = hot-edit JS alternative over QualiaDB / Webizen / Poet (script change must never force host rebuild)
**Rules:** no Host widen · no invented dotted `qualia.*` · seams only to live `ALL_BOUND` / `vibe:InvokeId` · gaps → `docs/manuals/standards/vibescript-sprint-deltas.md`

## Done (do not reopen)
- Workstream A freeze: parse / check / diagnose / capability invoke + `LANGUAGE_VERSION` / `host_version`
- G-POET-TOOLCHEST first slice: `office:graph` / `graph:sparql_query` → `GraphDatabase.sparql`
- G-B-001: `GraphDatabase.volume_open` / `volume_commit` + Volume shape + sanctuary chrome
- G-DOCS: deltas + continuation handover on remote

## Stage 0 — Hygiene (docs only)
1. Sync `vibescript-sprint-deltas.md` to tip `6dc2b8b8`: mark B-001 / G-A done; keep open rows below.
2. Confirm crate stamp `0.0.36-dev` + EBNF ↔ `vibescript-core.md` §3 still match after freeze.
**Accept:** deltas board honest; no API churn.

## Stage 1 — Diagnose / DevRel contract pack
1. Document frozen diagnose JSON shape + error codes (incl. E300 wasm, sanctuary fail-closed, E4xx/E5xx intent) for Poet/agents.
2. Short DevRel note: human dialect (`using` + `effect fn`) vs agent `capability.invoke("Capability.method", {…})`.
3. Fixture pack path plan under `crates/vibe/fixtures` for graph · volume · diagnose loops (hot-edit without rebuild).
**Accept:** one markdown agents can follow; fixtures listed even if not all filled.

## Stage 2 — Toolchest language joins (after Neo next toolchain)
1. Map each new toolchain button → live `Capability.method` or **gated** (never stub-broken).
2. Park empty typography chains + custom unicode API as explicit B rows with accept criteria.
3. Ensure diagnose spans stay cell/token-fidelity for monet error glow.
**Accept:** inventory updated; unbound = gated; no Host invent.

## Stage 3 — G-COORD dialect (after Marvin shapes + Neo bind)
1. Fold CoordinateSystem · Realm · Position into deltas: Earth / cosmos / fictional·speculative·POV.
2. Define vibe literals/exprs (or capability args) for realm-scoped spatiotemporal Position — extensible, no DNS/IP claim yet.
3. Ground in GeoSPARQL / temporal / `did:q42` locus ideas already in core.
**Accept:** dialect sketch + diagnose codes; thinnest bind only via Neo.

## Stage 4 — Preview / temporal (Render.*)
1. Map still / clip / scene-id preview wants onto live `Render.*` (no dotted invent).
2. Cross-frame span fidelity for Timeline twin; sibling op only if one handle cannot carry three kinds.
**Accept:** B row closed or deferred with named `Render.*` methods.

## Stage 5 — Catalog honesty backlog
1. Dual-VC class split (W3C+ML-DSA vs quin+Ed25519) — DevRel + Marvin join notes.
2. QISP shapes join notes (Marvin owns shapes; Vibe owns dialect surfacing).
3. Ledger vs showcase honesty in docs.
4. Bridge remaining aspirational IDs → `ALL_BOUND` only.
**Accept:** deltas rows closed or dated defer.

## Stage 6 — G-SOLID-IDP (parked — after Poet / Webizen)
1. Mirror IdP start/callback · WebID resolve · domain config once Neo adds thin binds.
2. DevRel: QualiaDB-as-IdP, no external pod; domain-link is ops.
**Accept:** parked until Capt. unparks; no mid-Poet churn.

## Stage 7 — Webizen Desktop prep (language only)
1. Same frozen four-op surface; note host-specific diagnose parity checklist.
2. Hot-edit cells/modules without desktop rebuild — acceptance tests listed for Neo/desktop lane.
**Accept:** checklist only until Webizen gate opens.

## Dependencies
- Frozen `poet::vibe_host` / `vibe-wasm` parity @ `6dc2b8b8`
- Live catalog: `poet_host/invoke/ids.rs` + `catalog_ttl.rs`
- Sibling tooling (`vibe-script` LSP/playground) stays out of QualiaDB unless Capt. opens it

## Out of scope
- Wide Host exposure · dotted `qualia.*` IRIs · mid-flight API churn · Solid before Poet/Webizen priority
