# core-ontologies — DONE (extracted from `PLAN.md`)

An index of the **completed** items pulled out of [`PLAN.md`](PLAN.md) (1500+ lines) so the live
plan reads lean. **`PLAN.md` remains the authoritative source** — every entry points back to its
section/line; this file copies, it does not replace. Generated 2026-06-22; not auto-maintained.

> Status legend in PLAN.md: `✅` / `DONE` / `RESOLVED` / `BUILT` / `wired+test`.

---

## §5 — Layered outputs (`.q42` + CML concept graph)
- ✅ **`.q42` layer + faithful Turtle parsing** — `tools/build_q42.py` compiles every `.n3` → a
  per-instrument `.q42` SuperBlock, round-trip verified; `parsers/turtle_doc.rs` (`@prefix`
  expansion, multi-line `;`/`,`, multi-word literals, Unicode-correct). *(§5 · L347, L1477)*
- ✅ **Front-of-file lexicon** — Q42LEX embedded at the front of the `.q42`; literals recoverable
  from the `.q42` alone (the CML TEXT-layer prerequisite). *(§5 · L381, L1475)*
- ✅ **CML Concept-Graph architecture** — `CML_CONCEPT_GRAPH.md`, 3-layer TEXT→CONCEPT→LOGIC +
  the Curation Prime Directive; pilot `concept:DutyToSuppressForcedLabour` runs against the
  concept (Active→Defeated). *(§5 / §21.1 · L392, L1482)*
- ✅ **CML Studio** — `docs/cml-studio.html` Canvas2D mind-map (18 modalities, live Turtle), on
  the Webizen render contract. *(§21.1 · L1485)*

## §10 — Spine built (vocabulary + person/agent axioms)
- ✅ **`tiering.n3`, `sense.n3` (+ PERSON example), `agency.n3`** (personhood/agent/guardian N3
  rules + SHACL flag shapes), **`traces/personhood_agency.trace.n3`** (6-case fixture) — authored
  & validated. *(§10.1 · L580; §10.4 · L969)*
- ✅ **State decision RESOLVED** — `LegalPerson` split into disjoint `PublicAuthority` (←`State`)
  vs `CorporatePerson`; the G1 capture-guard targets `CorporatePerson` only. *(§10.2 · L598)*

## §10.2g — CBOR-LD / one hash-space
- ✅ **One hash-space keystone** — `cbor_parser` `hash_str`/`hash_bytes` delegate to
  `q_hash`/`generate_60bit_token`; the same IRI hashes identically across Turtle/N3/CBOR-LD (was
  SipHash). *(§10.2g · L801/L815/L1480)* — remaining: CBOR-LD `@context` expansion.

## §13 — Namespace
- ✅ **Namespace scheme decided + migrated** — `https://ns.webcivics.net/` across
  values/sense/selfhood/agency/policy/humanitarian-ict. *(§13 · L1094/L1102/L1271)*

## §17.1 — First implementation slice (verified, no agents)
- ✅ **Webizen values-credential smoke test** — N3 rule → `compile_n3_rule_to_norm` →
  `evaluate_deontic_contract` → verdict; defeasibility flips Active→Defeated. *(§17.1 · L1214; §10.4 · L970)*
- ✅ **Deontic wiring guard path + FILE→ENGINE loop CLOSED.** *(L1224/L1229)*
- ✅ **`validate_core_ontologies` native governance gate** + `build_index.py` gap-report working. *(L1236/L1243)*

## §20 — Modality breadth (logic-modality wiring — all `✅ wired+test`)
- deontic (`evaluate_deontic_contract`) · defeasible (`DEFEATER_BIT`/`OP_DEFEASIBLE_OVERRIDE`) ·
  forward-chaining (`fire_guard_rules`) · temporal (`TemporalInterval` + expiry) · contrary-to-duty
  (`evaluate_contrary_to_duty`) · argumentation/Dung (`grounded_extension`) · spatial RCC-8/GeoSPARQL
  (`evaluate_rcc8`) · paraconsistent (`route_paraconsistent`). *(§20 · L1318–1331)*
