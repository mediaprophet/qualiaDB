# Human-surface parity audit (WIP) — Frames A–E

**Status:** work-in-progress · **Not a release gate**  
**Branch:** `0.0.38` · **Lexicon tip:** `541c3a6` · **Motion amend tip:** (this fold) · **Lexicon:** [`HUMAN_ONBOARDING_LEXICON_WIP.md`](./HUMAN_ONBOARDING_LEXICON_WIP.md)  
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
| Right-click **radial wheel** | **PASS** (Capt `d7f0bdc`) | **PASS** (Capt `f712e97`) | Empty-bay right-click → 8-sector wheel both surfaces | Frame B next |
| Toolchest / Layout · Stage · Timeline twins | PARTIAL | PARTIAL | G-A freeze landed; human discoverability still agent-guided | First-session map |
| Studio bay (not terminal-first) | PARTIAL | PARTIAL | Zone D / Catalog · Lexicon exists; cold-load still feels agent UI | Frame A empty-state |
| Sayables-first Ask · Keep · Play | PARTIAL | PARTIAL | Lexicon + office:graph sayables wishlist; Capability.method still too loud in places | Scan primary labels |
| held / not yet (never broken) | PASS | PASS* | Catalog · Lexicon held-gate UAT PASS on 0.0.36-dev lineage; confirm on 0.0.38 | Spot-check tip `541c3a6`+ |
| Catalog chips living · artifact · machine | PASS | PASS* | G-LEXICON-0 bay chrome | Confirm mixed framing split |
| Native Connected cold-load | PASS* | PASS* | Probe tips `64b21384`+; WASM needs same honesty | Cold-load alone |
| Soft-rise arrive (feel) | **PASS*** (monet · Capt `d7f0bdc`) | **PASS*** (monet · Capt `f712e97`) | Frame A soft-rise + wheel WASM↔Desktop **PASS*** | Frame B care-look next |
| Commit celebrate only on real write | PASS* | NEEDS_UAT | B-ui `f45212c` / Capt `volume_commit` written:1; WASM twin beat unproven | Frame D |

\*Mark with asterisk = prior tip evidence; re-confirm on `0.0.38` human cold-load.

---

## 4. Frames A–E audit

### Frame A — First arrive

> You’re in the studio bay. Ask a graph, Keep a volume, Play a cell…

| Check | D | W | Notes |
|-------|---|---|-------|
| Empty bay uses Frame A voice (not Capability dump) | **PASS** (Capt `d7f0bdc`) | **PASS** (Capt `f712e97`) | Both surfaces studio bay first-arrive |
| Soft-rise feel (first paint) | **PASS** (Capt `d7f0bdc`) | **PASS*** (Capt WASM) | Soft-rise OK both |
| Empty-bay arrive choreography | **PASS** (Capt `d7f0bdc`) | **PASS** (Capt `f712e97`) | A4 wheel PASS both; prior Desktop disk-HELD superseded |
| Advanced method names muted | **PASS** (Capt `d7f0bdc`) | PARTIAL | Desktop A5 PASS; WASM still PARTIAL scan |
| Human finds Ask / Keep / Play without agent | **PASS** (Capt `d7f0bdc`) | **PASS** (Capt `f712e97`) | Sayables-first both |

**Gap:** Frame A both surfaces **PASS** (`d7f0bdc` Desktop · `f712e97` WASM). Wave-22 still held until Frame B both surfaces. Prior Desktop disk-HELD on `c6007e0` superseded.

**Desktop apps walkthrough** (Capt `d7f0bdc`, fold tip follows): see [`HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md`](./HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md) — Studio/Poet·Dual·Talk·Browser·QApps·Sanctuary·Keep **PASS**; Settings/Library/Continuity copy **PARTIAL**; Catalog·Lexicon **HELD**.

### Frame B — Held / not yet

> Held / not yet — open a lexicon pack… Nothing is broken.

