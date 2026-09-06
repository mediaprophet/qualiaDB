# Vibescript-first incorporation swarm — 2026-09-06

**Branch:** `0.0.36-dev`  
**Constraint:** `docs/work-in-progress/VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`  
**Methodology:** **Host / VibeScript bind first** → then Poet Live consume.  
`vibe-host-0.1` = incorporation outcome (Host widen allowed).  
**Workspace:** `C:\github\qualiaDB` · no worktrees · not committed unless owner asks.

## Goal

Turn previously “surveillance-only” crate surfaces into real `Family.method` Host
capabilities, then wire Poet Tool Chest Live to those ids.

## Lanes (disjoint)

| Lane | Packet | Owns (write) | Must not touch |
|------|--------|--------------|----------------|
| A | `VIBE-CV` remaining + Poet Live | Host already has 10 `ComputerVision.*` — **Poet only**: image toolbox dual-path for the 5 uncited (`equalize_hist`, `rgb_to_gray`, `dhash`, `hamming_distance`, `cosine_similarity`); `live_args`, registration, chain_actions / image_chain if needed | `ids.rs`, cooperative, econ Host |
| B | `VIBE-ECON` next Host-cited Poet slice | Poet `econ_chain_actions` + `register_econ_toolbox` — add **≥5** more Live tools from uncited `Econ.*` (e.g. `cournot_duopoly`, `bertrand_duopoly`, `historical_var`, `atkinson`, `gordon_growth`) citing **existing** Host ids | Host ids (unless a method is missing — then claim and pair both catalogs) |
| C | `VIBE-COOP` new Host family | **New** family `CooperativeDelegation` (or `CooperativeWork`) — **not** `Agency.*` — bind `delegation_permits` (+ optional work-item board project) via `qualia-cooperative-core` dep on core-db; pair `ALL_BOUND` + `ALL_INVOKE_IDS` + dispatch + tests; then **one** Poet Live tool citing the new id | poet image/econ files; vision biosense |
| D | `VIBE-CHAT` ChatGraph Host starter | Minimal `ChatGraph.*` Host surface for session/fragment load (extract or thin-wrap; **no** core-db→client-core cycle — prefer portable types / duplicated thin codec in invoke, or move shared types). Pair catalogs + tests; Poet honesty or one Live tool | cooperative Host; image toolbox |

## Honesty

- Paired catalogs always; catalog drift test must pass.
- Consent/sensitivity fail-closed; no private keys in UI args.
- `Agency.evaluate` remains Ed25519 verify — never alias cooperative ABAC.
- Biosense deferred to next swarm if Lane D capacity insufficient (ChatGraph first as stranded client-core).

## Parent integrate — **COMPLETE** 2026-09-06

| Lane | Packet | Status | Evidence |
|------|--------|--------|----------|
| A | `VIBE-CV` | **Done** ([CV Live](1714d506-c581-480e-8a0f-c52c0d9eb8ec)) | Parent: live_args **6**; image_chain **3**; integrity **11** |
| B | `VIBE-ECON` | **Done** ([Econ Live](cb074e15-e2d2-452d-91d5-775c05be0fab)) | +5 Econ Live; lane econ **16** + integrity **11** |
| C | `VIBE-COOP` | **Done** ([Cooperative Host](d55c3ae3-31b3-4f86-ae5b-461ef1f8afbc)) | Parent: catalog **1**; cooperative **7** |
| D | `VIBE-CHAT` | **Done** ([ChatGraph Host](18faddeb-26e3-475e-8b07-73384c03ce28)) | Parent: catalog **1**; chat_graph **8**; also econ **16**; integrity **11** |

Register + session ledger updated. Not committed (await owner).

### Next
- Re-run `python scripts/vibe_incorporation_backlog.py` (Q1/Q2 should move)
- Next Q1 Host binds / Q2 Poet consume from backlog
- Biosense / more cooperative surface as follow-on packets
