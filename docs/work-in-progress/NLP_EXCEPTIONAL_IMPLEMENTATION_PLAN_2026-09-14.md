# QualiaDB Exceptional NLP Implementation Programme

**Status:** Active tracked plan · **Branch:** `0.0.38` · **Created:** 2026-09-14  
**Scope:** `crates/qualia-core-db/src/nlp/`, its host/API surfaces, evaluation assets, and direct graph/runtime integration  
**Source review:** repository audit completed 2026-09-14; 62 focused NLP/host tests passed and the `wasm-ontology` target checked successfully  
**Programme objective:** Deliver an evidence-backed, local-first NLP system that is exceptional in linguistic quality, deterministic execution, provenance, ontology grounding, privacy, and constrained-runtime operation.

**Revision:** 2 · reviewed 2026-09-14 against checkout `edd94f0fc695ba41adaf9ec38f857709a874c002`. The earlier test results above are historical observations, not tests rerun for this documentation revision. The original review's “15–20% complete” estimate had no defined denominator and is withdrawn; use the capability matrix and evaluation evidence instead.

**Tracking home:** This document is the editable source of truth. `docs/plans/` is ignored by Git in this checkout; keep this programme in `docs/work-in-progress/`. Work-package counts measure accepted packages, not effort completed or NLP accuracy. Run `scripts/validate-nlp-plan.ps1` after tracker changes. Review and planning do not themselves authorize executing every implementation package, downloading models, or publishing comparison claims.

---

## 1. Executive decision

The existing NLP code is a useful deterministic symbolic prototype, not a complete general-purpose NLP system. It should be retained as the seed of a hybrid architecture, but current public labels must not imply Stanford CoreNLP, Stanza, spaCy, FrameNet, OpenIE, or modern GraphRAG equivalence until benchmark gates prove it.

The programme will pursue superiority in two ways:

1. Reach competitive general linguistic quality on explicitly selected languages and tasks.
2. Exceed conventional toolkits where QualiaDB has structural advantages: exact source provenance, ontology-aware entity linking, graph-grounded semantics, deterministic constrained execution, privacy, audit receipts, native/WASM portability, and governance.

“Better than Stanford” is not a single deliverable. It is a benchmark claim that may be made only for a named task, dataset, language, runtime profile, model version, metric, and resource envelope. Aggregate superiority must not be claimed from a few examples.

Primary comparison systems:

