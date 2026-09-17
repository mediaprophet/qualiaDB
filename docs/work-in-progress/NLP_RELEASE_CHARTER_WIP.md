# NLP Release Charter (NLP-008)

**Status:** `REVIEW` (reopened 2026-09-14 after Swarm-R8; supervisor patched hardware class, “certified” wording, and NLP-002-current processor text) · **Package:** NLP-008 · **Date:** 2026-09-14  
**Owner (implementer):** Swarm-C · **Review:** independent (supervisor / named reviewer), not self-DONE  
**Normative programme:** [`NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md`](./NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md) §§1–3, 5.2, 6.2, 8.1, 8.2, 9.2, 11, 15  
**Allowed artifact:** this file only. Does not authorize Rust edits, model download, dataset download, or comparative publication.

This charter freezes the **first engineering profile**, two **selected Alpha-target** document-to-output use cases, one experimental retrieval use case, processor obligations, evaluation inventory, resource envelope, and in/out scope. A product owner can schedule **NLP-005** (seed corpora) and **NLP-009** (reuse/feasibility) from this document without inventing the first language, domain, or hardware class.

It is **not** NLP-009. It does not choose a learned runtime, train a model, or prove 42 MiB feasibility for weights. Selecting a profile here is **not** Alpha certification.

---

## 0. How to read this document

| Claim class | Meaning here |
|---|---|
| **Selected Alpha target** | First scorecard: English `symbolic-lite` on UC-A and UC-B. Still requires later package evidence (M0/M1 + NLP-200/202/400 per §15 Alpha). **Not** a certification that those gates have passed. |
| **Engineering default** | Bound or SLA used to implement now; not a measured performance receipt. |
| **Experimental / demo** | Invocable in the tree; must not be sold as Stanford/Stanza/spaCy parity or as a release gate. |
| **UNEVALUATED** | Required metric with no frozen `minimum_score` in a benchmark manifest. **Never passing.** |
| **Out of programme (deferred)** | Needs a new ID and gates if promoted. Sentiment, NLI, translation, speech, OCR stay here unless the principal promotes them. |

Current NLP code is a **deterministic symbolic prototype** (`crates/qualia-core-db/src/nlp/`). Year-one comment in `nlp/mod.rs`: tokenize + gazetteer + spans; not FrameNet, RST, OpenIE, NLI, or MT. Public invoke IDs exist for more processors; those IDs must keep their current semantics and be labelled honestly (NLP-003).

---

## 1. First profiles (what ships first)

Plan §5.2 names five runtime profiles. This charter selects which are in the **first engineering horizon**.

| Profile id | First-horizon role | Selected Alpha target? |
|---|---|---|
| `symbolic-lite` | WASM/offline deterministic extraction: boundaries, tokenizer, gazetteer, quantities/dates, annotation plans | **Yes — first selected profile** (Alpha target, UNEVALUATED until gates exist) |
| `retrieval-keyword` | Preserve `NLP.graphrag_query` as experimental keyword-triple search | **No.** Allowed to invoke; M7 replaces with hybrid retrieval |
| `linguistic-standard` | POS/morph/dependencies/NER with local weights | **No.** Blocked on NLP-009 + NLP-100 + NLP-306 |
| `semantic-graph` | standard + coref + frames/OpenIE + graph validation | **No.** After M4–M7 |
| `domain-clinical` | Governed health assistance | **No.** NLP-803 later; **not a medical certification** |
| Legal/governance pack | Deontic/temporal domain pack | **No.** NLP-804 later; **not a legal certification** |

**First language:** English, with Australian English measurement and ISO-date examples (plan §4.1). Locale and document time must be supplied explicitly where ambiguity matters; this charter does not assume a document timezone.

**First domain pack:** independently authored catchment / rainfall measurement notes (UC-A). This is an **application-domain engineering corpus**, not a clinical or legal profile.

**First deployable surfaces (engineering, not browser-certified):**

| Surface | Status in this charter |
|---|---|
| Native `qualia-core-db` / desktop host | **First measurement class:** Windows 10 x86_64 workstation (this checkout). Record **actual** CPU model, RAM GiB, and wgpu adapter name at NLP-007/NLP-009 run time — do not invent a SKU here. Per-pass cap remains **42 MiB Sentinel**, not host RAM. Planning class: desktop/workstation, not phone. |
| `wasm-ontology` compile of the NLP Rust module | In scope as a **compile** gate (plan §9.4). `cargo check` is not a browser test |
| Browser / WASM **runtime** (UTF-16 offsets, export, invocation) | **Not certified** until NLP-106 / NLP-905 execute a browser harness |
| Provider / cloud LLM | **Out.** Local-first. No provider LLM in any first profile |

