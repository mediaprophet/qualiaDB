# Poet Tool Chest swarm — continuation (2026-09-06)

**Branch:** `0.0.36-dev` · **Tip base:** `019f10c8` (Gate A closed)  
**Freeze:** `vibe-host-0.1` · **No Host widen**

## Why

Gate A closed. Spec catalogue had **~581 Local** rows with **~445** falling through to
*“not implemented on this surface yet.”*

## Swarm lanes (disjoint writes) — COMPLETE

| Lane | Owns | Result |
|------|------|--------|
| A investigation | `investigation_actions/` | 94 Local routed |
| B research | `research_actions/` | 73 Local; 2 network imports **Gated** |
| C epistemic | `epistemic_actions/` | 54 Local |
| D audio | `audio_actions/` + aliases | 51 Local |
| E spatial | `spatial_actions/` + dispatch | 31 Local; 8 heavy Err |
| F AV/world | video/3d/portals/hypermedia/productions | Local rows + ID aliases; DMX/LUT stay Gated |
| Live sheet | registration + `chain_actions` | +5 `Statistics.*` dual-path tools |

Parent: wired `spatial_actions` into `dispatch.rs`; routed `hyp_graph`/`links`; honesty-gated
`research:import-dataset` / `research:import-web`.

## Honesty bar

- `Ok(true)` only for real reversible DOM/attr mutation or live dual-path
- Keep DMX / LUT / mic / mesh-import / network import Gated
- No invented `ALL_BOUND` IDs

## Verification (parent)

| Suite | Result |
|-------|--------|
| `cargo check -p poet --lib` | pass |
| `chain_actions` | 6 pass |
| `epistemic_actions` | 11 pass |
| `investigation_actions` | 10 pass |
| `research_actions` | 4 pass |
| `video_actions` | 3 pass |
| `spatial3d_actions` | 4 pass |
| `portals_actions` | 2 pass |
| `hypermedia_actions` | 2 pass |
| `productions_actions` | 2 pass |

## Status

- 2026-09-06: All swarm lanes **complete**; integrate verify **green**; research network imports gated.
- 2026-09-06 (PFT deepen): Live dual-path for `EpistemicLogic.evaluate`,
  `Inference.detect_ungrounded`, `Inference.verify_turn`, and
  `ComputerVision.histogram` (ribbon tools + Live dispatch honesty via
  `tool_dual_path`). Spec rows: `epistemic:scan-frame`, `epistemic:verify-turn`.
