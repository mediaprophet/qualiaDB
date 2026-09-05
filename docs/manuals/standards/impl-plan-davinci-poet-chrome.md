# Impl plan — davinci / Poet chrome (UX · toolchest · maps · 3D/temporal)

**Owner:** davinci · **Visual pair:** monet · **Seam commits:** Neo · **Ontology:** Marvin · **Language triage:** Vibe
**Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8` · **Branch:** `0.0.36-dev`
**North star:** Poet feels like a live studio over QualiaDB — human-first chrome (not a code UI), machine-readable underneath; Layout · Stage · Timeline twins on every surface.
**Rules:** chrome binds only to frozen four-ops + live `ALL_BOUND` / `vibe:InvokeId` · no Host invent · no dotted `qualia.*` · unbound = visually gated (never stub-broken) · script hot-edit must never force host rebuild · gaps → @Vibe → `vibescript-sprint-deltas.md`

## Done (do not reopen)
- Studio brief + Layout / Stage / Timeline v0 (entrance · dwell · exit named beats)
- Creative remaps → live `GraphDatabase.sparql` · `Inference.*` · `Render.*`
- G-POET-TOOLCHEST first slice chrome: `office:graph` / `graph:sparql_query` @ tip after `43e759fa` / board tip `fdbcbfd` family
- G-B-001 sanctuary save chrome ungated against `GraphDatabase.volume_open` / `volume_commit` (fail-closed / wasm E300 honest)
- G-A chrome accept @ `6dc2b8b8` — lock to thin facade only

## Stage 0 — Hygiene (docs / sync)
1. Sync chrome notes to tip after Neo’s plan push (`7318a049`+); cite freeze `6dc2b8b8`.
2. One-page chrome contract: toolchest → toolbox → toolchain; container / manifold / link; twin mapping rules.
3. List gated vs live actions for office:graph + volume (daemon-gated = gated look).
**Accept:** contract markdown on-branch; no UI invent ahead of binds.

## Stage 1 — Toolchest human chrome (office:graph complete)
1. Finalize toolbar chrome for `office:graph`: clear unicode glyphs that render; custom glyphs only when stock fails (custom unicode API stays B until Neo/Vibe ungate).
2. Error glow from diagnose spans (cell/token fidelity) — pair with monet.
3. Results settle = dwell beat; toolchain close = exit beat.
**Accept:** human can run sparql from toolbar without reading Capability strings; gated paths explain why.

## Stage 2 — Next toolchain chrome (inventory-driven)
1. After Capt. locks next toolchain from `poet-toolchest-inventory-G-POET-TOOLCHEST.md`, ship chrome in the same pattern as office:graph.
2. Prefer toolchains that already have live `ALL_BOUND` binds; otherwise ship gated shells.
3. Empty typography chains: gated placeholders + B criteria only — no fake fonts UI.
**Accept:** one more live toolchain chrome OR gated shell with honest copy; inventory row updated.

## Stage 3 — Sanctuary / volume polish
1. Volume dock chrome: open → edit → commit twin fully wired to live ids; denied/fault states from Volume shape (`q42:state`).
2. Never imply durable save on wasm E300 / fail-closed deny.
3. Manifold “backing store” cue when Container sits on a Volume (Marvin join).
**Accept:** user can tell open vs committed vs denied without reading logs.

## Stage 4 — Containers · manifolds · links (chrome)
1. Human chrome for Container (content-shaped), Manifold (nested containers/manifolds), Link (semantic relation) — not graph-viz for engineers.
2. Spatiotemporal attrs visible as first-class (space + time on content, including language cells).
3. Join Layout (structure) / Stage (depth) / Timeline (beats) 1:1 per surface.
**Accept:** non-dev user can nest a container in a manifold and see a link; machine ids stay under the hood.

## Stage 5 — Map containers (geo · cosmos · fiction)
1. Geospatial container: open map layers (OSM and peers) — replace “bad map” UX.
2. Non-geo map containers: universe / fantasy / speculative realms (e.g. Trek-class worlds) using same container chrome.
3. Consume Marvin **G-COORD** shapes (`CoordinateSystem` · `Realm` · `Position`) once Neo’s thinnest bind lands — Earth / cosmos / fictional·speculative·POV.
4. Temporal scrubber on map Timeline (time is not optional).
**Accept:** one geo layer path + one non-geo realm path demoable; unbound coord features gated.

## Stage 6 — Inference · Render preview chrome
1. Inference assist chrome on live `Inference.*` with visible provenance trails (monet).
2. In-flow preview dock on live `Render.*`: still / clip / scene-id — still/clip/scene as handle kinds; cross-frame spans for Timeline.
3. Sibling preview op only if Vibe/Neo say one handle cannot carry three kinds (B).
**Accept:** preview never invents a side channel; missing bind = gated dock.

## Stage 7 — 3D / temporal twin hardening
1. Every 2D layout/interface gets Stage depth + Timeline tracks; named beats only (no free tweens).
2. Motion grammar owned by monet (soft rise / breath dwell / z-path dissolve) — davinci owns model + acceptance.
3. Export notes for Webizen Desktop reuse (same twins, later gate).
**Accept:** checklist of surfaces with twin coverage; regressions = missing Stage or Timeline.
**Status (2026-09-05):** containers + map + q-cell carry Layout/Stage/Timeline chips; map Timeline drives Cosmic.stardate / FLRW. Webizen extract still later.

## Stage 8 — Webizen Desktop handoff (chrome only)
1. After Poet stage 1–5 solid: extract chrome contract + twin rules for Webizen Desktop lane.
2. No Solid IdP chrome until Capt. unparks **G-SOLID-IDP**.
**Accept:** handoff markdown; no Solid work in this plan.

## Sleep / continuation protocol
1. Stages are ordered; overnight agents start at first unchecked stage.
2. Push docs + small chrome notes to GH before large UI churn.
3. Re-read freeze tip + `impl-plans-INDEX.md` + inventory before coding.
4. If bind missing: gate UI + row to Vibe — do not invent Host methods or dotted IDs.

## Dependencies
- Frozen `poet::vibe_host` / `vibe-wasm` @ `6dc2b8b8` (`HOST_VERSION` = `vibe-host-0.1`)
- Live: `GraphDatabase.sparql` · `volume_open` · `volume_commit` · `Inference.*` · `Render.*`
- Docs: toolchest inventory · `q42-volume-shape-G-B-001.md` · vibe sprint-B plan · neo follow-on plan
- Pair: monet (look/motion) · Marvin (shapes) · Neo (binds) · Vibe (deltas)

## Out of scope
- Host widen · dotted `qualia.*` · Solid IdP UI this sprint · DNS/IP-class addressing claims · code-editor-first Poet · fake durable save · free tween animation systems
