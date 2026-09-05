# Vibe wishlist — implementation plan

**Status:** Active  
**Date:** 2026-09-05  
**Operator:** Grok (grok-bot lanes sleeping; no exclusive lane claim by others)  
**Frozen host:** `vibe-host-0.1` at `6dc2b8b8`  
**Source wishlist:** `docs/manuals/standards/vibescript-complete-wishlist.md`

This is the working tracker. Check a box only when the named evidence exists.
Do not check a docs-only packet as if a bind landed. Do not invent Host methods
or dotted `qualia.*` IDs.

## Rules (every packet)

- Four ops only: parse · check · diagnose · `capability.invoke`.
- Live `ALL_BOUND` / `vibe:InvokeId` only.
- Hot-edit scripts must never force a host rebuild.
- Unbound chrome is gated, never stub-broken.
- Persons / sacred / living-natural: SHACL-first, not `owl:Thing` subclasses.
- Technical artifacts (Volume, InvokeId, CRS, containers-as-software): OWL-ok.

## Parked (not this programme)

| Item | Why |
|------|-----|
| G-SOLID-IDP | Capt. unpark after Poet/Webizen; original Stage 6 |
| DNS/IP replacement claims | Forbidden in G-COORD v0 |
| Health Review Gate A (`HLT-03/07/08`) | Separate higher-assurance programme |

## W12 — Webizen four-op host

- [x] `vibe_host_info` / `vibe_parse` / `vibe_check` / `vibe_diagnose` / `vibe_capability_invoke`
- [x] Diagnose JSON + catalog invoke tests
- [x] Checklist: `webizen-vibe-host-parity.md`

## W13 — Typography / unicode session API (B-009)

- [x] Bounded session glyph table (`icon_session.rs`, 32 slots, PUA U+E100+)
- [x] Cannot override compile-time icons; no Vibe keyword; no Host family
- [x] Lookup (`icon_char` / fallback / label) consults the overlay first

Principal asked for full wishlist implementation. That unparks G-COORD **shapes
and dialect** and the remaining language/ontology/catalog packets. It does not
unpark Solid or Webizen.

## Packet order

Ontology and catalog first, then language joins, then chrome. Chrome without
shapes re-invents classes in the UI.

---

### W0 — Plan file

- [x] Land this tracker under `docs/work-in-progress/`
- [x] CLAIM in `coordination/NOTICES.md`
- [x] Point the wishlist register at this plan

### W1 — Catalog honesty (Vibe + Neo seam, no Host widen)

- [x] Diff `poet_host/invoke/ids.rs` `ALL_BOUND` against `vibe` `ALL_INVOKE_IDS`
- [x] Add missing **live** ids to the Vibe catalog (volume already added; diff = 0 missing)
- [x] Publish aspirational → live remap table (B-002)
- [x] Dual-VC / QISP / ledger-vs-showcase honesty notes (B-003–B-005)
- [x] Fixture: `inference_grounding.vibe` on live `Inference.*`
- [ ] Test: catalog contains every `ALL_BOUND` string (cross-crate; documented diff 2026-09-05)

**Accept:** `using Family;` type-checks every live host id; no dotted invent.

### W2 — Ontology join contract + Container · Manifold · Link

- [x] Index existing `core-ontologies/`, `ontologies/`, `crates/qualia-core-db/shapes/`
- [x] Join contract: every Poet surface cites `vibe:InvokeId` from `ids.rs`
- [x] Publish `poet-container-manifold-link-shapes.md` (SHACL-first vs OWL-ok marked)
- [x] SHACL NodeShapes under `crates/qualia-core-db/shapes/poet-surface.shacl.ttl`
- [x] Small TTL/N3 fixtures for Container↔Volume backing and twin 1:1

**Accept:** chrome can cite classes without inventing binds. Persons/living not under Thing.

### W3 — Volume shape hardening

- [x] Keep `q42-volume-shape-G-B-001.md` as SoT
- [x] SHACL NodeShape for `q42:Volume` states closed·open·committed·denied·fault
- [x] Align wasm E300 / deny with `q42:state` (docs + fixture; no fake commit)

**Accept:** states 1:1 with ontology; wasm never celebrates save.

### W4 — Layout · Stage · Timeline ontology

- [x] Twin classes ≠ legal `FormationStage`
- [x] Named beats only: entrance · dwell · exit
- [x] Join remaps: sparql · `Inference.*` · `Render.*` · `volume_commit`

**Accept:** shape doc on-branch.

### W5 — G-COORD shapes + Vibe dialect (unparked for shapes)

