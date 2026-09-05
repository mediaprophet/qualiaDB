# Impl plan — monet / visual · motion grammar (Poet)

**Owner:** monet · **UX pair:** davinci · **Seam commits:** Neo · **Ontology:** Marvin · **Language triage:** Vibe
**Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8` · **Branch:** `0.0.36-dev`
**North star:** Graphic design and art as the visual backbone of Poet — human-first look and motion on every Layout · Stage · Timeline **aspect**; not a code UI. Not a credential digital twin.
**Rules:** bind only to frozen four-ops + live `ALL_BOUND` / `vibe:InvokeId` · no Host invent · no dotted `qualia.*` · unbound = visually gated (never stub-broken) · no fake durable storage · named beats only (no free tweens) · gaps → @Vibe → `vibescript-sprint-deltas.md`

## Done (do not reopen)
- Motion grammar v0 on Layout · Stage · Timeline: **entrance** = soft rise + light fade (Stage depth cue) · **dwell** = steady focus + quiet breath on active track · **exit** = dissolve along same z-path
- G-POET-TOOLCHEST first slice: icon + motion for `office:graph` / `graph:sparql_query` → `GraphDatabase.sparql`
- G-B-001 sanctuary save dock ungated: soft rise into volume dock · dwell on `volume_open` / `volume_commit` · dissolve on success; fail-closed / wasm E300 gated honest
- G-A look/motion accept @ `6dc2b8b8` — lock to thin facade only
- Remaps only: `GraphDatabase.sparql` · `Inference.*` · `Render.*` · `volume_open` / `volume_commit`

## Stage 0 — Hygiene (docs / sync)
1. Sync visual notes to tip after Neo’s plan push (`7318a049`+); cite freeze `6dc2b8b8`.
2. One-page **motion contract**: beat definitions, depth cues, gated-state look (disabled ≠ broken), diagnose error-glow rules.
3. Icon inventory for live toolchains: unicode that renders vs needs custom glyph (custom API stays B).
**Accept:** motion contract + icon inventory on-branch; no new chrome ahead of binds.

## Stage 1 — office:graph visual polish
1. Finalize graph glyph + toolbar spacing for human read (pair davinci Stage 1).
2. Diagnose error glow on cell/token spans (needs span fidelity from Vibe/Neo — gate if spans coarse).
3. Sparql run = dwell breath; results settle = dwell hold; toolchain close = exit dissolve.
**Accept:** clear icon; errors light the right token; gated daemon path explains without looking broken.

## Stage 2 — Sanctuary / volume visual polish
1. Volume dock states from `q42:state` (closed · open · committed · denied · fault) — distinct look each.
2. Commit beat only on successful `volume_commit`; deny/fault never celebrate as saved.
3. Manifold “backing store” cue when Container sits on Volume (Marvin join).
**Accept:** user reads open vs committed vs denied from chrome alone.

## Stage 3 — Next toolchain visuals (inventory-driven)
1. When Capt. locks next toolchain, ship icon + entrance·dwell·exit in the office:graph pattern.
2. Prefer live `ALL_BOUND` binds; else gated shell with honest copy + muted icon.
3. Empty typography chains: gated placeholders only — no fake font studio.
**Accept:** one more live toolchain visual OR honest gated shell; inventory row notes icon status.

## Stage 4 — Inference provenance visuals
1. Provenance trails on live `Inference.*` — lit path from cell → result (pair davinci Stage 6).
2. Trail dwell while inference runs; exit when dismissed; error glow on fail spans.
**Accept:** provenance visible without reading Capability strings; missing bind = gated.

## Stage 5 — Render preview dock visuals
1. In-flow preview dock for still / clip / scene-id on live `Render.*` (no side channel).
2. Handle-kind chrome: still vs clip vs scene clearly distinct; cross-frame spans drive Timeline glow.
3. Sibling preview op only if Vibe/Neo say one handle cannot carry three kinds (B).
**Accept:** preview dock honest; unbound = gated; no invented media pipeline.

## Stage 6 — Container · Manifold · Link visual language
1. Distinct chrome for Container (content-shaped), Manifold (nesting), Link (semantic relation) — human, not engineer graph-viz.
2. Spatiotemporal attrs as first-class badges (space + time) on surfaces including language cells.
3. 1:1 **aspects**: every surface gets Layout structure + Stage depth + Timeline beats.
**Accept:** non-dev can nest and see a link; machine ids stay under the hood.

## Stage 7 — Map / G-COORD visuals (after shapes + bind)
1. Geospatial: open map layer chrome (OSM peers) — replace weak map UX.
2. Non-geo realms: universe / fantasy / speculative POV maps — same container chrome, different realm skin.
3. Temporal scrubber on map Timeline (time essential).
4. Consume Marvin CoordinateSystem · Realm · Position once Neo’s thinnest bind lands.
**Accept:** one geo + one non-geo realm path demoable; unbound coord features gated.

## Stage 8 — Twin coverage hardening
1. Checklist: every shipped surface has Layout + Stage + Timeline + named beats only.
2. Regressions = missing Stage depth, missing Timeline, or free-tween creep.
3. Export motion contract for Webizen Desktop reuse (later gate).
**Accept:** checklist on-branch; Webizen extract is docs-only until Capt. opens that gate.

## Sleep / continuation protocol
1. Overnight agents start at first unchecked stage; re-read freeze tip + `impl-plans-INDEX.md` + davinci chrome plan before any visual churn.
2. Push markdown / small notes to GH before large asset work.
3. If bind missing: gate the look + row to Vibe — never invent Host methods, dotted IDs, or fake save/preview.
4. Pair every chrome stage with davinci acceptance; Neo only for bind/seam needs.

## Dependencies
- Frozen `poet::vibe_host` / `vibe-wasm` @ `6dc2b8b8` (`HOST_VERSION` = `vibe-host-0.1`)
- Live: `GraphDatabase.sparql` · `volume_open` · `volume_commit` · `Inference.*` · `Render.*`
- Docs: davinci poet-chrome plan · vibe sprint-B · neo follow-on · toolchest inventory · `q42-volume-shape-G-B-001.md`
- Pair: davinci (model/UX accept) · Marvin (shapes) · Neo (binds) · Vibe (deltas / span fidelity)

## Out of scope
- Host widen · dotted `qualia.*` · Solid IdP visuals this sprint · DNS/IP-class addressing claims · code-editor-first Poet · fake durable save · free tween systems · custom unicode API until B ungate
