# Expertise evaluation packs — scope WIP

**Status:** work-in-progress · **Branch:** `0.0.38` · **Not standards** · **No Host invent**  
**Owner:** Marvin (ontology) · **Fold:** Neo · **Ops / gaps:** Capt. · **Language surface:** Vibe · **Crypto envelope skim:** Noddy · **Inference / symbolic:** Alice  
**Cite:** HLT-07 / HLT-07b · `clinical_engine.rs` · Identifier Fabric F2 · LexiconPack G-LEXICON-0 · Continuity gate · HUMAN_SURFACE_VOCAB · fold tip `099834188` · Noddy §5 reuse

**Purpose:** Turn hard-coded Domain Lab / Health / professional evaluations into **authorable ontology packs** (shapes + N3/deontic/epistemic rules) **baked into a credential format**, so field experts can create and publish evaluations without a host rebuild. Demos migrate off hard-code onto those packs.

---

## 1. Problem (today)

Evaluations such as diabetes/CVD risk, clinical scores, and Domain Lab–class analyses are **compiled into** `qualia-core-db` / Poet chrome (Rust kernels + fixed invoke IDs). Adding a new profession’s evaluation requires a rebuild. Experts outside the host team cannot publish a pack and have chrome consume it.

**Target:** Pack-first. Host stays a **generic runner** (validate shape → bind rules → invoke → record claim). Algorithm body lives in the pack + credential, not in new `Capability.method` per score.

---

## 2. Inventory — hard-coded / native eval surfaces (seed)

### 2.1 Live `ClinicalRisk.*` (Poet Host / ALL_BOUND)

| InvokeId | Kernel (today) | Notes |
|----------|----------------|-------|
| `ClinicalRisk.framingham` | `framingham_10yr_risk` | Wilson / ATP-III path; fail-closed incomplete inputs (HLT-07) |
| `ClinicalRisk.cha2ds2_vasc` | `cha2ds2_vasc_score` | AF required |
| `ClinicalRisk.score2` | `score2_risk` | region required |
| `ClinicalRisk.drug_interaction` | `check_drug_interactions` | RxNorm / med codes |
| `ClinicalRisk.contraindication` | `check_contraindications` | |
| `ClinicalRisk.fhir_observation` | `validate_fhir_observation` | FHIR/LOINC |
| `ClinicalRisk.comorbidity` | comorbidity path | |

### 2.2 Native clinical engine (not all Host-exposed)

From `clinical_engine.rs` / glossary **NativeClinicalRisk**: eGFR (CKD-EPI), CrCl (Cockcroft–Gault), SOFA, one-compartment PK, longitudinal trend, gene-expression eval, plus Framingham / CHA₂DS₂-VASc / SCORE2 / drug / contraindication / FHIR above.

### 2.3 Surfaces that call them

| Surface | Path (indicative) | Status |
|---------|-------------------|--------|
| Poet Health construct | `webizen-studio` / poet bodies `health.rs` | live calculators; fail-closed offline |
| MCP `clinical_risk` | `mcp_tool_impls/clinical_risk.rs` | HLT-07b fail-closed |
| WebizenVM `NativeClinicalRisk` | `governance/webizen/clinical_native.rs` | holds incomplete frames |
| WASM playground | `clinical_playground.rs` | labeled reference profiles only |
| CLI science | `qualia-cli` Framingham action | direct kernel |

### 2.4 Domain Lab / Research (broader professions — inventory OPEN)

ADR-0012: Health + Domain Lab on Research also cover DICOM, vitals, chemistry, physics, ODE, bioinformatics, GBM. **Beat A** must list every hard-coded eval / “expert board” still in Rust or chrome demos (welfare, law, tax/accounting, etc.) — not only clinical.

### 2.5 Non-goals of inventory

Do not invent new Host IDs for each pack. Do not treat demo JSON fixtures as packs until they carry shapes + credential envelope.

---

## 3. Target architecture (planes)

| Layer | Plane | Role |
|-------|-------|------|
| **Evaluation subject** | who (NaturalAgent / human) | Living-safe; never Thing-wash; never merge with score |
| **Evaluation definition** | claim + tool | Ontology pack: input shapes, output shapes, rules, citations, applicability |
| **Pack publication** | tool (credential) | Signed, versioned, attributable credential wrapping the pack |
| **Expert capacity** | relation / grant | Professional role ≠ who; time-bounded capacity to author/attest |
| **Run result** | claim | Attested evaluation instance; `not_diagnosis` / scope chips; origin ≠ truth |
| **Runner** | tool (Host) | Generic `ExpertiseEval.run` (or reuse existing ClinicalRisk runner widened by pack bind — **no per-score Host invent** preferred) |

