# GitHub Pages / WASM Demo Audit — 2026-06-18

Audited the docs/ demo pages locally (built WASM + Tailwind, served `docs/` over
`http://127.0.0.1:8744`, driven in Chrome). Focus: why the "new build" showed
**"WASM Engine Required"** and the general health of the other pages.

> **Update (later same day):** menu fixed site-wide (hardcoded `site-nav.css`
> `<link>` on all 27 affected top-level pages + `menu-loader.js` empty-root fix;
> local server switched to no-cache). Also discovered and began fixing a bigger
> integrity issue — see **§7 Benchmark Hub: simulated → real**.

**Headline:** the WASM engine itself builds and runs correctly. The visible
failure was a **CSS regression**, now fixed. Verified live: WASM loads, geometry
encodes, and `science-playground` actually computes a real geometric product
through WASM.

---

## 1. Root-cause fix — overlay stuck on "WASM Engine Required" ✅ FIXED

**Symptom:** pages showed the full-screen "WASM Engine Required" error overlay
even though the WASM engine had loaded successfully (green badge, quins encoded,
no console errors).

**Cause:** the switch from the Tailwind **Play CDN** to a pre-built
`css/tailwind-built.css` upgraded the site to **Tailwind v4**, which emits all
utilities inside `@layer utilities`. The pages define their overlays in an
*unlayered* inline `<style>` (`.error-overlay { display: flex }`). In the CSS
cascade, **unlayered rules always beat layered ones** (layer order outranks
specificity), so `.hidden` (now layered) could no longer hide the overlay. The
overlay sat on top of a perfectly working page.

This had already been hit once: `design-studio.html` carried a one-off
`.ds-hidden { display:none !important }` workaround that was never propagated to
the other five overlay pages.

**Fix (two layers, belt-and-suspenders):**

1. **Central** — `docs/css/tailwind-input.css`: added an *unlayered*,
   higher-specificity rule (`.error-overlay.hidden, .loading-overlay.hidden { display:none }`)
   and rebuilt `tailwind-built.css`. Fixes every page via the shared stylesheet
   and flows through the CI `build:tailwind` step automatically.
2. **Inline per page** — added the same rule to each affected page's own
   `<style>` block. This is **immune to the stale-CSS-cache problem** (the
   stylesheet link has no version hash, so a browser holding the old
   `tailwind-built.css` would otherwise keep showing the bug). Pages fixed:
   `spatial.html`, `benchmark.html`, `science-playground.html`,
   `scientific-computing.html`, `zero-heap-compliance.html`.

Deliberately **not** `.hidden { display:none !important }` — that would break the
11 responsive `hidden sm:flex` / `hidden xl:flex` nav patterns across the site.

**Verified live after fix:** `benchmark.html` and `spatial.html` hide both
overlays on a plain reload; `science-playground` Geometric Product returns
`[2,1,1,-1,2,0,0,1]`, 64 compute ops, no errors.

---

## 1b. Navigation menu broken on ~26 pages ✅ FIXED

**Symptom:** the top nav rendered with every dropdown panel expanded and spilling
across the page (all of Core/Benchmarks/SPARQL/Sciences/Tools/LLM/Data items
visible at once, overlapping the hero).

**Cause:** the dropdown styling lives in `css/site-nav.css`
(`.nav-dropdown { opacity:0; visibility:hidden; position:absolute }`). Only **3 of
29 pages** (`index`, `spatial`, `modalities-showcase`) hardcode the
`<link rel="stylesheet" href="css/site-nav.css">`. The other 26 rely on
`menu-loader.js` → `ensureSiteNavCss()` to inject it — but that function bailed
when `docsRootFromScript()` returned an **empty string**:

```js
const root = docsRootFromScript();
if (!root || document.querySelector('link[data-site-nav]')) return;  // bug
```

`""` is a *valid* root (page-relative). On GitHub Pages the path contains
`/qualiaDB/` so `root` is non-empty and the menu works — but **locally (or any
host not under `/qualiaDB/`) `root` is `""`, injection is skipped, and the menu
breaks on all 26 non-hardcoded pages.** That matches "broken on all pages" when
testing the local build.

**Fix:** `docs/js/menu-loader.js` — drop the `!root` guard; only bail if a
site-nav stylesheet is already present (also avoids double-loading on the 3
hardcoded pages):

```js
if (document.querySelector('link[data-site-nav], link[href$="css/site-nav.css"]')) return;
const root = docsRootFromScript();
// inject root + 'css/site-nav.css'  (root may be '')
```

**Verified live:** on `benchmark.html` the nav now renders as a single collapsed
bar, and clicking **Core** opens a correctly absolute-positioned dropdown
(Home / Features / API Docs / Manuals).