---

## 2. Use cases

### UC-A — Catchment measurement notes (first domain corpus)

**Name:** `catchment-measurement-notes`  
**Intent:** A field note becomes spanned gazetteer hits, an ISO date, and a millimetre quantity, without leaving the local node.

**Representative input (already in-tree, `nlp/mod.rs` year-one fixture):**

```text
North Spring is the reference site. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15.
```

Sibling fixture in `nlp/emit.rs`:

```text
North Spring is the reference catchment. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15.
```

Default lexicon surfaces (`nlp/terms.rs`, seven entries): `North Spring`, `reference catchment`, `reference site`, `catchment`, `Timothy Charles Holborn`, `rain`, `hasCondition`. IRIs include `https://qualiadb.org/catchment/NorthSpring` and `did:qualia:timothy_charles_holborn`. **No third-party persons** in the default lexicon.

**Language / domain:** English; environmental / catchment measurement notes. Australian English unit/date examples. **Not** clinical. **Not** legal.

**Maximum document size:** Host transport **256 KiB** UTF-8 (`256 * 1024` on `nlp.analyze` and sibling NLP hosts; coref uses `CorefLimits::DEFAULT.max_source_bytes`, same number). Typical authored note for NLP-005: **≤ 32 KiB**. Larger notes are in-budget only if they stay under the transport cap; they are not a latency claim.

**Deployed hardware / browser:** Native Windows 10 x86_64 workstation class (this checkout). WASM module compile in scope. Browser execution **not** an Alpha target. CPU/RAM/GPU identity is a **measurement receipt** at NLP-007/009, not a frozen SKU in this charter.

**Sensitivity:** **Restricted.** The default lexicon embeds the principal identity DID. Source text must not appear in telemetry/receipts. Classified clinical context is out of scope. Graph emission must keep `webizen:SensitivityLabel` checks; Restricted notes must not egress.

**Acceptable latency (engineering defaults, UNEVALUATED):**

| Input class | Target | Status |
|---|---|---|
| Fixture paragraph (~120 bytes) | Native p95 tokenize + gazetteer + normalize **< 10 ms** | UNEVALUATED (unit tests only) |
| Typical note ≤ 8 KiB | Native p95 same path **< 50 ms** | UNEVALUATED until NLP-007 |
| Transport maximum 256 KiB | Finish within work budget or **typed overflow/budget error**; no silent truncation | Engineering rule; wall-clock **not** an SLA |

**Output acceptance (UC-A, `symbolic-lite`):**

1. `NLP.tokenize` / `tokenize()` yields a `Number` token whose source slice is `12.5` and Word tokens whose slices equal `North`, `Spring`, etc. Spans are half-open UTF-8 byte intervals in the **original** string (`DocSpan`).
2. `NLP.split_sentences` reports **exactly 2** sentences on the fixture. ASCII `.` between digits is a decimal, not an ender (NLP-002). Enders: ASCII `.?!`, newline, and `。！？` (U+3002/FF01/FF1F). Abbreviations (`Dr.`) remain NLP-102.
3. `NLP.gazetteer_run` returns a hit with surface `North Spring` and IRI `https://qualiadb.org/catchment/NorthSpring`. Overlap with `catchment` / `reference site` is allowed; longest-match policy is owned by NLP-400/401, not this charter.
4. `normalize_dates_and_numbers` emits `DateIso` for `2026-08-15` (Gregorian-valid) and a `Number` with unit `mm` from the static `KNOWN_UNITS` list (includes `mm`, `°C`, `m/s`, … — **not** full QUDT). Impossible dates such as `2026-02-31` are not `DateIso` (NLP-002).
5. `nlp.analyze` (`analyze_document`) returns non-zero `tokens`, `sentences`, `plans`, and a non-zero 60-bit `source_hash`. Current host response is **counts-only**; inspecting spans requires gazetteer/tokenize invokes or a later versioned API (plan §10.6).
6. **Must not** emit personal gender or identity facts from the name. Coref name-gender heuristic remains experimental (NLP-001/NLP-501).
7. Failure on oversize input is a typed host error (`nlp.analyze exceeds 256 KiB` or coref `SourceTooLarge`), never an empty success.

