# AI Agent Readiness for VibeScript

**Status:** Requirements overlay — **not** the language specification.  
**Normative language:** [`standards/vibescript-core.md`](standards/vibescript-core.md) (`vibe-0.1`).  
**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;

Source of this overlay: `C:\Projects\NLP\TechDesign\agent_vibescript_requirements.md`  
Public hub: [`../vibe/index.html`](../vibe/index.html)

This file says what agents need **in addition to** the 0.1 grammar. It does **not** add keywords, backends, or Quin literals. If this document and `vibescript-core.md` disagree, **the core spec wins**.

---

## 0. What this is, and what it is not

Foundation models have no pre-training on Vibe. Agents that author or repair scripts need:

1. A machine-readable map of the **actual** grammar, diagnostics, and `capability.invoke` ids.
2. Optional constrained-decoding artifacts derived from that grammar.
3. A small golden corpus of programs that already pass `vibe` tests.
4. The existing diagnostic loop (`E001`…`E600` + UTF-8 spans) — not a second validator crate.

It is **not**:

- a second language spec
- a reason to implement `<<[ s p o g prov ]>>` (illegal — core §13 N3)
- a reason to add Ollama / llama.cpp (Qualia inference is in-process)
- a reason to invent `pulse.broadcast` or `aura.apply_schema` (0.1 is `pulse.publish` / `aura.validate`)
- a claim that `.d10` is the 10D extension (canonical is `.10d`)
- a mandate for new crates `poet-grammar` / `poet-validator` (those jobs live in `vibe`)

---

## 1. Binding names that are real (0.1 + invoke)

Use these in examples and ontologies. Do not teach agents the struck names.

| Teach | Do not teach | Why |
|---|---|---|
| `quin.statement(...)` | `<<[ s p o g prov ]>>` | Host seals parity; overlay is illegal |
| `<<( s p o )>>` and `<< s p o ~ r >>` | `<< id \| s p o >>` | RDF 1.2 Turtle only |
| `pulse.publish` | `pulse.broadcast` | 0.1 binding table |
| `aura.validate` | `aura.apply_schema` | SHACL subset + `SHACL.validate` |
| `capability.invoke("…")` | new keywords per family | D15 |
| `graph.query(..., take: n)` | unbounded query | N5 / N7 |
| `GraphDatabase.sparql` | inventing SPARQL syntax in Vibe | SPARQL-star stays SPARQL |
| `SHACL.extensions` | undocumented constraint IRIs | catalog first |
| `.10d` | `.d10` as canonical | core §10 |
| `E001` `E100` `E200` `E300` `E400` `E500` `E600` | `VIBE_E0412` | core §9 |

Engine reach beyond 0.1 bindings is **`capability.invoke`**, listed at runtime by `CapabilityDiscovery.list` / `CapabilityDiscovery.coverage`.

---

## 2. Pillars (status as of 2026-08-18)

