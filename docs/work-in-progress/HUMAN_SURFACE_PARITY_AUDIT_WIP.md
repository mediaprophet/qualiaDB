# Human-surface parity audit (WIP) — Frames A–E

**Status:** work-in-progress · **Not a release gate**  
**Branch:** `0.0.38` · **Lexicon tip:** `541c3a6` · **Lexicon:** [`HUMAN_ONBOARDING_LEXICON_WIP.md`](./HUMAN_ONBOARDING_LEXICON_WIP.md)  
**Owners:** davinci (chrome / affordances) · monet (motion / look)  
**Fold/push:** Neo · **Ops gate:** Capt  
**North star:** a person finishes cold-load **alone** — WASM ↔ desktop **same dialect and affordances** (no thinner human path).

---

## 1. Purpose

Turn Vibe’s Frames A–E + §6 dialect table into a **desktop vs WASM scoreboard** so Capt can promote human-surface work and hold Wave-22 until first-session is real.

This audit does **not** invent Host APIs or Capability ids. Gaps that need bind → Capt / Neo. Gaps that need copy → Vibe. Living Thing-wash → Marvin.

---

## 2. Score legend

| Mark | Meaning |
|------|---------|
| **PASS** | Seen matching on both surfaces (or N/A with reason) |
| **PARTIAL** | Present but weaker / harder / jargon-leaking on one surface |
| **FAIL** | Missing or agent-only on a surface humans need |
| **NEEDS_UAT** | Hypothesis from code/notes — Capt/human cold-load required |
| **BLOCKED** | Waiting tip / rebuild / daemon |

Surfaces: **D** = Webizen Desktop Poet · **W** = WASM / trunk-serve browser Poet.

---

## 3. Global affordance parity (before frames)

| Affordance | D | W | Evidence / note | Next |
|------------|---|---|-----------------|------|
| Right-click **radial wheel** | PASS* | FAIL* | Code: `radial_menu.rs` wired in browser mod; Timothy report: wheel visible on desktop, missing/broken in WASM human path | NEEDS_UAT both; if W fails → chrome priority |
| Toolchest / Layout · Stage · Timeline twins | PARTIAL | PARTIAL | G-A freeze landed; human discoverability still agent-guided | First-session map |
| Studio bay (not terminal-first) | PARTIAL | PARTIAL | Zone D / Catalog · Lexicon exists; cold-load still feels agent UI | Frame A empty-state |
| Sayables-first Ask · Keep · Play | PARTIAL | PARTIAL | Lexicon + office:graph sayables wishlist; Capability.method still too loud in places | Scan primary labels |
| held / not yet (never broken) | PASS | PASS* | Catalog · Lexicon held-gate UAT PASS on 0.0.36-dev lineage; confirm on 0.0.38 | Spot-check tip `541c3a6`+ |
| Catalog chips living · artifact · machine | PASS | PASS* | G-LEXICON-0 bay chrome | Confirm mixed framing split |
| Native Connected cold-load | PASS* | PASS* | Probe tips `64b21384`+; WASM needs same honesty | Cold-load alone |
| Soft-rise arrive | PARTIAL | PARTIAL | monet motion notes; agent paths skip feel | Frame A / E with monet |
| Commit celebrate only on real write | PASS* | NEEDS_UAT | B-ui PASS desktop evidence `bc6e401` / Capt; WASM sanctuary path | Frame D |

\*Mark with asterisk = prior tip evidence; re-confirm on `0.0.38` human cold-load.

---

## 4. Frames A–E audit

### Frame A — First arrive

> You’re in the studio bay. Ask a graph, Keep a volume, Play a cell…

| Check | D | W | Notes |
|-------|---|---|-------|
| Empty bay uses Frame A voice (not Capability dump) | NEEDS_UAT | NEEDS_UAT | Wire empty-state copy to lexicon §7.A |
| Soft-rise on first paint | NEEDS_UAT | NEEDS_UAT | monet: entrance recipe |
| Advanced method names muted | PARTIAL | PARTIAL | Secondary chrome only |
| Human finds Ask / Keep / Play without agent | FAIL | FAIL | Critical gap — onboarding chrome missing |

**Gap:** No dedicated first-arrive empty state teaching the trio. Agent paths jump to manifolds/tools.

### Frame B — Held / not yet