UC-A does **not** require coreference, frames, OpenIE, morphology, POS, or GraphRAG to pass.

---

### UC-B — English general notes inspection (first general-language corpus)

**Name:** `english-general-notes`  
**Intent:** A short English note can be tokenized and sentence-split with exact source spans, under the same transport cap, without a domain gazetteer.

**Representative inputs (NLP-005 must author; not downloaded):**

| Working title | Phenomena the gold must cover |
|---|---|
| `notes-plain-en-001` | Two+ sentences, ASCII, no domain lexicon |
| `notes-decimal-en-001` | `3.5 km` / `12.5 mm` style decimals (must **not** split the sentence on the decimal point; NLP-002) |
| `notes-abbrev-en-001` | `Dr.` / `e.g.` vs sentence end (**still** splits on `.` after letters; NLP-102) |
| `notes-unicode-en-001` | Non-ASCII letters, curly quotes; CJK/fullwidth `。！？` **do** end sentences (NLP-002). U+2026 `…` and other scripts remain NLP-102 |

Until NLP-005 lands, UC-B regression uses only in-tree unit strings such as `"One. Two!"` (`tokenize.rs`) and `"North Spring recorded 12.5 mm."`. Those are **fixtures**, not a general-language corpus.

**Language / domain:** English general text. Separate scorecard from UC-A (plan R-06). No clinical/legal labels.

**Maximum document size:** 256 KiB transport; NLP-005 gold documents **≤ 16 KiB** each so CI stays offline and cheap.

**Deployed hardware / browser:** Same as UC-A.

**Sensitivity:** Default **Public** for independently authored notes that contain no principal DID and no sanctuary content. If a note includes names from `DEFAULT_LEXICON` or other PII, treat as Restricted.

**Acceptable latency:** Same table as UC-A.

**Output acceptance (UC-B, `symbolic-lite`):**

1. Every token/sentence span satisfies `0 ≤ start < end ≤ source.len()`, UTF-8 character boundaries, and `span.slice(source) == surface`.
2. Empty valid input succeeds with zero tokens/sentences (no panic, no forged spans).
3. Input over 256 KiB fails closed on host entry points.
4. `analyze_document` still **runs** the default gazetteer (it is not a separate optional stage on that path). For UC-B, **zero hits is success**; do not score catchment IRI recall. A UC-B-only host that skipped gazetteer is not the current `nlp.analyze` contract.
5. Do not score Stanford/Stanza/spaCy agreement until NLP-006 runners exist.

---

### UC-C — Local keyword-triple lookup (experimental; not Alpha-certified)

**Name:** `keyword-triple-lookup`  
**Intent:** Rank caller-supplied triples by query-term overlap. This is what `GraphRagIndex::query` and `NLP.graphrag_query` **actually do**.

**Representative input:**

- Query: `North Spring rain`
- Triples (host shape `{ query, k, triples: [[s,p,o], ...] }`): e.g.  
  `["https://qualiadb.org/catchment/NorthSpring", "recorded", "12.5 mm"]`  
  plus distractors that share no tokens.
- `k` default **10** if omitted.

**Language / domain:** English query tokens (alphanumeric split, ASCII-lowercased). Independent of UC-A extraction quality.

**Maximum size:** Query **64 KiB** (host). Triple list is cloned in the invoke adapter; a general work budget is **NLP-007** (defect D-004). Do not treat an unbounded triple list as in-budget.

**Deployed hardware / browser:** Native invoke. Not a persistent index (rebuilds per call). Not M7 hybrid retrieval.

**Sensitivity:** Whatever policy applies to the supplied triples. Access filtering **before** scoring is NLP-705; **not** claimed here. Restricted triples must not be returned across an unauthorized context.

**Acceptable latency:** Engineering default: ≤ 1 000 triples, k ≤ 10, query ≤ 1 KiB → native p95 **< 20 ms** (UNEVALUATED).

**Output acceptance (experimental only):**

1. Ranking is deterministic: score descending, triple index ascending on ties.
2. Score is hit-count / query-term-count on overlapping lowercased alphanumeric tokens. **Graph structure does not affect the score.**
3. Preserve invoke id `NLP.graphrag_query` and `{ subject, predicate, object, score }` shape (plan §10.8).
4. Maturity label must say **experimental keyword-triple search**, not GraphRAG / embeddings / evidence paths.