| # | Pillar | Artifact now | Status |
|---|---|---|---|
| 1 | Language knowledge graph | `crates/qualia-core-db/src/poet_host/catalog_ttl.rs` → `capability.invoke("CapabilityDiscovery.catalog", null)` emits Turtle from `VIBE_0_1` + `ids::ALL_BOUND` | **built** |
| 2 | Constrained decoding | `crates/vibe/grammar/vibe-0.1.ebnf`, `vibe-0.1.gbnf`, `source.schema.json` (for Qualia's in-process model, not Ollama) | **built** |
| 3 | Golden corpus | `crates/vibe/fixtures/` (44 .vibe files) + `tests/conformance.rs` (64 tests: §12/§13 examples, physics, EMF/spectral, geometry/SVG, CSS animation, reactive cells, hook dispatch, legal/governance, scientific, financial, negative must-reject) | **built** (Phase G complete — 9 domain verticals + 6 negative fixtures) |
| 4 | Diagnostic loop | `vibe::diagnose(src)` → JSON with `error_code`, `span`, `message`, `suggested_fix` (safe rewrites that never grant authority) | **built** |
| 5 | Physics + spectral invoke wrappers | `Physics.wave_1d`, `heat_diffusion_1d`, `advection_diffusion_1d`, `harmonic_oscillator`, `pendulum`, `n_body`, `molecular_dynamics`, `cfd_step`, `quantum_states_1d`, `logistic_growth`, `emf_interference`, `emf_attenuation`, `doppler_shift`, `emf_field_grid_3d`, `emf_sample_at_depth` + `Spectral.emf_to_spd`, `spd_to_xyz`, `emf_to_rgb`, `blend`, `gamut_map` + `Render.css_animation`, `css_color`, `css_transform`, `svg_path`, `svg_circle`, `svg_rect`, `svg_line`, `svg_bezier`, `svg_field` — all wrap existing tested solvers | **built** (Phases A–D) |
| 6 | Dynamic graph honesty | `catalog::resolve_id_with(id, attached)` — `graph.read`, `graph.write`, `aura.validate`, `pulse.publish` flip to "live" when attached to the daemon graph | **built** (Phase F) |

Skill file: `skills/vibescript/SKILL.md` → `docs/vibe/SKILL.md` (points at core §3–§13 and the invoke table).

---

## 3–7. Ontology, GBNF, corpus, diagnostics, skill

See the TechDesign source for the aligned examples. In short:

- Turtle must use `pulse.publish` / `aura.validate` / `quin.statement`.
- GBNF is for Qualia’s in-process model, not Ollama.
- Place grammar export in `crates/vibe/grammar/` — no new crate.
- Corpus starts from fixtures that already pass, not 100 illegal samples.
- Do not add `crates/poet-validator`.

---

## 8. Implementation order

| Order | Work | Status |
|---|---|---|
| 0 | This alignment (core spec + D1–D18) | done |
| 1 | Publish hub `docs/vibe/` on Pages | manuals + `index.html` exist; Pages merge is a publishing step, not a build step |
| 2 | JSON diagnostic export + `suggested_fix` | **done** — `vibe::diagnose` |
| 3 | GBNF/JSON-Schema from §3 EBNF | **done** — `crates/vibe/grammar/` |
| 4 | Turtle catalog of 0.1 bindings + `ALL_BOUND` | **done** — `catalog_ttl.rs` / `CapabilityDiscovery.catalog` |
| 5 | Skill + more fixture/NL pairs | skill done; corpus growth is ongoing |
| later | Grapheme / 3D IK / GeoSPARQL as invoke ids | reachable now via `capability.invoke` (e.g. `Geometry.Hull2`); first-class grammar is post-0.1 |

---

## 9. Completeness verdict

| Question | Answer |
|---|---|
| Complete **language** spec? | **No** — that is `vibescript-core.md`, and 0.1 is the closed core, not the destination. |
| Complete **agent-readiness** stack? | **Yes**: diagnostics, GBNF/JSON-Schema, Turtle catalog, skill, and golden corpus (64 conformance tests across 9 domain verticals + negatives) are all built and consistent with 0.1. Post-0.1 work (W0–W8 rendering, A1–A9 agent orchestration, VC3 zero-alloc uniform belt) is also implemented. |
| Is 0.1 language blocked on readiness work? | **No.** Interpreter + binding profile + invoke table are implemented and tested (`vibe` ~166 tests, `poet_host` 131 tests, `qualia-core-db` 6224 tests total). Hook dispatch (`on pulse:message`, `on tick`) and user-defined function resolution are wired. Pulse transport emits through a process-wide broadcast channel when attached, with SSE endpoint `/pulse/events`. Physics (15 invoke wrappers), spectral/EMF (5), render CSS/SVG (9) are wrapped. Reactive animation loop (`poet_tick`, `poet_pulse_event`, time-dependency tracking) is wired. Graph honesty labels are dynamic (live when attached, surfaced through `poet_capabilities`). Post-0.1: WebGPU/WebGL2 render invoke surface (W0–W8), CBOR-LD AST codec (A1), DOMINO constrained decoding (A2), reflection self-healing (A3), AST query engine (A4), semantic blackboard (A5), multi-agent DAGs (A6), paraconsistent Eτ evidential logic (A7), hardware deontic interrupts (A8), semantic skills/embeddings (A9), VC3 zero-alloc uniform belt + compute resource pool — all implemented and tested. Vibe-design to-do list (73 items + 18 wishes + 8 decisions) consolidated in `docs/vibescript-full-impl-PLAN.md` §8. |
| Does this overlay contradict 0.1? | **No** — core §11 bindings (incl. `time.unix`, now wired) are the only names taught; `<<[…]|>>` overlays, `pulse.broadcast`, `aura.apply_schema`, `<< id \| … >>` are all listed as "do not teach." |