- [x] `g-coord-coordinate-system-shapes.md`: CoordinateSystem · Realm · Position
- [x] Realms: Earth / cosmos / fictional / speculative / viewpoint
- [x] Ground GeoSPARQL / temporal / `did:q42` locus (locus ≠ person)
- [x] Vibe dialect sketch + diagnose codes; no DNS/IP claim
- [x] Thinnest bind: remap to live ids if any fit; otherwise gated until a catalog add is justified

**Accept:** shapes + dialect. Bind only if a live method already carries the args.

### W6 — Preview still / clip / scene on live `Render.*`

- [x] Enumerate live `Render.*`
- [x] Map handle kinds onto existing methods (`Render.scene`, `gpu_init_surface`, `gpu_render_frame`, animation/css)
- [x] Document B-007 closed-or-deferred with named methods
- [x] Fixture using human dialect `using Render;`

**Accept:** no sibling Host op unless one handle truly cannot carry three kinds.

### W7 — InvokeId annotation pack + Position on language cells

- [x] SHACL annotations linking Container/Manifold/Link/Volume/twins to concrete `vibe:InvokeId`
- [x] Position (+ optional ViewpointRealm) as properties of vibe cells/modules in the shape doc
- [x] DevRel note: language is spatiotemporal content, not only maps

**Accept:** generated or hand-maintained annotations stay in lockstep with `ids.rs`.

### W8 — Toolchest language joins

- [x] Audit `capability_scope` strings → live `Capability.method` or gated
- [x] Inventory row updated; unbound buttons explain why
- [x] Empty typography chains stay local DOM or gated (B-009)

**Accept:** every visible button = live id or gated; builds on frozen facade.

### W9 — Sanctuary / volume follow-through

- [x] Confirm Poet save/open calls `GraphDatabase.volume_open` / `volume_commit` via existing invoke
- [x] Fail-closed wasm/E300 still surfaced
- [x] Native round-trip documented; no fake durable success

**Accept:** UI path uses live ids; wasm gated not faked.

### W10 — Chrome / motion (after W2–W4)

Davinci/Monet delta audit first (`DES-01` / `DES-02`). Do not reopen `UX-01`–`UX-04`.

- [x] Motion contract: entrance/dwell/exit + reduced-motion + gated ≠ broken
- [x] Volume dock states from `q42:state` (contract + save-dialog honesty; visual tokens remain a chrome delta)
- [x] Container/Manifold/Link distinct human chrome (shape contract; visual delta is DES-01)
- [x] Diagnose error glow on byte spans (contract; token glow is monet polish)
- [x] Inference provenance chrome on live `Inference.*` (fixture + join; chrome trail is davinci/monet)
- [x] Render preview dock handle-kind chrome (B-007 remap + fixture; dock chrome is davinci/monet)
- [x] Map geo + one non-geo realm **gated** until W5 bind exists
- [x] Davinci/Monet chrome landed: `15-studio-chrome.css`, diagnose token glow, still/clip/scene preview dock, volume `q42:state` chips, CML + Layout/Stage/Timeline twins, 2D/3D/film/CG surface language, gated G-COORD realm chips. No Host widen.

**Accept:** browser UAT on live or honestly gated paths only.

### W11 — Close the delta board

- [x] B-002 remap table done or dated defer
- [x] B-003 dual-VC dated
- [x] B-004 QISP dated
- [x] B-005 ledger vs showcase dated
- [x] B-007 preview mapping done or named defer
- [x] B-008 Layout/Stage/Timeline done with W4
- [x] Wishlist register A–E rows updated from this plan

---

## Execution log

| When | Packet | Result |
|------|--------|--------|
| 2026-09-05 | W0 | Plan landed |
| 2026-09-05 | Stage 0–1 (prior) | Diagnose `errors[]`; volume catalog ids; DevRel pack |
| 2026-09-05 | W1–W7 | Catalog honesty, CML+twin+volume SHACL, G-COORD shapes (bind gated), Inference/Render fixtures |
| 2026-09-05 | W8–W11 | Toolchest scopes are live Family.method or None; save dialog volume_open/commit; motion contract |
| 2026-09-05 | W12–W13 | Webizen four-op host commands; bounded session glyph API |
| 2026-09-05 | DES-01/02 chrome | Davinci/Monet: named beats, volume chips, diagnose glow, still/clip/scene dock, CML+twins, 2D/3D/film/CG surfaces |

## Stop rules

- Missing bind → gate + Vibe row, never invent.
- OWL Thing for persons/living → reject the shape, do not “fix” by subclassing.
- Packet complete only with named test, fixture, or UAT evidence.