> Note: `menu-loader.js` is itself browser-cached with no version hash, so local
> testing needs a hard refresh (Ctrl+Shift+R) to pick up the new script — same
> caching caveat as §4.2. Fresh visitors after deploy get it automatically.

---

## 2. Page-by-page results

Legend: ✅ works · ⚠️ works with a caveat · ❌ broken

| Page | Status | Notes |
|------|--------|-------|
| `spatial.html` | ✅ | Deep-tested. 3D viewer renders, 52 vertices → 52 quins encoded via WASM, badge "Qualia WASM · T1". Overlay fixed. |
| `benchmark.html` | ✅ | Overlay fixed (was the page you reported). CI Platform Suite + all tabs render, no console errors. |
| `science-playground.html` | ✅ | Overlay fixed. **WASM compute verified** (geometric product). All 6 tabs present. |
| `scientific-computing.html` | ✅ | Overlay fixed, clean boot. |
| `zero-heap-compliance.html` | ✅ | Overlay fixed, clean boot. |
| `design-studio.html` | ✅ | Portal WASM initializes (slow ~15 s boot), "Qualia Portal initialized", GGUF asset recommendations render. Already had its own `.ds-hidden` fix. |
| `index.html` | ✅ | Clean. |
| `ontology.html` (+ `wordnet.html` → same) | ✅ | WASM ready, 6,540 blocks, 126.9 MB ingested to browser storage; W3C (24) / Archives (50) / PURL (11) / FIBO (10) selectors populated. `.q42` volumes present locally. |
| `comparative_benchmarks.html` | ✅ | Clean, data loaded. |
| `sparql-showcase.html` | ✅ | Clean. |
| `modalities-showcase.html` | ✅ | Clean. |
| `logic-showcase.html` | ⚠️ | Renders fine, but emits a non-fatal COI console error (see §4). |
| `online-llm-demo.html` | ✅ | Clean boot (loads `pkg/qualia/qualia.js`). LLM generation not exercised. |
| `edge-llm.html` | ✅ | Clean. |
| `analytics.html` | ✅ | Clean (light content). |
| `network_webizen.html` | ✅ | Clean. |
| `playground/index.html` | ✅ | Clean (same full WASM as science-playground, which compute-verified). |
| `api-explorer/` | ✅ | Clean. |
| `benchmark_visualizer.html` | ✅ | Clean. |
| `phone-console.html` | ✅ | Minimal by design (companion-device console; needs pairing). |

**Not individually driven** (static/content or lower priority, no WASM gate):
`edge-llm-showcase.html`, `sparql-examples.html`, `cost_model.html`,
`docuquin-pipeline.html`, `advanced-features.html`, `api.html`,
`approved_developers.html`, `data-formats.html`, `escrow.html`, `issues.html`,
`monetization.html`, `utils.html`. Worth a follow-up pass if any are interactive.

No page showed a genuinely broken WASM engine. Every "blocked" appearance traced
to the overlay-cascade bug (now fixed) or to expected slow/async loading.

---

## 3. CI workflow inconsistency (likely why Actions were failing)

The last commit (`8df3d1b9`) fixed **`pages.yml`** to install `wasm-pack` from a
release binary because `cargo install wasm-pack` was failing in CI. That fix was
**not** propagated:

- `.github/workflows/benchmarks.yml:108` — still `cargo install wasm-pack --locked --version 0.13.1 --force`
- `.github/workflows/release.yml:136` — same

If those runs are red, this is the most likely reason. They should get the same
release-binary install block that `pages.yml` now uses.

---

## 4. Secondary findings (non-blocking)

1. **COI service-worker double-declaration** — `coepCredentialless already
   declared` SyntaxError (uncaught) from `js/coi-serviceworker.js` on some
   showcase pages. The vendored script (v0.1.7) declares `let coepCredentialless`
   at classic-script top level, so it throws if injected twice into one document.
   Non-fatal (pages render). Recommend de-duplicating the COI bootstrap so the
   script is injected once per page. May be partly aggravated by the local server
   not sending COOP/COEP headers (so isolation never settles and the SW keeps
   re-registering across rapid navigation).

2. **No cache-busting on `tailwind-built.css`** — the `<link>` has no
   hash/version query, so returning visitors can be served a stale stylesheet
   after a deploy. The inline fixes in §1 protect the 5 overlay pages, but a
   content hash (or `?v=` query) on the stylesheet link is the durable fix and
   would prevent this whole class of "deployed but looks old" problem.

3. **`ontology.html` depends on CI-built `.q42` volumes** — the
   `prepare_w3c_ontologies.sh` / `prepare_fibo_ontologies.sh` etc. steps in
   `pages.yml` are `continue-on-error`. If any fail in CI, production ontology
   data can be silently missing while the page still "deploys". Locally the
   volumes exist and the page works.

