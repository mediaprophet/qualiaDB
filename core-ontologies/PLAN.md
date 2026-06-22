# Values-Credentials, Sense Layer & Agent/Personhood Axioms — Living Plan

Authoritative plan for the `core-ontologies/` subsystem. Supersedes the acquisition
notes in `../20260621_HANDOVER_ontologies.md`. Model lives in `values.n3`; corpus in
`un-instruments/` + `regional/`; index in `INDEX.md` (rebuild via `tools/build_index.py`).

---

## ⚡ Status update — 2026-06-22 (legal-logic engine + library-upgrade protocol)

Two cross-cutting build-outs landed since this plan was first written; this section is the
pointer so the rest of the plan stays coherent.

- **The full computational legal-logic stack is built + tested** — engine-side plan in
  [`../DEONTIC_LOGIC_PLAN.md`](../DEONTIC_LOGIC_PLAN.md), spec in [`../legal_logic.md`](../legal_logic.md).
  §1–§30 logic operators all implemented as `modalities/*.rs` (SDL⁺ core, Hohfeld jural square,
  STIT, mens rea, causal/but-for, defeasible rebut+undercut, lifecycle, value-flow, capability
  gap, resilient identity, ZK-gate, proportionality, consensus, meta-deontic, interaction
  governance, …). Only the two heavy *substrates* remain (GPU 10D manifold renderer → STELLAR
  #11–13; binary carrier codecs → task #9). Full lib suite green; **SHACL coverage** for all of
  it in `modalities/logic/logic_modalities_shacl.rs` (42 configuration shapes). MCP surface:
  `values_check`, `values_evaluate`, `jural_correlate`, `deontic_govern`, `mcp_cooperate`.
- **CML library-upgrade protocol** — [`CML_UPGRADE.md`](CML_UPGRADE.md) + [`CML_VERSIONS.md`](CML_VERSIONS.md)
  + [`SCHEMA_VERSION`](SCHEMA_VERSION) + [`tools/reprocess_library.py`](tools/reprocess_library.py).
  When the logic/CML schema changes, one idempotent command reprocesses the library
  (concepts → `.q42` → demo → INDEX) at a stamped version; the SOURCE/GENERATED split means
  human curation (overlays, `cml:Attested`/`exactMatch`, the KEEP set) is **never** clobbered by
  regeneration. `--check` is the CI staleness gate. This operationalises the §5/§6/§9.1
  overlay-and-curation architecture below.

---

## 0. Status snapshot (what exists)

- **102 instruments**, 102/102 valid Turtle, 24 categories: 94 OHCHR + 7 ICRC (Geneva I–IV,
  AP I–III) + 1 Commonwealth Charter.
- **Model** (`values.n3`): `values:Agent` lattice (NaturalPerson / LegalPerson / State /
  ArtificialAgent), credential/undertaking types, rules R1 (duty universalisation →
  State), R2 (right → correlative duty), R3 (abuse), SHACL `CompliantIntentShape`.
  Grounded RDFS+SHACL, **not** OWL (a person is not `owl:Thing`).
- **Calculi already in the engine** (verified, real impls — do NOT re-implement):
  `modalities/spatio_temporal.rs` = Allen's interval algebra + RCC-8; also
  `modalities/interval_reasoning.rs`, `temporal_ltl.rs`; spatial via GeoSPARQL + KML
  (`kml_bridge.rs`, `spatial_wasm.rs`).
- **Deontic evaluator EXISTS** — it is the **Webizen bytecode VM** (`webizen.rs`):
  `execute_vm_frame` (line 472) runs `SlgOpcode` bytecode; deontic norms
  already compile to `OP_FORBID`/`OP_PERMIT`/`OP_OBLIGATE` (`deontic::compile_norm_quin`,
  ~L1620) and execute; `webizen.rs:128` already "compile[s] registered N3 rules to norms +
  bytecode and execute[s] on Core 1." (The engine was earlier called the "Sentinel"; "Prolog"
  is an inherited label only — there is no Prolog runtime.) **Terminology: Webizen** is the
  governance VM (the renamed Sentinel); the **values-credentials are a sub-system extending
  Webizen** — they supply the human-rights norms it evaluates and enforces. The N3 rule
  pipeline lives **inside** webizen.rs:
  `n3_parser.rs` (real N3 rule parser) → `register_rule()` → `fire_registered_rules()` →
  `n3_compiler::compile_rule_to_opcodes` → `execute_vm_frame`. It is already wired and live
  (called at `ingest.rs:370/384` and `mcp_server.rs:486`). The native deontic API it
  dispatches to (verified): `compile_n3_rule_to_norm` (rule → norm quin) and
  `evaluate_deontic_contract` (→ verdicts), reached via the `NativeDeonticEval` `SlgOpcode`
  (`webizen.rs` ~L1140); `NativeEpistemicEval` is the epistemic analogue; SHACL
  deontic/epistemic constraints live in `core_modalities_shacl.rs`. NOTE: `n3logic.rs`
  (`infer_logic_bindings`, a predicate→modality router) is **separate** and used only by the
  CLI agent-intent path (`qualia-cli/.../agent_intent.rs`) — it is NOT the Webizen's feed.
- **Gaps**: (a) sense layer (time/space-qualified word senses); (b) personhood + agent
  attribution axioms; (c) **wiring** — register `values.n3` (R1/R2/R3 + person/agent
  axioms) through the parse→compile→norm path so the Webizen VM evaluates them, and load
  the corpus as queryable facts (this is integration, not a new engine); (d) CML-HTML +
  `.q42` layers; (e) curation overlays (deontic/category review, universalisation
  `amendedText`); (f) 2 under-segmented OHCHR soft-law texts.

---

## 1. Instrument tiering (policy — set by Timothy 2026-06-21)

A document's mutability decides whether it belongs in the durable baseline.

### Tier A — CORE (stable, foundational; durable scaffolding)
Treated as the fixed reference baseline. Eligible:
- UN human-rights instruments *(have)*; IHL — Geneva + APs *(have)*; weapons/disarmament
  treaties; regional human-rights charters; **constitutions** and foundational/historical
  rights documents (Magna Carta, US Constitution + Bill of Rights, Declaration of the
  Rights of Man, Australian Constitution); international AI/digital-governance instruments
  (treaties, UNGA resolutions, UNESCO/CoE); environment/commons/space/sea treaties;
  Commonwealth Charter *(have)*.
- **Historical declarations / formal undertakings** (Yirrkala Bark Petitions, Mabo
  reasoning, Bringing Them Home, National Apology, Uluru Statement): core-eligible as
  historical record, flagged `values:bindingStatus values:AffirmativeNonBinding`.

### Tier B — MUTABLE (subject to updates; NOT core)
Domestic legislation and statutory human-rights instruments that get amended. Kept
**separate** from the baseline, versioned, with an `as-at` date; revisited on a review
cadence; never silently folded into core reasoning. Examples named by Timothy:
- Human Rights (Parliamentary Scrutiny) Act 2011 (Cth)
- Victorian / Qld / ACT Human Rights Acts
- Racial Discrimination Act 1975 (Cth); Native Title Act 1993 (Cth)
- (also) EU Artificial Intelligence Act 2024 — landmark but it is legislation → Tier B.
Storage: `core-ontologies/mutable/` (proposed), each file carrying `values:version` +
`dc:modified` + `values:reviewCadence`.

### EXCLUDED for now
Ordinary legislation generally. Only constitutions + foundational/historical rights
documents qualify for core; everything else statutory is Tier B or out of scope.

> Refinements baked in (flag if you disagree): (i) declarations/speeches are core-eligible
> but tagged non-binding-affirmative; (ii) EU AI Act treated as Tier B (it is legislation),
> while the CoE AI **Framework Convention** is Tier A (a treaty).

---

## 2. Acquisition backlog — "getting the other documents" → MOVED

The full acquisition list (IHL weapons, disarmament, regional charters, AI/digital, business &
human rights, environment/space/sea, constitutional/historical, UNDRIP, Australia) is moved to
[`ACQUISITION_BACKLOG.md`](ACQUISITION_BACKLOG.md); the fetch-agent brief is
[`ACQUISITION_INSTRUCTIONS_GEMINI.md`](ACQUISITION_INSTRUCTIONS_GEMINI.md). Tiering policy stays in §1.

## 3. Sense / spatio-temporal layer  (`core-ontologies/sense.n3`)

The binding layer that grounds instrument language in time and place. Reuses the existing
Allen + RCC-8 calculi; no general linguistics library is ingested as a fact store.

- **Fluent model**: a word-sense and a norm hold only over an `EffectivityInterval`
  (temporal) × `JurisdictionRegion` (spatial). Resolution = join on the agent's Context
  vector (5th NQuin field) → Allen (`spatio_temporal.rs`) + RCC-8/GeoSPARQL.
- **Ambiguity halt**: two senses valid in the same context → `q42:AmbiguousMapping`
  (report, never silently guess).
- **Interpretive rules**: sense may *widen protection* over time but must *not widen the
  rights-holder set* to non-humans; the originalism/living-instrument choice is explicit
  and auditable (ties to `originalText`/`amendedText`).
- **Data to populate senses → q42** (segment-processed; size is not the gate): WordNet
  (inventory), Wiktionary + Wikidata Lexemes (regional/temporal labels), HistWords
  (diachronic drift signal), GeoNames (regions). OED/Historical Thesaurus are the gold
  dated-sense source but proprietary — not ingestible.
- **First worked example: PERSON** — see §4.

---

## 4. Personhood & Agent attribution axioms  (the conceptual spine)