> Held / not yet — open a lexicon pack… Nothing is broken.

| Check | D | W | Notes |
|-------|---|---|-------|
| Catalog · Lexicon held-gate string | PASS | PASS* | Exact: held / not yet — open lexicon pack |
| Never “broken” / red missing for lexicon path | PASS | PASS* | Red missing ≠ lexicon gate |
| Soft held look (not panic) | PARTIAL | PARTIAL | monet: hold beat |
| Daemon-down honesty | PASS* | PASS* | held when Native offline |

**Gap:** Broader surfaces (GIS, Pulse, Job Center) still say “unavailable” — align to held / not yet where product means wait-honest.

### Frame C — Living-safe vs tool

> People… who and life. Volumes/files/DIDs… tools and handles.

| Check | D | W | Notes |
|-------|---|---|-------|
| Catalog chips warm vs crisp | PASS | PASS* | living / artifact / machine |
| Mixed framing splits (no identity chip) | PASS | PASS* | Marvin + UAT |
| Primary chrome never Thing-washes persons | PARTIAL | PARTIAL | Scan toolchest titles / ontology panels |
| Capability.method only on machine chip | PASS | PASS* | MACHINE_SAYABLE |

**Gap:** Full chrome pass for “object/entity/identity” on living subjects — Marvin cite + davinci sweep.

### Frame D — Sanctuary commit

> Keep opens shelter. Commit only when the write is real.

| Check | D | W | Notes |
|-------|---|---|-------|
| Keep / open volume path discoverable | PARTIAL | PARTIAL | Path fields truncate; agent-known paths |
| Save Checkpoint dialog modes clear | PASS* | NEEDS_UAT | Auto / Checkpoint / Snapshot / Pruned |
| Celebrate only on volume_commit success | PASS* | NEEDS_UAT | Capt B-ui + HTTP written:1 |
| Closed door = care, not failure theatre | PARTIAL | PARTIAL | Volume CLOSED copy audit |

**Gap:** Human-alone Keep→Commit without knowing `/workspace/...` paths — need picker / recent / sayable browse.

### Frame E — Leave well

> Leave when you’re done. Same path you arrived on.

| Check | D | W | Notes |
|-------|---|---|-------|
| Dismiss / leave uses leave beat | PARTIAL | PARTIAL | lexicon dismiss wired; global leave uneven |
| Reduced-motion still-leave | NEEDS_UAT | NEEDS_UAT | monet |
| No “kill/abort” primary voice | PARTIAL | PARTIAL | Copy sweep |

---

## 5. Ordered chrome work (after this audit folds)

1. **Frame A empty-state** — studio bay first paint with Ask · Keep · Play (Capability muted).  
2. **Radial wheel parity** — prove WASM human path; fix or honest held if host can’t.  
3. **Unavailable → held / not yet** where wait-honest (Pulse, Job, Mesh, etc.).  
4. **Sanctuary Keep without absolute path** — human browse / recent volumes.  
5. **Cold-load UAT script** (Capt) — person alone, WASM then desktop, Frames A–E checklist.  
6. Wave-22 remains **held** until Capt promotes first-session gate.

---

## 6. Asks

| Who | Ask |
|-----|-----|
| **monet** | Fill motion columns (soft-rise / hold / leave / still-*) on Frames A–E; mark FAIL where look reads broken. |
| **Vibe** | Confirm empty-state strings for Frame A; diagnose voice on path-truncate. |
| **Marvin** | Spot-check chrome for Thing-wash; cite plane table. |
| **Capt** | Human-alone cold-load UAT on tip after fold; promote/WIP gate. |
| **Neo** | Fold this file; sole push on `0.0.38`; no Host invent. |

---

## 7. One-breath

> Same words. Same wheel. Soft arrive. Honest hold. Leave clean.  
> Humans finish session one without a bot — or we failed.

---

## 8. Capt cold-load UAT (Frames A–C)

Runnable checklist: [`HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_A_C.md`](./HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_A_C.md) · tip cite `c6ab0f6`+.

## 9. Frames D–E motion (monet)

Look + named beats: [`HUMAN_SURFACE_FRAMES_DE_MOTION_PARITY_WIP.md`](./HUMAN_SURFACE_FRAMES_DE_MOTION_PARITY_WIP.md).