- ✅ domain reasoning — algebra/CAS **proportionality** + **economic** (CAS + `financial_modeling`). *(§20.1 · L1344–1345)*
- ✅ **ASP rewritten** — `compute_answer_sets` (Gelfond-Lifschitz reduct). *(§20.2 · L1383)*
- ✅ **MCP `values_check` LIVE**; ✅ **MCP `values_evaluate` LIVE.** *(§20.2 · L1389/L1399)*
- ✅ **Identity/personhood spine DONE** (identifier ≠ identity made computable). *(L1369)*

## §19 — Computational foundations (now available)
- ✅ **ALGEBRA_MANIFOLD_PLAN closed** (polynomial roots, determinant, symmetric+general eigen, SVD,
  symbolic CAS); **deterministic sense-layer metric** (GPU = CPU for all topology classes);
  **GPU `COPY_DST` bug fixed** (was misread as "no GPU"). *(§19.1 · L1443–1459)*

---

## Landed in the 2026-06-22 build-outs (see PLAN §Status-update L9; `../DEONTIC_LOGIC_PLAN.md`; `../CHANGELOG.md` [0.0.19])
- ✅ **Full §1–§30 computational legal-logic stack** — SDL⁺ core, Hohfeld jural square, STIT, mens
  rea, causal/but-for, lifecycle, value-flow, capability gap, resilient identity, ZK-gate,
  proportionality, sense-translation, consensus, meta-deontic, interaction governance.
- ✅ **101 instruments → CML concept graph** (3,518 concepts / 3,619 norms); **220 `.q42` volumes**;
  **BCP-47 `@en`** tags; `docs/values-credentials.html`.
- ✅ **42-shape SHACL coverage** (`logic_modalities_shacl.rs`) + fleshed-out epistemic shape.
- ✅ **MCP cooperation gate + flag-gated enforcement** (`QUALIA_MCP_ENFORCE`); tools `values_check`,
  `values_evaluate`, `jural_correlate`, `deontic_govern`, `mcp_cooperate`, `graph_resolve`.
- ✅ **CML library-upgrade protocol** — `SCHEMA_VERSION`, `tools/reprocess_library.py`, `CML_UPGRADE.md`.
- ✅ **38-card Modalities Observatory.**

---

## NOT done (pointer only — see PLAN §21.2/§21.3, `../MULTI_AGENT_MCP_PLAN.md`, `../RENDERER_SURVEY.md`)
- **Renderer (per our discussion):** render is a **projection of the 10D manifold**, not a
  separate 2D engine — *(this corrects an earlier "2D renderer" mischaracterisation by the tool)*.
  Its **current** output is a **~2.5D particle field**: the view-proj + camera + PGA are in place,
  but there is **no depth-stencil, no mesh, no `.obj`/`.stl`/OpenUSD asset import** — 3D assets are
  **not yet rendered**. The close-out is a world-space scene + physics-of-artefacts + spatio-temporal
  binding (RCC-8 / Allen). The renderer belongs in the qualiaDB libraries (single-source engine;
  the browser is an optional shell) per `20260621_webizen-browser-engine-migration-review.md`, and
  the full multi-modal definition (EM-spectrum source-of-truth, visual SPD + audio STFT/CQT) is
  `10d/q42-10d-volumetric-tensor-spec.md`. Spectral wave-physics (§D), singular pipeline (§F),
  manifold-native transcode (§A), heterogeneous compute core (§G) = **decided, not built**.
  *(PLAN §21.2; `../RENDERER_SURVEY.md`)*
- **LLM → `.q42` pipeline** (broken); CBOR-LD `@context` expansion; document codec (PDF); chained
  credentials; the deferred backlog (PLAN §13); the unified to-do (PLAN §21.3).
