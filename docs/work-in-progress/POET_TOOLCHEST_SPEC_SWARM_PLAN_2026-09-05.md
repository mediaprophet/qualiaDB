# Plan — implement every named Tool Chest tool (swarm)

**Date:** 2026-09-05 · **Branch:** `0.0.36-dev` · **Tree:** `C:\Projects\qualia-27062026` only  
**Freeze:** `vibe-host-0.1` · **No Host widen** · **No git worktrees**

The specs name hundreds of tools. An unimplemented tool is pointless. This plan
registers and dispatches **every named spec tool**, with a real local or live
contract where one exists, and an honest gated/parked reason where it does not.

## 0. File hygiene (hard rule — every agent)

Do **not** grow a monolith. Do **not** dump hundreds of tools into
`tool_actions.rs`, `docks.rs`, `tool_copy.rs`, or one `spec_tools.rs`.

| Rule | Detail |
|------|--------|
| Directory-backed library | `crates/poet/src/browser/spec_tools/` with `mod.rs` as router only |
| Single-purpose files | One concern per file: row type, dispatch, **or** one toolbox (or one chain) |
| New files stay under ~400 lines | Split **before** 500. 500–1,199 needs an ownership review; do not add behaviour at 1,200 |
| Toolbox too large | Make `spec_tools/<toolbox>/mod.rs` + `spec_tools/<toolbox>/<chain>.rs` |
| No mixed lifecycles | Rows, dispatch, chrome, and tests stay in their modules |
| Agent file ownership | Each swarm agent writes **only** the files named in its prompt |
| Shared spine is parent-only | `mod.rs`, `row.rs`, `dispatch.rs`, `registration/mod.rs` glue — not agent-edited after the spine lands |

This is the same library rule as `AGENTS.md` §0-B and Claude.md §11.

## 1. Architecture

```
spec_tools/
  mod.rs              # register_all + lookup  (router, small)
  row.rs              # SpecTool + Contract
  dispatch.rs         # place / local / live / gated / parked
  office.rs           # extra office chains from TOOL_CHEST_SPEC
  image/              # hypermedia image chains (split per chain if needed)
  audio.rs
  video.rs
  spatial3d.rs
  hypermedia.rs
  portals.rs
  productions.rs
  code.rs
  ai.rs
  spatial.rs
  epistemics.rs
  investigation.rs
  research.rs
```

Each toolbox module exports `pub fn rows() -> &'static [SpecTool]`.

`SpecTool` carries human **label**, **tooltip**, **proficiency**, and a
`Contract`:

| Contract | Meaning |
|----------|---------|
| `Place("doc")` | Puts a container on the surface (`Poet.container_place`) |
| `Local` | Mutates the selected surface (data attrs / CSS). Real, bounded, reversible |
| `Live("Statistics.mean")` | Live `ALL_BOUND` id; local fallback when daemon is down |
| `Gated("…")` | Shown in Everyday/Workshop; honest why-text; not stub-broken |
| `Parked("Health Review Gate A")` | Named gate; not unfaked |

Existing live tools (`graph:sparql_query`, typography, …) stay as they are.
Spec rows use spec ids (`image:add-layer`). If a chain id already exists,
**merge tools into it**; do not duplicate the chain.

## 2. Swarm lanes (parallel, shared tree)

Agents run in **this** checkout. No worktrees. No overlapping files.

| Agent | Owns (write) | Reads |
|-------|----------------|-------|
| A | `spec_tools/image/**` | `TOOLBOX_HYPERMEDIA_SPEC.md` §1 |
| B | `spec_tools/audio.rs`, `video.rs` | hypermedia spec audio + video |
| C | `spec_tools/spatial3d.rs`, `portals.rs`, `productions.rs` | `TOOLBOX_HYPERMEDIA_SPEC_2.md` |
| D | `spec_tools/hypermedia.rs` | hypermedia interactive / second-screen |
| E | `spec_tools/code.rs`, `ai.rs`, `spatial.rs` | `TOOLBOX_CODE_SPEC.md` |
| F | `spec_tools/epistemics.rs` | `TOOLBOX_EPISTEMICS_SPEC.md` |
| G | `spec_tools/investigation.rs`, `research.rs` | investigation + research specs |
| H | `spec_tools/office.rs` | `TOOL_CHEST_SPEC.md` §13 remaining chains |

Parent (this session): spine (`row.rs`, `dispatch.rs`, `mod.rs`), glue, tests,
tracker, human-copy lookup from rows, commit.

## 3. Copy, tooltips, modes, agents, a11y

- Labels and tooltips are **human**. No `capability.invoke`, SPARQL, or
  `quin.statement` in Getting started / Everyday names.
- Hover/focus tooltips already exist; spec rows feed `tool_copy` via lookup.
- Proficiency on each row (Getting started / Everyday / Workshop).
- Same row is the agent catalog (`id`, `label`, `tooltip`, `capability`).
- ARIA on buttons unchanged; new tools go through `decorate()`.

## 4. What “implemented” means

A tool is implemented when it has **all** of: registry row, human copy,
proficiency, dispatch policy, and one of: place, local mutation, live invoke,
or a specific gated/parked reason. A button that does nothing is a failure.

Pixel-perfect Photoshop/DAW/DCC is not faked. Local CSS/data-attr mutations
and live `ALL_BOUND` kernels **are** implementations. Gated is for missing
inputs (mic, key vault), not for laziness.