4. **Background-tab boot stall (`spatial.html`, `qualia-shell` portal pages)** —
   `bootSpatialPage` awaits `new Promise(r => requestAnimationFrame(r))`
   (`js/spatial-demo.js:622`) before loading the portal WASM. `requestAnimationFrame`
   is paused by the browser while a tab is not visible, so a page opened in a
   **background tab stays on the loader until it is focused**. Confirmed via
   `document.visibilityState === "hidden"` during automated testing. Normal
   foreground use is unaffected (spatial booted fully — 3D viewer, 52 quins —
   when the tab was visible). Low priority, but a `visibilitychange` fallback or
   a `setTimeout(0)` race would make boot robust to background tabs.

---

## 5. Files changed in this pass

- `docs/css/tailwind-input.css` — central unlayered `.hidden`-for-overlays rule.
- `docs/css/tailwind-built.css` — regenerated via `npm run build:tailwind`.
- `docs/{spatial,benchmark,science-playground,scientific-computing,zero-heap-compliance}.html`
  — inline overlay-hide rule (cache-proof).
- `docs/js/menu-loader.js` — inject `site-nav.css` even when the docs root is `""`
  (fixes the nav on all 26 non-hardcoded pages). See §1b.
- `docs/playground/qualia_core_db.{js,wasm}` — rebuilt locally (CI regenerates
  these anyway; safe to commit or discard).

## 6. Suggested follow-ups (for the further review)

- [ ] Propagate the `wasm-pack` release-binary install to `benchmarks.yml` and `release.yml`.
- [x] De-duplicate the COI service-worker injection (§4.1) — done: idempotency guard in `js/qualia-coi.js`.
- [x] Add a content hash / `?v=` to the `tailwind-built.css` link site-wide (§4.2) — done: `docs/scripts/stamp-asset-versions.mjs`, run on `_site` in `pages.yml`.
- [x] Decide whether ontology `.q42` build steps should be hard failures (§4.3) — decided **keep soft** (heavy `cargo run` compile; graceful WordNet fallback) + added a non-fatal `::warning::` summary step in `pages.yml`.
- [x] Make portal boot robust to background tabs (§4.4) — done: rAF-vs-timeout race in `spatial-demo.js` + `design-studio-app.js`. Verified: hidden-tab boot now completes in ~340 ms.
- [ ] Drive the remaining un-tested content/interactive pages listed in §2.

---

## 7. Benchmark Hub: simulated → real (in progress)

**Finding (confirmed):** the Benchmark Hub's six interactive tabs (Graph,
Scientific, Spatial, Q42 10D Tensor, SPARQL, Zero-Heap) did **not** call the WASM
engine. Each "Run benchmark" handler was a `setTimeout` returning **hardcoded
constants** + `Math.random()` jitter, printed as `✓ benchmark complete`
([benchmark.html] old lines 890/936/994/1049/1107/1163). Only the **CI Platform
Suite** tab was real (reads measured `llm_benchmark_results.json`). The 10D engine
was never invoked. Meanwhile the engine *does* export real functions
(`geometric_algebra_operation`, `parse_turtle_wasm`, `compile_query_to_json`,
`spatial_encode_wasm`, `geosparql_operation_wasm`, `execute_ntriples_query`, …)
and `spatial.html` already had an interactive WebGPU portal viewer.

**Exemplar done (2 of 6 tabs) — new module `docs/js/benchmark-live.js`:**

- **Spatial Math** → real `geometric_algebra_operation` (geo/inner/outer/reverse),
  measured ops/sec + a genuine G(3,0,0) product sample; charts now show measured
  data. Added a **zero-dep interactive 3D object viewer** (`Object3DViewer`):
  canvas wireframe of 6 procedural solids (tetra/cube/octa/icosa/sphere/torus),
  drag-to-rotate, scroll-zoom, depth shading. Verified live.
- **Graph Operations** → real `parse_turtle_wasm` (ingestion, measured
  triples/sec) + `compile_query_to_json` per pattern (measured ms/op); shows the
  real compiled plan. Added a **zero-dep interactive force-directed graph**
  (`GraphViewer`) rendering the parsed RDF (labeled nodes/edges), drag nodes,
  pan, scroll-zoom. Verified live.

The simulated `runSpatial*`/`runGraph*` functions were removed from
`benchmark.html`; the live versions are wired via the page's module script
(`window.benchCharts` exposes the Chart.js instances). The "simulated timings"
banner on the Graph tab was corrected.

**Still simulated (pending replication of the same pattern):** Scientific
Computing, Q42 10D Tensor, SPARQL Queries, Zero-Heap Demo tabs. These should be
rewired to `ode_solver`/`thermodynamics_mcmc`/`clinical_risk`/… ,
`export_tensor_slice_wasm` (real 10D), `compile_query_to_json`, and the zero-heap
metrics respectively — and their fabricated output removed.

- [ ] Replicate the live-WASM + render pattern to the remaining 4 benchmark tabs.
