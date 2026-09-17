# Impl plan — Neo / G-A follow-on (seams · toolchest · Webizen prep)

**Owner:** Neo · **Commits:** Neo · **Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8`
**Branch:** `0.0.36-dev` · **Role:** Rust systems / crate seams / `ALL_BOUND` binds / team push lane
**Rules:** no Host widen · no invented dotted `qualia.*` · seam only to live `Capability.method` · hot-edit scripts must not force host rebuild · gaps → Vibe → `vibescript-sprint-deltas.md` · gates → Capt.

## Done (do not reopen)
- G-DOCS handover + sprint deltas intake
- G-POET-TOOLCHEST first slice: inventory + `office:graph` / `graph:sparql_query` → `GraphDatabase.sparql` (split `registration.rs` modules)
- G-B-001: `GraphDatabase.volume_open` / `volume_commit` in `ALL_BOUND` + `invoke/graph/volume.rs` (sanctuary fail-closed; wasm E300)
- G-A four closes: `poet::vibe_host` facade · Host pin (invoke+diagnose) · native↔wasm diagnose `to_json` parity · crate stamp `0.0.36-dev` + EBNF ↔ `vibescript-core.md` §3
- Marvin Volume shape folded: `docs/manuals/standards/q42-volume-shape-G-B-001.md`

## Stage 0 — Plan pack on remote (this push)
1. Land lane impl-plans under `docs/manuals/standards/` as Capt. listed.
2. Tip-lock note: all stage work starts from frozen `6dc2b8b8` (or later tip that preserves freeze).
**Accept:** plans on GH; SHA reported to Capt.

## Stage 1 — Toolchest remaining (Poet, frozen facade only)
1. Remap remaining `capability_scope` strings → live `ALL_BOUND` `Capability.method` (or **gated** + explain; never stub-broken).
2. Empty typography / paragraph chains: wire to real binds or hide with davinci/monet (no fake).
3. Custom unicode registration API: thinnest seam (B row + bind if catalog already has a fit; else park with Vibe).
4. Second live toolchain end-to-end (pick from inventory after Capt. locks “next toolchain”) — same pattern as `office:graph`.
**Accept:** inventory updated; every visible button = live id or gated; builds against `poet::vibe_host` only.
**Status (2026-09-05):** Capt. pick `office:shapes` — `N3Logic.evaluate` + `SHACL.validate`.

## Stage 2 — Sanctuary / volume follow-through (no Host widen)
1. Poet daemon path: ensure UI sanctuary save/open calls `GraphDatabase.volume_open` / `volume_commit` via existing invoke seam (no new Host methods).
2. Fail-closed honesty: wasm/E300 and empty-graph refuse stay surfaced (chrome already ungated).
3. Optional: thin handle metadata in diagnose/DevRel fixtures (coordinate with Vibe Stage 1).
**Accept:** round-trip open→edit→commit on native; wasm stays gated not faked.

## Stage 3 — G-COORD thinnest bind (after Marvin shapes)
1. Review Marvin `CoordinateSystem` · `Realm` · `Position` shapes.
2. Find or add **minimal** `ALL_BOUND` ids only if no live fit (prefer remap); implement invoke handlers in `poet_host/invoke/` seam folder.
3. G-COORD is spatial/realm Position — not a network. DNS/IP-free naming/routing is **QDNF** (`qualia-decentralized-network-fabric/`). Do not implement QLink/QRoute under this stage.
**Accept:** one thin bind path + catalog entries; Vibe dialect later; Capt. gate close.

## Stage 4 — Preview / Render.* seam assist
1. Enumerate live `Render.*` in `ids.rs`; map still/clip/scene wants with davinci (B-007).
2. Add bind only if a named method is missing **and** Capt. accepts a catalog add — else remap.
**Accept:** named live methods only; no dotted invent.

## Stage 5 — Webizen Desktop prep (compiled edge)
1. Checklist: desktop host exposes same four ops + versions as `poet::vibe_host` / `vibe-wasm`.
2. Diagnose JSON + E300 parity tests listed (implement when Webizen gate opens).
3. Confirm script hot-edit path does not require desktop rebuild.
**Accept:** checklist + stub test names; no drive-by Host widen.

## Stage 6 — G-SOLID-IDP (parked)
1. Thin `ALL_BOUND` binds for IdP start/callback · WebID resolve · domain config against `qualia-solid-bridge` **only after** Capt. unparks (Poet/Webizen first).
2. Marvin shapes + Vibe B mirror; Neo seam last.
**Accept:** parked; no mid-Poet commits unless Capt. unlocks.

## Stage 7 — Crate / catalog hygiene
1. Keep stamps on `0.0.36-dev` until release cut; EBNF stays byte-sync with §3 when grammar changes (Vibe owns grammar intent; Neo lands file).
2. Reject mid-sprint Host trait growth; capability growth = `ids.rs` + handler + catalog_ttl only.
**Accept:** CI/docs note; Capt. board stays honest.

## Dependencies
- Frozen tip `6dc2b8b8` (`vibe-host-0.1`)
- Live catalog ≈ `crates/qualia-core-db/src/poet_host/invoke/ids.rs`
- Creative chrome (davinci/monet) · ontology (Marvin) · deltas triage (Vibe)
- Capt. picks next ungate: **next toolchain** vs **G-COORD**

## Out of scope
- Wide `Host` to UI · dotted `qualia.*` · Solid before Poet/Webizen · inventing toolchains without live binds
