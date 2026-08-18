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
3. A small golden corpus of programs that already pass `poet-vibe` tests.
4. The existing diagnostic loop (`E001`…`E600` + UTF-8 spans) — not a second validator crate.

It is **not**:

- a second language spec
- a reason to implement `<<[ s p o g prov ]>>` (illegal — core §13 N3)
- a reason to add Ollama / llama.cpp (Qualia inference is in-process)
- a reason to invent `pulse.broadcast` or `aura.apply_schema` (0.1 is `pulse.publish` / `aura.validate`)
- a claim that `.d10` is the 10D extension (canonical is `.10d`)
- a mandate for new crates `poet-grammar` / `poet-validator` (those jobs live in `poet-vibe`)

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
| 2 | Constrained decoding | `crates/poet-vibe/grammar/vibe-0.1.ebnf`, `vibe-0.1.gbnf`, `source.schema.json` (for Qualia's in-process model, not Ollama) | **built** |
| 3 | Golden corpus | `crates/poet-vibe/fixtures/` + `tests/conformance.rs` (16 tests: §12 examples, §13 negatives, `time.unix`, `quin.statement`, `capability.resolve`) | **partial** (fixtures pass; not yet a large curated corpus) |
| 4 | Diagnostic loop | `poet_vibe::diagnose(src)` → JSON with `error_code`, `span`, `message`, `suggested_fix` (safe rewrites that never grant authority) | **built** |

Skill file: `skills/vibescript/SKILL.md` → `docs/vibe/SKILL.md` (points at core §3–§13 and the invoke table).

---

## 3–7. Ontology, GBNF, corpus, diagnostics, skill

See the TechDesign source for the aligned examples. In short:

- Turtle must use `pulse.publish` / `aura.validate` / `quin.statement`.
- GBNF is for Qualia’s in-process model, not Ollama.
- Place grammar export in `crates/poet-vibe/grammar/` — no new crate.
- Corpus starts from fixtures that already pass, not 100 illegal samples.
- Do not add `crates/poet-validator`.

---

## 8. Implementation order

| Order | Work | Status |
|---|---|---|
| 0 | This alignment (core spec + D1–D18) | done |
| 1 | Publish hub `docs/vibe/` on Pages | manuals + `index.html` exist; Pages merge is a publishing step, not a build step |
| 2 | JSON diagnostic export + `suggested_fix` | **done** — `poet_vibe::diagnose` |
| 3 | GBNF/JSON-Schema from §3 EBNF | **done** — `crates/poet-vibe/grammar/` |
| 4 | Turtle catalog of 0.1 bindings + `ALL_BOUND` | **done** — `catalog_ttl.rs` / `CapabilityDiscovery.catalog` |
| 5 | Skill + more fixture/NL pairs | skill done; corpus growth is ongoing |
| later | Grapheme / 3D IK / GeoSPARQL as invoke ids | reachable now via `capability.invoke` (e.g. `Geometry.Hull2`); first-class grammar is post-0.1 |

---

## 9. Completeness verdict

| Question | Answer |
|---|---|
| Complete **language** spec? | **No** — that is `vibescript-core.md`, and 0.1 is the closed core, not the destination. |
| Complete **agent-readiness** stack? | **Yes for M2–M5**: diagnostics, GBNF/JSON-Schema, Turtle catalog, and skill are all built and consistent with 0.1. Corpus depth (pillar 3) is the remaining open-ended grow item. |
| Is 0.1 language blocked on readiness work? | **No.** Interpreter + binding profile + invoke table are implemented and tested (`poet-vibe` 19 tests, `poet_host` 54 tests). |
| Does this overlay contradict 0.1? | **No** — core §11 bindings (incl. `time.unix`, now wired) are the only names taught; `<<[…]|>>` overlays, `pulse.broadcast`, `aura.apply_schema`, `<< id \| … >>` are all listed as "do not teach." |