UC-C is **out** of Alpha certification. It exists so NLP-003/NLP-700 do not rename the ID and so NLP-005 can optionally include a tiny triple/query smoke set.

---

## 3. Mandatory vs optional processors

Honesty rule: capability names in Poet/host IDs overstate several algorithms. The table uses **repository modules and invoke IDs**, not marketing names.

Maturity vocabulary matches the programme: experimental symbolic prototype ≠ production linguistic quality.

| Processor | Code / invoke | What it is today | `symbolic-lite` (UC-A/B) | `retrieval-keyword` (UC-C) | Later profiles |
|---|---|---|---|---|---|
| UTF-8 source + size check | Host `256 * 1024`; coref `CorefLimits` | Transport reject | **Mandatory** | Query 64 KiB **mandatory** | Mandatory |
| Tokenize | `nlp::tokenize::tokenize` · `NLP.tokenize` | Word / Number / Punct / Other; skip whitespace; ASCII digits + `.` `,`; alphabetic words with `_` `'` `-` | **Mandatory** | Not required for current query path (independent split) | Mandatory; UAX #29 is NLP-101 |
| Sentences | `split_sentences` · `NLP.split_sentences` | ASCII `.?!` + newline + `。！？`; ASCII `.` between digits is **not** an ender | **Mandatory** | Optional | Abbreviations/ellipsis NLP-102 |
| One-shot counts | `analyze_document` · `nlp.analyze` | Tokenize + sentences + default gazetteer + normalize + emit plans; **counts-only** host record | **Mandatory** host for Alpha smoke | Out | Versioned inspect API is NLP-900 |
| Gazetteer scan | `Gazetteer::default().find` · `NLP.gazetteer_run` | Aho-Corasick, 7 static `DEFAULT_LEXICON` entries, ASCII-fold | **Always executed** on `analyze_document`. **UC-A:** hits required for North Spring. **UC-B:** zero hits OK (do not score IRI recall) | Out | Compiled caller gazetteer is NLP-400 |
| Gazetteer “build” | `NLP.gazetteer_build` | Metadata only (`patterns`, `status=default_lexicon`). **Does not accept caller entries** | Optional inspect | Out | Real build contract is NLP-400 |
| Known-IRI filter | `nlp::link::filter_known` | Drops hits whose IRI is not in `DEFAULT_LEXICON` | **Mandatory** on `analyze_document` path | Out | Candidate generation NLP-403 |
| Date/quantity normalize | `normalize_dates_and_numbers` | ISO `YYYY-MM-DD` with Gregorian month lengths + leap years; static unit list; longest-match compounds (`m/s`) | **Mandatory** | Out | TIMEX3 / QUDT mapping NLP-200..203 |
| Annotation plans | `nlp::emit` | Plans from hits + norms; kinds `gazetteer` / `date` / `number` | **Mandatory** on analyze path | Out | NQuin emission NLP-903 |
| Morphology FST | `nlp::fst` · `NLP.fst_lookup` | Exact trie + six suffix rules (`ies`/`es`/`s`/`ied`/`ed`/`ing`) | **Optional experimental** | Out | Mandatory after NLP-204/205 |
| Coreference | `nlp::coref` · `NLP.coref_resolve` | Exact-string grouping + experimental pronoun sieve; empty host `mentions` **skips detection**; substrate mentions every word | **Optional experimental** | Out | Mandatory after NLP-500/501 |
| Frames | `nlp::frame` · `NLP.frame_extract` | Triggers `"bought"` → BUY, `"gave"` → TRANSFER | **Optional demo** | Out | After NLP-602 |
| Relations | `nlp::relation` · `NLP.relation_extract` | `"is"` / `"has"` / `"located in"`; hardcoded confidence **0.9 / 0.85 / 0.8** (not calibrated) | **Optional demo** | Out | After NLP-603 |
| Substrate | `extract_substrate` · `NLP.substrate_extract` | tokenize + gazetteer + normalize + relations + frames + coref | **Optional experimental** (not Alpha-certified) | Out | NLP-107 pipeline |
| Keyword retrieval | `GraphRagIndex` · `NLP.graphrag_query` | Full scan, term overlap, per-call rebuild, no embeddings | **Out** of lite certification | **Mandatory experimental** | Replaced by NLP-700..704 |
| Language ID | — | Absent | Out (explicit English override) | Out | NLP-800 |
| POS / UD features | — | Absent | Out | Out | NLP-300+ |
| Dependency parse | — | Absent | Out | Out | NLP-302 |
| Model NER | — | Gazetteer only | Out | Out | NLP-402 |
| Hybrid GraphRAG | — | Graph does not influence retrieval | Out | Out | M7 |
| Provider LLM | — | None | **Forbidden** | **Forbidden** | Forbidden unless principal + NLP-009 |

