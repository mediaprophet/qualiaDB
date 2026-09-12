# Constraint correction — vibescript-first incorporation

**Date:** 2026-09-06  
**Authority:** Project owner (Timothy)

## What was wrong

Agents treated `vibe-host-0.1` as a **freeze** (“no Host widen”) and spent effort on
**surveillance / classification** of built crates as “Desktop-only / out of scope.”
That burned tokens without incorporating existing infrastructure into the product
surface the owner asked for.

## Correct rules

1. **`vibe-host-0.1` is the outcome**, not a freeze. It is defined when full
   incorporation of the existing codebase into Vibe (then Poet) is done.
2. **Methodology:** **VibeScript / Host first** — add real `Family.method` ids +
   invoke handlers over built libraries → **then** Poet Tool Chest / Live consumes
   those ids. Poet does not invent the capability surface.
3. **Prefer implementation** over further gap reports. Inventory is only a short
   guide for *what to wire next*.
4. Desktop/FRB remains valid for shell/windowing chrome — **not** a default dumping
   ground for every unbound library.

## Immediate programme

1. Run exhaustive backlog: `python scripts/vibe_incorporation_backlog.py`
   (methodology: `VIBE_INCORPORATION_METHODOLOGY_2026-09-06.md`).
2. Re-open previously “do not bind under freeze” items as **Host bind + Poet Live**
   packets using Q1/Q2 queues (cooperative ABAC ≠ `Agency.evaluate`; vision /
   biosense with consent honesty; chat-graph; remaining CV/Econ consume).

Paired catalogs must stay in sync: `crates/vibe/src/catalog/ids.rs` (`ALL_INVOKE_IDS`)
and `crates/qualia-core-db/src/poet_host/invoke/ids.rs` (`ALL_BOUND`) + dispatch.

## 2026-09-08 — Host-widen for apps / REPL (owner)

Timothy: **VibeScript is a REPL language intended to support apps.** The Host catalog **grows**. Do **not** treat `vibe-host-0.1` as a freeze of
`ALL_BOUND`. Linear algebra is first-class: bind missing app primitives and route
`LinearAlgebra.gemm` through the engine solver (CPU floor, GPU when `caps()` says so).
Production polish (live invoke with the JSON Poet sends, honesty labels, machine
schemas, dual-path scan of late-wave Live files) is in-scope — not inventory-only.

Still not in scope: binding every Q1 CoreDb `pub fn`. Host-widen where it improves the
app/REPL surface, starting with `LinearAlgebra.*`.

## 2026-09-12 — dialect may grow (owner)

Timothy rejected “grammar is locked/closed” as an artificial freeze. **`vibe-0.1` is the current dialect and MAY grow** when humans need a better form — versioned and documented. `vibe-host-0.1` remains the incorporation **outcome**, not an `ALL_BOUND` freeze. Customer chips: **live** / **planned**. `held` is an internal unbound gate only.
