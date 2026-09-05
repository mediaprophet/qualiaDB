# Poet Tool-Chest implementation tracker

**Date:** 2026-09-05 · **Branch:** `0.0.36-dev` · **Freeze:** `vibe-host-0.1`  
**Live source:** `crates/poet/src/browser/registration/` via `build_registry()`  
**Dispatch:** `crates/poet/src/browser/tool_actions.rs` + `chain_actions.rs` + `shapes_actions.rs`  
**Regression:** `every_chain_has_at_least_one_tool` · `every_registered_nonplacement_tool_has_an_explicit_policy`

Status vocabulary:

| Status | Meaning |
|--------|---------|
| **place** | `PlaceContainer` → canvas via `Poet.container_place` |
| **local** | Executable in standalone Poet (DOM / four-ops / in-process catalog) |
| **live** | Live `ALL_BOUND` `Family.method`; daemon upgrades, local fallback when noted |
| **gated** | Honest `unavailable_reason`; not stub-broken |
| **parked** | Named programme gate (Health Review Gate A, Solid, QDNF) |
| **spec** | Named in `crates/poet/tool-chest/TOOLBOX_*.md`, not in the live registry yet |

Empty chains are a regression. Spec-scale toolboxes are tracked below, not bulk-registered.

## Presentation (humans + agents)

Hover and keyboard focus show a short **what it is** tooltip (`data-tooltip`, `title`, `.tool-tip`, `aria-label`). Visible names come from `tool_copy.rs`, not from machine ids.

| Mode (UI) | Token | Shows |
|-----------|--------|--------|
| Getting started | `novice` | Everyday add/use tools, plain language |
| Everyday | `intermediate` | Working set; machine ids still hidden |
| Workshop | `expert` | Full set; tooltip may append the live capability id |

Mode is global (dock switcher) and remembered on the device. Each tool also has a **minimum** mode, so some tools only appear in Everyday or Workshop. Agents read the same records from `data-agent-catalog` on the tool chest (`audience: human, agent`).

Coder verbs (`capability.invoke`, SPARQL, quin.statement) stay out of novice/everyday labels.

## Live registry (15 toolboxes)

### epistemic

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `epistemic:modalities` | `epistemic:tag_objective` | local | annotate selected |
| | `epistemic:tag_subjective` | local | annotate selected |
| | `epistemic:tag_intersubjective` | local | annotate selected |
| | `epistemic:tag_normative` | local | annotate selected |

### office

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `office:typography` | `office:typography_bold` / `_italic` / `_code` | local | format selected `.doc-editor` |
| `office:paragraph` | `office:paragraph_heading` / `_align_left` / `_align_center` | local | format selected `.doc-editor` |
| `office:containers` | `office:place_doc` / `_ontology` / `_slide` | place | `Poet.container_place` |
| `office:graph` | `graph:sparql_query` | live | `GraphDatabase.sparql` + local graph |
| `office:shapes` | `n3:evaluate` | live | `N3Logic.evaluate` + local sketch |
| | `shacl:validate` | live | `SHACL.validate` + annotation minCount |

### image

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `image:brushes` | `image:brush_stroke` / `image:brush_clear` | local | outline on selected surface |
| `image:palette` | `image:fill_warm` / `image:fill_cool` | local | fill token |
| `image:tools` | `image:place_media` | place | |
| | `image:marker` | local | annotate |
| | `image:heatmap` | local | numeric density overlay |

### sheet

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `sheet:grid` | `sheet:stats_mean` | live | local mean; daemon `Statistics.mean` |
| `sheet:tools` | `sheet:place_sheet` | place | |
| | `sheet:import` | local | CSV/TSV into sheet from A1 |

### spatial

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `spatial:viewport` | `spatial:camera_reset` | local | yaw/pitch/zoom attrs |
| | `spatial:orbit_preview` | live | in-process `Animation.evaluate_preset` |
| `spatial:tools` | `spatial:place_map` / `_dual_studio` / `_scene_view` / `_3d` | place | |
| | `spatial:pin` | local | annotate |
| | `spatial:track` | gated | consenting agent + trajectory |

### audio

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `audio:tools` | `audio:place_audio_session` / `audio:place_media` | place | |
| | `audio:mic_capture` | gated | mic permission + capture surface |
| | `audio:neural_latents` | gated | mounted P64 + stream |

### communication

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `comm:pulse` | `comm:pulse_presence` | live | local mark; daemon `Pulse.publish_presence` |
| `comm:containers` | `comm:place_social` / `_webrtc` / `_webview` | place | |