`symbolic-lite` therefore means: **validate → tokenize → sentences → default gazetteer (always run on `analyze_document`; UC-B may have zero hits) → normalize → emit plans**, with English assumed by the caller until NLP-800. Everything else may be called, must fail closed on budgets, and must stay labelled experimental.

---

## 4. Evaluation inventory

No dataset or model is downloaded by this charter. Unset required thresholds stay **UNEVALUATED** and **must not pass** a release gate (plan §6.2).

### 4.1 Present in this checkout (2026-09-14)

| Asset | Location | What it can support | What it cannot |
|---|---|---|---|
| Catchment paragraph fixture | `nlp/mod.rs`, `emit.rs`, `gazetteer.rs` tests | Span/IRI smoke for UC-A | Statistical quality; held-out test |
| Seven-entry lexicon | `nlp/terms.rs` `DEFAULT_LEXICON` | Gazetteer regression | General NER |
| Tokenizer/normalizer unit tests | `nlp/tokenize.rs`, `nlp/normalize.rs` | Decimal/Unicode/degree/compound-unit/Gregorian ISO (NLP-002) | UAX #29; remaining Unicode enders; locale numbers |
| Coref resource tests | `nlp/coref/` | Budget/span contract (NLP-001) | CoNLL F1 / LEA |
| Keyword-triple tests | `nlp/graphrag.rs` | Deterministic overlap ranking | nDCG / path faithfulness |
| Non-NLP JSON dumps | `docs/benchmark-datasets/synthetic-10k.json`, `schemaorg-30-current-https.json` | Unrelated graph benches | Any NLP task |
| Official UD / OntoNotes / GUM / FrameNet / CoNLL | **Absent** | — | All §6.2 standard gates |
| Unicode UAX #29 pinned files | **Absent** | — | 100% declared Unicode conformance |
| Frozen CoreNLP / Stanza / spaCy runners | **Absent** | — | Differential / superiority claims |
| `benchmarks/nlp/manifests/` numeric gates | **Not a completed NLP-004 receipt in this charter** | NLP-004 is a separate Wave 0 lane | Do not assume `minimum_score` exists |
| Browser harness | **Absent** | — | Browser certification |

Tiny example suites **cannot** substantiate a statistical claim (plan §6.2).

### 4.2 NLP-005 must create (offline, authored, licensed)

Schedule NLP-005 **after** NLP-004’s manifest/receipt schema exists. NLP-005 does **not** download UD or OntoNotes unless the principal later authorizes a specific license and retrieval.

| Corpus id | Use case | Language / domain | Allowed use | Annotation | Split |
|---|---|---|---|---|---|
| `qualia-catchment-notes-v0` | UC-A | English / catchment measurement | Bundled (project-authored) | Gazetteer surfaces, ISO dates, quantities+units, exact UTF-8 spans | Immutable train/dev/test hashes; **test held out from rule authors after freeze** |
| `qualia-english-notes-v0` | UC-B | English general | Bundled (project-authored) | Token and sentence gold; adversarial decimals, abbreviations, quotes, Unicode | Same split policy |
| `qualia-keyword-triples-v0` (optional smoke) | UC-C | English queries + triples | Bundled | Query, triple list, expected rank order | May be fully in CI; not an nDCG claim |

Each entry must record the §9.2 fields: canonical source (here: in-repo path), retrieval date, license (project copyright / CC or internal), redistribution, checksum, language, domain, scheme, limitations, split hashes, allowed use, PII/sensitivity, retention, preprocessing.

**Gold-set process:** annotation guidelines; two annotators on a representative sample; agreement + adjudication. Clinical/legal judgments stay **blocked** until domain expertise exists; do not put them in v0.

**Explicitly not in v0 (record as `unavailable` in NLP-005 `DATASETS.md`):**

- Universal Dependencies English EWT/GUM (needs license + NLP-005 principal OK)
- OntoNotes coref / NER (license)
- CorefUD / GUM coref
- FrameNet / PropBank
- Licensed temporal/geo sets
- Any Stanza/spaCy/CoreNLP model package

