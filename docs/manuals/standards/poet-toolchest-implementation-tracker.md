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
| **parked** | Named programme gate (Solid, QDNF; Health Review Gate A closed 2026-09-06) |
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
| `health:clinical` | `health:framingham` / `health:cha2ds2` / `health:score2` | live | place calculators; daemon `ClinicalRisk.*` after complete inputs |
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

Named spec tools are registered and dispatched from small per-toolbox files
and focused chain modules (plan: `docs/work-in-progress/POET_TOOLCHEST_SPEC_SWARM_PLAN_2026-09-05.md`).
All multi-chain toolboxes (`image/`, `audio/`, `video/`, `spatial3d/`, `portals/`,
`productions/`, `hypermedia/`) are directory-backed, keeping all implementation
files strictly under 350 lines (below the 400-line budget limit).

| Module | Rows (approx) | Contract mix |
|--------|---------------|--------------|
| office extras | 25 | Local / Place |
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
| **spec total** | **702** | exact, including cross-listed chains |

## Execution gap audit & operational roadmap (2026-09-05)

A comprehensive code audit of `crates/poet/src/browser/spec_tools/dispatch.rs` revealed:
- **Total spec tools registered:** 702 rows (catalog complete, unique IDs verified, all files <350 lines).
- **Place contracts:** 8 rows (place container on canvas via `Poet.container_place`).
- **Live contracts:** 58 rows (checked argument builders via `live_args.rs` invoking daemon capabilities).
- **Gated contracts:** 55 rows (honest missing prerequisites for external hardware/services).
- **Local contracts:** 581 rows.
  - **30 rows actually executed:**
    - 12 document formatting commands (`local_effects.rs`)
    - 5 DOM structural insertions (`office_actions/`)
    - 7 media transport controls (`media_actions/`)
    - 6 CSS style toggles (`local_effects.rs`)
  - **551 rows unexecuted:** fall through `dispatch.rs` lines 61-71 to `"Tool selected. Its editing action is not implemented on this surface yet."`

### Phased implementation packets:

1. **Packet 1 — Semantic & Epistemic Engines (`epistemic_actions/`):**
   - Implement `epistemic_actions/` (directory-backed, <350 lines/file): `assessments.rs`, `context.rs`, `mod.rs`.
   - Wire `epistemic:create-assessment`, `epistemic:set-epistemic-mode`, `epistemic:set-reality-category`, `epistemic:mark-disputed`, `epistemic:set-spatio-temporal-context`, `epistemic:set-social-context`, `epistemic:query-assessments` to real container metadata attributes, RDF annotation microformats, and local assessment querying.
2. **Packet 2 — AI & NLP In-Browser Engine (`ai_actions/`):**
   - Implement `ai_actions/`: `symbolic.rs`, `neural.rs`, `graph_bridge.rs`, `mod.rs`.
   - Wire `ai:run-gazetteer`, `ai:run-fst`, `ai:run-temporal-parser`, `ai:run-geo-parser`, `ai:run-quantity-normalizer` to document text analysis; wire `ai:run-embedder` and `ai:extract-substrate` to local FNV/vector representations and graph micro-indexing.
3. **Packet 3 — Investigation & Evidence Engine (`investigation_actions/`):**
   - Implement `investigation_actions/`: `case.rs`, `evidence.rs`, `mod.rs`.
   - Wire `investigation:new-investigation`, `investigation:set-mode`, `investigation:set-status`, `investigation:add-subject`, `investigation:add-event`, `investigation:collect-evidence`, `investigation:set-reliability` to container state and local graph case records.
4. **Packet 4 — Research & Enquiry Engine (`research_actions/`):**
   - Implement `research_actions/`: `enquiry.rs`, `corpus.rs`, `mod.rs`.
   - Wire `research:new-research`, `research:set-purpose`, `research:define-scope`, `research:add-question`, `research:add-corpus-item` to project datasets and local enquiry stores.
5. **Packet 5 — Code & Vibe Scripting Engine (`code_actions/`):**
   - Implement `code_actions/`: `vibe.rs`, `quin.rs`, `mod.rs`.
   - Wire `code:vibe-syntax-check`, `code:vibe-format`, `code:vibe-outline`, `code:quin-inspect`, `code:quin-ref`, `code:quin-reify`.