### erp

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `erp:tools` | `erp:place_kanban` / `_gantt` / `_voting` | place | |

### mail

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `mail:tools` | `mail:place_mail` | place | |
| | `mail:composer` | local | place mail container |
| | `mail:publisher` | gated | artefact + destination + authorisation |

### scientific

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `scientific:tools` | `scientific:place_health` / `_3d` | place | |
| | `scientific:thermodynamics` | gated | MCMC target + sampler inputs |

### rights

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `rights:fiduciary` | `rights:deontic_obligate` | live | local tag; daemon `DeonticLogic.evaluate` |
| `rights:tools` | `rights:authors_group` | local | place rights container |
| | `rights:fiduciary_sign` / `rights:did_sign` | gated | agreement + identity + key vault |

### health

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `health:clinical` | `health:framingham` / `health:cha2ds2` | parked | Health Review Gate A |
| `health:tools` | `health:place_*` (overview, documents, disclosure, conditions, vault, anatomy) | place | |
| | `health:pathology` | gated | consent-gated assay |

### code

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `code:repl` | `code:vibe_diagnose` | local | frozen four-op `diagnose` |
| `code:tools` | `code:place_vibe` | place | |
| | `code:quin_statement` | local | three UTF-8 tokens → data-quin-* |

### ai

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `ai:copilot` | `ai:grounding` | live | local citation sketch; daemon `Inference.grounding` |
| `ai:tools` | `ai:co_author` | gated | document + prompt + activated model |
| | `ai:extractor` | live | `NLP.gazetteer_run` + local extract |
| | `ai:sentinel` | live | `Sentinel.inspect` + local DOM check |
| | `ai:triad` | place | |

### sdn

| Chain | Tool | Status | Bind / contract |
|-------|------|--------|-----------------|
| `sdn:tools` | `sdn:place_webrtc` / `sdn:place_finance` | place | |
| | `sdn:energy_governor` | gated | battery/solar telemetry |

## Spec swarm (directory-backed `spec_tools/`)

Named spec tools are now registered and dispatched from small per-toolbox files
(plan: `docs/work-in-progress/POET_TOOLCHEST_SPEC_SWARM_PLAN_2026-09-05.md`).
No monolith. Each file stays under ~400 lines.

| Module | Rows (approx) | Contract mix |
|--------|---------------|--------------|
| office extras | 24 | Local / Place |
| image | 55 | Local / Live (vision blur, histogram) |
| audio | 58 | Local / Gated |
| video | 58 | Local / Live (Render.scene, Animation.*) |
| spatial3d | 52 | Local / Live (camera, scene) |
| portals | 36 | Local / Live |
| productions | 43 | Local / Gated (DMX desk) |
| hypermedia | 37 | Local / Live (Pulse.*) |
| code extras | 35 | Local / Live (N3, SHACL) |
| ai extras | 31 | Local / Live |
| spatial extras | 41 | Local / Live |
| epistemics extras | 58 | Local / Live (Inference.*) |
| investigation | 98 | Local / Live (causal) |
| research | 75 | Local |
| **spec total** | **~700** | |

## Remaining depth (not missing buttons)

Named spec tools are registered. Next packets deepen Local/Live contracts
(real canvas, DAW, lighting desk) without growing a monolith. Health clinical
calculators stay parked on Review Gate A.

## Parked (do not ungate here)

| Item | Gate |
|------|------|
| Clinical calculators (`health:framingham`, `health:cha2ds2`, pathology integrity) | Health Review Gate A |
| Solid IdP chrome | Qualia-first; Solid is exit |
| QDNF QLink/QRoute | Spec still being written |
| Host widen / dotted `qualia.*` | Freeze |

## Change log

- 2026-09-05: Spec swarm plan (`docs/work-in-progress/POET_TOOLCHEST_SPEC_SWARM_PLAN_2026-09-05.md`): directory-backed `spec_tools/` (no monoliths); office extras landed; remaining toolboxes filled by parallel lanes.
- 2026-09-05: Human copy, hover tooltips, Getting started / Everyday / Workshop mode, ARIA + agent catalog.
- 2026-09-05: Tracker created from live registry. Empty chains filled (brushes, palette, viewport, grid, clinical-gated, repl, copilot, pulse, fiduciary). `sheet:import`, `image:heatmap`, `code:quin_statement` moved from gated to local/live.