Those rows remain **UNEVALUATED**. NLP-006 cannot mark their `release_required` gates passing.

### 4.3 Gates vs first profile

| Task (plan §6.2) | Needed for Alpha `symbolic-lite`? | First data | Threshold now |
|---|---|---|---|
| Token/sentence boundaries | **Yes** | Authored UC-B + Unicode files later (NLP-105) | UNEVALUATED until NLP-006 freezes `minimum_score` |
| Language ID | No (English override) | — | N/A for first profile |
| Lemmas/POS/dependencies | No | UD when licensed | UNEVALUATED |
| NER/linking | Gazetteer-only for UC-A | Catchment gold | UNEVALUATED as NER; fixture smoke is not F1 |
| Coreference | No | GUM/OntoNotes when licensed | UNEVALUATED |
| Temporal/quantity | **Yes** (ISO + units) | Catchment gold | UNEVALUATED |
| Frames/SRL/OpenIE | No | — | UNEVALUATED |
| Retrieval nDCG | No | — | UNEVALUATED; UC-C is smoke |
| Calibration | No | — | Relation `confidence` constants are **not** probabilities |
| Provenance/determinism | **Yes** | All v0 docs | 100% valid source refs in test coverage is a **required** Alpha condition once NLP-006 writes the manifest; until then UNEVALUATED |
| Differential vs Stanford/Stanza/spaCy | No for Alpha | NLP-006 | UNEVALUATED; **no comparison claim** |

---

## 5. Resource envelope

Distinguish **transport cap** (bytes accepted at a host/API boundary) from **processing bound** (workspace, mentions, hops, Sentinel). A 256 KiB text limit is **not** a complexity or latency guarantee (plan §11.1–11.2).

| Bound | Value | Applies to | Notes |
|---|---|---|---|
| Host NLP transport (default) | **256 KiB** source UTF-8 | `nlp.analyze`, `NLP.tokenize`, `NLP.split_sentences`, `NLP.gazetteer_run`, `NLP.frame_extract`, `NLP.relation_extract`; coref/substrate via `CorefLimits::DEFAULT.max_source_bytes` | Same number, not always the same constant (D-004) |
| GraphRAG query transport | **64 KiB** | `NLP.graphrag_query` query string | Triple-list cap not generalized; NLP-007 |
| Coref mentions | 4 096 | `NLP.coref_resolve` / substrate | NLP-001 |
| Coref mention text | 1 MiB aggregate | supplied mention strings | |
| Coref chains | 4 096 | output | |
| Antecedents / pronoun | 64 | search window | |
| Antecedent checks / call | 65 536 | work counter | |
| Coref workspace | 2 MiB | caller-owned scratch | Together with source, under Sentinel |
| Sentinel ceiling | **42 MiB** per execution pass | All NLP | Account source + output + workspace + stack + any future resident weights (NLP-009). mmap does not exempt RSS |
| Local-first | No outbound model/API | All first profiles | Daemon `:4242` is the graph engine, not an LLM |
| Provider LLM | **Forbidden** | All first profiles | `AgentBackend::Local` only if a later profile uses in-process GGUF; not authorized here |

**Processing vs transport example:** a 200 KiB note is inside transport and may still exceed mention, comparison, or workspace budgets. Exceeding a processing bound is a **typed error**, not a partial graph mutation (plan §5.3).

**WASM `symbolic-lite`:** no learned weights. Bundle-size growth is measured per milestone (NLP-106), not in this charter.

**No model download** is authorized by NLP-008. NLP-009 may **name** candidate artifacts and resident-byte estimates; fetching them needs an explicit principal action.

---

## 6. Scope decisions

### 6.1 In this programme’s first engineering horizon

- English general text (UC-B) and independently authored catchment/measurement notes (UC-A).
- Exact UTF-8 span provenance; `DocSpan` / `text_span` alignment later (NLP-100).
- Deterministic symbolic tokenize, sentence split, default gazetteer, ISO date + listed units, annotation plans.
- Honest maturity labels (NLP-003) for existing invoke IDs listed in §3.
- Bounded coreference **resource** behavior (NLP-001) without a quality claim.
- Keyword-triple invoke preserved as experimental (UC-C).
- Evaluation scaffolding (NLP-004) and authored seed corpora (NLP-005).
- Reuse/feasibility study (NLP-009) **after** this charter.
- Native + `wasm-ontology` **compile** checks. Browser runtime later.
- Privacy: local processing; Restricted catchment notes; no source in receipts.

