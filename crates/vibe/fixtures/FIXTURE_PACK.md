# Vibe fixture pack (Sprint B Stage 1)

Hot-edit these without rebuilding a host. Parse + check via `vibe::parse_program` /
`vibe::check_program` / `vibe::diagnose`. Execution still goes through the frozen
four-op surface; LocalHost does not pretend to be a durable `.q42` volume.

## Graph · volume · render (human dialect)

| File | Live bind | Notes |
|------|-----------|-------|
| `graph_sparql.vibe` | `GraphDatabase.sparql` | `using GraphDatabase;` — never `qualia.graph.*` |
| `volume_sanctuary.vibe` | `GraphDatabase.volume_open` / `volume_commit` | Sanctuary save/open; wasm stays fail-closed |
| `inference_grounding.vibe` | `Inference.grounding` / `verify_turn` / `detect_ungrounded` | Provenance path; no `qualia.infer.*` |
| `render_preview_handles.vibe` | still / clip / scene on `Render.*` | B-007 remap; no sibling Host op |
| `gpu1_portal.vibe` | `Render.gpu_*` | Existing Render preview probe |

## Diagnose loops (expected invalid)

Run `vibe::diagnose(src)` and repair from `suggested_fix`. Do not execute.

| File | Code | What it proves |
|------|------|----------------|
| `n1_nospace_lt.vibe` | E001 | Relational operators need spaces |
| `n3_quin_overlay.vibe` | E001 | `<<[` overlay is illegal; use `quin.statement` |
| `n7_time_in_pure_cell.vibe` | E200 | Pure cells cannot perform External effects |

## Agent dialect

Agents call `capability.invoke("Capability.method", {…})` after a matching
`requires [ capability("…") ];`. Workshop `.vibe` files must not teach that
path — see `docs/vibe/devrel-frozen-host.md`.