| Check | D | W | Notes |
|-------|---|---|-------|
| Catalog · Lexicon held-gate string | PASS | PASS* | Exact: held / not yet — open lexicon pack |
| Never “broken” / red missing for lexicon path | PASS | PASS* | Red missing ≠ lexicon gate |
| Soft held look (not panic) | PASS* | PASS* | Steady dwell; confirm on tip ≥ `c6ab0f6` |
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
| Keep open = entrance soft-rise | PASS* | **HELD** | Checkpoint modal only; no daemon — retake pending (monet ≥ `cfac542`) |
| Save Checkpoint dialog modes clear | PASS* | NEEDS_UAT | Auto / Checkpoint / Snapshot / Pruned |
| Celebrate only on volume_commit success | PASS* | **PASS*** (deny) / **HELD** (commit) | Deny care look · no celebrate on DENIED·NO DAEMON (`d1-*`); real commit path still held |
| Closed door = care look | PASS* | PARTIAL | CLOSED = shelter, not failure flash |

**Gap:** Human-alone Keep→Commit without knowing `/workspace/...` paths — need picker / recent / sayable browse.

### Frame E — Leave well

> Leave when you’re done. Same path you arrived on.

| Check | D | W | Notes |
|-------|---|---|-------|
| Dismiss / leave uses leave beat | PARTIAL | PARTIAL | Exit = dissolve on same z-path as arrive; uneven globally |
| Reduced-motion still-leave | NEEDS_UAT | NEEDS_UAT | Named beat; state ≠ motion-only |
| Wheel feel D↔W same dialect | **PASS** | **PASS** | Capt Desktop `d7f0bdc` + WASM `f712e97` A4 both PASS |
| No “kill/abort” primary voice | PARTIAL | PARTIAL | Copy sweep |

---

## 5. Ordered chrome work (after this audit folds)

1. ~~**Frame A empty-state**~~ — **PASS** both surfaces (`d7f0bdc` / `f712e97`).  
2. ~~**Radial wheel parity**~~ — **PASS** both surfaces.  
3. **Unavailable → held / not yet** where wait-honest (Pulse, Job, Mesh, etc.).  
4. **Sanctuary Keep without absolute path** — human browse / recent volumes.  
5. **Cold-load UAT script** (Capt) — person alone, WASM then desktop, Frames A–E checklist.  
6. Wave-22 remains **held** until real Keep entrance + commit path (Capt first-session gate).

---

## 6. Asks

| Who | Ask |
|-----|-----|
| **monet** | Frame A soft-rise + wheel **PASS*** both; Frame B care-look + Continuity pending Capt scores. |
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

Runnable checklist: [`HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_A_C.md`](./HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_A_C.md) · tip cite `c42f8e2`+.

Capt Frame A WASM score: [`HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_A_WASM.md`](./HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_A_WASM.md) (pre-PR).

Re-UAT after PR #90: [`HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_A_WASM_cfac542.md`](./HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_A_WASM_cfac542.md) — A1–A3/A5 **PASS** · A4 wheel **FAIL**.

Frames D–E motion checklist: [`HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_D_E.md`](./HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_D_E.md).

## 8b. Frame A motion (monet)

Soft-rise + wheel WASM↔Desktop **PASS*** on fold tip `0d2106e` (Capt `d7f0bdc` Desktop · `f712e97` WASM). Frame B held/not-yet care-look (+ Continuity) pending Capt.

## 9. Frames D–E motion (monet)

Look + named beats: [`HUMAN_SURFACE_FRAMES_DE_MOTION_PARITY_WIP.md`](./HUMAN_SURFACE_FRAMES_DE_MOTION_PARITY_WIP.md) · tip `f22851d`.

Cold-load A–C: tip `c42f8e2` · D–E checklist: [`HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_D_E.md`](./HUMAN_SURFACE_COLDLOAD_UAT_FRAMES_D_E.md).