### 6.2 In the programme later, not certified here

| Item | Owning IDs |
|---|---|
| UAX-style tokenizer, robust sentences | NLP-101, NLP-102, NLP-105 |
| Caller-compiled gazetteers | NLP-400, NLP-401 |
| Morphology language packs, POS, dependencies | NLP-204, NLP-300..305 |
| Model NER, linking, NIL | NLP-402..406 |
| Real coreference quality | NLP-500..505 |
| Frames, SRL, OpenIE, qualified claims | NLP-600..606 |
| Hybrid lexical/vector/graph retrieval | NLP-700..707 |
| Language ID and non-English pack | NLP-800, NLP-802 |
| Clinical / legal **packs** (assistance, not certification of practice) | NLP-803, NLP-804 |
| Geographic extraction | NLP-806 |
| Training/adaptation of a local artifact | NLP-306, only after NLP-009 |

### 6.3 Out of this programme unless promoted with new IDs and gates

Plan §4.1 / §8.1. **Deferred.** Do not implement inside NLP-xxx rows already assigned to other tasks.

| Capability | Status | Promotion rule |
|---|---|---|
| Sentiment analysis | Deferred | New package ID, dataset, resource proof, owner |
| Text classification (topic/intent beyond extraction) | Deferred | Same |
| NLI / RTE | Deferred | Same |
| RST / discourse parsing | Deferred | Same |
| Constituency parsing | Deferred (optional profile only if a validated consumer requires it) | Plan already allows a consumer-gated exception |
| Machine translation | Deferred — adjacent system | Integration contract only (NLP-902) |
| Speech recognition | Deferred — adjacent | Integration contract only |
| OCR / PDF decoding | Deferred — adjacent | Input to NLP is **extracted UTF-8**; ingestion owns decoding |
| Answer generation / chat over GraphRAG | Not implied by `NLP.graphrag_query` | Separate spec: citation support + abstention |
| Stanford/Stanza/spaCy **parity or superiority** | Not a deliverable of this charter | NLP-006 + NLP-906 + principal publication approval |
| Clinical deployment certification | Out | NLP-803 ≠ device/clinical sign-off |
| Legal advice / filing certification | Out | NLP-804 ≠ practice certification |
| Provider-hosted LLM backends | Out | Forbidden in first profiles |

---

## 7. Effort honesty and first slices

The tracker has **75 capability packages**. That is not a 75-session promise, not a percent-complete of linguistic quality, and not a schedule. Many packages need several sessions plus a real data/model programme (plan §8.1, §7.1).

Estimate **ranges** only after NLP-009 inspects reuse. This charter does not invent person-days.

**Parallelism:** at most one primary implementation slice per owner; multiple owners if file lanes do not overlap (plan §14.7).

### 7.1 First slices for remaining ready / next packages

These slices are scheduling inputs for a product owner. They are not started by this document.

| Package | Tracker state (at charter draft) | First slice (bounded) | Evidence deliverable | Blocked on |
|---|---|---|---|---|
| NLP-003 | `READY` | Label existing invoke IDs with algorithm class + maturity; do not rename IDs | Capability descriptors; no new linguistic behavior | Wave 0 shared freeze (`lib.rs` / copy) |
| NLP-004 | `IN_PROGRESS` (other lane) | Manifest schema + JSON/Markdown receipts + authored smoke that runs offline | `tests/nlp_suite.rs` / `benchmarks/nlp/manifests/` | — |
| NLP-005 | `PLANNED` | Author `qualia-catchment-notes-v0` + `qualia-english-notes-v0` with §9.2 records and split hashes; mark external sets `unavailable` | `docs/benchmark-datasets/nlp/DATASETS.md` + gold files in an explicit artifact dir | NLP-004 |
| NLP-006 | `PLANNED` | Import v0 gold; freeze **dataset-specific** `minimum_score` only where coverage exists; leave UD/coref/SRL UNEVALUATED | Gate manifests; no Stanford claim | NLP-004, NLP-005, **this charter** |
| NLP-007 | `PLANNED` | Replace remaining private 256 KiB literals with shared limits; cap GraphRAG triple lists; work counters | Resource receipts | NLP-001, NLP-004 |
| NLP-009 | `PLANNED` | Inventory `inference/runtime/`, `inference/semantic_skills.rs`, `inference/reranker/`, `text_span/`, WordNet/GeoNames scripts, lexicon, GGUF loaders; compare rules/FST vs compact statistical vs encoder **on paper + resident-byte envelopes** | Feasibility table; **no download** | **This charter** |
| NLP-100 | `PLANNED` | Versioned `DocumentView` / span / error contracts | ABI notes | NLP-004 |
| NLP-107 + NLP-800 | `PLANNED` | One host-visible `symbolic-lite` graph; English override; unknown language explicit | Pipeline smoke | NLP-100 |
| NLP-101/102/105 | `PLANNED` | Unicode tokenize + sentences + conformance files | Boundary F1 on v0; Unicode files when pinned | NLP-100 |
| NLP-400 then NLP-200/202/204 | `PLANNED` | Caller gazetteer compiler; then temporal/units/morph interface | Overflow tests; unit exact match on v0 | NLP-100 / NLP-101 |