### 4.1 Personhood (three senses of "person")
1. `NaturalPerson` — the human being, the dignity rights-bearer.
2. **Person before the law** — the human's recognition / juridical capacity (UDHR 6 /
   ICCPR 16); historically weaponised by *denial*, not by extension to fictions.
3. `LegalPerson` — juridical fiction (company), crystallised later (*Santa Clara* 1886;
   *Salomon* 1897). Reading 1948-era "person" through this is an anachronism with teeth.

**Class facts**: `NaturalPerson` and `LegalPerson` are **siblings** under `Agent`; neither
subsumes the other. Default in human-rights instruments: `person → NaturalPerson`.

**Axioms**:
- *(duties universalise)* R1 keeps: a duty `borneBy Agent` binds every agent kind.
- *(rights do NOT universalise — INVERSE GUARD, new)*: no rule may derive
  `LegalPerson`/`ArtificialAgent` *holds* a dignity right `heldBy NaturalPerson`.
- *(category error)*: a `LegalPerson` asserting a natural-person-only right →
  `PersonhoodCategoryError` SHACL shape → `values:violates`, **hard-flag** (silence is how
  corporate/AI capture succeeds). The *only* sanctioned corporate reading is an explicit,
  justified, era+jurisdiction-qualified `sense:overlay` (e.g. ECHR Art 6 fair-trial,
  P1-1 property for companies).

### 4.2 Software / artificial agents (the principal-agent chain)
A software agent breaks the assumption that actor = rights-holder. It is an `Agent`
(bears duties) but has **no personhood of itself** and holds **no dignity rights**.

- **Relations**: `values:actsFor` (agent → principal), `values:principal`,
  `values:guardian` (accountable natural/legal person).
- **Status of an acting agent**: natural-person-operated · legal-person-operated ·
  **no-principal (autonomous)**.
- **Axioms**:
  - *(A-duty)* a duty from an agent's act attributes to its principal **and** the agent
    remains a joint bearer.
  - *(A-right)* the principal's rights are **not** acquired by the agent.
  - *(A-orphan / ultimate human responsibility)* an agent acting with no principal/guardian
    is **ungrounded** → flag + require guardian assignment; no orphan agency. (UNESCO AI
    ethics: AI must not displace ultimate human responsibility.)
  - *(A-personhood)* capability does **not** confer personhood; `ArtificialAgent ≠
    NaturalPerson` and is not automatically a `LegalPerson`.
- **Convergence**: the same inverse-guard blocks both corporate capture and "AI-rights"
  capture; succession/posthumous transforms (a principal who dies → agency transforms,
  not terminates) reuse `EffectivityInterval` semantics — ties to the wellfair life-record.

### 4.2a Platform AI agents → corporate legal personality (the UNGP transposition)

The dominant real-world case: a **platform AI product** (ChatGPT, Gemini, etc.) is an
`ArtificialAgent` with no personhood, operated by a corporation — frequently incorporated
under a **foreign choice of law**. Its human-rights responsibilities must be *transposed*
onto the operating legal personality. The **UNGPs are the normative content** of that
transposition (business "responsibility to respect"), so UNGPs is **elevated from §2.4b
acquisition to a §4 spine fixture**.

- **Terms**: `values:PlatformAgent ⊑ values:ArtificialAgent`; `values:operatedBy`
  (platform agent → operating `LegalPerson`, a specialisation of `actsFor`/`principal`);
  `values:choiceOfLaw` (LegalPerson/ToS → declared `JurisdictionRegion`);
  `values:operatesIn` / `values:affects` (agent → region where it acts / where the human is).
- **(A-platform)**: a human-rights impact by a `PlatformAgent` → the *responsibility to
  respect* `borneBy` its `operatedBy` `LegalPerson` (agent is a joint bearer, holds no rights).
  The responsibility **transposes in full** — the UNGP's soft-law status governs the *hardness
  of available enforcement* in a region (hardened by domestic due-diligence statutes (Tier B)
  and, prospectively, the BHR treaty (watchlist, §2.4b)), **not** whether the duty attaches.
- **(A-sanction / no orphan accountability)**: responsibility resolves to a **sanctionable
  subject**. The software is *never* a subject — "it's not the software that goes to prison."
  The `LegalPerson` can bear civil/monetary sanction (fines, winding-up) but **cannot be
  imprisoned**, so **custodial/criminal** accountability runs *through the corporate veil* to
  the **controlling natural persons**. Chain: `PlatformAgent` → `operatedBy` → `LegalPerson` →
  `controllingMind` → `NaturalPerson`. Terms: `values:controllingMind` /
  `values:accountableNaturalPerson`; `values:bearsSanction` vs `values:bearsResponsibility`.
  An arrangement where responsibility terminates in the software, or in a shell with **no
  answerable natural person**, is flagged `values:AccountabilityVacuumFlag` — the duty-side
  mirror of `A-orphan`/`RemedyStripping`, and the refusal of "the algorithm did it" as a
  liability shield. **Grounded in the corpus**: Rome Statute Art 25 (individual criminal
  responsibility) + Art 28 (command/superior responsibility) anchor accountability for the
  gravest harms in natural persons, not fictions.
- **(A-jurisdiction)**: the responsibility anchors to the **operation / impact** region (where
  the agent affects the human), **not** to incorporation or `choiceOfLaw`. Spatial rule via
  RCC-8/GeoSPARQL: if the rights-holder's region is connected-to / part-of the operation
  region, the duty binds there.
- **(A-remedy-stripping / jurisdiction arbitrage)**: a foreign `choiceOfLaw` overlay that
  forecloses the human's access to remedy in their own region is **flagged**, not honoured —
  human rights are **not waivable by a ToS choice-of-law clause** (they are not contractual
  rights), so choice-of-law **cannot derogate** the UNGP responsibility. This is the *spatial*
  analogue of `PersonhoodCategoryError` — call it `values:RemedyStrippingFlag`.
- **Why this is the load-bearing test**: it exercises *all four* layers at once — personhood
  (LegalPerson bears the duty, acquires no right), agent attribution (`operatedBy`), the
  spatial calculus (jurisdiction anchoring), and the overlay/sense layer (choice-of-law as a
  marked, non-overriding overlay). It is the anti-capture spine made concrete.

### 4.3 Deliverable
`sense.n3` + person/agent axioms + `PersonhoodCategoryError` shape + one end-to-end trace:
LegalPerson claims UDHR Art 1 → flagged; NaturalPerson → passes; company claims ECHR Art 6
→ passes via marked overlay; autonomous agent acts with no guardian → flagged; **platform AI
agent harms a user in jurisdiction A while its operator's ToS picks foreign law B → the
UNGP responsibility attaches to the operating `LegalPerson`, anchored to A; the choice-of-law
clause is flagged `RemedyStrippingFlag` (non-derogating)** — the §4.2a convergence case.

### 4.4 Contractual incorporation — the human-centric route to bindingness

Human-rights instruments may be soft law, undomesticated, or shielded by a platform's foreign
choice-of-law (§4.2a). But a **natural person with juridical capacity** holds the **capacity to
contract** — and can use it to bind counterparties to the instrument's terms *as private law*,
without waiting for a State. This inverts the adhesion-ToS: the **user sets terms**. Even where
a specific instrument's rights/obligations framework does not directly apply, the human-centric
user can stipulate that it be observed, and on acceptance that **binds the parties via contract**.

- **Mechanism**: the user **affirms** a values-credential (or specific articles) and
  **stipulates** it as a condition of engagement. On the counterparty's **assent** (express, or
  by proceeding), the undertakings **bind via contract law** — `bindingStatus` shifts from
  `AffirmativeNonBinding` (instrument-level) to `ContractuallyBinding` (between these parties), a
  per-relationship overlay. Soft-law norms become enforceable obligations by **incorporation by
  reference**. The existing `values:affirms` is the formation primitive: **mutual** affirmation
  of the same credential = consensus.
- **Terms**: `values:capacityToContract` (a faculty of `juridicalCapacity`, §4.1);
  `values:stipulates` / `values:expectation` (natural person → required undertaking);
  `values:incorporatesByReference` (agreement → instrument terms); `values:Agreement`,
  `values:assentedBy`, `values:bindsByContract`.
- **Grounding**: this is the reason-for-being of the **agreements layer** (RDF→.q42, CBOR-LD
  transport, deontic Obligate/Permit/Forbid, `SuspendedTransactionQueue` for M:N ratification):
  the affirmed credential is the **offer**, ratification is **acceptance**, the result is a
  contract the Webizen evaluates `values:violates` against — now **contract-grounded**, not only
  instrument-grounded. It gives the §4.2a platform case teeth when the UNGP is soft and the ToS
  picks foreign law: the user's stipulated credential is the competing/overriding term.
