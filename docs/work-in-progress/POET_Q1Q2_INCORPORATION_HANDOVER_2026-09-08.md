# Q1/Q2 incorporation handover — stop after wave 20 (+ partial wave 21)

**Date:** 2026-09-08  
**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Workspace:** QualiaDB cloud agent

## Status

| Wave | Status | Notes |
|------|--------|-------|
| 1–21 | **Complete (integrated)** | Last full close: wave 21 on `0.0.37` |
| 22 | **Complete (integrated)** | Audio FX×13 · Scene graph×11 · Image×15 Live; Host CG×8 |

### Wave 20 (last complete)

- Live: Inf×8 · Cosmic×11 · Orch/ThreeD×16
- Host×8: Audio/Scene/CG
- Verify: Live asserts 9 · dispatch policy ok · Host `wave20_*` 9 · integrity 11
- Backlog after wave 20: **`ALL_BOUND=1056` · `PoetLive=623` · `Q2=433` · `Q1≈12074`**

### Wave 21 (partial — resume here on cloud)

| Lane | Status | Delivered |
|------|--------|-----------|
| A Audio | **partial in tree** | `audio_chain_actions.rs` + ~9 `audio:dsp_*` Live; `mod audio_chain_actions` wired; dispatch policy IDs added |
| B Scene | **partial in tree** | `scene_chain_actions.rs` + assert `spatial_scene_binds_wave21_scene_caps` (len `>= 8`) |
| C NLP | **not started** | Host NLP.* (~9 Q2) still unbound Live |
| D Host | **not started** | Prefer Image / Dmx / HID / Research math |

Swarm brief: `docs/work-in-progress/POET_Q1Q2_WAVE21_SWARM_2026-09-08.md`

## Remaining waves (estimate)

Throughput recent waves: **~25–35 Poet Live + ~8 Host / wave**.

| Bucket | Count (post-w20) | Est. Live waves @ ~30/wave |
|--------|------------------|----------------------------|
| **Q2 Host-bound not Live** | **433** | **~14–15** if pure numeric continues |
| Large families | Research 73, Render 51, Audio ~22, Asset 21, Scene ~19, HID 16, Image 15, Dmx 14, Video 10, NLP 9, … | Research/Render alone ≈ 4+ waves |
| **Q1 Host-missing** | **~12k** (mostly CoreDb / shellish) | Curated Host lane stays **4–8 pure specialized_libs per wave**; not “exhaust Q1” |

**Practical estimate to finish curated Q2 (numeric / media / spatial / AI families above):** **~15–20 more waves** (including finishing wave 21 + Host lanes that feed new Q2).

**Not in that estimate:** exhaustively Host-binding all of Q1 CoreDb.

## How to continue (cloud)

1. Finish wave 21: NLP Live (≥8 or all remaining) + Host×4–8; parent-verify `wave21` + policy + integrity + Host `wave21_*`.
2. Refresh backlog (`scripts/vibe_incorporation_backlog.py`; rename locked `.md`/`.json` on Windows Errno 22).
3. Wave 22+: prefer remaining Audio/Scene rem → NLP rem → Image/Dmx/HID → Asset → Research/Render (curated slices).
4. Methodology: `VIBE_INCORPORATION_METHODOLOGY_2026-09-06.md` · Host constraint: `VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`
5. **Never `Proficiency::Advanced`**. Dual-path. Exact Host scopes. Append-only shared files.
6. Subagents often stall at 0 tool calls — interrupt+resume or parent-finish the lane.
7. Avoid `LinearAlgebra.gemm` CUDA/`caps()` Host path.

## Register / ledger

- Register: `docs/work-in-progress/POET_NEXT_WORK_REGISTER_2026-09-05.md`
- Ledger: `docs/POET_IMPLEMENTATION_SESSION_LEDGER.md`
- Swarm docs: `docs/work-in-progress/POET_Q1Q2_WAVE*_SWARM_*.md`

## Build dirs

Repo-local `target/` and `target-*` are gitignored. Delete before commit if present (disk bloat / lockup). Cargo may also use a sandbox cache outside the repo — that is not committed.