**Do not** integrate learned syntax, NER, or coreference models before NLP-100 and evaluation receipts are stable (plan §8 recommended packet 12 footnote).

**No model download authorization** in this charter, in NLP-005 v0, or in NLP-009’s inventory phase.

---

## 8. Scheduling NLP-005 and NLP-009 (product-owner brief)

### NLP-005 — seed corpora

1. Use **UC-A** and **UC-B** as the only v0 gold tasks. Optional UC-C smoke triples.
2. Author text in-repo or in a caller-selected artifact directory; do not fetch UD/OntoNotes/GUM.
3. Include the North Spring fixture as one **included** document, not the whole corpus. Add independent notes with other sites/units so rules cannot overfit seven lexicon strings.
4. Record Restricted vs Public per document. Catchment notes that include `Timothy Charles Holborn` are Restricted.
5. Freeze splits before anyone tunes tokenizer/normalizer rules against test (R-13).
6. Write `unavailable` rows for every §6.2 dataset not on disk.

### NLP-009 — reuse and feasibility (not this package)

1. First profile remains **`symbolic-lite` without weights**.
2. Inspect existing loaders **before** proposing a new one. Adjacent ML/sentiment/document-decode code is not automatically an NLP backend (plan §4 initial audit caveat).
3. Compare three families: (a) rules/dictionary/FST, (b) compact statistical sequence models, (c) a learned encoder **if** resident bytes can plausibly sit under 42 MiB + transport/workspace for a named hardware target.
4. Deliver a quality/resource **table**, not a selected production model.
5. Benchmark-only upstream toolkits may be reference processes later; they are not deployable components until they meet budget and license.
6. Still no silent download.

---

## 9. Principal decisions still needed

These are the only items that should block later gates. Everything else in this charter is the engineering default.

1. **Confirm UC-A + UC-B** as the selected Alpha **target** pair, with UC-C experimental-only. This confirmation does not pass Alpha gates.
2. **Confirm first language is English only** (no NLP-802 language in Alpha).
3. **Confirm Restricted** handling of catchment notes that contain the principal DID already present in `DEFAULT_LEXICON`.
4. **Locale / document time:** keep “explicit locale and document time required for ambiguous dates” (plan §4.1), or name a default timezone for UC-A field notes.
5. **Latency:** accept the UNEVALUATED engineering targets in §2, or replace them with product SLAs before NLP-007 freezes budgets.
6. **NLP-005 gold** may repeat the in-tree principal name as in the default lexicon; if that is unwanted, NLP-005 must substitute a synthetic observer string and NLP-400 must version the lexicon.

**Standing prohibitions (not open decisions):** no download of UD/OntoNotes/GUM/FrameNet/weights until a dated license action; no comparative publication until NLP-906 + explicit approval; clinical/legal remain uncertified; CPU/RAM/GPU identity is recorded at NLP-007/009 measurement time (workstation class is already selected).

---

## 10. Status recommendation

**REVIEW.**

Implementer (Swarm-C) must not mark NLP-008 `DONE`. Independent review should check: (1) use cases are concrete and match `src/nlp/` APIs; (2) clinical/legal are not certified; (3) thresholds stay UNEVALUATED; (4) NLP-009 is not pre-empted; (5) no model/data download is authorized.

**Handoff to supervisor:** path `docs/work-in-progress/NLP_RELEASE_CHARTER_WIP.md`; pointer in the programme plan is supervisor-owned.
