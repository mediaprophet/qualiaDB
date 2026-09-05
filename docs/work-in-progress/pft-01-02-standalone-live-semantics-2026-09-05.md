# PFT-01 / PFT-02 — Standalone vs live Tool Chest honesty (2026-09-05)

**Status:** audit complete · repairs landed · focused tests pending this revision  
**Branch:** `cursor/poet-grok-handover-ac52` off `0.0.36-dev`  
**Freeze:** `vibe-host-0.1` — no Host widen, no new dotted IDs  
**Does not close:** Review Gate A (`D5`) · does not select `PFT-03`

## PFT-01 findings

| ID | Surface | Defect | Repair |
|---|---|---|---|
| F1 | `graph:sparql_query` | Daemon rejection ran `local_graph_query` and showed **success**, so a canvas DOM ASK could look like `GraphDatabase.sparql` | Rejection is **error**. Local sketch only when disconnected; labelled `Standalone (not GraphDatabase.sparql)` |
| F2 | `ai:extractor` / `ai:sentinel` | Same: gazetteer/Sentinel failure fell back to local analysis as **success** | Live deny is error; offline uses status `local` |
| F3 | `n3:evaluate` / `shacl:validate` | Local sketch after daemon rejection as **success** | Same dual-path rule |
| F4 | `sheet:stats_mean` | Local mean after `Statistics.mean` rejection as **success** | Same |
| F5 | Grounding / pulse / deontic offline | Honest copy, but status **success** (checkmark) | Status `local` (hollow glyph), message names the live id it is not |
| F6 | `requires_daemon` always `false` | Intentional: `data-requires-daemon` **disables** the control when the daemon is down. Dual-path tools must stay runnable standalone | **Held.** Do not set `requires_daemon` true for SPARQL/extractor/sentinel/N3/SHACL |

Search workbench SPARQL and `spec_tools` live dispatch already used unavailable/error. G-COORD SPARQL note already distinguishes held vs live.

## PFT-02 mechanism

`tool_dual_path.rs`: `local_sketch` / `live_ok` / `live_denied`. Notifications set `data-honesty` to `local` | `live` | `error`. Local JSON envelopes keep `"source": "poet-local"`.

## How to run

```bash
cargo +stable test -p poet --lib tool_dual_path
cargo +stable test -p poet --lib tool_actions
cargo +stable test -p poet --lib shapes_actions
cargo +stable test -p poet --test product_integrity --test surface_inventory
```

## Not claimed

- Review Gate A
- `PFT-03` next Tool Chest chain (owner selection)
- Live daemon UAT of SPARQL / Sentinel / gazetteer
- `RM-06` `containers.rs` split
