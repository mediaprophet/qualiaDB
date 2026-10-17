# QualiaDB Website Update — audit-assisted review

**Date:** 2026-10-17 · **Method:** cross-referenced the tree-sitter audit registry
(`audit-docs/` — 5,056 files / 54,077 functions / 38 modules) and the Ecosphere capability
reference against every page in `docs/` (126 HTML pages, Jekyll site, `menu.json` nav).

**Audit domain totals** (functions classified by keyword, `audit-docs/CAPABILITIES.md`):
graph 1,549 · 3d 1,107 · semantics 943 · computational geometry 913 · numerics 911 ·
physics 593 · economics 256 · 2d 252.

---

## 1. Headline finding

The site demos the *semantic-engine / compute-library / LLM / logic* story well —
`compute-engine.html` alone exposes **87 live WASM exports** (cas, crypto, exact, graph,
linalg, numerics, stats, transforms, units), the Modalities Observatory runs 21 logic
families in-browser, and the Vibe/SPARQL/LLM sections are deep.

**The entire living-world capability stack — the engine's actual differentiator — is
invisible.** These audit-verified capability families have **zero pages and zero mentions**:

| Capability family | Audit evidence | Site mentions |
|---|---|---|
| **Accrual modality** (standing stocks: biomass, habitat, hydrological function, formation/destruction asymmetry, 6 irreversibility classes) | `modalities/accrual.rs`; described as the engine's reason to exist | **0** — not even in the 21-modality observatory or test suites |
| Flora/biosphere entity representation kernels — the substrate for *rendering* flora entities, not a PlantGL-style product (turtle_3d L-systems, sweep_extrusion, space_colonization, tree_skeleton_3d, canopy_radiation, botanical_hulls → entity bundles → render lanes) | `specialized_libs/computational_geometry/` — the repo's PlantGL gap-review is internal evidence of kernel coverage, not the product goal | 0 |
| Physics simulation (`physics_simulation/` ~25 files: nbody, molecular dynamics, CFD, EMF, **3-D boids flocking**; `pbd_softbody` XPBD) | 593 physics functions | 0 |
| Engineering analysis (2-node truss + Euler-Bernoulli FEM, Newmark-β, buckling, modal/FRF, D2Q9 LBM, reliability) | `specialized_libs/engineering_analysis/` | 0 |
| Audio — the whole surface (qualia-audio crate: **1,502 functions** — MIR stack, MIDI 1/2 UMP, bounded production engine, AQA, sonification; core `audio/` holosonic wavefield synthesis, HRTF, bioacoustics) | module registry | 0 pages; `js/qualia-audio-worklet.js` exists but **nothing loads it** |
| Vision / bio-imaging (`computer_vision/` morphology/watershed/Voronoi-Otsu; `qualia-vision` VisionQuin ABI) | module registry | 0 |
| Economics suite (`computational_economics/` ~33 files: environmental_resource, spatial_economics, game_theory, mechanism/market design, welfare) | 256 economics functions | 0 (`monetization.html` covers ILP/obligations, not the library) |
| Ethograms / entity affordances (`entity_view/ethogram.rs` EONT1↔HFSM↔SKL1 binding; `DevelopmentalKeyframeRecord` ontogeny) | §10.1 of capability ref | 0 |
| Spectral pipeline (`spectral_basis` Jakob-Hanika R(λ) basis, metamer, gamut, hyperspectral-cube admission) | `render/spectral*.rs` | 0 |
| Commons economics mechanics (`socioeconomics` obligation-cost/royalty-tree/usury/EROI; `CommonsObligation::discharge_to_commons`; QDNF metering WP-4.1) | `modalities/socioeconomics.rs`, `bundle/cbor_ld_manifest.rs` | 0 (monetization page is ILP-focused) |
| Oct-15→17 additions: holographic 3D GPU lane + `holographic.wgsl`, holosonic field synthesis, ecology SHACL shapes, HyperspectralCube, hypermedia lifecycles | commits bc9a8683, c3a870be, 5395fb7b, 7bfc84d4 | 0 |

**Net effect:** a visitor sees a capable semantic/SPARQL/LLM engine and misses that the same
binary represents and renders living-world entities — flora morphology, bounded physics,
spectral rendering, bioacoustic MIR, and commons-discharge economics.

---

## 2. Broken / orphaned assets (fix first — cheap wins)

- **Broken nav entry**: `menu.json` links `vibe-host-native-bridge.html` — file does not exist.
- **Orphaned JS** (loads from nothing):
  - `js/qualia-audio-worklet.js` — AudioWorklet plumbing with no host page.
  - `js/ambient-viz.js` — no referencing page.
- **`wordnet.html`** is a bare redirect stub to `ontology.html?dataset=wordnet`; either fold it
  into the redirect map (`_redirects`) or give it a real landing.
- **Content freshness**: most pages are dated 2026-10-01; newest progress page is 0.0.40
  (2026-10-10). Nothing reflects the 10-15→10-17 commits (holographic engine, holosonic,
  ecology shapes, hyperspectral, hypermedia lifecycles). A `progress-0.0.41.html` or a dated
  "latest" banner on index would keep the site honest.

---

## 3. Improvement areas for existing demos

- **`compute-engine.html`** — currently exposes only `wasm_bridge/engine/*`. The bridge also
  builds `bio`, `chemistry`, `geometry`, `governance`, `logic`, `medical`, `meta`, `semantic`
  modules. Adding a second domain rack (even 10–15 more exports: a geometry predicate,
  `shacl_validate`, a governance/deontic call, `sense`-path bins) would demonstrate the
  "engine is one ABI" claim better than prose does.