- [Stanford CoreNLP annotators](https://stanfordnlp.github.io/corenlp-docs-dev/annotators.html)
- [Stanford Stanza pipeline](https://stanfordnlp.github.io/stanza/pipeline.html)
- [Stanza models and performance](https://stanfordnlp.github.io/stanza/models.html)
- [spaCy processing pipelines](https://spacy.io/usage/processing-pipelines)
- [spaCy linguistic features](https://spacy.io/usage/linguistic-features)

---

## 2. Non-negotiable engineering constraints

All implementation work remains subordinate to the repository `AGENTS.md` rules.

| Constraint | Programme interpretation |
|---|---|
| Zero heap in hot paths | Token scans, model kernels, graph scoring kernels, and ABI-crossing operations use caller-owned slices/fixed buffers. Owned convenience APIs are cold adapters. |
| 42 MB Sentinel | Every execution pass declares and enforces the 42 × 1024 × 1024 byte ceiling. Account for source, output, workspace, stack, resident weight pages and device buffers. Memory mapping does not exempt resident weight pages; report total process RSS and device residency separately. NLP-009 must prove feasibility before choosing a model. |
| Deterministic, non-recursive | Stable ordering and bounded iterative algorithms are mandatory. Any probabilistic model reports a deterministic mode and seed/decoding policy. |
| 48-byte NQuin | Persisted semantic facts use the existing NQuin ABI; rich transient annotations live in bounded side buffers and emit graph facts through explicit plans. |
| Hashed identifiers | Ontology and relation identifiers use `q_hash()`/60-bit token rules; string authoring stays outside hot evaluation. |
| Tier-2 construction | Gazetteers, model loading, index building, and corpus compilation use bounded construction workspaces, caller-buffered library output, and explicit retention. Transport serialization is a separate bounded adapter; returning an arbitrary Vec does not establish Tier-2 compliance. |
| Module hygiene | Focused implementation files remain below 500 lines where practical. Separate planning, hot execution, model backends, receipts, tests, and assets. |
| Honest capability status | Experimental, partial, and production-ready capabilities remain visibly distinct in code, API metadata, UI, documentation, and audit records. |

No implementation may weaken these constraints merely to match a reference toolkit.

Allocation classes describe measured behavior; assigning a cold class cannot exempt a query kernel. Large reference toolkits run as separate benchmark processes with their resource costs reported. The project's per-pass limit remains binding for Qualia execution. If the intended interpretation of shared residency cannot accommodate a candidate, record the failed feasibility result and choose another candidate or request an explicit architectural decision; do not silently reclassify memory.

---

## 3. Definition of exceptional

The system is exceptional only when all applicable dimensions are evidenced.

### 3.1 Linguistic quality

- Robust non-destructive tokenization and sentence segmentation.
- Language identification and per-language processing profiles.
- Lemmas, universal and language-specific POS, and morphological features.
- Dependency syntax and noun/chunk spans; constituency parsing where justified.
- General and domain NER with entity linking and calibrated ambiguity.
- End-to-end mention detection and coreference.
- Negation, modality, temporality, semantic roles, frames, and open relation extraction.
- Hybrid lexical/vector/graph retrieval with provenance-preserving evidence paths.

### 3.2 QualiaDB-specific superiority

- Every annotation retains exact UTF-8 byte provenance and optional token/code-point coordinates.
- Every semantic assertion can carry confidence, method/model identity, source hash, graph context, and evidence span.
- Ontology constraints participate in decoding, linking, validation, and reranking rather than being applied only afterward.
- Sensitive documents can run locally without an external service.
- A deterministic symbolic profile works in constrained native and WASM environments.
- Learned outputs are fail-closed when weights, calibration, schema, or integrity receipts are absent.
- Graph extraction never presents uncertain NLP output as signed or clinician-authored source data.

### 3.3 Operational quality

- Bounded latency and memory under adversarial input.
- Stable, versioned schemas and migration policy.
- Corpus, model, and benchmark provenance with license records.
- Reproducible native/WASM results for deterministic profiles.
- Differential tests against frozen reference versions.
- No production claim without a machine-readable evaluation receipt.

---

## 4. Current baseline and gap map

Status vocabulary used throughout this plan:

- `DONE` — acceptance evidence is recorded and current.
- `REVIEW` — implementation exists and awaits independent review/gates.
- `IN_PROGRESS` — owned active work.
- `READY` — specified and unblocked.
- `BLOCKED` — named dependency or decision prevents work.
- `PLANNED` — specified but not yet scheduled.
- `DEFERRED` — explicitly outside the current release horizon.

| Capability | Current state | Required end state | Initial status |
|---|---|---|---|
| UTF-8 token spans | Simple borrowed word/number/punctuation tokens | UAX-style, language-aware, non-destructive, streaming/caller-buffered | `READY` |
| Sentence splitting | `. ! ?` and newline rules | Numeric/abbreviation/quote/list/Unicode-aware segmentation | `READY` |
| Normalization | ISO-shaped date and selected units | Calendar-valid TIMEX3-style time and full governed quantity/currency layer | `READY` |
| Gazetteer | Aho-Corasick over seven static entries | Compiled versioned ontology gazetteers, aliases, priorities, ambiguity | `READY` |
| Morphology | Exact trie and six suffix rules | Evaluated dictionary/rule or finite-state analyses with language packs, ambiguity, irregulars, POS-conditioned lemmas | `PLANNED` |
| POS/morph tags | Absent | UPOS/XPOS/Universal Features with confidence | `PLANNED` |
| Dependency syntax | Absent | Bounded parser with LAS/UAS evidence | `PLANNED` |
| Constituency | Absent | Optional profile; only if a validated consumer requires it | `DEFERRED` |
| NER | Static gazetteer matching only | General + domain NER, nested entities where needed | `PLANNED` |
| Entity linking | Known-IRI filter | Candidate generation, ontology constraints, disambiguation, NIL decisions | `PLANNED` |
| Coreference | Exact string + small gendered-name heuristic | End-to-end mention/coreference with bounded candidate generation | `READY` |
| Frame semantics | Two literal triggers | Frame inventory, lexical units, semantic-role spans and calibration | `PLANNED` |
| Relation extraction | Three surface patterns | Sentence-aware OpenIE + ontology relation mapping | `PLANNED` |
| GraphRAG | Full scan with token overlap | Hybrid lexical/vector/graph retrieval and evidence paths | `PLANNED` |
| Language ID | Absent | Bounded document/span language ID with explicit unknown result | `PLANNED` |
| Evaluation | Happy-path unit tests | Frozen corpora, standard scorers, differential/performance gates | `READY` |
| WASM | Current minimal target compiles | Quality-tested deterministic WASM profiles with size/memory budgets | `READY` |
| Public capability truth | Names overstate some behavior | Versioned capability descriptors with maturity and evidence | `READY` |

Initial statuses describe readiness to work on a gap. They do not certify existing quality. The initial audit covered `src/nlp/` and its direct host paths; generic ML, LLM tokenizers, research sentiment, document decoding, and broader retrieval libraries require the reuse inventory in NLP-009 before any claim that the repository lacks those capabilities.

### 4.1 Scope and requirements coverage

The working first delivery is English text processing, with Australian English measurement/date examples. Locale and document time must be supplied explicitly where ambiguity matters. General-language evaluation and application-domain evaluation have separate scorecards. This default permits useful work now; NLP-008 records supported languages, use cases and deployable profiles before release gates are frozen.

| Requirement | Owning packages | Scope boundary / evidence |
|---|---|---|
| Source spans, Unicode, document structure | NLP-100..107 | Input is extracted UTF-8 text; preserve optional paragraph/section/page offsets supplied by ingestion |
| Quantities, time, morphology | NLP-200..206 | Declared unit/date/language coverage; “full QUDT” or “full TIMEX3” requires a tested inventory |
| Syntax and learned component lifecycle | NLP-300..306 | Gold-input and end-to-end metrics; trained artifact provenance |
| Entity recognition, linking and geographic disambiguation | NLP-400..406, NLP-806 | Geo parser from the earlier project backlog is explicitly restored |
| Reference resolution | NLP-001, NLP-500..505 | Candidate recall, mention/link quality, abstention, document boundaries |
| Roles, claims, attribution, polarity and time | NLP-600..606 | Quoted, conditional, denied and hypothetical statements remain qualified claims |
| Graph-grounded retrieval | NLP-700..707 | Graph paths must affect retrieval; vector component evaluated by ablation |
| Inspection by document/IRI and correction | NLP-107, NLP-902, NLP-903 | Inspect typed annotations and evidence; human corrections retain history |
| Multilingual and governed domains | NLP-800..806 | Separate language/domain cards; specialized profiles do not certify clinical use |
| Sentiment, text classification, NLI, RST/discourse, constituency | NLP-008, NLP-009 | Inventory existing code and assess demand; explicitly deferred extensions unless a release charter promotes them with IDs and gates |
| Translation, speech recognition and OCR/PDF decoding | NLP-008, NLP-902 | Adjacent systems, outside this programme's default implementation scope; integration contract only |

Do not equate a complete extraction pipeline with every NLP task. A promoted extension needs its own dataset, resource proof, owner and work package; no untracked “complete NLP” promise.

---

## 5. Target architecture

### 5.1 Processing flow

```text
Input bytes + document metadata
        |
        v
Boundary layer: validate -> Unicode normalization view -> language/profile selection
        |
        v
Lexical layer: sentences -> tokens -> multi-word tokens -> morphology/lemma/POS
        |
        v
Syntactic layer: dependency arcs -> chunks/clauses -> optional constituency
        |
        v
Mention layer: entities -> ontology candidates -> links/NIL -> coreference
        |
        v
Semantic layer: negation/modality/time -> roles/frames -> OpenIE relations
        |
        v
Graph layer: annotation plans -> validated NQuins -> hybrid retrieval index
        |
        v
Receipts: source hash + spans + component versions + confidence/calibration + policy verdict
```

Each layer consumes a versioned annotation view and declares prerequisites. A downstream layer cannot silently infer that a missing prerequisite succeeded.

This is a dependency overview, not a mandatory linear invocation order. Language ID can run on the raw text; morphology candidate generation precedes POS, while POS-conditioned lemma selection follows tagging. Model NER need not depend on a dependency parser. Components run only when requested and prerequisites are available; bounded second passes are explicit and cannot become recursive feedback loops. NLP-107 delivers the first runnable profile and shared annotations early, with NLP-901 completing integration later.

### 5.2 Runtime profiles

| Profile | Purpose | Required components | Resource posture |
|---|---|---|---|
| `symbolic-lite` | Small WASM/offline deterministic extraction | boundaries, tokenizer, gazetteer, quantities, rules | No learned weights; tight caller buffers |
| `linguistic-standard` | Competitive general document analysis | language ID, lexical, POS/morph, dependencies, NER/linking | Quantized/local weights permitted |
| `semantic-graph` | QualiaDB knowledge extraction | standard + coref, time/modality, frames/OpenIE, graph validation | Native first; bounded WASM subset |
| `domain-clinical` | Governed health-document assistance | domain NER/linking/relations, strict evidence labels | Never upgrades NLP evidence to signed clinical evidence |
| `retrieval` | GraphRAG indexing/query | hybrid retrieval, graph expansion, reranking, receipts | Persistent Tier-2 index; bounded query kernels |

### 5.3 Core data contracts

The programme will introduce versioned contracts rather than extending unrelated structs indefinitely.

Minimum transient annotations:

- `DocumentView`: source hash, language candidates, profile, component receipt IDs.
- `SentenceSpan`: UTF-8 span, boundary reason, confidence/method.
- `TokenView`: UTF-8 span, normalized view reference, token kind, sentence ID.
- `MorphAnalysis`: lemma hash/string-table reference, UPOS, XPOS, feature bits, confidence.
- `DependencyArc`: dependent token, head token/root, relation hash, confidence.
- `Mention`: span set, head token, mention type, entity type candidates.
- `EntityLink`: mention, candidate IRI hash, score, NIL/ambiguous state, evidence.
- `CorefLink`/`CorefChain`: mention IDs, antecedent/link score, sieve/model receipt.
- `SemanticRole`: predicate token/span, frame/relation hash, role hash, filler spans.
- `ExtractedAssertion`: subject/object mention or literal, predicate IRI, confidence, polarity, modality, time, provenance.
- `RetrievalHit`: assertion/graph IDs, lexical/vector/graph sub-scores, combined score, evidence path.

Hot-path structs must be `repr(C)` where they cross ABI boundaries, use fixed-width fields, avoid owned strings, and expose caller-buffered functions. Cold convenience adapters may materialize owned API responses.

Contract requirements for NLP-100:

- Spans are half-open byte intervals in the immutable original source. Preserve a reversible alignment map for normalization expansions/contractions, ligatures, combining marks and model subtokens; never reuse normalized offsets as original offsets. Rust byte offsets, browser UTF-16 offsets, code points, graphemes and model-token indices are distinct types.
- Keep orthographic tokens, multi-word-token expansions and model subtokens distinct. Every model processor publishes its alignment policy. Nested/discontinuous mentions use bounded span lists rather than a universal non-overlap restriction.
- Typed states distinguish `complete`, `partial`, `unsupported`, `not_requested`, `budget_exceeded`, `cancelled` and `invalid_input`. An empty result from successful analysis differs from an unavailable processor. Candidate rank scores and rule weights are not probabilities without calibration.
- Caller-buffer APIs define required capacities, output initialization, retry behavior and output state after an error. Default overflow is a typed error with no partial graph mutation. Explicit partial analysis includes coverage ranges and dropped-item counts.
- Within-document mention IDs are not stable person DIDs. Repeated names do not establish identity. Entity-link NIL results remain available when the graph is incomplete.
- Use existing frame layout and text-span representations. Represent rich annotations by linked Quins or references; do not squeeze all metadata into one 48-byte record or allocate overlapping opcode/type bits.
- Check original strings on hash collisions. FNV/q_hash identifies graph terms; use an existing cryptographic digest such as SHA-256/BLAKE3 for artifact integrity and reproducibility receipts. No authorization decision relies only on a 60-bit hash.

### 5.4 Component receipts

Every analysis run records:

- source hash and optional document DID;
- runtime profile and language pack;
- component IDs, semantic versions, model/content hashes;
- parameter/calibration-set hashes;
- resource budget requested and consumed;
- deterministic seed/mode where applicable;
- warnings, truncation, overflow, or degraded-mode decisions;
- output counts and validation status.

Receipts must be serializable without embedding source text and must not leak sanctuary content.

Separate deterministic semantic payloads from runtime timestamps, elapsed-time counters and signatures. Byte-identical semantic output does not imply identical wall-clock receipts. Document digests can identify sensitive text by guessing, so retain and expose them only within the same authorized scope as the source. Include document revision, graph snapshot, ontology/schema version and policy context in cache keys. Cancellation, correction, revocation and model changes invalidate relevant derived artifacts.

---

## 6. Quality and comparison gates

### 6.1 Benchmark policy

Reference versions, model packages, hardware, datasets, and scorer versions must be frozen in a benchmark manifest. Stanford/Stanza/spaCy results are generated locally from the same original inputs, with normalization/alignment policies recorded; marketing pages are not performance evidence. Reference disagreement is an error-analysis signal, not ground truth. Score against adjudicated held-out annotations using each task's official scorer.

Every metric report includes:

- language/domain and dataset split;
- exact-match and partial-match definitions;
- micro/macro averaging policy;
- confidence intervals or bootstrap intervals where suitable;
- latency p50/p95/p99, throughput, peak working memory, model size, startup time;
- failures, skipped documents, truncation, and unsupported constructions;
- native and WASM profile differences.

### 6.2 Dataset-specific acceptance gates

Revision 1's universal F1/accuracy thresholds are retired. Their difficulty varies by dataset, language, label scheme, gold versus predicted upstream annotations, and resource profile. NLP-008/NLP-005/NLP-006 must populate and freeze numeric gates before candidate selection or tuning. An unset required threshold is `UNEVALUATED`, never passing.

| Task | Candidate evaluation data; license/version checked in NLP-005 | Required metrics and conditions |
|---|---|---|
| Token/sentence boundaries | Pinned Unicode conformance files; UD English EWT/GUM; authored adversarial/domain text | 100% declared Unicode conformance cases; boundary F1 by corpus; document exact match; source alignment validity |
| Language ID | Held-out supported languages, mixed scripts, unknown languages and short text | Macro F1 by length/language; unknown precision/recall and coverage |
| Lemmas/POS/dependencies | Pinned UD treebanks with official scorer | Lemma/UPOS/UFeats plus LAS/UAS; separate gold-token and raw-text end-to-end results |
| NER and linking | Licensed CoNLL/OntoNotes-style sets where available; adjudicated target-domain mentions | Strict span+type P/R/F1; link accuracy plus candidate recall@k and NIL P/R/F1; no score on gold mentions presented as end-to-end quality |
| Coreference | GUM/CorefUD; licensed OntoNotes if available | CoNLL F1 and LEA; mention recall; singleton policy; document-level splits; predicted mentions primary |
| Temporal/quantity/geo | Licensed temporal/geographic sets plus independently annotated locale/unit cases | Span P/R/F1 and normalized-value exact match; false assertions, ambiguity and abstention coverage |
| Frames and SRL | Licensed FrameNet/PropBank-derived data with distinct schemes | Trigger/frame accuracy and labeled argument-span F1 separately; full pipeline plus diagnostic gold-input results |
| Relations/OpenIE | Licensed task-specific relation/OpenIE sets plus qualified-claim challenge set | P/R curves, F1, polarity/modality/attribution correctness; minimum recall at selected precision |
| Calibration | Separate calibration split followed by untouched test split | Brier score, reliability curves, ECE with bins specified, selective risk versus coverage; per-domain reports |
| Retrieval | Versioned corpus, graph snapshot, queries, relevance judgments, multi-hop and unanswerable cases | Recall@k/MRR/nDCG with k/averaging frozen; path correctness, unauthorized-hit count; lexical/vector/graph ablations |
| Provenance/determinism | All above plus cross-target fixtures | 100% valid source references in test coverage; identical canonical semantic outputs for certified deterministic profile |

Each benchmark manifest must contain `dataset_revision`, `split_hash`, `task`, `language`, `domain`, `profile`, `label_mapping`, `scorer_revision`, `primary_metric`, `minimum_score`, `baseline_score`, `noninferiority_margin`, `minimum_effect`, `confidence_method`, `coverage_floor`, `latency_budget`, `memory_budget`, and `release_required`. It also identifies the candidate/model and frozen comparator configurations. Units are explicit: percentage points versus relative percentages cannot be interchanged.

Production acceptance requires both the frozen absolute floor and the declared baseline comparison, together with resource and provenance gates. For a superiority claim, the paired confidence interval for improvement must exceed the predeclared minimum useful effect; equality supports parity only. A cheaper constrained profile can be a better deployment tradeoff while having lower linguistic accuracy, but the scorecard must show that tradeoff. “Exceptional” requires a measurable improvement on a declared dimension while satisfying the relevant quality floor, not superiority on every task.

Prevent evaluation leakage: split by document/source (and author/time when relevant), deduplicate near-identical documents, isolate train/dev/calibration/test roles, and never tune rules, retrieval fusion or thresholds on release test data. Refresh a test set after repeated inspection. Reference-only and unavailable tasks remain explicitly unscored. Tiny example suites cannot substantiate a statistical claim.

### 6.3 Resource gates

- No Tier-1 allocation in functions classified `HotZeroHeap`.
- Per-pass working memory remains below 42 MiB with explicit fail-closed accounting.
- No unbounded document, mention, candidate, graph-hop, beam, or result count.
- Pathological input completes within its configured work budget or returns a typed budget error.
- `symbolic-lite` WASM bundle growth is measured and approved per milestone.
- Repeated runs do not rebuild immutable gazetteers/models/indexes unless their content hash changes.

Pinned starting references: [Unicode UAX #29 revision 47](https://www.unicode.org/reports/tr29/tr29-47.html) defines default boundaries and documented tailoring; it does not itself provide all language-specific tokenization. [UD evaluation](https://universaldependencies.org/conll18/evaluation.html) distinguishes token/word alignment and end-to-end syntactic scoring. Pin the actual data and scorer revisions during NLP-005; these links are methodological references, not benchmark results.

---

## 7. Programme tracker

### 7.1 Dashboard

Update this table whenever a work package changes state.

| Milestone | Theme | Done / total | Status | Exit evidence |
|---|---|---:|---|---|
| M0 | Truth, safety, and measurement | 1 / 10 | `IN_PROGRESS` | Immediate resource defects closed; evaluation and feasibility receipts |
| M1 | Unicode text foundation | 0 / 8 | `PLANNED` | Token/sentence gates, first runnable pipeline and streaming parity |
| M2 | Normalization and morphology | 0 / 7 | `PLANNED` | Temporal/quantity/lemma gates pass |
| M3 | POS and syntax | 0 / 7 | `PLANNED` | POS/LAS/UAS gates and artifact reproduction pass |
| M4 | Entities and ontology linking | 0 / 7 | `PLANNED` | NER/linking/calibration gates pass |
| M5 | Coreference | 0 / 6 | `PLANNED` | CoNLL/LEA and resource gates pass |
| M6 | Frames, roles, and relations | 0 / 7 | `PLANNED` | SRL/frame/OpenIE gates pass |
| M7 | Real GraphRAG | 0 / 8 | `PLANNED` | Retrieval/faithfulness/resource gates pass |
| M8 | Multilingual and domain packs | 0 / 7 | `PLANNED` | Per-language/domain/geo release cards pass |
| M9 | Productization and exceptional release | 0 / 8 | `PLANNED` | Full release gate and comparison report pass |
| **Total** |  | **1 / 75** | `IN_PROGRESS` | Package counts are not effort or linguistic completeness percentages |

### 7.2 Work packages

#### M0 — Truth, safety, and measurement

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-000 | `DONE` | P0 | — | Baseline code review, focused test run, WASM check, gap inventory | Codex review 2026-09-14 | 62 tests pass; WASM check pass; this plan |
| NLP-001 | `IN_PROGRESS` | P0 | NLP-000 | Bound existing coreference with collision-safe grouping, preflight validation and explicit errors; preserve linguistic behavior except documented defects; §7.3 contract | Grok 2026-09-14 · review: self-verify then independent | — |
| NLP-002 | `READY` | P0 | NLP-000 | Correct decimal sentence boundaries, Unicode terminators, degree-unit spans, compound-unit precedence, calendar validity | — | — |
| NLP-003 | `READY` | P0 | NLP-000 | Capability maturity descriptors; label keyword retrieval and rule demos honestly in APIs/UI | — | — |
| NLP-004 | `READY` | P0 | NLP-000 | Evaluation crate/module, benchmark manifest schema, JSON/Markdown receipts, frozen seed policy | — | — |
| NLP-005 | `PLANNED` | P0 | NLP-004 | Initial general/domain/adversarial corpus with licensing and split manifests | — | — |
| NLP-006 | `PLANNED` | P0 | NLP-004, NLP-005, NLP-008 | Frozen local reference runners, gold scoring, numeric gate manifests and reproducible baseline reports | — | — |
| NLP-007 | `PLANNED` | P1 | NLP-001, NLP-004 | Per-component resource budgets, cancellation, allocation and 42 MiB gates | — | — |
| NLP-008 | `READY` | P0 | NLP-000 | Release charter: first use cases/languages/profiles, evaluation inventory, resource envelope and scope decisions; §8.1 | — | — |
| NLP-009 | `PLANNED` | P0 | NLP-008 | Reuse inventory and measured feasibility of candidate runtimes/models, with licensing, resident bytes and quality tradeoffs; §8.2 | — | — |

M0 exit criteria:

- Existing NLP entry points validate total work and allocation bounds before heavy processing. NLP-001 closes coreference first; NLP-007 covers remaining list/output/index construction paths.
- All known tokenizer/normalizer defects from the 2026-09-14 review have regression tests.
- Every public NLP capability reports maturity, algorithm class, version, and evidence availability.
- A benchmark command emits reproducible machine-readable results, even before quality targets are met.
- The initial release scope and candidate runtime feasibility are recorded. M0 is an exit milestone, not a requirement to finish all its rows before starting any M1 row; use row-level dependencies.

#### M1 — Unicode text foundation

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-100 | `PLANNED` | P0 | NLP-004 | Versioned annotation contracts and caller-buffered APIs | — | — |
| NLP-101 | `PLANNED` | P0 | NLP-100 | Unicode-aware, non-destructive tokenizer with exact original spans | — | — |
| NLP-102 | `PLANNED` | P0 | NLP-101 | Robust sentence segmenter: abbreviations, decimals, quotes, lists, emoji and Unicode enders | — | — |
| NLP-103 | `PLANNED` | P1 | NLP-101 | Multi-word-token and contraction layer without altering source spans | — | — |
| NLP-104 | `PLANNED` | P1 | NLP-100 | Streaming/chunked processing with overlap and stable global offsets | — | — |
| NLP-105 | `PLANNED` | P1 | NLP-101, NLP-102 | Property/fuzz/adversarial Unicode suite | — | — |
| NLP-106 | `PLANNED` | P1 | NLP-101..105 | Native/WASM differential and resource certification | — | — |
| NLP-107 | `PLANNED` | P0 | NLP-100 | Early shared-document processor graph: prerequisite resolution, borrowed tokens, explicit partial states and one host-visible symbolic profile | — | — |

#### M2 — Normalization and morphology

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-200 | `PLANNED` | P0 | NLP-102 | Calendar-valid temporal parser with explicit locale/document time/timezone, ranges and unresolved relative expressions | — | — |
| NLP-201 | `PLANNED` | P0 | NLP-200 | TIMEX3-compatible value model for dates, times, durations and sets | — | — |
| NLP-202 | `PLANNED` | P0 | NLP-101 | Longest-match units, signed/exponent/locale numbers, ranges, percentages and currency ambiguity; preserve exact decimal values | — | — |
| NLP-203 | `PLANNED` | P1 | NLP-202 | QUDT mapping, conversions and ambiguity policy | — | — |
| NLP-204 | `PLANNED` | P0 | NLP-101, NLP-100 | Real finite-state morphology interface and compiled language-pack format | — | — |
| NLP-205 | `PLANNED` | P0 | NLP-204, NLP-301 | English lemmas/irregulars/derivations with ambiguity and POS conditioning; candidate generation remains in NLP-204 | — | — |
| NLP-206 | `PLANNED` | P1 | NLP-200..205, NLP-004 | Temporal, quantity and lemma evaluation gates | — | — |

#### M3 — POS and syntax

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-300 | `PLANNED` | P0 | NLP-100, NLP-204 | UPOS/XPOS/Universal Features schemas and model interface | — | — |
| NLP-301 | `PLANNED` | P0 | NLP-300, NLP-306 | Evaluated POS/morph baseline using selected reproducible artifact | — | — |
| NLP-302 | `PLANNED` | P0 | NLP-301 | Dependency parser with bounded sentence/token limits | — | — |
| NLP-303 | `PLANNED` | P1 | NLP-302 | Noun chunks and clause boundaries derived from syntax | — | — |
| NLP-304 | `PLANNED` | P1 | NLP-300..303 | UD import/scoring and error-analysis reports | — | — |
| NLP-305 | `PLANNED` | P1 | NLP-300..304 | Quantization, native/WASM parity, latency and memory certification | — | — |
| NLP-306 | `PLANNED` | P0 | NLP-009, NLP-004, NLP-100 | Training/adaptation or upstream-artifact reproduction, tokenizer alignment, conversion parity, calibration and deployment packaging; §8.2 | — | — |

#### M4 — Entities and ontology linking

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-400 | `PLANNED` | P0 | NLP-100, NLP-101 | Caller-entry gazetteer compiler with node/pattern/byte caps, checked index widths, suffix-output deduplication and overflow tests | — | — |
| NLP-401 | `PLANNED` | P0 | NLP-400 | Alias, case/normalization, overlap, priority and boundary semantics | — | — |
| NLP-402 | `PLANNED` | P0 | NLP-100, NLP-306 | General NER with span/type confidence; selected backend declares optional POS/syntax prerequisites | — | — |
| NLP-403 | `PLANNED` | P0 | NLP-400, NLP-402 | Candidate generation across gazetteer, graph aliases and model mentions | — | — |
| NLP-404 | `PLANNED` | P0 | NLP-302, NLP-403 | Ontology-aware entity disambiguation with NIL/ambiguous result states | — | — |
| NLP-405 | `PLANNED` | P1 | NLP-404 | Nested/domain entities and sensitivity-aware candidate filtering | — | — |
| NLP-406 | `PLANNED` | P0 | NLP-402..405, NLP-004 | NER/linking/calibration benchmarks and model cards | — | — |

#### M5 — Coreference

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-500 | `PLANNED` | P0 | NLP-001, NLP-302, NLP-402 | End-to-end mention detection including nominals, pronouns and proper names | — | — |
| NLP-501 | `PLANNED` | P0 | NLP-500 | Deterministic bounded multi-sieve baseline using syntax, number, animacy and discourse | — | — |
| NLP-502 | `PLANNED` | P1 | NLP-501 | Acronym, apposition, speaker/quote, conjunction and event handling | — | — |
| NLP-503 | `PLANNED` | P0 | NLP-500 | Optional compact learned antecedent scorer with deterministic inference | — | — |
| NLP-504 | `PLANNED` | P0 | NLP-501..503, NLP-004 | OntoNotes/GUM/CorefUD scoring: CoNLL F1, LEA, mention and link diagnostics | — | — |
| NLP-505 | `PLANNED` | P0 | NLP-504 | Long-document, adversarial, bias and resource certification | — | — |

NLP-501 replaces the legacy name heuristic with contextual, evaluated agreement features and abstention. Grammatical gender, textual pronoun agreement, animacy and a person's identity are separate concepts. Do not emit personal gender/identity facts from a name or a pronoun. NLP-001 first preserves and documents legacy linking behavior so a resource fix does not silently change the linguistic contract; the heuristic remains explicitly experimental until NLP-501 replaces it.

#### M6 — Frames, roles, temporality, and relations

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-600 | `PLANNED` | P0 | NLP-302 | Predicate/clause/voice, negation scope, modality, quote/attribution and experiencer annotations | — | — |
| NLP-601 | `PLANNED` | P0 | NLP-600 | Semantic-role labeling interface and bounded baseline | — | — |
| NLP-602 | `PLANNED` | P0 | NLP-601 | Versioned frame/lexical-unit inventory with ontology mappings | — | — |
| NLP-603 | `PLANNED` | P0 | NLP-600, NLP-501 | Sentence-aware OpenIE propositions with coreference-resolved optional view | — | — |
| NLP-604 | `PLANNED` | P0 | NLP-200, NLP-603 | Temporal qualification and event/entity relation linking | — | — |
| NLP-605 | `PLANNED` | P0 | NLP-404, NLP-603, NLP-604 | Qualified claim planner with SHACL validation, open-world NIL and contradiction handling; SHACL conformance is not truth verification | — | — |
| NLP-606 | `PLANNED` | P0 | NLP-601..605, NLP-004 | Frame/SRL/OpenIE benchmark, calibration and contradiction tests | — | — |

#### M7 — Real GraphRAG

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-700 | `PLANNED` | P0 | NLP-007, NLP-100, NLP-009 | Retrieval index contract reusing graph storage; snapshot, access context, bounded updates and workspace budgets | — | — |
| NLP-701 | `PLANNED` | P0 | NLP-700 | Inverted lexical index with BM25-style scoring and field weights | — | — |
| NLP-702 | `PLANNED` | P0 | NLP-700, NLP-306 | Validated text embedding/index path, pooling/tokenizer/normalization receipts and exact-search recall oracle | — | — |
| NLP-703 | `PLANNED` | P0 | NLP-700 | Bounded graph-neighborhood expansion with relation/path policies | — | — |
| NLP-704 | `PLANNED` | P0 | NLP-701..703 | Hybrid fusion/reranking with decomposed scores and deterministic ties | — | — |
| NLP-705 | `PLANNED` | P0 | NLP-704 | Provenance, sensitivity, access-control and contradiction-aware filtering | — | — |
| NLP-706 | `PLANNED` | P0 | NLP-704, NLP-004 | Recall/MRR/nDCG and evidence-faithfulness evaluation suite | — | — |
| NLP-707 | `PLANNED` | P0 | NLP-700..706 | Incremental update, deletion/tombstone, compaction and resource certification | — | — |

Keep existing public IDs compatible and label their current semantics as experimental keyword-triple search. M7 implements this project's chosen lexical/vector/graph retrieval profile, with each component justified by ablation. Graph-augmented retrieval does not universally require embeddings; the defect in the current implementation is that graph structure does not influence retrieval at all. The retrieval API returns ranked evidence and graph paths. Answer generation, if later requested, must be separately specified and evaluated for citation support and abstention; it is not implied by an index query. See the different query approaches in [Microsoft GraphRAG](https://microsoft.github.io/graphrag/query/overview/).

#### M8 — Multilingual and governed domain packs

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-800 | `PLANNED` | P0 | NLP-008, NLP-100 | Early raw-text language/profile routing with explicit override, unknown and mixed-language states; learned artifacts gated by NLP-306 | — | — |
| NLP-801 | `PLANNED` | P0 | NLP-204, NLP-300, NLP-306 | Signed/versioned language-pack format and loader | — | — |
| NLP-802 | `PLANNED` | P1 | NLP-801 | First non-English language selected by product need and corpus availability | — | — |
| NLP-803 | `PLANNED` | P0 | NLP-402, NLP-404 | Clinical/domain pack with terminology provenance and strict evidence labels | — | — |
| NLP-804 | `PLANNED` | P1 | NLP-803 | Legal/governance domain pack aligned with deontic and temporal modalities | — | — |
| NLP-805 | `PLANNED` | P0 | NLP-800..804, NLP-806, NLP-004 | Per-pack dataset, license, bias, quality, memory and release cards | — | — |
| NLP-806 | `PLANNED` | P1 | NLP-404, NLP-203 | Geographic mention/coordinate extraction, place ambiguity, CRS/unit validation and GeoSPARQL mapping using existing spatial libraries | — | — |

Language order is chosen from actual user/product requirements and high-quality licensable corpora, not headline language count.

#### M9 — Productization and exceptional release

| ID | Status | Priority | Depends on | Deliverable and acceptance summary | Owner | Evidence |
|---|---|---|---|---|---|---|
| NLP-900 | `PLANNED` | P0 | NLP-100, NLP-003 | Versioned public API and migration compatibility suite | — | — |
| NLP-901 | `PLANNED` | P0 | NLP-107, M1..M7 | Complete profile orchestration and processor interoperability, extending the early NLP-107 pipeline | — | — |
| NLP-902 | `PLANNED` | P0 | NLP-901 | Host, daemon, Poet and document-ingest integration with capability receipts | — | — |
| NLP-903 | `PLANNED` | P0 | NLP-605, NLP-902 | NQuin emission, SHACL validation and reversible correction receipts | — | — |
| NLP-904 | `PLANNED` | P0 | M1..M8 | Security/privacy/adversarial red-team suite | — | — |
| NLP-905 | `PLANNED` | P0 | M1..M8 | Native/WASM/browser performance and reproducibility certification | — | — |
| NLP-906 | `PLANNED` | P0 | NLP-006, NLP-900..905 | Reviewable comparison report with frozen baselines and limitations; publication is a separate release action | — | — |
| NLP-907 | `PLANNED` | P0 | NLP-900..906 | Release gate, operator manual, model/data cards, rollback plan | — | — |

### 7.3 First executable slice: NLP-001 acceptance contract

**Problem:** `coref.rs` performs pairwise string normalization/comparison; `substrate.rs` creates a mention for every word; the host clones the supplied list before validating it. The immediate work is predictable resource consumption and truthful error reporting. Mention recall, gender/identity semantics and the full resolver redesign belong to NLP-500/NLP-501.

**Scope:** `src/nlp/coref.rs` (or a directory-backed replacement with its re-export), `src/nlp/substrate.rs`, direct coreference/substrate host adapters, their callers/tests, and this tracker. Read call sites before changing signatures. Keep current mention-generation and valid-input heuristic behavior during this repair; document existing defects rather than making a new detector part of a resource fix.

**Starting budgets:** 256 KiB source; 4,096 mentions; 1 MiB aggregate supplied mention text; 4,096 chains; at most 64 antecedent candidates inspected per pronoun; 65,536 total antecedent checks per call. These are engineering defaults to validate against fixtures, not measured performance claims. Reject excess before cloning/allocating; callers can request lower limits. Document changes to these values with measured resource evidence. Independently cap normalization, comparisons and emission bytes, and account for the whole caller-owned workspace under the 42 MiB ceiling.

**Algorithm:** Normalize once into bounded scratch. Deterministic sorting/indexing with O(n log n) grouping is acceptable; an average-case hash table alone does not establish an adversarial bound. Compare normalized bytes to resolve hash collisions and count byte comparisons, including long shared prefixes. Exact surface grouping preserves the legacy heuristic for this repair; it is not proof of real-world identity. Antecedent lookup must stop at the configured window/work limit. Natural lack of an antecedent yields an unresolved mention; exhausting a work budget yields a typed error, not a false “no match.”

**API behavior:** Add checked/caller-buffered entry points as needed and migrate all in-scope production callers to them. Validate source size, list length and aggregate text bytes before list cloning; use checked numeric conversions; require `0 <= start < end <= source.len()`, UTF-8 boundaries, exact source-slice/text equality and a supported kind. Reject unsorted mentions in the checked external API with an explicit error; do not confuse sorted document order with a requirement that mentions never overlap. Missing/wrongly typed required fields fail explicitly. Preserve an existing empty-mentions mode only if callers depend on it and document that it does not perform mention detection.

Do not convert an error to an empty Vec, panic, or silent truncation in legacy wrappers. If a signature must change to propagate errors, migrate its callers and record the compatibility change. Coreference-only output may include unresolved singleton chains. No checked failure can be reported as a successful full substrate. On output-buffer exhaustion or cancellation, report the error and specify that partial buffers are not consumable.

**Acceptance cases:** Existing valid exact-match examples retain grouping/order; correct invalid historical test offsets explicitly. Negative/overflowing offsets, byte-interior UTF-8 spans, reversed/out-of-range spans, text mismatch, invalid kinds and unsorted inputs fail before expensive work. Empty valid input succeeds. Source/list/byte/output limits are tested at the boundary and one past it. Repeated names, all-distinct mentions, long shared prefixes and forced hash collisions exercise bounded work. Calls at n, 2n and 4n provide normalization/comparison/candidate counters demonstrating the chosen bound; wall time is supporting evidence only. Within-budget legacy results are compared to a small reference oracle; exceeding a configured search window has the documented error/unresolved distinction.

**Resource evidence:** Measure the new hot entry point with the repository's thread-local allocation counter and caller-owned buffers. Also measure cold-adapter peak bytes and check cleanup on success, error and unwind. The declaration “Tier-2 authoring output” in old source comments is not an exemption. Keep the repair focused; the general processor/workspace framework remains NLP-007/NLP-100.

**Handoff:** Record commands, work counters, actual limits, changed call sites and unresolved linguistic issues. NLP-001 reaches `REVIEW` after self-verification. NLP-500/NLP-501 retain the mention-detection and heuristic replacement work; this slice cannot claim coreference accuracy parity.

---

## 8. Dependency and delivery sequence

Critical path:

```text
M0 safety + evaluation
    -> M1 text contracts/boundaries
        -> M2 morphology ------+
        -> M3 POS/syntax ------+-> M4 entities/linking -> M5 coreference
                                      |                    |
                                      +-------> M6 semantics
                                                    |
                                                    v
                                               M7 GraphRAG
                                                    |
M8 language/domain packs can begin after M1/M2 -----+
                                                    v
                                               M9 release
```

Milestones group ownership and exit evidence; row-level dependencies are authoritative. M7 may start against an existing validated graph before M6 extraction is finished. Language routing (NLP-800), trained artifact feasibility (NLP-009/NLP-306), and pipeline integration (NLP-107) begin early. Dependencies do not require every earlier-numbered milestone to finish. If parallel agents are explicitly authorized, allocate disjoint files after shared contracts land; this plan does not itself initiate delegation.

### 8.1 Release charter and effort planning (NLP-008)

Record two or three concrete document-to-output use cases with representative inputs, target languages/domains, maximum document size, deployed hardware/browser, sensitivity, acceptable latency and output acceptance. Defaults for initial engineering are English general text plus an independently authored measurement/catchment corpus; clinical and legal deployment remain separate later profiles. Resolve mandatory versus optional processors for each profile. A domain pack is not a medical or legal performance certification.

Estimate effort only after inventorying reuse and completing the feasibility spike. The 75 rows are capability packages, many requiring several sessions and a real data/model programme; they are not a 75-session promise. Each ready package gets a small first slice, effort range/confidence, owner, review method and evidence deliverable. Keep at most one primary implementation slice active per owner. Milestone scheduling uses remaining estimated effort, dependency blockers and data availability rather than raw checkbox counts.

### 8.2 Reuse and model lifecycle (NLP-009 / NLP-306)

First inspect existing `inference/runtime/`, `inference/semantic_skills.rs`, `inference/reranker/`, `text_span/`, graph search/storage, WordNet/GeoNames ingestion scripts, lexicon, model artifact handling and scoped access checks. Record verified APIs and gaps; adjacent existence does not prove suitability. Reuse measured capabilities instead of creating a second model loader, graph store, vocabulary registry or inference device.

Compare a small set of candidates: deterministic rules/dictionary or finite-state methods, compact statistical sequence models, and a learned encoder where feasible. A small baseline plus a quality/resource table is the NLP-009 deliverable. Benchmark-only upstream runtimes can serve as reference/teacher processes, but deployed components must meet the repository budget and licensing constraints. No provider/API dependency is assumed.

NLP-306 specifies data preprocessing, initialization or upstream checkpoint identity, training/adaptation/distillation objective, train/dev/calibration splits, seed, hardware/compute budget, stopping criteria and reproducible export. Teacher-output licenses and contamination checks apply to distillation. The existing training code's support must be verified before promising a new model family. If weights are reused unchanged, reproduce their loading/tokenizer/output behavior rather than claiming training occurred.

Deployment conversion includes tokenizer/subtoken alignment, tensor names/shapes, pooling, quantization and reference-output differential tests. Separate model conversion fidelity from NLP accuracy. Ship a manifest, checksum/signature verification, license/model card, compatible schemas, loading/eviction behavior and rollback version. Measure resident memory and latency on the actual first target. Unsupported model architectures must fail explicitly.

### Recommended first twelve implementation packets

1. NLP-001 — coreference resource repair; bounded scope in §7.3.
2. NLP-002 — regression fixes for segmentation and normalization.
3. NLP-008 — release charter and initial corpus/use-case decisions.
4. NLP-003 — truthful maturity metadata with existing IDs preserved.
5. NLP-004 — benchmark manifest/receipt scaffold; original behavior captured.
6. NLP-005 — licensed/adjudicated seed corpora and locked splits.
7. NLP-009 — reuse inventory and measured runtime/model feasibility.
8. NLP-006 — reference runners and dataset-specific numeric gates.
9. NLP-100 — annotation/error/alignment contracts.
10. NLP-107 and NLP-800 — shared pipeline and explicit language routing.
11. NLP-101/NLP-102/NLP-105 — Unicode tokenization, sentence segmentation and conformance tests.
12. NLP-400, then NLP-200/NLP-202/NLP-204 — gazetteer and lexical foundations.

No learned syntax, NER, or coreference model should be integrated before annotation contracts and evaluation receipts are stable.

---

## 9. Testing and evaluation programme

### 9.1 Test layers

| Layer | Required evidence |
|---|---|
| Unit | Boundary conditions, error types, exact spans, deterministic ordering |
| Property | Source/alignment round trips, valid spans, task-specific nesting/overlap rules, normalization idempotence where defined |
| Fuzz | Arbitrary UTF-8, hostile punctuation, malformed host arguments, extreme counts |
| Golden corpus | Versioned expected annotations with documented adjudication |
| Standard corpus | Official format importers and scorers with license manifests |
| Differential | Frozen CoreNLP/Stanza/spaCy outputs and explicit normalization policy |
| Metamorphic | Only transformations with declared expected invariants: reversible offset changes, serialization round trips and bounded chunk parity; case/punctuation can legitimately change meaning |
| Bias/fairness | Names, pronouns, dialects, demographic terms, clinical/legal sensitivity cases |
| Performance | p50/p95/p99, throughput, peak bytes, allocations, startup/model load |
| Cross-target | Native/WASM output parity and profile-specific declared differences |
| Integration | Host/daemon/Poet/schema/NQuin/SHACL end-to-end paths |

### 9.2 Dataset governance

Every dataset entry records:

- canonical source URL and retrieval date;
- license, redistribution status, attribution and checksum;
- language, domain, annotation scheme and known limitations;
- immutable train/dev/test split hashes;
- allowed use: bundled, CI-download, local-only, or external benchmark;
- PII/sensitivity assessment and deletion/retention policy;
- preprocessing script/version and whether text was modified.

No dataset or model artifact is committed merely because it is publicly downloadable.

Gold-set creation needs annotation guidelines, two annotators for a representative sample, agreement measurements and adjudication of disagreements. Keep held-out passages hidden from rule authors after freezing. Clinical/legal domain judgments require relevant domain expertise; until available, mark that domain gate blocked while continuing general-text work. Synthetic examples are regression fixtures, not a substitute for representative accuracy data.

### 9.3 Evaluation artifact layout

Proposed layout:

```text
crates/qualia-core-db/src/nlp/
  contracts/
  boundary/
  morphology/
  syntax/
  entities/
  coreference/
  semantics/
  retrieval/
  receipts/

crates/qualia-core-db/tests/nlp_suite.rs   # explicit integration-test entry point
crates/qualia-core-db/tests/nlp/
  regression/
  adversarial/
  differential/
  integration/

benchmarks/nlp/
  manifests/
  runners/
  scorers/
  reports/

docs/benchmark-datasets/nlp/
  DATASETS.md
  LICENSES.md
  MODEL_CARDS.md
```

Large or generated benchmark output must live in an explicitly selected artifact directory, not the repository root.

Cargo does not automatically discover arbitrary Rust files nested under `tests/nlp/`. The `nlp_suite.rs` entry point must declare those modules, or register an explicit test target. NLP-004 should provide a tiny authored smoke corpus that runs offline in ordinary CI; external-data tests list `unavailable` with a reason and cannot pass the corresponding release gate.

### 9.4 Verification commands and evidence limits

For implementation changes, verify against the actual current checkout:

```powershell
cargo test -p qualia-core-db --lib nlp::
cargo test -p qualia-core-db --lib
cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-ontology
cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features portal
pwsh -NoProfile -File scripts/validate-nlp-plan.ps1
```

The full library run is the repository handoff requirement; report unrelated failures without overwriting other work. `wasm-ontology` includes the NLP Rust module but excludes the Poet/Vibe host in `lib.rs`; `portal` checks the integrated host compile path. Neither `cargo check` executes a browser. NLP-106/NLP-905 must add and run an explicit browser test harness with export, UTF-16/UTF-8 translation, invocation and output parity tests before browser support is certified. Format touched Rust files and inspect the diff; avoid unrelated workspace formatting changes. Capture source revision, dirty-file scope and actual command exit status in each new receipt. Do not reuse the historical 62-test count as current evidence.

---

## 10. API and compatibility policy

1. Existing functions remain available during migration, but maturity is explicit.
2. Correctness fixes that change output require golden migration notes.
3. New annotation schemas carry a version and do not expose internal model tensor shapes.
4. Host APIs validate total input size, list counts, span ordering/bounds, result limits, graph hops, and work budgets before allocation-heavy processing.
5. Negative integers must never be silently cast to unsigned spans/counts.
6. Add the profile-driven analysis response through an explicit version/options path, retaining the current counts-only contract for existing consumers. Inspect results by document/annotation IDs without reparsing the source.
7. Implement a compatible gazetteer build contract for supplied entries, with a bounded scoped handle and content/version receipt; preserve the metadata-only call while it has callers. Labels must distinguish inspection from construction.
8. Preserve the existing GraphRAG invoke ID and response shape during migration, label its actual keyword behavior, and expose richer graph/path evidence through a versioned response. Do not rename capability IDs without a caller/migration inventory.
9. Unsupported language/task/profile combinations return explicit capability errors.
10. Every response reports truncation/degradation instead of silently dropping candidates.

---

## 11. Performance design

### 11.1 Immediate corrections

- Replace pairwise coreference exact matching with precomputed normalized keys and stable indexing.
- Restrict antecedent candidates by sentences/tokens and a caller-visible budget.
- Bound the existing mention list in NLP-001; replace its overly broad detector with measured mention recall in NLP-500.
- Compile the default gazetteer once per content hash, not once per request.
- Tokenize once per orchestrated document and pass shared annotations downstream.
- Use the GraphRAG inverted index rather than rescanning and retokenizing every triple.
- Cap all host lists and `k` values before cloning input values.

Interpret the existing 256 KiB cap as a transport bound, not a processing-time guarantee. Inspect gazetteer node/pattern `u16` casts and inherited failure outputs in NLP-400: large lexicons must return a budget error before index wraparound, and inherited outputs must not be counted both at construction and again along failure walks. A faster implementation must keep documented overlapping-match semantics.

### 11.2 Budget model

Every operation receives or derives:

- maximum source bytes;
- maximum sentences/tokens/mentions/candidates/assertions;
- maximum sentence length for learned syntax;
- maximum graph hops/frontier/results;
- maximum workspace bytes;
- cancellation token/work counter;
- overflow behavior (`error`, explicit `truncated`, or profile downgrade).

Defaults are conservative and endpoint-specific. A 256 KiB text limit alone is not a sufficient complexity bound.

---

## 12. Security, privacy, and harm controls

- Treat input text, gazetteer entries, graph labels, model metadata and retrieved passages as untrusted data, never instructions.
- Prevent catastrophic regex/backtracking and algorithmic-complexity attacks.
- Validate UTF-8 spans against source boundaries before slicing or emitting.
- Apply sensitivity/access filters before retrieval scoring and before result materialization.
- Apply authorization during graph traversal, cache lookup and vector candidate filtering; test that inaccessible nodes do not influence result counts, scores or path explanations. Document unavoidable shared-index timing effects rather than promising absence of all side channels.
- Keep source text out of telemetry and receipts unless the user explicitly chooses a secure artifact.
- Require explicit model/data integrity checks before learned inference.
- Preserve uncertainty and provenance through entity linking and relation emission.
- For health/legal uses, distinguish extracted, inferred, user-entered, signed, and clinician-observed evidence.
- Evaluate demographic, dialect, disability, gender, cultural and multilingual harms; unknown is a valid outcome.
- Never derive or assert protected traits from names or pronouns without an explicitly justified, reviewed requirement.

In NLP-600..606, retain speaker/source attribution, experiencer, polarity, hypothetical/conditional status and time. Fixtures such as “The report denies X,” “If X were true,” quoted false claims and family-history mentions must not become unqualified facts about the subject. SHACL validation verifies a shape, not factual truth; paraconsistent reasoning does not repair an extraction that lost negation. Human acceptance/correction is a separate graph action with provenance and idempotency keys.

---

## 13. Risk register

| Risk ID | Risk | Likelihood / impact | Mitigation | Status |
|---|---|---|---|---|
| R-01 | Feature count advances faster than measured quality | High / Critical | M0 evaluation gate; maturity metadata; evidence-required `DONE` | Open |
| R-02 | Learned models breach WASM/42 MiB constraints | High / High | Separate profiles, quantization, mmap weights, bounded workspace, fail closed | Open |
| R-03 | Rule-only approach plateaus far below reference accuracy | High / High | Hybrid architecture; use rules for constraints/provenance, learned models for ambiguity | Open |
| R-04 | Model-first work hardens unstable annotation schemas | Medium / High | Land NLP-100 before model integrations | Open |
| R-05 | Dataset/model licensing prevents distribution | Medium / High | Dataset/model manifests and legal status before adoption | Open |
| R-06 | Domain performance hides poor general-language behavior | High / Medium | Separate general and domain scorecards; prohibit aggregate claims | Open |
| R-07 | General benchmarks hide clinical/legal safety failures | High / Critical | Governed domain corpora, calibration and provenance gates | Open |
| R-08 | Cross-target numerical differences break determinism | Medium / High | Deterministic profile, fixed reductions, native/WASM differential suite | Open |
| R-09 | GraphRAG leaks restricted graph neighborhoods | Medium / Critical | Access filtering before expansion/scoring; adversarial tests | Open |
| R-10 | Long documents trigger time/memory denial of service | High / High | Work counters, candidate windows, bounded chunks, typed truncation | Open |
| R-11 | Confidence fields imply calibration without evidence | High / High | Remove constants or label heuristic; require calibration receipts | Open |
| R-12 | Reference-version drift invalidates superiority claims | Medium / Medium | Freeze manifests and date every comparison; rerun at release | Open |
| R-13 | Release threshold tuning leaks test labels | High / High | Locked splits; separate calibration; paired comparisons and minimum coverage | Open |
| R-14 | A resource fix silently changes linguistic behavior | Medium / High | NLP-001 bounded contract and legacy oracle; detector/model changes in M5 | Open |
| R-15 | Feature count or compilation is reported as production readiness | High / High | Package/maturity/quality evidence tracked separately; actual browser execution | Open |

---

## 14. Tracking protocol

Every implementation session must update this document as part of the same change.

### 14.1 Starting a work package

1. Confirm dependencies and read all referenced contracts.
2. Change exactly one primary work package from `READY`/`PLANNED` to `IN_PROGRESS`.
3. Add owner/agent and start date to its row.
4. Record any scope adjustment in the decision log before coding.
5. Do not claim adjacent work packages merely because supporting code was touched.

Priority `P0` in this tracker means required before the applicable release gate; it is not automatically a security severity or permission to preempt the first slice. `P1` is sequenced improvement. Current immediate work is NLP-001/NLP-002. Use the numbered sequence and explicit dependencies to schedule the other required packages.

### 14.2 Completing a work package

1. Run its focused tests and all named cross-cutting gates.
2. Record exact commands, pass/fail counts, relevant artifact paths and commit in Evidence.
3. Add a progress-log entry describing behavior, limitations and compatibility changes.
4. Change status to `REVIEW`; an independent review or explicit owner acceptance changes it to `DONE`.
5. Update milestone and total dashboard counts.
6. Update risks and decisions affected by the work.

Choose the review method when claiming the packet. `REVIEW` requires evidence and a named reviewer/next action; it must not become an indefinite parking state. Review need not require a new user confirmation for each routine edit: use an already authorized review process, or report precisely what acceptance remains. Creating this plan does not dispatch review agents. A `DONE` package can reopen when its behavior regresses; its prior receipt stays in history. Record `uncommitted` with a patch/source digest when no commit exists; this workflow does not require a push to attach evidence.

### 14.3 Blocking a work package

Set `BLOCKED` only with:

- the exact missing dependency/decision;
- checks already performed;
- the smallest user/owner action needed;
- any safe work that can continue independently.

### 14.4 Evidence format

Use compact entries such as:

```text
2026-09-14 · NLP-001 · REVIEW · commit <sha>
Tests: cargo test -p qualia-core-db --lib nlp::coref (N passed)
Adversarial: <command> (<N> passed; max time/memory ...)
WASM: <command> (pass)
Limits: <documented values>
Known limitations: <explicit list>
```

### 14.5 Status claim rule

Presence of a function, invoke binding, UI button, or passing happy-path test is not proof that a linguistic capability is complete. `DONE` means its stated acceptance criteria and evidence gates passed.

### 14.6 Slice record and automated consistency check

For any package beyond the fully specified NLP-001, add this record before implementation:

```text
Package/slice: NLP-xxx / <bounded slice name>
Owner/start/reviewer: <owner> / <date> / <review method>
Allowed files and verified reuse: <paths/APIs>
Inputs, outputs, limits, error and compatibility behavior: <contract>
Acceptance cases and dataset/metric gate: <positive/negative cases>
Verification commands and artifact retention: <exact commands/paths>
Estimate range/confidence: <after inspection; no fixed session promise>
Evidence: <source revision, command exits, receipts>
Next action / blocker: <concrete step>
```

Run `pwsh -NoProfile -File scripts/validate-nlp-plan.ps1` after editing. Use the available PowerShell 7 runtime; no execution-policy changes are needed. The read-only validator checks row shape, unique IDs, legal statuses, dependency references/ranges, cycles, per-milestone totals, completion counts, owners for active/reviewed/completed rows, and evidence for reviewed/completed rows. `READY`/active/completed rows require completed prerequisites. It validates structure, not the truth of a receipt or benchmark score. `-SelfTest` exercises rejected malformed plans in memory. Keep Markdown as the only status source to avoid a second stale spreadsheet/JSON tracker.

---

## 15. Milestone release gates

### Alpha: safe symbolic substrate

Requires M0 and M1, plus NLP-200/NLP-202/NLP-400.

- Robust boundaries and exact spans.
- Bounded behavior under adversarial documents.
- Compiled caller-supplied gazetteers.
- Honest maturity and evaluation receipts.
- Native/WASM deterministic parity.

### Beta: competitive linguistic pipeline

Requires M2–M5.

- Lemma/POS/dependency/NER/linking/coreference quality gates.
- Frozen reference comparison.
- Per-language model/data cards.
- No critical resource, privacy or bias findings.

### Semantic release

Requires M6 and M7.

- Calibrated frames/OpenIE and ontology-constrained assertion plans.
- Real hybrid GraphRAG with evidence paths.
- Graph access-control and contradiction tests.
- End-to-end document-to-validated-NQuin receipts.

### Exceptional release

Requires M8 and M9.

- At least one general language and one governed domain meet their exceptional gates.
- QualiaDB advantages are measured, not asserted.
- Published limitations, comparison report, reproducibility bundle and rollback path.
- Product owner explicitly approves each external comparative claim.

---

## 16. Decision log

| Date | Decision | Reason | Consequence |
|---|---|---|---|
| 2026-09-14 | Evaluate a hybrid symbolic + compact learned architecture | Different components need different quality/resource tradeoffs; algorithm choice needs evidence | NLP-009 compares candidates; symbolic-lite remains supported and no unmeasured model is mandated |
| 2026-09-14 | Treat “better than Stanford” as task-specific evidence | Toolkits differ in breadth, runtime and models; an aggregate claim is not falsifiable | Comparisons freeze task, dataset, metric, version and resource envelope |
| 2026-09-14 | Put safety/measurement before feature expansion | Current quadratic coreference and missing evaluation invalidate production-quality claims | Row-level contract/measurement/feasibility prerequisites gate learned components; M0 exit does not serialize all foundational work |
| 2026-09-14 | GraphRAG contract corrected in revision 2 | Graph structure must affect retrieval; embeddings are a project design choice, not a universal definition | Preserve invoke IDs, expose truthful maturity, measure lexical/vector/graph ablations |
| 2026-09-14 | Stage the legacy name-heuristic replacement in M5 | The first resource repair should not silently remove existing pronoun behavior | NLP-001 documents experimental behavior; NLP-501 replaces it with contextual agreement and abstention |
| 2026-09-14 | Freeze dataset-specific thresholds and measure feasibility early | Universal F1 targets and exempting mapped weights would create unsupported release promises | NLP-008/NLP-009/NLP-306 added; release claims require resource and quality evidence |
| 2026-09-14 | Add an early pipeline and explicit geo/training work | Late integration and missing deliverables would hide major effort | NLP-107, NLP-306, NLP-806; 75 packages with an acyclic validated tracker |

---

## 17. Progress log

| Date | Work package | Status | Summary | Evidence/limitations |
|---|---|---|---|---|
| 2026-09-14 | NLP-000 | `DONE` | Audited current NLP modules, host wiring, tests, WASM compilation and comparison surface | 62 focused tests pass; minimal WASM check passes; no corpus quality metrics found |
| 2026-09-14 | Programme plan | Active | Established 70 tracked work packages, gates, architecture, metrics, risks and start prompt | Planning artifact only; no NLP behavior changed |
| 2026-09-14 | Programme revision 2 | Active | Corrected benchmark, memory, dependency and compatibility assumptions; added five packages and tracker validator; narrowed first slice | Validator: 75 unique packages, 1 DONE, 10 milestones, acyclic; 2 valid and 10 rejected malformed self-test fixtures. Planning/tooling only; historical Rust/WASM results not rerun |
| 2026-09-14 | NLP-001 | `IN_PROGRESS` | Started bounded coreference resource repair per §7.3 | Owner Grok; review method: self-verify then independent; no other packages claimed |

---

## 18. First implementation prompt

Copy the prompt below into an implementation task. The exact acceptance contract is §7.3; the older starter prompt is superseded by this revision.

```text
Implement work package NLP-001 from
docs/work-in-progress/NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md.

Objective:
Complete the bounded coreference resource repair specified in section 7.3.

Before editing:
1. Read the repository AGENTS.md completely and obey its zero-heap, 42 MiB,
   deterministic, module-size, temporary-artifact, and honesty constraints.
2. Read the NLP programme plan, especially sections 7.3, 9.4 and 14, and claim
   NLP-001 as IN_PROGRESS with owner/date and a review method. Check dependencies.
3. Inspect the current nlp/coref.rs, nlp/substrate.rs, related host bindings, tests,
   and any shared allocation/resource-budget patterns already used by qualia-core-db.
4. Preserve unrelated user changes. Do not create a new public capability family.

Required implementation:
- Precompute normalized keys and use deterministic bounded grouping. O(n log n)
  sorting is acceptable. Resolve hash collisions by comparing bytes and account for
  normalization/comparison work, including long shared prefixes.
- Enforce the source, mention, text-byte, candidate, chain and workspace budgets in
  section 7.3 before cloning input. Use caller-owned scratch/output for hot processing.
- Bound antecedent work and propagate typed budget/cancellation/output-capacity errors
  through the host and full substrate. Never convert errors into empty successful output.
- Validate host-provided mention spans: non-negative, ordered, within the UTF-8 source,
  on character boundaries, and consistent with the supplied mention text. Reject unknown
  mention kinds rather than coercing them to Common.
- Reject unsorted external mentions and incorrect/missing required field types. Preserve
  the existing documented empty-list mode only if callers require it.
- Preserve current mention detection and valid within-budget heuristic behavior in this
  resource repair. Do not add a new linguistic detector or silently remove name-based
  pronoun links. Document those experimental limitations for NLP-500/NLP-501. Do not
  emit identity/gender facts. Keep deterministic grouping and output ordering.
- Add checked APIs and migrate all production callers. If a signature change is needed
  for error propagation, update callers and document it. A Vec-returning wrapper must
  not hide failure. Do not classify query work as cold to bypass zero-heap requirements.

Tests and evidence:
- Compare valid legacy exact-match and pronoun examples to a small oracle; correct
  invalid historical span offsets instead of weakening span validation.
- Test empty input, negative/overflowed/reversed/out-of-range spans, text mismatch,
  Unicode interiors, unsorted inputs, unknown kinds, every budget boundary and one past
  it, output exhaustion, cancellation, repeated/distinct mentions and forced collisions.
- Add a test or benchmark proving the exact-match stage no longer performs quadratic
  pair comparisons. Prefer an instrumented work counter over flaky wall-clock assertions.
- Measure allocations for any Tier-1 function and exercise the configured resource limits.
- Format touched files, run the focused NLP/host tests and full library handoff suite,
  then the wasm-ontology and portal checks listed in section 9.4. Record unrelated
  failures without editing other work. Compilation alone does not certify browser runtime.

Tracking and handoff:
- When implementation and self-tests pass, change it to REVIEW, not DONE.
- Update the dashboard counts, Evidence cell, progress log, risk register, and decision
  log if behavior or contracts changed.
- Report exact files changed, commands/results, performance/work-count evidence, remaining
  limitations, and any follow-on work. Do not claim general coreference completeness;
  that belongs to M5 and its CoNLL/LEA gates.
- Run pwsh -NoProfile -File scripts/validate-nlp-plan.ps1 and provide the concrete
  next review action. Keep the scope to NLP-001; do not mark other packages complete.
```

---

## 19. Change log

| Date | Change |
|---|---|
| 2026-09-14 | Initial programme created from the NLP completeness review. Added architecture, quality targets, 70 work packages, tracking protocol, risk/decision/progress logs, release gates, and NLP-001 start prompt. |
| 2026-09-14 | Revision 2: replaced ungrounded universal thresholds with frozen dataset-specific gates; corrected graph dependencies, memory accounting, GraphRAG terminology and API compatibility; added early integration, model lifecycle, release charter, reuse feasibility and geographic extraction; 75 packages; revised starter prompt and added a read-only tracker validator. |