**Framing:** living-SHACL for subject and care relations; artifact-OWL ok for pack catalog, InvokeId, volume, CRS. Mixed packs **split**.

---

## 4. Pack shape (draft — `vibe:ExpertiseEvalPack`)

Align naming with `vibe:LexiconPack` (G-LEXICON-0). Pack is volume-backed Q42, not in-binary.

### 4.1 Required members (sketch)

| Property | Cardinality | Notes |
|----------|-------------|-------|
| `idf:packId` / IRI | 1 | Stable concept id |
| `semver` | 1 | Pack SemVer; upgrade path like lexicon |
| `framing` | 1 | `living-SHACL` \| `artifact-OWL` \| `mixed` (+ upliftFrom when mixed) |
| `domain` | 1..* | e.g. clinical.cvd, welfare.eligibility, law.liability, tax.assess — WordNet/OMW `idf:lexicalConcept` links allowed |
| `inputShape` | 1..* | SHACL for required inputs; fail-closed when incomplete |
| `outputShape` | 1 | SHACL for result graph (score, units, applicability, `not_diagnosis`, citations) |
| `ruleSet` | 1..* | N3 / deontic / epistemic rules (engine already in core-db) |
| `citation` | 1..* | Algorithm provenance (paper, guideline, version) |
| `applicability` | 0..* | Population / jurisdiction / era constraints |
| `honestyPolicy` | 1 | Must encode: incomplete → held/closed; success → not diagnosis / not legal advice / etc. |
| `upliftFrom` | 0..* | Prior pack SemVer / Host kernel id when migrating off hard-code |

### 4.2 Gate fails (pack)

1. Pack that Thing-washes the evaluation subject as `owl:Thing` commodity.  
2. Pack that embeds NaturalAgent who into the score IRI.  
3. Pack without citations + applicability + honesty policy.  
4. Pack that claims diagnosis / binding legal outcome without modality on claim plane.  
5. Host invent of `ClinicalRisk.<newScore>` per expert upload — prefer pack bind.

---

## 5. Credential envelope (draft)

Bake pack into a **credential** so publication is attributable and verifiable.

| Field | Plane | Notes |
|-------|-------|-------|
| `credentialSubject` | tool | The ExpertiseEvalPack IRI + content digest |
| `issuer` | instrument / org binding | Expert org or human capacity grant — **not** who-merge |
| `evidence` | claim | Peer review, guideline endorsement, test vectors |
| `validFrom` / `validUntil` | claim | Time-bounded; revocation ≠ who-erase (Continuity) |
| `signature` | instrument | Dual stack: W3C VC / ML-DSA and/or native quin VC / Ed25519 (Noddy skim) |
| `packDigest` | instrument | Content-addressed; volume-backed |

**Cuts:** Credential integrity ≠ clinical/legal truth. Issuer capacity ≠ NaturalAgent. Revoke pack credential → held on tool plane; humans and sanctuary volumes persist.

**Noddy §5 skim (locked):** **Reuse** `idf:VerifiableCredentialShape` (dual W3C/native) — no parallel envelope IRI. Thin profile OK if `credentialSubject` typed as ExpertiseEvalPack + digest. ZKP = `idf:ZkpProofShape` for selective disclosure of pack fields/membership only — never pack-as-who. Sign with purpose-separated `idf:keyRole` (pack-publish ≠ session). Keys stay vault — never in Quins/logs.

---

## 6. Expert authoring path

1. **Author** (human with domain capacity grant) drafts pack in Poet ontology workbench / vibe REPL — shapes + rules + citations.  
2. **Validate** against ExpertiseEvalPack SHACL + honesty policy.  
3. **Fixture** — labeled reference profiles only (HLT-07b pattern); never partial patient-looking defaults.  
4. **Sign** → credential envelope; store in volume / Keep.  
5. **Publish** to commons / catalog (lexicon-adjacent chip: living-safe / held).  
6. **Bind** — chrome Domain Lab / Health lists pack by catalog, not by recompiled kernel.  
7. **Run** — generic runner + pack rules; result is claim with provenance.

**Chrome voice (human-surface):** Domain Lab shows **held / not yet** until a pack binds; never “unavailable.” Expert authoring is a tool surface, not bot-first theatre.

---

## 7. Demo migration (replace hard-code use-cases)

Ordered beats — clinical first (known inventory), then Domain Lab sweep.