- **`science-playground.html`** — only 6 panels (clinical risk, G(3,0,0) PGA, organic chem,
  RK4, sequence alignment, thermo MC). Natural additions from the audit: a flora-entity panel
  (turtle_3d or space_colonization → entity representation → rendered result — the biosphere
  story, not a bare kernel), a `physics_simulation` panel (boids or nbody conservation), an
  `environmental_resource` econ panel.
- **`modalities-showcase.html`** — 21 live modalities is strong, but accrual is absent and it
  is the ecologically distinctive one. Adding an accrual panel (multi-dimensional standing
  stock + formation/destruction asymmetry) would anchor the "logic for the living world"
  narrative. The 8-category grid should also say *which* families exist natively vs.
  WASM-only vs. not-yet-exported.
- **`data-formats.html`** — doc-only today. The site could run a live format sniff:
  paste/upload bytes → detect `Q42!`/`m64`/`p64\0`/`hmc`/`VBC1` magic and dump the header
  parse (all trivially WASM-able since readers are `#[no_std]`-lean).
- **`graphics.html`** — 356-line architecture essay for the render stack. The actual kernels
  (spectral basis, Cook-Torrance path, atmosphere presets, water) could get a small live
  strip: a spectral-reflectance swatch computed via `spectral_basis`, or a static canvas fed
  by the holographic uniform block.
- **`qdnf*.html`** — 4 status/docs pages + `qdnf-map.js` (used on `qdnf.html`). Given
  `net/qdnf` is 323 files including economics meters, a tiny "metered exchange" walkthrough
  (two cells, `MeterReading` → `accumulate_meter_to_pool` → `discharge_to_commons`) would
  make the commons-sunset story concrete — it is the flagship economics demo and currently
  exists only as Rust tests.
- **`forge-showcase.html`** — 142 lines, descriptive only; `wgsl_forge/` (72 files) supports
  real kernel generation/certification — at minimum embed one generated WGSL kernel with its
  certification receipt.

---

## 4. Suggested new pages (priority order)

1. **`ecosystem-lab.html`** (or extend science-playground) — the missing flagship: accrual
   stocks + flora entity growth + ethogram state + commons economics in one scene. Even
   canned/precomputed data would show what no other site can claim: a semantic engine that
   *represents and renders living things*, not just facts about them.
2. **`audio.html`** — host `qualia-audio-worklet.js`: HRTF binaural demo + MIR feature panel
   (pitch/chroma/tempo on mic/upload) + holosonic wavefield visual. The library is the
   largest unexposed crate surface (1,502 functions).
3. **`flora-entities.html`** — the biosphere-rendering pipeline end-to-end: L-system /
   space-colonization generation → entity bundle (EONT1 affordances, developmental keyframes)
   → rendered output. A small live path (generate → inspect entity record → render) shows the
   point of the geometry kernels: they exist to serve flora entity representation, not to be
   a PlantGL-style library. Supporting panel: exact predicates (orient3d/incircle vs. float64
   failure cases) and tree-skeletonization on an uploaded point cloud for the technically
   curious. `geometry-bench.js` already proves WASM geometry runs.
4. **`physics-sim.html`** — boids flocking (already zero-heap), nbody conservation meter,
   D2Q9 LBM cavity flow, FEM truss deflection. One panel per `physics_simulation`/`engineering_analysis` module.
5. **`econ-sandbox.html`** — `environmental_resource` extraction-vs-stock curves, mechanism
   design auction, and the `socioeconomics` obligation→discharge walk (with the honest caveat
   that production callers aren't wired — matching the capability reference's evidence
   levels).
6. **`formats-inspector.html`** — upgrade `data-formats.html` into a live magic/header parser
   for q42/10d/hmc/p64/vibe (m64 can follow when it lands).
7. **`vision.html`** — `qualia-vision` CPU reference detector → `VisionQuin` records;
   bio-imaging morphology pipeline (watershed/Otsu) on a sample image.

## 5. Structural / meta improvements

- **A capabilities map page**: the audit already generates `audit-docs/CAPABILITIES.md`
  (domain → module → function counts). Publishing a rendered version as `capabilities.html`
  gives the site a single "what does it do" surface, kept honest by regeneration.
- **Freshness contract**: add a "site verified against commit <hash> on <date>" footer fed
  from CI; several pages currently describe pre-October code.
- **Honesty labels**: the repo's own evidence-level vocabulary (Implemented / Library-only /
  Partial / Plan-only) could badge each demo — pages like `forge-showcase` would visibly say
  "descriptive; runtime not wired to browser," matching the reference doc's conventions.
- **`menu.json` hygiene**: remove/fix `vibe-host-native-bridge.html`; consider grouping the
  ~60 entries under clearer hubs (Engine / Science / Living-world / Tools) — the flat list
  hides how much exists.

---

### Appendix — evidence basis

- Registry: `C:\github\qualiadb\audit-docs\` (`FUNCTIONS.md` 54,077 rows, `CAPABILITIES.md`,
  38 `modules/*.md`) — regenerate with `audit docs C:\github\qualiadb`.
- Audit graph: `C:\github\qualiadb\.audit\codebase.q42` (1.39 M triples) — SPARQL-queryable.
- Capability claims cross-checked against `C:\github\ecopunk\docs\QualiaDB Capability Reference.md`
  (verification pass 2026-10-17) — same evidence levels apply here.