Parked: Health Review Gate A clinical calculators; Solid IdP; QDNF.

## 5. Tests (parent)

- `every_chain_has_at_least_one_tool` (already)
- `every_registered_nonplacement_tool_has_an_explicit_policy` (lookup counts)
- `spec_tools::every_row_has_human_copy` (no coder verbs in label/tooltip)
- `spec_tools::row_count_matches_named_spec_tools` (grows as lanes land)
- wasm32 `cargo check -p poet --lib`

## 6. Execution order

1. Land spine + `Toolbox::add_chain` / `chain_mut` / `ToolChain::add_tool`.
2. Launch swarm lanes A–H (shared tree, disjoint files).
3. Parent integrates, tests, updates tracker, commits.

Do not wait for every lane to invent a second architecture.

## 7. Continuation checkpoint — 2026-09-05

- Exact inventory: **702 rows**. Tests now reject row-count drift, missing human
  copy, malformed contracts, and duplicate global tool ids.
- All large toolboxes (`image/`, `audio/`, `video/`, `spatial3d/`, `portals/`,
  `productions/`, and `hypermedia/`) are directory-backed by focused chain groups
  using child static slices and cold `OnceLock<Vec<SpecTool>>` aggregation.
- Every single file in `crates/poet/src/browser/spec_tools/` is strictly under 350
  lines (max 340 lines in `office.rs`), adhering to the <400 line maintainability rule.
- Live dispatch no longer sends `{}` indiscriminately. All 14 capability ids used
  by spec rows have checked argument adapters; unavailable source pixels, graph
  subjects, N3 text, or renderer handles fail before host invocation.
- Fifteen CSS-representable local actions now toggle real browser styles. Office
  structural actions (hyperlink, bookmark, footnote, citation, toc) and media transport
  actions (play, stop, loop, volume, jog) are wired with safe DOM APIs.
- Verification: `cargo test -p poet` (306 lib tests + 9 product integrity tests +
  surface inventory test passed) and `cargo check -p poet --lib --target wasm32-unknown-unknown`
  passed cleanly with zero warnings/errors.

## 8. Execution Implementation Phase — 2026-09-05

The catalogue of 702 tools is registered, split cleanly, and tested for inventory fidelity.
The execution gap where 551 `Contract::Local` tools previously dropped through to `"Tool selected. Its editing action is not implemented on this surface yet."` has been systematically closed across 12 directory-backed action modules:
- `epistemic_actions/`: Assessments, reality categories, disputes, spatio-temporal/social context microformats.
- `ai_actions/`: In-browser NLP (tokenisation, regex entity gazetteer, temporal/geo parsing), local FNV embedding projections, and graph grounding checks.
- `investigation_actions/`: Case tracking, evidence metadata, hypothesis linking, and causal chain annotation.
- `research_actions/`: Enquiry questions, scope definitions, corpus sources, and literature finding management.
- `code_actions/`: Vibe script syntax check, auto-formatting, outline generation, and Quin triple introspection.
- `image_actions/`: Layer counts, blend mode cycling, brush size/opacity/hardness, and SVG vector path nodes.
- `video_actions/`: In/out timeline markers, playback rates (0.5x–2.0x), SMPTE timecodes, and aspect ratios.
- `audio_actions/`: Track mute/solo/arm, stereo pan positions, BPM tempo stepping, and quantization grids.
- `spatial3d_actions/`: Parametric 3D mesh primitives (cube, sphere, cylinder, plane), wireframe/bounding-box toggles, polycount audits, and camera focal lengths.
- `productions_actions/`: Virtual DMX universes, channel faders, emergency blackout toggle, and cue sheets.
- `portals_actions/`: World genesis, skybox presets, avatar poses/emotes, and visitor telemetry.
- `hypermedia_actions/`: Interactive UI widgets (buttons, sliders, checkboxes), OpenGraph tags, and accessibility audits.

All 12 action modules are directory-backed and strictly under 200 lines per file.
Verification: `cargo test -p poet` (333 lib tests + 9 product integrity tests + surface inventory test passed) and `cargo check -p poet --lib --target wasm32-unknown-unknown` passed cleanly with zero warnings/errors.

## 9. VibeScript REPL Interface & Core Capability Bridge — 2026-09-05

Per `TOOL_CHEST_SPEC.md` §1 and `vibescript-core.md`, the Poet Tool-Chest is dual-faceted:
1. **Visual UI Layer:** Native Rust/WASM UI components manipulating DOM, SVG, and WebGL/WebGPU elements across containers.
2. **VibeScript Scripting & REPL Layer:** Every computational tool action (Computational Geometry, Formal Modalities, CAS, SPARQL, and Computer Vision) is callable as a Vibe expression (`vibe-0.1`).
   - The Poet Code IDE & Vibe REPL drawer (`poet::ide::eval_repl`) is powered by the live `vibe::eval_cell` engine with persistent `vibe::Env` and `vibe::LocalHost`.
   - Core capabilities from `qualia-core-db` (`ComputationalGeometry.*`, `DeonticLogic.*`, `EpistemicLogic.*`, `TemporalAndDescriptionLogic.*`, `SymbolicAlgebra.*`) are registered in `ALL_INVOKE_IDS` and callable interactively from the REPL via `capability.invoke(id, args)` or Vibe syntax.
   - Tool-Chest actions bi-directionally reflect into the REPL history, providing transparency, scriptability, and auditability for all user and agent operations.