6. **Packet 6 — Bounded Media & Creative Canvas Depth + Gating Honesty:**
   - Expand `local_effects.rs` and `media_actions/` for 2D canvas transforms (grayscale, invert, rotate, flip, contrast) and video transport (playback rates, aspect ratio classes).
   - Reclassify remaining hardware-only tools (DMX moving heads, laser projectors, physical audio synthesizers) honestly to `Contract::Gated` with precise prerequisite messages rather than claiming local execution.

## Parked (do not ungate here)

| Item | Gate |
|------|------|
| Solid IdP chrome | Qualia-first; Solid is exit |
| QDNF QLink/QRoute | Spec still being written |
| Host widen / dotted `qualia.*` | Freeze |

Clinical calculators unparked 2026-09-06 under Review Gate A close
(`docs/work-in-progress/GATE_A_CLOSE_2026-09-06.md`).

## Change log

- 2026-09-06: Review Gate A closed (`GATE_A_CLOSE_2026-09-06.md`). Clinical calculators unparked to live `ClinicalRisk.*`.
- 2026-09-05: Integrated VibeScript REPL and Core Capability Bridge architecture:
  Mapped `qualia-core-db` zero-heap libraries (Computational Geometry: Delaunay, Convex Hull, CSG Booleans, Alpha Shapes, DDG; Formal Logic: Deontic, Epistemic, Paraconsistent, LTL, ASP; Symbolic Algebra CAS; Computer Vision) to Vibe's `ALL_INVOKE_IDS`. Established dual-surface model connecting visual Tool-Chest actions to live VibeScript execution in the Poet IDE REPL (`eval_cell` over `LocalHost` and persistent `Env`).
- 2026-09-05: Completed operational execution suites across all toolboxes in Poet Tool Chest:
  `epistemic_actions/` (assessments, reality categories, disputes, context), `ai_actions/` (symbolic NLP, FNV embeddings, graph bridge),
  `investigation_actions/` (cases, Admiralty evidence, hypotheses), `research_actions/` (enquiry questions, corpus bibliography, findings),
  `code_actions/` (Vibe delimiter balance, formatting, outlines, Quin inspection), `image_actions/` (layer stack, blend modes, brush parameters, vector paths),
  `video_actions/` (timeline in/out points, speed rates, SMPTE timecode, aspect ratios), `audio_actions/` (mute/solo/arm, pan, BPM tempo, quantize),
  `spatial3d_actions/` (parametric primitives, wireframe, bounding boxes, camera focal lengths), `productions_actions/` (DMX universes, fixtures, blackout, cues),
  `portals_actions/` (worlds, skyboxes, avatar poses, telemetry), and `hypermedia_actions/` (interactive widgets, OpenGraph, accessibility audits).
  WASM checks and full test suite passing: 333 unit tests + 9 product integrity tests + surface inventory passing. All files <200 lines.
- 2026-09-05: Formal audit of 702 spec tools identified execution gap (30 operational vs 551 fall-through local tools). Established execution roadmap across Packet 1 (`epistemic_actions/`), Packet 2 (`ai_actions/`), Packet 3 (`investigation_actions/`), Packet 4 (`research_actions/`), and Packet 5 (`code_actions/`).
- 2026-09-05: Completed directory-backed mechanical split for all multi-chain toolboxes
  (`audio/`, `video/`, `spatial3d/`, `portals/`, `productions/`, `hypermedia/`), bringing every
  single file in `spec_tools/` under 350 lines (max 340 lines). Fixed Office DOM actions and
  media transport bindings; 306 lib tests + 9 product integrity tests + wasm32 check passing.
- 2026-09-05: Live spec rows gained checked argument adapters for all 14 bound
  capabilities; local CSS behavior expanded to 15 honest browser effects; exact
  702-row, human-copy, contract, and unique-ID tests added. Split image into
  directory-backed chain modules and disambiguated the Inspect-chain DMX monitor.
- 2026-09-05: Spec swarm plan (`docs/work-in-progress/POET_TOOLCHEST_SPEC_SWARM_PLAN_2026-09-05.md`): directory-backed `spec_tools/` (no monoliths); office extras landed; remaining toolboxes filled by parallel lanes.
- 2026-09-05: Human copy, hover tooltips, Getting started / Everyday / Workshop mode, ARIA + agent catalog.
- 2026-09-05: Tracker created from live registry. Empty chains filled (brushes, palette, viewport, grid, clinical-gated, repl, copilot, pulse, fiduciary). `sheet:import`, `image:heatmap`, `code:quin_statement` moved from gated to local/live.