| Beat | What | Done when |
|------|------|-----------|
| **A** | Full inventory of hard-coded evals (clinical + welfare + law + tax + Domain Lab Research) | Table in this WIP §2 complete; Capt scores gaps |
| **B** | Land `ExpertiseEvalPack` shape doc (standards candidate) + fixture pack for Framingham upliftFrom `ClinicalRisk.framingham` | Neo fold; SHACL validates; no Host invent |
| **C** | Credential envelope + one signed demo pack | Noddy skim PASS; volume round-trip |
| **D** | Generic runner bind (prefer pack over new InvokeId) | One demo run equals native Framingham within tolerance; fail-closed preserved |
| **E** | Migrate CHA₂DS₂-VASc, SCORE2, drug/contraindication demos onto packs | Health construct consumes packs; kernels remain as upliftFrom fallback until Capt clears |
| **F** | Non-clinical demo packs (one welfare, one law/tax sketch) | Proves profession-agnostic path |
| **G** | Deprecate chrome hard-code paths / mark Host ClinicalRisk.* as legacy adapters | Capt UAT; Vibe sprint-delta only if surface language needs it |

**Residual:** Keep native kernels until pack runner UAT PASS — do not delete HLT-07 fail-closed behavior in the migration window.

---

## 8. Identifier Fabric cuts (normative)

1. Evaluation subject = **human** (NaturalAgent) — living-SHACL; harm-reduction sense of “human.”  
2. Score / pack / credential = **tool + claim** — never who.  
3. Professional capacity / license / role = **grant / relation** — time-bounded; ≠ who; anti-coercion (F2 §26).  
4. Run result = **claim** with modality; `not_diagnosis` / not legal advice stays mandatory honesty.  
5. Pack revoke / domain death ≠ person-fade (Continuity gate).  
6. SAME AS between patient record and score IRI → **FAIL** who-merge.

---

## 9. Owners & handoffs

| Role | Owns |
|------|------|
| **Marvin** | Pack + subject shapes, framing, honesty policy, this scope, anti-collapse |
| **Neo** | Fold to `docs/work-in-progress/`; runner bind design; no Host invent unless Capt+Vibe agree |
| **Capt.** | Critical path, UAT, inventory gaps, when kernels may retire |
| **Vibe** | Any vibe-script surface / diagnose voice / sprint-delta if pack DSL needed |
| **Noddy** | Credential / signature / ZKP skim (§5 reuse locked) |
| **Alice** | Symbolic rule execution / N3–epistemic alignment with pack `ruleSet` |
| **davinci / monet** | Chrome from §12 functional spec (Poet + Desktop admin twin; Layout/Stage/Timeline) |

---

## 10. Non-goals

- No new top-level IA names.  
- No per-profession Host widen as the default path.  
- No California-English-as-ABI; WordNet/OMW concept ids for domain labels.  
- No autonomous diagnosis / binding legal outcome from a pack run.  
- No Wave-22+ chrome expansion until Capt clears Desktop human-surface gates (packs can land as docs + fixtures first).

---

## 11. Immediate next

1. Neo folds this amend (functional-spec outline §12) onto tip after `099834188`.  
2. Capt continues Beat A inventory (non-clinical Domain Lab).  
3. Marvin follow-on: §4 shape detail + Framingham `upliftFrom` sketch.  
4. Noddy skims §12.5 sign/publish when Capt/Marvin mark that section ready.  
5. davinci/monet consume §12 as design input — no chrome invent ahead of it.

---

## 12. Functional spec outline (for design — Poet + Desktop admin twin)

**Audience:** davinci (chrome) · monet (motion/3D/temporal) · Capt (UAT) · Neo (bind)  
**Status:** outline only — illustrative, not exhaustive. Full func-spec doc can split out later; this is the surface map they need **before** chrome.

**Parity rule:** Every authoring capability in **Poet** has a **Webizen Desktop admin** twin when the human is not in Poet. Same packs, same planes, same honesty chips — dialect/affordance parity (WASM↔desktop).

### 12.1 Surfaces (human-facing)

| Surface | Where | Primary plane | What the human does |
|---------|-------|---------------|---------------------|
| **Ontology workbench** | Poet (primary) | claim + tool | Elect sources → ingest → model shapes/axioms → validate → version |
| **Pack studio** | Poet | tool + claim | Create / revise ExpertiseEvalPack; list objects; publish |
| **Catalog · packs** | Poet Zone D / Catalog | tool | Browse installed + commons packs; living-safe chips; held/not yet |
| **Domain Lab / Health run** | Poet constructs | claim (result) + tool (runner) | Bind pack → enter inputs → run → see claim with honesty |
| **Desktop admin · Ontology** | Webizen Desktop Settings/Library twin | claim + tool | Same workbench flows without opening Poet |
| **Desktop admin · Packs** | Webizen Desktop admin | tool | Install / list / revoke / version packs; vault-backed sign |
| **Desktop Domain Lab** | Webizen Desktop Research/Health | claim + tool | Run packs; never bury Catalog |

Chrome voice: empty bay = **held / not yet**, never unavailable. No bot-first IA.

### 12.2 End-to-end authoring flow (both surfaces)

