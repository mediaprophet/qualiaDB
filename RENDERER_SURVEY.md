# Renderer — code-location status check AGAINST Timothy's authored specs (2026-06-22)

**Correction note (2026-06-22):** an earlier version of this file framed this as a fresh
"survey" and credited the architecture as a hunch ("Timothy's read was right"). That was wrong
and is retracted. The renderer's architecture, scope, and the decision to host it in qualiaDB
were **defined by Timothy** in the documents below; this file only *locates the current code
against those specs* and records two concrete verified facts. It also corrects an earlier
mischaracterisation of the renderer as "2D" — it is not (see §1).

## 0. Authoritative definition (Timothy's, not to be re-derived)
- **[`10d/q42-10d-volumetric-tensor-spec.md`](10d/q42-10d-volumetric-tensor-spec.md)** — the
  renderer/manifold spec: a **10-D spacetime manifold** `[q, v, w, x, y, z, t, α, μ, σ]` with a
  **Spectral-Logical payload `[α, μ, σ]`** (Amplitude / Modulation / Spectral Signature) — *EM
  spectrum as the source of truth, device-specific projection at render time*; multi-modal
  spectral decomposition (**visual SPD + audio STFT/CQT**); gravito-thermodynamic operators; q-
  dimension for epistemic superposition. **This is a multi-modal physics-of-perception engine,
  NOT a 2D renderer.** Companion standard: `docs/manuals/standards/q42-10d-tensor-standard.md`.
- **[`20260621_webizen-browser-engine-migration-review.md`](20260621_webizen-browser-engine-migration-review.md)**
  — already answers "why it lives in the qualiaDB libraries" and makes the consolidation call:
  the heavy engine is single-source in `qualia-core-db`; **"Render belongs in qualiaDB, not the
  browser"**; consolidate the overlapping `webizen-render` + `portal_*` into **one engine-side
  render layer (`qualia-render`) + a minimal dev-bench**, with the browser as an **optional native
  shell** that consumes it. It also already flags the duplication hazard (below).
- Sonic plane: `docs/manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md` +
  `docs/manuals/standards/q42-acoustic-plane-draft.md` (the auditory/STFT-CQT path).

## 1. Where the code currently sits (verified, for status only)
- `crates/webizen-render/` — the N-D **PGA** renderer (Motors, scene contract, wgpu PNG/data-URI
  output, projector/epistemic shaders, semantic culling, epistemic LoD). Ported from
  webizen-browser.
- `crates/qualia-core-db/src/portal_*` (12 modules: gpu/camera/navigation/pga/**acoustic**/
  **spectral**/standpoint/control/telemetry/phenomenal_contract/wasm) — the WASM browser portal
  surface; **this is where the EMF/visual + acoustic/spectral projection lives** (so the
  multi-modal rendering Timothy described is partly present, not 2D).
- `compute_universe.rs`, `tensor/manifold.rs`, `shaders/viewport/*.wgsl`,
  `modalities/manifold_logic.rs` — the 10D substrate + the §20 continuous→discrete bridge.
- `docs/spatial.html` (36 KB, 2 canvases), `docs/playground/anatomy.html`, `js/spatial-demo.js`,
  `js/ambient-viz.js` — the GH-Pages surfaces.

## 2. Two verified facts that matter for the next step
1. **Build wiring:** the root `Cargo.toml` `members` lists only `qualia-core-db, qualia-cli,
   qualia-solid-bridge, qualia-client-core, qualia-extensions, webizen-component-harvester,
   qualia-mobile-harness, wellfare-core`. The render crates (`webizen-render/desktop/studio/web/
   runtime`) are **present but NOT in the workspace**, so the main `cargo build` doesn't build them.
   The in-engine `portal_*` path **is** built (part of `qualia-core-db`). This is the
   already-known duplication/divergence the migration review flagged — consolidation (Timothy's
   `qualia-render` decision) is the fix.
2. **GH-Pages runtime:** `spatial.html` is wired (2 canvases, WebGPU present) but shows **"WASM
   Engine Required"** — it's gated on a WASM engine bundle that isn't loading (same WASM-bundle
   family as the LLM-demo breakage). This is the visible "disabled" symptom.

## 3. Open decisions (Timothy's) — not new questions, just the remaining forks
Per the migration review the *direction* is already set (consolidate into `qualia-render` + dev-bench;
browser optional). The remaining calls: (a) finish that consolidation (bring the canonical render
crate into the workspace), (b) fix the WASM bundle so a 10D/spectral demo visibly runs, (c) write the
"how to define / create apps / pages" authoring docs against the 10D spec. These execute Timothy's
plan; they do not redefine it.
