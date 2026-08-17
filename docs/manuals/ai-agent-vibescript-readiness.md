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

## 2. Pillars (still wanted — after alignment)

| # | Pillar | Artifact now | Status |
|---|---|---|---|
| 1 | Language knowledge graph | Derive TTL from `vibescript-core` + `ids::ALL_BOUND` | **not built** |
| 2 | Constrained decoding | EBNF is in core §3; GBNF/JSON-Schema export from that EBNF | **not built** |
| 3 | Golden corpus | `crates/poet-vibe/fixtures/` + `tests/conformance.rs` | **partial** |
| 4 | Diagnostic loop | `poet-vibe` `Diagnostic { code, span, message }` | **partial** (no `suggested_fix` yet) |

Skill file (later): `skills/vibescript/SKILL.md` pointing at core §3–§13 and the invoke table.

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

| Order | Work | Depends on |
|---|---|---|
| 0 | This alignment | core spec + D1–D18 |
| 1 | Publish hub `docs/vibe/` on Pages | manuals already in `docs/` |
| 2 | JSON diagnostic export + optional `suggested_fix` | existing `Diagnostic` |
| 3 | GBNF/JSON-Schema generated from §3 EBNF | 2 |
| 4 | Turtle catalog of 0.1 bindings + `ALL_BOUND` | live ids |
| 5 | Skill + more fixture/NL pairs | 1 + 4 |
| later | Grapheme / 3D IK / GeoSPARQL as invoke ids | those kernels |

---

## 9. Completeness verdict

| Question | Answer |
|---|---|
| Complete **language** spec? | **No.** That is `vibescript-core.md`. |
| Complete **agent-readiness** spec? | After this revision: complete enough to schedule M2–M5 without contradicting 0.1. |
| Update before implementing the readiness stack? | **Yes.** The first draft would have taught illegal syntax and the wrong inference stack. |
| Is 0.1 language blocked on M1–M5? | **No.** Interpreter + invoke table already exist. |