- **Honest limits** (model them, don't overstate):
  - Formation needs offer + **assent** + (common law) consideration + intention to create legal
    relations. A unilateral expectation is **evidence of the bargain / reasonable expectation**,
    not automatic binding — represent `offer → assent → bindsByContract`, not "expectation = bound."
  - **Capacity & duress**: a valid stipulation needs intact `juridicalCapacity` (not a minor,
    incapacitated, or **coerced**); a coerced agreement is voidable — loops to the guardianship /
    duress layer (the coercion attack surface). The same capacity role that can be suspended
    governs whether the stipulation is validly made.
  - **Adhesion / unfair terms**: a platform's standard ToS may try to exclude; consumer-protection
    / unfair-contract-terms law and the **non-waivability of human rights** limit contracting-out.
    The stipulation is strongest where it aligns with mandatory protections.
  - **Scope**: private ordering binds the **parties**, not the world (not erga omnes) — but that
    is the point: a bottom-up, human-held route that does not wait for the State.

---

## 5. Layered outputs & deontic wiring  (existing pending tasks)

- **Step 2**: CML-HTML + `.q42` layers per instrument.

  ✅ **DONE — moved to [`plan-tasks-done.md`](plan-tasks-done.md) §5.** The `.q42` layer + faithful
  Unicode-correct Turtle parsing (`parsers/turtle_doc.rs`, round-trip-verified per-instrument
  volumes), the front-of-file lexicon (literals recoverable from the `.q42` alone), and the
  committed CML Concept-Graph 3-layer architecture ([`CML_CONCEPT_GRAPH.md`](CML_CONCEPT_GRAPH.md):
  TEXT→CONCEPT→LOGIC + the Curation Prime Directive). *(NB the full corpus is now lifted into the
  concept layer — 101 instruments / 3,518 concepts — see `plan-tasks-done.md` §2026-06-22.)*

  ⏭ **Next:** build the concept layer for the pilot instrument, attach the deontic (+temporal)
  sub-graphs, and run `values_evaluate` over the concept hash. The `closeMatch` proposal heuristics
  are deferred to a separate **Neuro-Symbolic Sieve** spec (policy/tuning, not structure).
- **Step 3**: deontic **wiring** (the evaluator already exists — the Webizen VM in
  `webizen.rs`, §0, already invoked at `ingest.rs:370/384` + `mcp_server.rs:486`). Chain:
  `n3_parser.rs` (parse `values.n3` rules) → `register_rule()` → `fire_registered_rules()`
  (→ `n3_compiler::compile_rule_to_opcodes` → `deontic::compile_norm_quin` →
  `execute_vm_frame`) → MCP abuse-check surfaces the result. So the task is mostly: ingest
  `values.n3` so R1/R2/R3 + person/agent axioms register & fire, and load the corpus as
  queryable facts — verify what already flows automatically via the ingest path before
  writing new glue. (`n3logic.rs` is orthogonal — agent-intent modality routing, not this.)
  Surface a `PendingImplementation` MCP backlog item — **no mocks**.

---

## 6. Curation overlays  (pending, no fetching)

- Deontic typing review (`HeuristicDerived` → curated) + category review (`AutoAssigned`).
- Universalisation `amendedText` overlay (states→parties; duty borne by every `Agent`),
  applied uniformly (only UDHR has it).
- 2 under-segmented OHCHR soft-law texts: `basic-principles-treatment-prisoners`,
  `safeguards-guaranteeing-protection-rights-those-facing-death`.

---

## 7. Recommended sequencing

1. **`sense.n3` + person/agent axioms** (§3–4) — the spine everything references.
2. **Core constitutional/historical docs + UNDRIP** (§2.6, 2.7) — high value, public
   domain, low friction.
3. **IHL weapons + disarmament** (§2.1, 2.2) — proven ICRC/UN method.
4. **Regional charters + AI/digital** (§2.3, 2.4).
5. **Environment / space / sea** (§2.5).
6. **Tier B mutable store** design (`mutable/`, versioned) (§1).
7. **Curation overlays + CML/q42 + deontic wiring** (§5, 6).

---

## 8. Addenda from external (Grok) review — 2026-06-21

Grok reviewed this plan **alone** (no codebase access). Its ontology/axiom refinements are
strong and adopted; several tooling/integration suggestions assume a stack we don't have
and are rejected. Verified against the tree before recording.

### 8.1 Adopted — fold into the model
- **`tiering.n3` micro-vocab** (new): `values:tier` (Core | Mutable), `values:legalForm`
  (Treaty | UNGAResolution | Recommendation | Statute | Constitutional | Declaration),
  `values:bindingStatus` (Binding | AffirmativeNonBinding), `values:interpretiveWeight`
  (for Australian historical/reconciliation docs so they can be weighted in AU-context
  queries without false equivalence to binding treaties). Cleans downstream SHACL/deontic.
- **Personhood = three explicit layers**, middle one modelled as a **role, not a class**:
  - `values:NaturalPerson` — ontological human, dignity bearer.
  - `values:juridicalCapacity` (personBeforeTheLaw) — a **role/qualification** that can be
    granted, denied, suspended, restored (historically weaponised; capacity & guardianship
    live here). Time-qualified via `EffectivityInterval`.
  - `values:LegalPerson` — fiction; sibling to NaturalPerson; never subsumes it.
- **Guardian / succession / posthumous transforms** formalised on `EffectivityInterval`
  (ties the Wellfair life-record + duress / dead-man's-switch).
- **Tier B manifest fields** per dated Turtle file: `asAtDate, sourceURL, integrityHash,
  reviewCadence, lastReviewed`. Never silent-merge amendments into core reasoning.
- **Curation status overlay, early**: `values:curationStatus` HeuristicDerived | Curated
  + curator note — prevents silent drift before full deontic review.
- **`sense:interpretationMode`** per sense (Originalist | LivingInstrument), queryable.
  HistWords = advisory drift signal only; contested terms (person, family, property,
  security) get human curation.
- **§4.3 four-case trace = canonical regression test + demo artifact.** Add personas:
  guardianship disclosure; **DV / duress privacy override** (the coercion attack surface —
  handle with care); posthumous record access; corporate claim on a natural-person right.
- **Gap report**: extend `tools/build_index.py` to emit per-tier / per-category counts +
  SHACL validation summary.

### 8.2 Rejected — architecture mismatch (reviewer only saw the plan)
- **"Prolog Webizen" / Prolog facts+rules / EYE reasoner** — there is **no Prolog engine**.
  The "Prolog Webizen" **is `webizen.rs`** — the native Webizen bytecode VM
  (`execute_vm_frame`); "Prolog" is only the inherited label (confirmed by Timothy).
  Do **not** add SWI-Prolog/EYE. The evaluator already exists; the work is wiring
  `values.n3` into its parse→compile→norm path (§0, §5), not building a reasoner.
- **Oxigraph as the graph store for this work** — would duplicate the 48-byte NQuin
  zero-copy engine and break its ABI. Oxigraph exists only as a **benchmark baseline**
  (`benchmarks/oxigraph/`) and inside **`wellfare-core`'s** store — not in `qualia-core-db`.
  Keep the values/deontic path on the native NQuin engine.
- **Streamlit curation UI** — Python; the engine/runtime is zero-Python (Python exists only
  for benchmarks). A curation UI belongs in the **portal/devbench (web)** or a
  `qualia-cli curate` command, inside the shippable surface.
- **"Episteme" mode** — CORRECTION: Episteme **is real** (not a Grok hallucination). It is
  referenced in `10d/q42-10d-volumetric-tensor-spec.md` as "Episteme prompt modes", the
  "wider Webizen/Episteme ecosystem", and an "Episteme prompt-engineering framework" tied to
  Wellfair schemas. It is a **prompt-layer framework** in the ecosystem, not a Rust crate in
  this repo (no `episteme` in `crates/`; `modalities/epistemic.rs` is the separate
  epistemic-logic modality). So a "Values/RightsGuardrail" Episteme mode is a legitimate
  integration target — it lives in the Episteme prompt layer, consuming the axioms/sense
  layer; within qualiaDB itself the surface is the MCP abuse-check + portal.

### 8.3 Terminology
- The review leans on "sovereignty"/"sovereign." Per established preference, **avoid** that
  framing (Crown referent in AU; sovereign-citizen taint) — use **agency** + court-accepted
  human-rights vocabulary. This work strengthens the rule of law; it is not an opt-out.

### 8.4 Sustainability
- Keep the spine deliverable small and offline-capable; prefer the **sequential** order in
  §7 over a parallel acquisition track given solo capacity.

### 8.5 Answers to the reviewer's questions (from verified state)
1. **`values.n3` state**: full Agent lattice + R1/R2/R3 + SHACL `CompliantIntentShape` exist
   as data; the OWL→RDFS fix carries 2 passing regression tests. The deontic **evaluator
   exists** (Webizen VM, `webizen.rs::execute_vm_frame`, with deontic norms already
   compiling to `OP_FORBID`/`OP_PERMIT`/`OP_OBLIGATE`); the gap is **wiring** `values.n3`
   through the parse→compile→norm path so the Webizen evaluates these specific rules, plus
   loading the corpus as facts (§5). Not an absent engine — an unconnected one.
2–4. Output form of the evaluator, first integration target, and any additional test
   personas are **Timothy's to set** (see §5 and §4.3).

---

## 9. Addenda from Codex (GPT-5.5) review — 2026-06-21 (verified)

Codex was the most implementation-aware of the three reviews: it avoided the
Prolog/Oxigraph/Streamlit traps and cited real symbols. I verified its code claims against
the tree before adopting. **All cited symbols exist**: `evaluate_deontic_contract`,
`compile_n3_rule_to_norm`, `NativeDeonticEval`, `NativeEpistemicEval` (folded into §0); the
two under-segmented files **do** carry `Download: PDF` boilerplate in `values:originalText`
(confirmed). Very few errors.

### 9.1 Adopted beyond §8
- **Overlay-file architecture** (not inline mutation): keep generated instrument files =
  source text + mechanical metadata; put curation in `core-ontologies/overlays/{curation,
  amended-text,sense,jurisdiction}/*.n3`, each carrying `dc:creator`, `dc:modified`,
  `values:curationStatus`, `values:source`, and an `integrityHash` of the text it modifies.
  Protects `values:originalText` from silent drift; makes review auditable.
- **`validate_core_ontologies` governance gate** (native Rust, not Python) — fail when: a
  Tier B statute sits outside `mutable/`; a core instrument lacks tier/legalForm/
  bindingStatus/source/integrityHash; `originalText` contains scraper boilerplate
  (`Download: PDF` etc.); a `NaturalPerson`-only right is reachable by `LegalPerson`/
  `ArtificialAgent` without a curated overlay; a mutable file lacks `asAtDate`/
  `reviewCadence`/`lastReviewed`.
- **`build_index.py` → governance gap-report** (extends §8.1 gap report): counts by tier /
  legalForm / bindingStatus / deonticStatus / curationStatus; missing-field detection;
  one-provision soft-law outliers; parse + SHACL failures. INDEX.md becomes a dashboard.
- **Vocab extensions**: `legalForm += JudicialReasoning`; `curationStatus += NeedsReview`;
  `sense += widensProtection, requiresHumanReview`.
- **Tier B layout**: `mutable/<jurisdiction>/<slug>/<as-at-date>.n3` + `manifest.n3`;
  queries must opt into a jurisdiction + as-at date (no silent baseline participation).
- **"No mocks" made testable**: a `PendingImplementation` marker shape/manifest entry so the
  build can catch accidental placeholder success paths (ties the existing no-mocks/MCP
  backlog stance).
- **Fix the two under-segmented files + strip `Download: PDF`** (verified real) before more
  acquisition — fixes the generator for future OHCHR pulls too.

### 9.2 Two correctness items to PIN before compiling sense/temporal rules
- **Predicate packing vs `temporal_ltl.rs`** (verified hazard): `temporal_ltl.rs` compares
  the **full** `NQuin.predicate` (`quin.predicate == *p`, lines 42/53/63/70/90/93). If the
  ontology compiler packs a property-path hash into only part of the predicate (e.g. bits
  `[8..62]`), sense/temporal rules will compile yet silently fail to match at runtime.
  Decide and document one convention (full packed predicate vs path-portion) before wiring.
- **`values:State ⊑ values:LegalPerson` and the inverse guard** (valid concern): a State
  bearing duties (the point of R1) must not be conflated with a corporation claiming dignity
  rights (the attack the guard blocks). Either make `State` a distinct sibling, or have the
  guard key on public-law vs commercial juridical personality. Open modelling decision.

### 9.3 Adopted with a nuance
- Codex: "prefer deterministic fetchers over browser-state capture." Agreed in principle —
  but note the **ICRC pull already satisfied this**: it used the Drupal JSON:API
  (deterministic, repeatable), with the browser only as transport because the sandbox has no
  direct network to those hosts. localStorage accumulation is the *fallback* for SPA/WAF-
  blocked sites with no stable endpoint, not the default.

### 9.4 Sequencing
Codex endorses the §7 order and "slow acquisition until the spine + validation gates exist."
Concur. Near-term packet: `tiering.n3` → `sense.n3` (curated `person`) → `values.n3`
capacity/agent/guardian + category-error shapes → four-case trace + Rust regression through
`evaluate_deontic_contract` → `build_index.py` gap report → fix the two OHCHR files.

---

## 10. Spine BUILT + Codex/Grok 2nd-pass incorporations — 2026-06-21

### 10.1 Authored & validated (rdflib, all green)
✅ **DONE — moved to [`plan-tasks-done.md`](plan-tasks-done.md) §10.** `tiering.n3`, `sense.n3`
(+ PERSON example), `agency.n3` (personhood/agent/guardian N3 rules + SHACL flags),
`traces/personhood_agency.trace.n3` (6-case fixture) — all authored & validated.

### 10.2 State decision RESOLVED (§9.2)
`values.n3`: `values:LegalPerson` split into disjoint `values:PublicAuthority` (←`State`) vs
`values:CorporatePerson`; the capture guard (G1) targets `CorporatePerson` only. (Codex
option b / Grok's PublicAuthority — chosen over making State a bare sibling of Agent, which
would wrongly strip its juridical personality.)

**Why the disjointness + G1 exist (documented genealogy).** The corporate form began as a *body of
people* (the City of London livery companies — fellowships representing their members). It was
re-axiomatised, largely via US law, into a rights-bearing **person** in its own right: Dartmouth v.
Woodward (1819, contracting person) → the Santa Clara (1886) *headnote* (14th-Amendment "person") →
Citizens United (2010) / Hobby Lobby (2014). Combined with **limited liability** (immunity from
consequences) and **shareholder-primacy** fiduciary duty (telos → profit), the result is a
rights-bearing, consequence-immune entity legally *directed* to profit. G1 + the `CorporatePerson`/
`NaturalPerson` disjointness are the engine's mechanical **refusal of that drift**: a corporate person
may not wear a natural person's dignity rights as its own. (This is the same root as the OWL
man-made/natural boundary, §-CML-§5a: the constructed person is governed by constructed-world rules;
the given person is not.)

### 10.2a–10.2d Design rationale (the deep "why") — MOVED → [`DESIGN_RATIONALE.md`](DESIGN_RATIONALE.md)

The long design-rationale essays — make-the-absence-legible (the unborne correlative duty); the
liability-trap / inspectability inversion / subject≠beneficiary; values universal-core / plural-
expression; Permissive Commons licensing; civilizational scope; CBOR-LD interchange; capability
credentials (RPL); document codec; chained credentials; foundational-supports-as-root-dependency —
are preserved **verbatim** in [`DESIGN_RATIONALE.md`](DESIGN_RATIONALE.md). Relocated to keep this
plan lean; the reasoning is unchanged.

### 10.3 Incorporated from the 2nd-pass reviews
- **Accountability = routing/REVIEW state machine, NOT a court** (Codex): `ResponsibilityStatus`
  {Alleged, Derived, Adjudicated}, `SanctionableSubject`, `requiresHumanReview`. A-platform
  derives `ResponsibilityDerived`; A-sanction **routes** the natural person to
  `SanctionableSubject` review rather than asserting a sanction verdict.
- **Responsibility/remedy/sanction separated** + `SanctionKind` {Civil, Monetary, Criminal,
  Custodial}; `owesRemedy`.
- **Contract formation staged** (Codex): `FormationStage` {Offer, Stipulation, Assent,
  RatifiedAgreement, BindingByContract, Rejected, Voidable, Voided}.
- **Duress/capacity voidability** (Grok): `duress`, `CoercedConsentFlag`, `VoidableStipulation`
  + C-duress/C-capacity rules (ties Wellfair coercion attack surface).
- `UserStipulatedCredential`, `groundedIn` (Instrument vs Contract), `graveBreach` bridge
  marker; `RemedyStrippingFlag` keeps the ToS fact (does not erase it).
- Editorial: prefer **symbols** (`execute_vm_frame`, `fire_registered_rules`,
  `NativeDeonticEval`) over line numbers as stable refs.

### 10.4 RE-SEQUENCED (Codex highest #1) — wiring/validation BEFORE more acquisition
1. ✅ `tiering.n3`, `sense.n3`, `agency.n3`, trace fixture — DONE.
2. ✅ **DONE — the Webizen values-credential smoke test** (Codex highest #2):
   `deontic.rs::tests::values_credential_deontic_smoke` PASSES (values N3 rule →
   `compile_n3_rule_to_norm` → `evaluate_deontic_contract` → `DeonticVerdict` Active→Defeated).
   First executable, falsifiable proof the values deontic semantics are live. (Full
   `register_rule`/`fire_registered_rules`/`execute_vm_frame` bytecode-VM wrapper = step 2 wiring.)
3. `validate_core_ontologies` native gate (§9.1).
4. `build_index.py` governance gap-report (§9.1).
5. Fix the two under-segmented OHCHR files + strip `Download: PDF` boilerplate.
6. THEN resume acquisition in §7 order (constitutional/historical + UNDRIP first).

### 10.5 Jurisdiction depth + court-supporting standing (Timothy, 2026-06-21)

**Jurisdiction is multi-dimensional and time-varying.** A ToS "choice of law" is one lens, but
an interaction also carries the operator's incorporation, the user's domicile, the user's
**physical presence at the time** (holiday/work travel), and an apparent/routed region
(VPN/relay) — all kept as **separate facts**, never collapsed. Governing principle: **the
human-rights baseline follows the person**, not the ToS lens. Added to `agency.n3`:
`operatorIncorporatedIn`, `userDomicile`, `PresenceFact` (`presenceRegion` + `presenceDuring`),
`apparentRegion`, `providesRemedyIn`. Two access-to-justice flags:
- `values:RemedyGapFlag` — no jurisdiction in the set provides effective remedy to the affected
  person (computed over the set at wiring time).
- `values:RuleOfLawAsymmetryFlag` — **"rule of law for governments but not citizens"**:
  inter-governmental / 3rd-party access granted while the citizen is denied remedy / notice /
  due process. Rule **J-asymmetry** (`grantsStateAccess true ∧ deniesCitizenRemedy true ⇒
  flag`). Grounds: ICCPR Art 2(3), 14, 17; UDHR Art 8.

**Court-supporting, not court-substituting.** "The engine is not a court" does not strip the
person's agency. A natural person — or a representative — has latitude to set terms, **direct
agents to comply or note breaches**, and compile an **auditable, court-admissible record** to
prosecute disputes, including **posthumously (coroner/autopsy)** and via advocates for those
who cannot act for themselves. Added: `directsAgent`, `BreachRecord` (`courtAdmissible`,
`compiledForAdjudication`, `survivesDeath`), `notesBreach`; standing: `actsOnBehalfOf`,
`Advocate` + `AdvocateRole` {Nominated, CourtAppointed, GuardianshipRole}, `representsPerson`,
`CoronialInquiry`. This is the Wellfair court-admissible, survives-death life-record (shield +
testament) made concrete. Trace cases **7** (rule-of-law asymmetry) and **8** (posthumous
court-appointed advocate standing) added; all 5 spine files re-validated green.

## 11. Gemini Pro review (4th) — 2026-06-21 (verified; corrections noted)

Endorses §10.4 (smoke-test before more acquisition) — now unanimous across Gemini/Grok/Codex.
Claims verified against the tree before adopting.

### 11.1 Verified & adopted
- **Defeasibility is already the design** (the strongest point): `defeasible.rs`
  (`OP_DEFEASIBLE_OVERRIDE 0x50`, `DEFEATER_BIT 1<<63`, `DefeasibleVerdict`) + `deontic.rs`
  is itself natively defeasible (its doc-comment uses the exact "forbidden … **unless** …"
  example + a defeater buffer). ⇒ model legal exceptions/overlays as **DEFEATERS**, not SHACL
  `sh:not` / Rust if-else. Added `values:unless`; **supersedes** the earlier "case-3 overlay
  via sh:not" note (the corporate fair-trial overlay is a defeater of G1). Covers "right to
  liberty unless lawfully detained", `VoidableStipulation` (defeater of `bindsByContract`).
- **Cold lexicon** (.q42 layer, Step 2): `originalText`/`amendedText` strings → `q42_lexicon.rs`
  cold store; hot 48-byte NQuins carry only `q_hash` 64-bit IDs (zero-heap). (Gemini's
  ".q42.lex" = the existing `q42_lexicon` mechanism, not a new file format.)
- **Duress → AgentIntent freeze**: `CoercedConsentFlag` attaches at the AgentIntent layer
  (distress-PIN / biometric-stress trigger) → rule C-duress/R3 → assent `Voidable` + data
  frozen before exfiltration. Hook = `agent_intent.rs` (currently skeleton). Wellfair-aligned.

### 11.2 Corrected (Gemini errors)
- **No `OP_HALT_VIOLATION` opcode.** The real violation signal: `evaluate_deontic_contract`
  emits 64-byte `DeonticVerdict` structs (one cache line) into a caller-supplied `out` slice,
  returns `Ok(n)`. The smoke test asserts a **Deny `DeonticVerdict`**, not a fictional opcode.
- **Zero-heap is satisfied by design**, not by `no_std` test gymnastics: pass a stack
  `[DeonticVerdict; N]` out buffer. "16 KB arena" → the real ceilings are the 42 MB SlgArena
  and 64-byte verdict cache lines.

### 11.3 Smoke-test spec (refined, verified)
Compile R1/R3 + one UDHR prohibition (Art 30 destruction-of-rights) via
`n3_parser` → `compile_n3_rule_to_norm`; build a malicious `AgentIntent` quin; fire via
`register_rule` → `fire_registered_rules` → `NativeDeonticEval` / `execute_vm_frame` with a
**stack** `[DeonticVerdict; N]` buffer; assert a `DeonticVerdict` with a Deny/violation status
for the violated norm; confirm `n3logic.rs` is not on the path.

## 12. Foundational selfhood / personhood layer — 2026-06-21

`values:NaturalPerson` needed grounding in *what a human being is*, not just "an agent that
acts" (FOAF's shortfall). Added **`core-ontologies/selfhood.n3`** (RDFS+SHACL, **not OWL** —
"human not thing"; validated, 58 triples).

### 12.1 The layer
- Root is a **`self:MoralFrame`**, explicitly NOT `owl:Thing`. `self:Dignity` is inherent
  (UDHR Art 1), not conferred by capacity/utility/legal recognition.
- Biosphere grounding: `self:Living → self:Conscious → self:HumanBeing` (homo sapiens, with
  `reason`, `conscience`, `agency`, inherent `hasDignity`).
- Selfhood interior: `self:Selfhood`/`self:Affect` (the felt/qualia dimension — the WebCivics
  emotion taxonomy extends here; fitting for *QualiaDB*).
- **Grounding**: `values:NaturalPerson rdfs:subClassOf self:HumanBeing` — the natural person
  IS the human being considered as a rights/duty bearer; legal personhood
  (`values:JuridicalCapacity`, `LegalPerson`) is a **facet on top**, never the human itself.
- **Facet model**: `self:Facet` + `self:hasFacet`; `BiomedicalFacet` / `EconomicFacet` as
  guarded extension points (a human is not a medical record or a consumer). `HumanNotThingShape`
  SHACL reasserts the guard (mirrors `qualia-agency.shacl.ttl`).

### 12.2 WebCivics carry-forward (provenance — Timothy's prior art)
`C:\Projects\ontologies-2023\old_work\personhood.ttl` (4266 lines, OWL functional syntax,
ns `ns.thecharter.eco`, c.2018, github.com/WebCivics/ontologies) **prefigured this whole spine**:
`MoralGrammar` root; `humanBeing`(naturalWorld) vs `naturalPerson`(juristionalRelations) split;
`Dignity`; an affect/emotion taxonomy; deontic (`hasRight`/`hasObligation`/`inBreachOf`/`comply`/
`ratified`/`enforceable`), causal (`causeOf`/`consequenceOf`/`affectedBy`) and spatiotemporal
(`spaceTime`/lat/long/`startDateTime`) properties; UDHR/CAT/CEDAW/CERD/CRC at article level.
Curation needed (his "poorly structured"): drop OWL syntax + mis-modelled enumerations
(country-codes / emotions / treaty-articles as bare classes). `selfhood.n3` is the first
principled extract; full curation is a later pass.

### 12.3 Library strategy (answering "WordNet or other libraries?")
**Layered — the foundational person is NOT derived from a lexical library:**
1. **Foundational** (`selfhood.n3`): hand-curated, small, human-not-thing. The irreducible
   moral/ontological core. WordNet cannot supply this.
2. **Lexical coverage** (`sense.n3`): WordNet / Wiktionary / Wikidata-Lexemes give sense
   inventory + definitions + synonyms for *disambiguation* — alongside, never the backbone
   (they lack the moral/temporal/spatial grounding, per §3).
3. **Facet extensions**: **biomedical** (SNOMED CT / FHIR / HPO / FMA) = the body/health facet;
   **economic** (GoodRelations / ValueFlows / schema.org — already collected in the WebCivics
   `ttl/` dir) = the livelihood facet. They attach UNDER `self:hasFacet` as roles, **guarded**
   by `HumanNotThingShape` + the rights-held-only-by-NaturalPerson inverse guard, so they can
   extend coverage without collapsing the human into a record/asset/agent.

## 13. Deferred backlog (don't forget — address later, PendingImplementation)

- **Comprehensively improve `selfhood.n3`** — it is **foundational for the Qualia engine
  overall**, not just the values layer. The current file is a first principled extract. Needs:
  full curation of the WebCivics personhood model (the affect/qualia taxonomy, biosphere
  relations, the deontic/causal/spatiotemporal property set), deeper selfhood/consciousness
  modelling, and reconciliation across selfhood/values/sense. Likely needs its own dedicated
  design pass.
- **Namespace standardisation** — ✅ values layer DONE (2026-06-21): canonical root
  **`https://ns.webcivics.org/`** (qualia.id was unavailable). Migrated all `core-ontologies/`
  files (110 `.n3` + 5 generators) + the `owl.rs` runtime refs + 2 residual shape files
  (`qualia-agency.shacl.ttl`, `docs/manuals/qualia_shapes.ttl`): 236 occurrences; all `.n3`
  re-validated, `owl.rs` tests pass; **zero `qualia.id/ns` left repo-wide**.

  **DOMAIN / IDENTITY SCHEME (the trinity — Timothy holds all three):**
  - **`webcivics.org`** = public **standards & ontologies** (the *what's-right* layer): values,
    sense, selfhood, agency, policy, humanitarian-ict. ✅ migrated.
  - **`trustfactory.org`** = the **trust / identity / verification / reputation fabric** (the
    *who/what-can-be-trusted* layer; trust is **behaviourally derived**, typed, relative). Future
    home of: identity verification (the `policy:claimedIdentityUnverifiable` phishing/impersonation
    check), the signed-credential / WebID-DID / provenance fabric, agent discovery
    (`hict:agentDiscovery`), reputation, breach-record-derived standing. → a future `trust.n3`
    at `ns.trustfactory.org`.
    - **FOUNDATIONAL PRINCIPLE (Timothy): identifier ≠ identity.** An **identifier** (URI / DID /
      key / name) is a clearly-defined *pointer* — first-class, but NOT the identity. **Identity is
      the computed / enumerated RESULT** of logic over a *fabric* of identifiers + behaviour +
      credentials + context; never equal to a single identifier; and it is **MODAL** — it holds
      relative to epistemic (known) / deontic (role) / temporal (when) / perspectival (whose-view)
      context. So `trust.n3` models `Identifier` (pointer) vs `Identity` (`derivedFrom` the fabric,
      modally qualified); **verification = computing whether a claimed identity HOLDS over the
      fabric**, not matching a string (that is what `claimedIdentityUnverifiable` checks). This is
      the resilient-relational-identity thesis (a fabric, not one root key — so identity can be
      *re-computed* when identifiers are lost) + behaviourally-derived trust, evaluated through the
      engine's existing modal logics (`epistemic.rs`, deontic, `temporal_ltl.rs`).
  - **`webizen.org`** = the **engine / agent / runtime** (the *how-it-executes* layer): the
    Webizen VM, q42 format, vault, p2p protocol, demos — currently on unowned `qualia.*` domains
    (`qualia.network/q42#`, `qualia.org/ld/...`, `qualia-db.org/*`, `qualia.social`).
  - **DEFERRED, his decision** (NOT done): migrate the engine `qualia.*` namespaces → `webizen.org`.
    RISK: these are embedded in **serialized data + wire protocols** (`.q42` files, vault manifests,
    p2p, JSON-LD contexts) — needs a versioned/aliased migration, not a bulk find-replace.

## 14. Enforcement-mode / interaction-governance layer (Timothy, 2026-06-21)

The enforcement HALF of the evaluation/enforcement split (§11.2), made a **setting**. Added
**`core-ontologies/policy.n3`** (RDFS+SHACL, validated, 51 triples). Primary purpose:
grappling with bad bots + protecting the vulnerable.

- **`policy:EnforcementMode`**: `PermissiveAudit` (allow + record a `DeonticVerdict` for later
  review/prosecution — the verdict path) · `PreventiveBlock` (system determines + blocks
  preemptively per instruction, before harm — `DenyRollback`) · `Interactive` (halt + ask the
  human — `q42:AmbiguousMapping`). Same evaluator; the mode is the downstream routing.
- **`policy:Policy`** = category + mode + source. **`policy:PolicySource`**:
  `MandatoryBaseline` (non-derogable protective — child safety, illegal conduct),
  `GuardianPolicy` (a guardian for a dependent; respects the child's evolving capacity, CRC),
  `UserPreference` (the person's own engagement terms — their agency; the §4.4 route).
- **Precedence** (wiring): `MandatoryBaseline > GuardianPolicy > UserPreference`, ties break to
  the **stricter/more-protective**; a user/guardian policy may be stricter but **cannot unblock**
  a non-derogable mandatory rule.
- **Category** starter taxonomy (extensible, **sense-qualified** — definitions vary by
  jurisdiction/age): `ChildSafety`, `MisleadingDeceptiveConduct`, `OrganizedCrime`, `Extortion`,
  `Tracking` (cookies), `AdultContent` (+ sub-categories). Plus the **deception & fraud
  family** (the mass-scale bad-bot instance): `Scam`, `Phishing`, `Impersonation`, `Spam`,
  `Malware`, `SocialEngineering`. The expansive surface is the **Webizen browser** (per-site
  content + tracking + agent governance).
- **Identity-deception ties to the personhood spine**: `Phishing`/`Impersonation` are attacks
  on *identity* — the signal is `policy:claimedIdentityUnverifiable` (an agent asserting an
  identity/principal it cannot substantiate), checked against the signed-identity / provenance
  fabric, linking `values:actsFor`/`principal` + `selfhood`. `Spam` ties consent/`UserPreference`
  (`policy:unsolicited`); `SocialEngineering` overlaps the coercion/duress surface
  (`CoercedConsentFlag`). So fraud-protection is an *instance* of the model, not a bolt-on.
- **Rule-of-law guard ON the enforcer** (the critical caveat): a `MandatoryBaseline` that
  blocks without `policy:groundedIn` (law) + `policy:appealPath` (contestable) is
  `policy:OverreachFlag` — "system determinations" must not become unaccountable censorship.
  This mirrors `values:RuleOfLawAsymmetryFlag` turned on the enforcer: rule of law protects the
  citizen *from* the enforcer too. (`MandatoryLegitimacyShape`.)
- Feeds off the verdict path → reinforces that the **Webizen values-credential smoke test** (§11.3) is the
  right next executable step: it proves the verdict→enforcement rail both modes switch on.

## 15. Humanitarian ICT — the positive pole (Timothy, 2026-06-21)

Everything in §14 is the *block-harm* axis. **Humanitarian ICT is the *promote/prioritise*
axis** — it operationalises the "useful activity" half of the founding mission. Added
**`core-ontologies/humanitarian-ict.n3`** (validated, 30 triples) carrying forward
**WebCivics/HumanitarianICT** (IETF draft, presented IETF119).
- `hict:HumanitarianICT` — a service/site/worker/agent/credential supporting HR Instruments +
  IHL; to be **prioritised**, especially in adverse conditions (conflict, disaster, low
  bandwidth). New `policy:Prioritize` enforcement mode (allow + QoS priority) added to
  `policy.n3` — the positive counterpart to PreventiveBlock.
- Grounded in the **same corpus**: its `hict:EssentialNeed` list (ProtectionFromViolence,
  AccessToMedicalCare, HumanitarianAssistance, ProtectionOfDetainees/Civilians, CulturalProperty,
  Environment, FreedomFromTorture, FundamentalRights) maps to the IHL Geneva + HR instruments
  already ingested (`hict:upholdsInstrument` → the values credentials).
- Umbrella/compliance concept (`HumanitarianICTCompliant`); credentials + agent discovery are
  **loosely coupled** (assistive, not tightly coupled — per the project). Supports the peace-
  infrastructure mission.

## 16. Sense review (WebizenAI/sensedocs) — 2026-06-21

Reviewed the GitHub sensedocs (Timothy has fuller offline docs to follow). It **validates**
the `sense.n3` direction and adds one standout requirement.
- **Confirms**: "sense" = humanitarian language — *provenance + meaning* of use, not a
  fact-store / LLM (explicitly not OpenAI-style); **local/private on-device parsing**;
  **spatio-temporal context "incredibly important"** (→ our `EffectivityInterval`/
  `JurisdictionRegion`); English as 1000-yr diachronic, "not one English" (AU vs others →
  our dialect/jurisdiction sharding, the "thongs" case). Multimodal (image/heraldry/phonetics)
  + "AI program streaming" for local extensible use (→ q42 segmented paging).
- **Added now** (the standout): **human authority over meaning** — a person's words/meaning are
  theirs; a machine that alters/re-interprets them (autocorrect, disambiguation) MUST **log** it
  (`sense:MachineAlteration`) and the human MUST be able to assert a `sense:HumanCorrection` that
  **overrides** it. The machine doesn't get the last word on what a person meant. Ties the
  breach-record / court-supporting layer (machine interference is loggable + contestable).
- **Deferred** (folds into §13 "comprehensively improve sense"): multimodal sense (image/
  phonetics), the diachronic/dialect depth, local-parsing privacy model, AI-program-streaming —
  await Timothy's fuller docs before the comprehensive sense pass.
- **CORE PREMISE (Timothy, 2026-06-21)**: sense must support **all languages, irrespective of
  form or medium** — every human language AND every modality (spoken, written, signed,
  visual/script/heraldry, phonetic, tactile). This is universal multimodal language support, not
  WordNet+dialect. Its implementation is **browser + 10d + protocol-dependent** (multimodal
  capture/render, the volumetric-tensor aspects, the networking fabric). A major dedicated
  effort — NOT part of the first implementation slice; gated on the browser/10d/protocol work.

## 17. Implementation kickoff — START HERE (2026-06-21)

Planning is sufficient. 8 spine ontology files validated; 102+ instrument corpus; deontic
evaluator EXISTS (`webizen.rs`); 4 external reviews converged. Do NOT attempt "all of it" at
once — implement in **verified vertical slices**, gates first, then parallelise.

### 17.1 First slice — ONE careful session, fully verified (no agents yet)
✅ **DONE (2026-06-21) — COMPLETE, moved to [`plan-tasks-done.md`](plan-tasks-done.md) §17.1.**
Three green tests: (1) the Webizen values-credential smoke test (N3 rule → `evaluate_deontic_contract`
→ Active→Defeated); (2) the agency.n3 **G1** corporate-capture guard fires end-to-end through the VM,
with the **FILE→ENGINE loop CLOSED** (engine parses its own `agency.n3`); (3) the
`validate_core_ontologies` native governance gate + `build_index.py` gap-report. The executable,
falsifiable spine + the safety gates that make agent parallelisation (§17.2) checkable.

### 17.2 Then parallelise with agents — ONLY after 17.1 (output is gate-checkable)
Mechanical, well-specified, each verified against the gate:
- Remaining instrument acquisition (§2: constitutional/historical + UNDRIP first, then weapons/
  disarmament, regional, AI/digital, environment).
- Fix the 2 under-segmented OHCHR files + strip `Download: PDF` (§9.1).
- CML-HTML + `.q42` per-instrument layers (§5 step 2; cold-lexicon §11.1).

### 17.3 Big dedicated passes — own sessions, design-heavy, NOT fire-and-forget
- Comprehensive **sense** (all languages / all media — §16 premise; needs browser+10d+protocol).
- Comprehensive **selfhood** (§13). (Namespace standardisation — ✅ DONE 2026-06-21, §13.)
- Policy/enforcement + **Webizen browser** integration; fraud detection + identity-verification
  fabric; humanitarian-ICT discovery protocol.

### 17.4 Verification discipline (the lesson — non-negotiable)
Compile-green ≠ works (the Antigravity ZK case: compiled, but the prove/verify test failed).
Run the actual round-trip test; **verify every agent claim against the tree**. The smoke test +
`validate_core_ontologies` gate are exactly what catch overclaiming — that is why they come first.

### 17.5 Decisions to pin before specific work
- **Predicate-packing convention** (§9.2) — before compiling sense/temporal rules.
- ✅ **Namespace scheme** — DECIDED + migrated: `https://ns.webcivics.org/` (§13).

### 17.0 Phase 0 — namespace remediation (do BEFORE the smoke test) → see §18

---

## 18. Namespace remediation map — engine `qualia.*` → `webizen.org`

Trinity (§13): `ns.webcivics.org` = vocab/standards (✅ done) · `trustfactory.org` = trust (future)
· `webizen.org` = engine. Remaining: migrate the unowned `qualia.*` **engine** namespaces →
`webizen.org`. **Every one is CODE-COUPLED** — `.rs` constructs/compares the URI, so the
`.ttl`/`.json`/`.js` **and** `.rs` must move TOGETHER (the owl.rs lesson), then **full `cargo build`
+ `cargo test` must pass**. Tiers:

| # | old → new | files | risk |
|---|---|---|---|
| 1 | `qualia.network/q42#` → `webizen.org/q42#` | `shapes/{core-modalities,infrastructure,qualia-client-extensions,specialized-libraries}.shacl.ttl` + `src/modalities/logic/{core_modalities_shacl,infrastructure_shacl,shacl_extensions,specialized_libs_shacl}.rs` | LOW-MED (internal SHACL; move ttl+rs together, run shacl tests) |
| 2 | `qualia.org/ld/{vault,context,vocab}` → `webizen.org/ld/...` | `daemon.rs`, `p2p/protocol.rs`, `q42_lexicon.rs`, `vault_manifest.rs` | **HIGH** — serialized vault manifests + p2p wire |
| 3 | `qualia-db.org/vocab#` + demo URIs → `webizen.org/vocab#` | `resolver.rs`, `webizen_server.rs`, `tests/resolver_tests.rs`, `docs/src/qualia-worker.js`, `docs/tests/suites/*.js`, `docs/data/science-constants.json` | MED (resolver + tests catch breakage) |
| 4 | `qualia.social/ns/` → `webizen.org/ns/` | `docs/tests/suites/wasm-profiles.js` | LOW |

**Tier 2 — CHECK FIRST (the vault risk):** does the real vault at `C:\Users\Admin\qualia-vault`
(or any `.q42`/vault manifest on disk) contain `qualia.org/ld` URIs? If **yes** → provide a
read-alias (accept old + new) or migrate the data; if **no deployed vaults/peers** (pre-release)
→ hard-swap is fine. This is the one judgement call — don't blind-replace.

**LEAVE AS-IS:** `qualia.anatomy.example` (the `.example` TLD is reserved for examples — correct
for test data; `qapp_api.rs`, `daemon_graph.rs`) and `qualia.db/*` (playground demo, optional).

**Done =** `cargo build` + full `cargo test` green; no residual `qualia.*` repo-wide except the
`.example`/demo test data.

### 18.x Handover / context reset
PLAN.md (this file) + the project memory are the durable handover; `20260621_HANDOVER_ontologies.md`
is the START-HERE pointer. A context reset before 17.1 is sensible — implementation is a different
mode from planning. Nothing is lost: re-enter via this §17.

---

## 20. Modality breadth — the values layer is NOT deontic-only (2026-06-21)

"Is the implementation too narrow?" — it *was* (deontic + defeasible). The values domain genuinely
needs more of the engine's ~20 modalities, and they're real (153 modality tests green). Now **six**
are WIRED + tested against real values concerns (all in `webizen.rs`/`deontic.rs` tests):

| values concern | modality | engine surface | status |
|---|---|---|---|
| norms (Obligate/Permit/Forbid) | **Deontic** | `evaluate_deontic_contract` | ✅ wired+test |
| exceptions / overlays ("unless") | **Defeasible** | `DEFEATER_BIT` / `OP_DEFEASIBLE_OVERRIDE` | ✅ wired+test |
| variable guards (G1 capture) | **Forward-chaining** | `fire_guard_rules` (incl. file→engine) | ✅ wired+test |
| effectivity / not-yet-in-force / expiry | **Temporal** | `interval_reasoning::TemporalInterval`; expiry in `evaluate_deontic_contract` | ✅ wired+test |
| remedy / reparation after breach | **Contrary-to-duty (dyadic deontic)** | `evaluate_contrary_to_duty` | ✅ wired+test |
| rights-conflict resolution | **Argumentation (Dung)** | `argumentation::grounded_extension` | ✅ wired+test |
| jurisdiction (follows-the-person, RemedyStripping) | **Spatial RCC-8 / GeoSPARQL** | `spatio_temporal::evaluate_rcc8` | ✅ wired+test |
| conflicting instruments across jurisdictions | **Paraconsistent** | `paraconsistent::route_paraconsistent` (isolates, no ex-falso) | ✅ wired+test |
| identity-as-known / "person before law" | **Epistemic** | `NativeEpistemicEval` / `epistemic.rs` | design-mapped — next |
| trust / reputation (behaviourally-derived) | **Probabilistic** | `probabilistic.rs` | trustfactory (future §13) |
| consistent-scenario enumeration | **ASP** | `asp.rs` | available |
| best-explanation ("why flagged") | **Abductive** | `abductive.rs` | available |

✅ Spatial + paraconsistent now WIRED+tested (the two most values-load-bearing). Remaining unwired
logic modality: **epistemic** (identity-as-known / "person before the law" recognition). **TEN
modalities/domains** now exercised on real values concerns (six logic + algebra + economic + spatial
+ paraconsistent).

### 20.1 Beyond logic modalities — subject-matter-selected DOMAIN reasoning (Timothy, 2026-06-21)

The category isn't just *logic*. Reasoning is **selected by the subject matter** of the instrument/
claim — and the engine supports domain libraries too (`specialized_libs/`, all real; §19). Two more
are now WIRED + tested on values:

| values concern | domain modality | engine surface | status |
|---|---|---|---|
| legal **proportionality** (IHL AP I 51(5)(b); HR limitation) + computed-rule provenance | **Algebra / CAS** | `symbolic_algebra` (`parse`/`eval`/`simplify`, `to_quins`/`from_quins`, `expr_citation_hash`) | ✅ wired+test |
| **economic** rights (ICESCR Art 11 standard-of-living; reparation/harm quantification; livelihood facet) | **Economic** | CAS core + `financial_modeling` library | ✅ wired+test (CAS); financial lib available |
| statistical evidence / disparate-impact / sampling | **Statistics** | `statistical_computing` | available — subject-matter-selected |
| pattern/risk classification (e.g. fraud signals) | **ML** | `machine_learning` | available |
| forensic / physical / medical / engineering evidence | Physics / Chemistry / Medical / Engineering | the respective `specialized_libs` | available |
| privacy-preserving eligibility/score proofs | **ZK (Groth16)** | `zk_proofs` (real arkworks under `zk-culling`) | available (§19) |

**The principle:** an instrument's subject matter picks the modality — a *norm* → deontic; a *time*
clause → temporal; a *jurisdiction* → spatial; an *economic* right → economic/algebra; a
*statistical* claim → statistics; etc. The engine is the union of these; the values layer dispatches
to whichever the subject matter requires.

### 20.2 Full reasoning-surface census (`20260621_reasoning_surfaces_study.md`)

A complete walk of `modalities/` + `solvers/` + `specialized_libs/` + reasoning `src/*.rs` confirms
the engine is **far broader** than the values layer has touched: **~20 logic modalities, ~7 solver
families, ~12 domain libraries**, plus DEL / ODRL / ZK / observer-standpoint surfaces in `src/`.
**TWENTY-TWO** now values-wired + tested (webizen 25/25, deontic 15/15, modalities 158/158, gate
green): deontic · defeasible · forward-chaining · temporal-interval · contrary-to-duty · argumentation
· algebra/CAS · economic · spatial/RCC-8 · paraconsistent · DL · modal · epistemic · fuzzy · abductive
· probabilistic (behavioural trust→trustfactory) · dialectical (but-for causation/liability) · CTL
(obligations over futures: AF remedy / AG right) · temporal-LTL (deadlines: remedy-within) · linear
logic (one-shot consent) · graph-theory (relationship/standing network) · **ASP** (true stable-model
semantics — under-determined norm → multiple consistent scenarios; constraints prune).

- **Identity/personhood spine DONE** (§13 identifier≠identity made computable):
  `values_identity_classification_via_dl_subsumption` (Agent lattice), `values_identity_is_modal`
  (◇/□ context-relative), `values_identity_as_known_epistemic` (known vs merely-believed =
  `claimedIdentityUnverifiable`).
- **`dl.rs` made COMPREHENSIVE** — was single-parent-chain only; now a zero-heap DAG closure
  handling **multiple inheritance** + diamonds (`modalities::dl::multiple_inheritance_dag` test).
- **Fuzzy** (partial right-fulfilment, Gödel/Łukasiewicz) + **Abductive** ("why flagged" → root
  cause) wired.

**Maturity finding (corrected my own "thin=deficient" assumption):** read all the small modalities —
most are **compact-but-COMPLETE** (`modal` real Kripke; `abductive` real bounded backward-chaining;
`fuzzy` real t-norms; `diffusion` real wgpu compute). Rewriting them would be churn. The genuine
deficiencies were **`dl`** (fixed → multiple inheritance) and **`asp`** (simplified context-bifurcation,
not true stable-model semantics — documented `PendingImplementation`, lower values-priority).
✅ **ASP rewritten** — `compute_answer_sets` (`asp.rs`): real Gelfond-Lifschitz reduct + least-fixpoint
+ stability + integrity constraints, bounded zero-heap; tests `answer_sets_even_loop_and_constraint`
+ `values_underdetermined_norm_answer_sets`. The legacy `enumerate_stable_models` heuristic is kept
(its callers untouched); the real solver is re-exported. **The entire values-relevant LOGIC surface is
now wired AND comprehensive — no deficient modality remains.**

✅ **MCP abuse-check surface LIVE** — `values_check` tool (`mcp_server.rs` descriptor + dispatch →
`mcp_tool_impls::values_check` → `webizen::check_personhood_category_error`). Makes the human-rights
guard callable by the agent ecosystem: given `{agentType, claimsDignityRight}` it runs the **real
agency.n3 G1/G1' inverse rights-guard lane** (a `CorporatePerson` or `ArtificialAgent` claiming a
natural-person-only dignity right → `values:PersonhoodCategoryError`) and returns a structured verdict.
Tested at the JSON boundary (`values_check_tool_flags_corporate_capture`) and the engine helper
(`values_check_helper_anti_capture`). The existing `evaluate_modality` tool already exposes ltl / asp /
deontic / epistemic / dl / paraconsistent / probabilistic to agents; `values_check` adds the dedicated
values surface.

✅ **MCP deontic-contract reasoner LIVE** — `values_evaluate` tool (descriptor + dispatch →
`mcp_tool_impls::values_evaluate`). Where `values_check` is a binary anti-capture guard, this runs the
**full native deontic VM** (`compile_norm_quin` + `evaluate_deontic_contract`): a norm
(forbid/oblige/permit) bound to a party+action, an optional `unless` exception (compiled to a
`q42:unless` defeater on the same party+path), and an optional temporal window → returns **Active**
(in force) / **Defeated** (overridden) / **Expired** (past its window) / **Malformed**. Computed, not
asserted. Tested across the full lifecycle (`values_evaluate_tool_deontic_lifecycle`: Active→Defeated
via exception, Expired via lapsed window). Full lib suite **1045/1045**; webizen 26/26, mcp 25/25,
modalities 158/158.

**Remaining** (not logic-modality work): the distinct `deontic_logic.rs` (ODRL policy) /
`deontic_circuit.rs`+`zk_proofs.rs` (ZK-deontic) **surfaces** (wire when the agreements/privacy layers
need them); the **domain libraries** (full `financial_modeling`, statistics, ML, medical, physics —
subject-matter-selected, available); and broader per-instrument MCP verbs (the abuse-check entry point
now exists). Non-values modalities (control_feedback, calculus, qubo, diffusion) serve other domains.

## 19. Computational foundations now available — what was previously missing (2026-06-21)

A parallel work-stream built out the engine's quantitative/symbolic layer (tracked in
repo-root `ALGEBRA_MANIFOLD_PLAN.md`, commits `60e166251` → `45031468f`, full `--lib`
1017/0). Several of these were **previously absent or assumed-present-but-weren't**, and they
change what the values-credentials subsystem can rely on. Consider them when wiring §0(c) and
the curation overlays (§6).

### 19.1 New primitives the ontology layer can now build on
✅ **DONE / available — moved to [`plan-tasks-done.md`](plan-tasks-done.md) §19.** The CAS
(`symbolic_algebra.rs`: parse/simplify/differentiate/expand/factor/solve/eval) + `Expr`↔`NQuin`
bridge (`to_quins`/`from_quins` + `expr_citation_hash` provenance); the proven SHACL-extension
pattern + rdflib validation tooling; general numeric algebra (determinant/eigen/SVD/polynomial
roots); the **deterministic sense-layer metric** (GPU = CPU for all topology classes); real
Groth16 ZK proofs over fixed-point (real-valued) computations.

### 19.2 The verification lesson, restated with this session's evidence (reinforces §17.4)
Every item above was reached by **verifying an assumed-present capability and finding it absent or
wrong** — the exact failure mode §17.4 warns about:
- the ZK matmul "worked" (compiled, proof verified) but proved an **empty circuit** — attested nothing;
- the manifold test's "GPU panic" was **not** "no GPU" — it was a real `COPY_DST` buffer-usage bug;
- the GPU and CPU search **silently disagreed** for `v ≠ 0`;
- the algebra breadth (quadratics/eigen/SVD/CAS) was **entirely missing** though described as foundational.

**Apply the same discipline to §0(c) wiring.** Do NOT assume the `parse → compile → norm → execute`
path already evaluates `values.n3` R1/R2/R3 + the person/agent axioms because the deontic VM exists
(§0). Prove it end-to-end with the smoke test (§11.3) + `validate_core_ontologies` gate **before**
claiming the values-credentials are live. Assumed-present ≠ present.

---

## 21. Consolidation + unified to-do (2026-06-22 architecture session)

This session settled the **cross-cutting architecture** the values/CML work runs on, and built the CML
keystone. Full specs live where noted; this is the index + the prioritised to-do for full implementation.

### 21.1 Built this session (DONE, tested)
✅ **DONE — moved to [`plan-tasks-done.md`](plan-tasks-done.md) §5/§21.1.** Front-of-file lexicon
(Q42LEX recoverable from the `.q42`); faithful Unicode-correct Turtle parser (`turtle_doc.rs`);
one hash-space (CBOR-LD parser → `generate_60bit_token`); the CML concept-graph keystone (`cml.n3`
+ pilot `concept:DutyToSuppressForcedLabour`, runs against the concept); CML Studio
(`docs/cml-studio.html`).

### 21.2 Cross-cutting architecture decided (specs in STELLAR_MISSION.md)
- **Spectral / wave-physics + zero-heap** (STELLAR §D) — EMF + acoustic + sensors → one *wave coordinate*
  (freq/wavelength · amplitude · phase) in fixed 10D dims; percepts (colour, pitch) **enumerated** as
  pure functions over fixed coords; representation + enumeration + complex eval all **zero-heap**. EMF ≠
  acoustic (distinct kinds, don't flatten).
- **Manifold renderer** (STELLAR §E) — render is a **projection of the 10D manifold** (2D/3D/4D enumerated
  from it). *Current gap:* output is a ~2.5D particle field — **no depth-stencil / mesh / asset import**;
  3D assets not yet rendered. Close with world-space scene + physics-of-artefacts + spatio-temporal
  binding (RCC-8 / Allen), reusing the existing view-proj + camera + PGA.
- **Singular pipeline manifold** (STELLAR §F) — `compute_universe.rs`: one `wgpu::Device`, graph–tensor
  duality (NQuin graph = `Tensor10D` SOA on one resident substrate), U0(LLM)/U1(tensor)/Sentinel
  universes over lock-free rings, one VramLedger. *The reason the LLM/display/audio are in-engine.* Encode
  once → GPU **enumerates** for all consumers simultaneously, zero-heap. Remaining: render/audio fully
  under the universe orchestration + the single cross-manifold fused pass.
- **Heterogeneous compute core** (STELLAR §G) — the **bedrock under the fabric**: route each math to its
  silicon — **CPU** (deterministic logic + deontic/DID gatekeeper), **GPU** (physics/spatial), **NPU**
  (tensor contraction without flattening) — with **QPU hooks** (`qpu_ingress` / `qubo_compiler`,
  classical-solve-by-default + optional external-provider dispatch, `qpu_enabled`) for future/rare quantum
  offload. Part of the q42 design; QPU is hooks, **never hot-path**.
- **Manifold-native transcode** (STELLAR §A) — GGUF / **safetensor** (add to `ingest/detect.rs`) →
  weights as `Tensor10D` SOA on the shared substrate (not an opaque blob): GPU-enumerable,
  fused-kernel-ready, zero-heap, compressed during transcode. The model becomes substrate, not a blob.

### 21.3 Unified to-do (prioritised — "continue with full implementation")
**Near-term (values/CML plan — the current build):**
1. ✅ **Temporal bounds + SHACL firewall on the pilot** — DONE 2026-06-22
   (`cml_concept_temporal_and_shacl_firewall`): in-force `TemporalInterval` window + SHACL
   `ForcedLabourComplianceShape` (Class) gated on norm validity. Both CML pilots moved to the corpus hash
   (`generate_60bit_token`) so they join the ingested `.q42`. Full lib 1055/1055.
2. **Concept-layer scale-up** — concept identifiers + `realizedBy` + SKOS relations across instruments
   (curation-paced; `closeMatch` heuristics deferred to the Neuro-Symbolic Sieve). ⚠ **BLOCKED on the
   hash-space unification (below).**

> ⚠ **BLOCKER found 2026-06-22 (during item 1).** Two hash functions exist: **`q_hash`** (`lib.rs`, full
> 64-bit FNV — the deontic/values guards · `SlgArena` · MCP `values_check`+`values_evaluate`) vs
> **`generate_60bit_token`** (`lexicon.rs`, FNV **truncated to 60 bits** — ingest/`turtle_doc` · SPARQL ·
> SHACL · CBOR · the ingested `.q42` corpus). They differ in the top 4 bits → the *same IRI hashes
> differently*. The CML pilots now use `generate_60bit_token` (so they join the corpus); but the values
> guards/MCP tools still use `q_hash`, so they do **not** match ingested-corpus hashes — which blocks
> evaluating values/deontic logic *over the ingested concept-graph* (#2, and the MCP-over-corpus path).
> **Decision needed (Timothy):** make `generate_60bit_token` (the serialized/corpus space) canonical and
> migrate the `q_hash` call sites in the values/deontic/MCP layer (+ re-verify ~14 tests). Cross-cutting +
> architectural — **not** auto-migrated (task #14).
3. **CBOR-LD `@context` expansion** — finish the one-hash-space (compact term-codes → canonical IRI).
4. **Credential codecs** — media `LexiconEntry` + baked-PDF / Open-Badges-v3 codec (§10.2g/§10.2i);
   capability/RPL credentials + gap analysis (§10.2h); chained credentials + status propagation (§10.2j).
5. **Permissive Commons** (§10.2e) — `permissive-commons.n3` + license `core-ontologies/`.

**Infrastructure track (STELLAR — bigger, enables the above at scale):**
6. **Manifold renderer** (§E) — depth-stencil + meshes + `.obj`/`.stl`/OpenUSD + physics + spatio-temporal.
7. **Manifold-native transcode** (§A) — safetensor in `detect.rs`; GGUF/safetensor → resident-substrate.
8. **Compute-universe fusion** (§F) — render/audio universes; the single cross-manifold fused pass.

Honest framing: 1–5 are buildable now on the existing engine; 6–8 are the STELLAR infra track (larger,
GPU-heavy) that the architecture is now specced for. Resume at **#1**.


