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