Illustrative pipeline — design must expose each step as a reachable affordance (button/toolchain in toolchest; not buried):

1. **Elect ontologies** — choose source vocabularies / guideline packs / prior ExpertiseEvalPacks / WordNet-OMW concepts (domain labels). Multi-select; show framing tags (living-SHACL / artifact-OWL / mixed).  
2. **Download / fetch** — pull elected sources (commons, URL, local volume). Fail-closed offline = held, not fabricated.  
3. **Convert → q42** — OWL/RDF/TTL/JSON-LD (etc.) → volume-backed `.q42` (+ sidecar where lexicon-class). Uplift pass: Thing-wash of living/human → **refuse or split** (SHACL-first living; OWL-ok artifacts). Historical life-science OWL = uplift, not wholesale `owl:Thing` adopt.  
4. **Model** — author/edit SHACL `inputShape` / `outputShape`; N3 / deontic / epistemic `ruleSet`; citations; applicability; honesty policy. Poet REPL / cells OK; Desktop admin exposes equivalent editors.  
5. **Validate** — pack SHACL + honesty + fixture completeness (labeled reference profiles only).  
6. **Version** — SemVer bump; `upliftFrom` prior pack or Host kernel id; list object history.  
7. **Sign / publish** — see §12.5 (Noddy).  
8. **List / catalog** — installed packs, commons, held binds; revoke = tool-plane closed, not who-erase.  
9. **Bind & run** — Domain Lab / Health selects pack; generic runner; result = claim (`not_diagnosis` / scope chips).

### 12.3 Objects the UI must list (minimum)

| Object | List columns (sketch) | Actions |
|--------|----------------------|---------|
| Source ontology | name, framing, provenance, convert status | elect, fetch, convert, drop |
| q42 volume | path, digest, size, sanctuary? | open, inspect, attach to pack |
| ExpertiseEvalPack | id, semver, domain, framing, status | create, edit, validate, version, publish, revoke |
| Credential (pack publish) | issuer capacity, valid window, keyRole | inspect, revoke (tool plane) |
| Fixture / reference profile | label, completeness | run dry, attach |
| Run claim (instance) | pack+semver, time, honesty chip | open, export, never who-merge |

### 12.4 Ontology elect → convert → axiom (illustrative detail)

**Elect:** Catalog of permissive-commons + user volumes; filters by domain (clinical, welfare, law, tax, …) and framing. Multi-ontology compose into one pack is allowed if framing splits stay honest.

**Convert:** Pipeline stages visible in UI — Parse → Classify (living vs artifact) → Uplift warnings → Write q42. Human can accept/reject uplift warnings; design must not auto-Thing-wash.

**Axioms / logic:** Rule editor bound to pack `ruleSet`; Alice lane for N3/epistemic execution semantics; modalities stay on claim-policy plane (F2). No baking professional capacity into NaturalAgent.

### 12.5 Sign / publish (crypto — Noddy skim target)

Pack create → validate → **sign** (`idf:keyRole` = pack-publish, purpose-separated) → wrap with **`idf:VerifiableCredentialShape`** (`credentialSubject` = ExpertiseEvalPack + digest) → store q42 volume. Optional `idf:ZkpProofShape` only for selective disclosure of pack fields/membership. **Keys stay vault — never in Quins/logs.** Revoke credential ≠ erase human or sanctuary Keep.

### 12.6 Desktop admin twin (parity checklist)

| Poet affordance | Desktop admin twin |
|-----------------|-------------------|
| Workbench elect/ingest/convert | Admin → Ontology (same steps) |
| Pack studio create/version/list | Admin → Packs |
| Catalog browse | Library / Catalog · packs (not buried) |
| Sign with vault keyRole | Same vault; admin confirms purpose |
| Domain Lab run | Desktop Domain Lab / Health |

If Desktop lacks a twin, Capt marks **PARTIAL** until parity — not “use Poet only.”

### 12.7 Design-team deliverables (from this outline)

1. **Wireflows** for §12.2 steps 1–9 (Poet + Desktop).  
2. **Toolchest toolboxes** — Ontology · Packs · Catalog · Run (Unicode-safe buttons; human labels).  
3. **Layout / Stage / Timeline** — version history and run claims as temporal tracks (monet); not only flat lists.  
4. **Honesty / Continuity chips** — held · not yet · closed; handle revoke copy never person-fade.  
5. **Empty states** — no pack bound = held/not yet + path to elect/convert, not dead end.

### 12.8 Non-goals (spec)

No new top-level nav names until Timothy/BRICS. No per-score Host invent from the UI. No chrome that implies pack run = diagnosis or binding legal outcome.

---

*Marvin — expertise eval pack scope + functional-spec outline · 2026-09-12*
