# NLP Reuse Inventory and Feasibility (NLP-009)

**Status:** `REVIEW` (not `DONE`) · **Package:** NLP-009 · **Date:** 2026-09-14  
**Owner (implementer):** Swarm-H · **Review:** independent (supervisor / named reviewer), not self-DONE  
**Normative programme:** [`NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md`](./NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md) §8.2  
**Charter:** [`NLP_RELEASE_CHARTER_WIP.md`](./NLP_RELEASE_CHARTER_WIP.md) — NLP-008 `DONE` (second independent review ACCEPT_DONE 2026-09-14; D-016 closed). This inventory ran in the same finish-wave after that acceptance.  
**Allowed artifacts:** this file; optional one-line pointer in [`README.md`](./README.md). No Rust edits. No model or dataset download.

This is a **reuse inventory plus measured hardware receipt**. It does not choose a production learned model, train weights, or certify Alpha linguistic quality. Adjacent existence is not suitability.

---

## 0. How to read this document

| Claim class | Meaning here |
|---|---|
| **Verified API** | Inspected in this checkout; module path named. |
| **Gap** | Missing, overclaimed, or unsuitable for the named NLP task. |
| **UNEVALUATED** | Required quality or resident-byte metric with no receipt this session. **Never passing.** |
| **Hardware receipt** | WMI/CIM measurement on this workstation. Not a certification. Not a wgpu adapter name unless wgpu was queried. |
| **Alpha recommendation** | Keep `symbolic-lite`. Not a scorecard pass. |

Standing prohibitions held: no download; no claim that BERT / UDPipe / Stanza can ship inside 42 MiB without a resident-byte receipt; no comparative publication.

---

## 1. Hardware receipt (this session, 2026-09-14)

Measured on the canonical tree host with Windows CIM (`Get-CimInstance`). This is a **measurement**, not a product certification and not a frozen SKU in the charter.

| Item | Value | How measured |
|---|---|---|
| OS | Microsoft Windows 10 Pro 10.0.19045 | `Win32_OperatingSystem` |
| Arch | AMD64 (`x86_64`) | `$env:PROCESSOR_ARCHITECTURE` |
| CPU | `Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz` | `Win32_Processor.Name` (first processor) |
| RAM | **63.84 GiB** (`68_548_255_744` bytes) | `Win32_ComputerSystem.TotalPhysicalMemory` |
| Display adapter 1 (WMI) | `NVIDIA RTX A2000 12GB`; DriverVersion `32.0.16.1047`; PNP `VEN_10DE&DEV_2571` | `Win32_VideoController` |
| Display adapter 1 reported AdapterRAM | `4_293_918_720` bytes (~4.00 GiB) | WMI `AdapterRAM` is a 32-bit field; **do not treat this as a 12 GiB VRAM receipt** even though the name string includes `12GB` |
| Display adapter 2 (WMI) | `Intel(R) HD Graphics 530`; AdapterRAM `1_073_741_824`; DriverVersion `31.0.101.2111` | `Win32_VideoController` |
| **wgpu adapter name** | **not measured this session** | Querying wgpu requires initializing `gpu-runtime` / `Render.gpu_adapter_info` (`crates/qualia-core-db/src/poet_host/invoke/render/gpu.rs`). That is a GPU session, not this inventory. WMI names are **not** substituted for `wgpu::AdapterInfo::name`. |

Planning class remains **desktop/workstation**, not phone. Per-pass cap remains **42 MiB Sentinel** (`nlp::coref::limits::SENTINEL_BYTES` = `42 * 1024 * 1024`), not host RAM. Host RAM being ~64 GiB does not authorize resident NLP weights inside a Sentinel pass.

---

## 2. Recommendation (Alpha and later)

1. **Keep `symbolic-lite` as the Alpha profile.** Current `nlp/*` tokenize + gazetteer + ISO/units + plans is the deployable first path (charter UC-A / UC-B). No learned weights.
2. **Do not start NLP-306** until **NLP-100 is `DONE`** and **NLP-006 gates exist**. This inventory is not a training authorization and not a model download authorization.
3. **Compact statistical (CRF/HMM) is a later option only if a CRF (or an NLP-specific sequence tagger) is written in-tree.** A generic discrete HMM already exists and is **not** a POS/NER tagger. **Do not vendor a Python CRF.**
4. **A GGUF encoder on the existing wgpu path would be a new NLP-306 programme**, not a reuse of LLM decode. LLM decode ≠ POS / NER / coref.
5. **Do not create a second graph store.** Persist NLP facts as `NQuin`s into the existing daemon graph / volume.

**Recommended next package:** **NLP-006** (dataset-specific numeric gates on authored v0 gold; UD/OntoNotes/coref remain `UNEVALUATED`). NLP-100 contracts already exist in-tree as `nlp/contracts/` (`REVIEW` until independently accepted). NLP-306 stays blocked.

---

## 3. Verified APIs versus gaps

Inspected in this checkout. Citations are module paths, not marketing names.

### 3.1 `inference/runtime/` — artifact budget, scheduler, prepared lifecycle

**Verified**

| Surface | Path | What it actually is |
|---|---|---|
| Library root | `crates/qualia-core-db/src/inference/runtime/mod.rs` | Plan/run boundary for **production LLM inference**. Cold: discovery, compile, upload, receipts, artifacts. Hot: prepared decode (Tier-1 zero-heap). |
| Artifact budget | `…/artifacts/budget.rs`, `…/artifacts/run_dir.rs` | `RunArtifactDir::write_bounded` fails closed with `ArtifactError::BudgetExceeded { budget_bytes, attempted_bytes }`. Labels ASCII `[A-Za-z0-9_-]`. Relative child paths only. Marker file `.qualia-inference-run`. Retention: ephemeral `TempDir` or atomic promote. |
| Scheduler | `…/scheduler/mod.rs`, `…/scheduler/request_table.rs` | `RequestScheduler<const REQUESTS>` bounded concurrent slots. `admit_with_prefix` attaches graph-derived KV prefix pages. States: `Empty` / `Prefill` / `Decode`. Hot path is **paged autoregressive decode**, not NLP tagging. |
| Prepared lifecycle | `…/prepared/lifecycle.rs`, `…/prepared/decode_plan.rs` | `PreparedPlanState::{Unbuilt, Ready, Ineligible, Failed}`. Trait `PreparedDecodePlan::run_decode_step` is **one autoregressive token step** (`token_position`, `token_id`, hidden, KV page table). Backends: `Wgpu`, `Cuda`, `Metal`, `Cpu`. |
| Graph assist | `…/graph_assist/query.rs` | `query_graph_into(&[NQuin], GraphQuery, GraphAssistPolicy, …)` scans the **caller-supplied Quin slice**. Policy includes `sensitivity_ceiling` and `require_valid_parity`. Rejects classified-above-ceiling facts (`rejected_sensitivity`). Default ceiling `0` (Public). |
| Receipts | `…/receipt/` | SHA-256 file/token provenance, execution counters, benchmark manifest schema. Cold evidence for **decode**, not UD POS F1. |

**Gaps for NLP**

- No POS, NER, coref, or encoder-pool prepared plan.
- Artifact dirs are inference-run scratch, not NLP model cards.
- Reuse for NLP-306 later: budgets, receipts, sensitivity-filtered `query_graph_into`, eviction/cleanup. **Do not fork a second loader** if a learned NLP artifact is ever authorized.

### 3.2 `inference/semantic_skills.rs` — `TextEmbedder` is FNV-1a, not neural

**Verified:** `crates/qualia-core-db/src/inference/semantic_skills.rs`

- Header and `TextEmbedder` docs state FNV-1a character n-gram hashing into a fixed `EMBED_DIM = 256` vector, then L2 normalize.
- `embed()` lowercases, hashes n-grams with 32-bit FNV-1a (`0x811C_9DC5`), increments `dims[h % dim]`.
- `VectorStore` is k-NN by cosine over those hashed vectors. `MAX_VECTORS = 4096`.
- Suitable (as the file says) for **approximate dedup / bag-of-ngrams search**. **Not** a replacement for neural embeddings. **Not** a sentence encoder. **Not** GraphRAG vectors.

**Gap:** naming “semantic skills / embeddings” must not be read as MiniLM/BERT. Adjacent cosine search ≠ NLP-700 hybrid retrieval.

### 3.3 `inference/reranker/` — symbolic Jaccard/BM25, not a cross-encoder net

**Verified:** `crates/qualia-core-db/src/inference/reranker/mod.rs`

- File title: “Cross-encoder reranker for candidate re-scoring.”
- Implementation immediately describes **deterministic symbolic scoring (no neural network)**: Jaccard token overlap, simplified BM25 (single-doc IDF is not corpus IDF), phrase bonus, position bonus. Public API: `rerank(query, candidates, config) -> Vec<RerankResult>`.
- Tokenisation: alphanumeric split + ASCII lowercase. Heap `HashMap` / `Vec<String>` (cold).

**Truth:** this is **not** a transformer cross-encoder. The header overclaims “cross-encoder”. Record it as **symbolic lexical rerank**. Unsuitable as a Stanza/spaCy substitute. Reusable later as a cheap lexical stage in NLP-700, not as M7 neural rerank.

### 3.4 `text_span/`

**Verified:** `crates/qualia-core-db/src/text_span/mod.rs`

- `TextSpan { start_utf8, end_utf8, content_hash }` — half-open UTF-8, char-boundary checked, hash = `lexicon::generate_60bit_token`.
- `annotation_quin(term_iri, span, source_hash) -> NQuin` packs start<<32|end in `object`, predicate `https://qualiadb.org/schema/annotatesSpan`.
- **Does not set** `NQuin::set_sensitivity_byte`. `metadata = 0`.

**NLP sibling:** `crates/qualia-core-db/src/nlp/span.rs` `DocSpan` is the same offsets **without** hash (hash computed at emit). NLP-100 views (`nlp/contracts/`) borrow via `DocSpan`.

**Reuse:** one span ABI; host seals plans with `text_span::annotation_quin`. Do not invent a third span type. NLP emit (`nlp/emit.rs`) still produces owned `AnnotationPlan` strings (cold).

### 3.5 Graph search / storage — second store not needed

**Verified**

| Surface | Path | Role |
|---|---|---|
| Daemon graph | `crates/qualia-core-db/src/services/daemon_graph.rs` | Fixed `[NQuin; MAX_GRAPH_QUINS]` with `MAX_GRAPH_QUINS = 65_536`. In-process backing for loopback `/query`. |
| Query egress | `crates/qualia-core-db/src/services/daemon_query.rs` `filter_classified` | Fail-closed `QueryExecError::ClassifiedEgress` if any result has `SENSITIVITY_CLASSIFIED`. |
| NLP keyword index | `crates/qualia-core-db/src/nlp/graphrag.rs` | Per-call `GraphRagIndex` of **caller-supplied string triples**, term-overlap rank. Rebuilds in memory. **Not** a persistent graph and **not** hybrid GraphRAG. |
| Graph assist | `inference/runtime/graph_assist/query.rs` | Hashed-field scan of a Quin slice (see §3.1). |

**Verdict:** NLP facts should emit into the **existing** Quin store (daemon graph / `.q42` volume) via `annotation_quin` + sensitivity stamp. `GraphRagIndex` is an experimental invoke adapter (charter UC-C). **A second graph database is not required** and would violate §8.2 (“reuse … instead of creating a second … graph store”).

### 3.6 WordNet — not in-binary; not a POS tagger

**Verified**

- `GraphDatabase.lexicon_manifest` (`crates/qualia-core-db/src/poet_host/invoke/graph/lexicon.rs`): reads a **volume-backed pack manifest** (`*.lexicon.json` or `.q42` + sidecar). Explicit: **no in-binary WordNet**. Missing pack → diagnose held / not yet (`E300`). WASM denied (needs native FS).
- Optional local paths named in `crates/qualia-client-core/src/chat_ontology.rs`: `wordnet.q42`, `princeton.q42`, `english-wordnet.q42`, `docs/data/wordnet/princeton.q42`, `docs/playground/wordnet.q42`, `Local_LIbraries/wordnet/`.
- Fetch script (not run this session): `scripts/fetch_wordnet.sh` downloads Open English WordNet TTL — **download is not authorized by NLP-009**.

**This session (no download):** all of `wordnet.q42`, `princeton.q42`, `docs/data/wordnet/princeton.q42`, `docs/playground/wordnet.q42`, `Local_LIbraries/wordnet/wordnet.q42`, `Local_LIbraries/wordnet/princeton.q42` are **ABSENT**.

**Gap:** a WordNet `.q42` (when a principal later installs one) is a **synset/graph lexicon pack**, not a UPOS tagger, not morphology with features, and not NER. `lexicon_manifest` returns pack metadata (`pack_id`, `framing`, `conceptIds`), not analyses of a sentence.

### 3.7 GeoNames — bundled ontology only; not a NER model

**Verified**

- `bundled/ontologies/geonames/catalog.json`: license **CC-BY 3.0**, file `geonames.ttl`, namespace `http://www.geonames.org/ontology#`.
- `geonames.ttl` size this session: **383_930 bytes**. Content is the **GeoNames OWL ontology** (classes/properties such as `gn:Feature`), not a gazetteer of populated places.
- NLP-806 (geographic extraction) is later; this bundle does not implement NER.

**Gap:** existence of the ontology ≠ place-name recognition. Do not treat `bundled/ontologies/geonames` as a GeoNames dump or a model.

### 3.8 `nlp/gazetteer.rs`, `nlp/fst.rs`, `nlp/terms.rs`

**Verified**

| Module | Path | What it is |
|---|---|---|
| Lexicon | `nlp/terms.rs` | Seven static `DEFAULT_LEXICON` entries (North Spring, reference catchment/site, catchment, principal DID, rain, hasCondition). |
| Gazetteer | `nlp/gazetteer.rs` | In-tree Aho-Corasick over interned surfaces; ASCII-fold; longest non-overlapping. `Gazetteer::default()` / `from_lexicon`. **Not NER.** |
| Link filter | `nlp/link.rs` | `filter_known` drops hits whose IRI is not in `DEFAULT_LEXICON`. |
| FST | `nlp/fst.rs` | Trie `FstDict::from_entries` + six English suffix rules (`ies`/`es`/`s`/`ied`/`ed`/`ing`). Hot walk is pointer-chasing; results own `String`s (Tier-2). Empty caller entries → no lemmas. **Not** a language pack, **not** UD features. |
| Analyze | `nlp/mod.rs`, `nlp/contracts/analyze.rs` | `analyze_document` / `analyze_document_into`: tokenize → sentences → gazetteer → known-IRI filter → ISO/units → plans. Host `nlp.analyze` is counts-oriented; inspect spans via tokenize/gazetteer or NLP-100 buffers. |
| Budgets | `nlp/budget.rs` | `MAX_SOURCE_BYTES = 256 KiB` (same number as coref default); GraphRAG query 64 KiB; triples 4096; FST entries 4096. Sentinel re-export. |
| Honesty table | `nlp/capability.rs` | Public `NLP.*` IDs labelled experimental; gazetteer “not NER”; GraphRAG “keyword-triple”. |

**Gap:** no UPOS/XPOS, no dependency parser, no model NER, no compiled caller gazetteer on the default path (`NLP.gazetteer_build` is metadata-only per charter). Morphology FST is optional experimental.

### 3.9 GGUF / P64 / wgpu in-process LLM — no Ollama; decode ≠ NLP model

**Verified**

| Surface | Path | Truth |
|---|---|---|
| Runtime | `crates/qualia-core-db/src/gguf_bridge/mod.rs` | In-process GGUF mmap (`memmap2`) + GPU dispatch (DirectML on Windows x64 / wgpu elsewhere). **Not** an Ollama server. **Not** llama.cpp HTTP. |
| Alias | `inference/mod.rs` | `pub use inference_agent as llm_agent`. Comment: no Ollama. |
| `AgentBackend` | `inference/inference_agent/types.rs` | `Local` / `Remote` / `Hybrid`. `Local` comment still names “llama.cpp WASM / ONNX Runtime / WebLLM” — **stale relative to `gguf_bridge`**. Actual local path is GGUF + wgpu/DirectML. Provider/Ollama is not a backend. |
| Token embedding | `gguf_bridge/mod.rs` `dequantize_token_embedding_into`; `gguf_bridge/embedding.rs` `dispatch_quantized_token_embedding` | Lookup of **`token_embd.weight` rows for decode**, not a pooled sentence encoder. |
| Prepared decode | `inference/runtime/prepared/decode_plan.rs` | Autoregressive **one token**. |
| P64 | `crates/qualia-core-db/src/q42/p64_weight/` | AOT GGUF → `.p64` **LLM-weight** container (`p64\0` magic), CRC policy, tensor index. Sibling of semantic `.q42`, not an NLP tagger format. |
| ML GGUF preview | `specialized_libs/machine_learning/mod.rs` | `GGUF_EMBEDDING_PREVIEW_TOKENS = 256` — bounded preview into `Vec<f64>`, **not** a hot encoder. |
| Training in ML lib | `specialized_libs/machine_learning/training.rs` | Real SGD for a **single Linear layer** (linear regression). Not CRF, not transformer fine-tune, not POS. |

**Could a GGUF encoder be reused?**

- **Weight I/O, mmap, P64, artifact budgets, receipts, wgpu device, tokenizer decode of GGUF vocab:** reusable **infrastructure** if a principal later authorizes a named encoder artifact.
- **The running decode loop, KV cache, `PreparedDecodePlan`, logit sampling, chat `infer()`:** **not** a POS/NER/coref model.
- An encoder-only forward (full sequence, pooling, token-classification head, tokenizer/subtoken alignment, calibration) **does not exist**. That is **NLP-306**, plus NLP-100 schemas and NLP-006 gates. It is not a flag flip on `llm_agent`.

**42 MiB:** this session loaded **no** GGUF/P64 NLP encoder. Host RAM ~64 GiB is irrelevant to the per-pass Sentinel. **This inventory does not claim BERT, UDPipe, or Stanza can ship inside 42 MiB.**

### 3.10 Model artifact handling; scoped access / sensitivity byte

**Verified**

- Inference artifacts: bounded `RunArtifactDir` (§3.1). Stale cleanup is marker-owned under a configured parent (`artifacts/cleanup.rs` pattern).
- P64: `IntegrityMode::{Full, Metadata, Structure}` + CRC-32C (`q42/p64_weight/reader.rs`).
- Sensitivity ABI: `NQuin::SENSITIVITY_PUBLIC/RESTRICTED/CLASSIFIED` and `get_sensitivity_byte` / `set_sensitivity_byte` in `crates/qualia-core-db/src/lib.rs` (context bits 56–63).
- Enforced on **graph assist** (`sensitivity_ceiling`) and **daemon query classified egress**. Tensor commons lane requires Public (`daemon_tensor.rs`).
- **`nlp/*` does not read or write the sensitivity byte.** Sealing via `text_span::annotation_quin` currently XORs `content_hash ^ source_hash` into `NQuin.context`. `get_sensitivity_byte` reads context bits 56–63, so those hash bits are **misread as a sensitivity class** (pseudo-random 0x00–0x0F). That is an **ABI overlap**, not merely a missing stamp: some annotation Quins will look Classified to `filter_classified`, and `set_sensitivity_byte` would clobber hash bits. NLP-903 must separate span identity from the sensitivity byte before stamping Restricted. UC-A gold is Restricted because **source text contains the principal name**; the lexicon DID is a linkage fact, not the classification driver.

**Gap:** emission hosts (NLP-903 / graph commit) must **not** treat current `annotation_quin` context as a sensitivity stamp. Fix the overlap, then set `SENSITIVITY_RESTRICTED` on UC-A notes and never put source text in receipts. Current analyze path is local and does not egress. Classified clinical context remains out of Alpha. Do not add a second vocabulary registry (reuse `query/lexicon.rs`, `nlp/terms.rs`, and `lexicon_manifest`).

### 3.11 Adjacent NLP-shaped code (inventory only; deferred by charter)

| Module | Path | Suitability |
|---|---|---|
| Sentiment | `crates/qualia-core-db/src/research/sentiment.rs` | Deterministic lexical valence. Charter §6.3 **deferred** unless promoted with a new ID. Not Alpha. |
| Frames / relations | `nlp/frame.rs`, `nlp/relation.rs` | Tiny trigger/pattern demos. Not FrameNet / OpenIE. |
| Coref | `nlp/coref/` | Bounded exact-string + experimental pronoun sieve. Resource-repaired; quality UNEVALUATED. |

---

## 4. Candidate comparison (NLP-009 deliverable)

Three families for Alpha `symbolic-lite` versus later learned work. Quality cells are **UNEVALUATED** unless a receipt exists. No weights downloaded.

| Family | In-tree today | Licensing | Resident-byte envelope vs 42 MiB Sentinel | Quality tradeoff | Deployability on this Windows 10 x86_64 workstation |
|---|---|---|---|---|---|
| **1. Deterministic rules / dictionary / FST** (`nlp/*`) | **Yes.** Tokenize, sentences, 7-entry Aho-Corasick gazetteer, ISO dates + static units, `FstDict`, plans, optional coref/frames/relations. | Project code. Default lexicon includes a principal DID (Restricted notes). GeoNames ontology CC-BY 3.0 is **not** used by this path. | **Fits.** No learned weights. Fixture source 99 bytes; transport cap 256 KiB; gazetteer automaton is seven patterns. Workspace still counts toward Sentinel if hosts allocate, but there is no weight RSS. | **Alpha-capable extraction of known surfaces and ISO/units. Not general NER/POS.** Linguistic quality vs Stanford/Stanza/spaCy: **UNEVALUATED** (no NLP-006 `minimum_score`). Unit tests + NLP-005 gold exist; those are not F1. | **Yes, now.** Native `qualia-core-db` on this i7-6700 / ~64 GiB RAM class. `wasm-ontology` compile is a charter compile gate, not a browser runtime cert. No GPU required. |
| **2. Compact statistical sequence models (CRF / HMM)** | **HMM yes, CRF no, POS/NER tagger no.** Discrete HMM: `crates/qualia-core-db/src/solvers/learning/sequential/hmm.rs` (`Hmm::{log_likelihood, viterbi}`, `baum_welch`). Host: `MachineLearning.hmm_baum_welch` (`poet_host/invoke/ml/fitters2.rs`). `specialized_libs/machine_learning` has Linear SGD and GGUF embedding **preview** — **no CRF, no UPOS emission, no feature functions, no BIO scheme.** | In-tree Qualia. A future CRF must be written here (license = project) or a **named** artifact with a recorded license. **Do not vendor Python CRF/CRFsuite/sklearn-crfsuite.** | A compact CRF **could** be designed under 42 MiB; **none was measured** because none exists as an NLP component. Generic HMM parameter `Vec`s are unbounded unless a caller budget is added. | Adjacent HMM is for **discrete observation sequences** (solvers/learning), not English UPOS. Using it as a tagger without features/gold is not a quality path. **UNEVALUATED.** Later option **only after** an in-tree CRF/tagger + NLP-006. | HMM kernels compile on this host. That is **not** a POS deploy. Workstation class is sufficient for a future compact CRF; phone/WASM would need a separate envelope. |
| **3. Learned encoder (GGUF/P64 on existing wgpu path)** | **LLM decode yes; NLP encoder no.** GGUF mmap + P64 + `PreparedDecodePlan` + DirectML/wgpu. Token-embedding row lookup exists. No encoder-only plan, no token-classification head, no measured NLP GGUF in this tree. | Per-artifact (GGUF/P64 license + model card). **None loaded this session.** BERT/Stanza/UDPipe packages are **not** in-tree and were **not** fetched. | **No receipt that any BERT / UDPipe / Stanza / MiniLM checkpoint fits in 42 MiB** (weights + activations + tokenizer + workspace + device buffers). mmap does not exempt RSS. Host 64 GiB RAM **does not** satisfy Sentinel. Do not claim it. | LLM next-token decode is the wrong task for POS/NER/coref. Encoder-for-NLP quality is **UNEVALUATED** (no artifact, no gate). | Workstation **can** run in-process GGUF **decode** when a GGUF is present (historical LLM work on this class of box). That is **not** NLP-306. wgpu adapter **not measured** this session (two WMI GPUs: NVIDIA RTX A2000 12GB name-string + Intel HD 530). Starting NLP-306 on this machine still requires NLP-100 `DONE` + NLP-006 gates + principal license action. |

**Benchmark-only upstream toolkits** (CoreNLP, Stanza, spaCy) may later run as **reference processes** (NLP-006). They are not deployable Qualia components until budget + license + Sentinel evidence exist.

---

## 5. Quality / resource table (baseline + families)

No F1 scores are invented. Latency for the fixture was **not timed** this session (no extra `qualia-core-db` compile; user-permitted UNEVALUATED).

| Row | Task / path | Input | Resource envelope | Quality evidence | Status |
|---|---|---|---|---|---|
| **B0 — current `analyze_document` / tokenize+gazetteer** | Year-one pipeline `nlp::analyze_document` → tokenize, split_sentences, `Gazetteer::default().find`, `link::filter_known`, `normalize_dates_and_numbers`, emit plans | NLP-005 `docs/benchmark-datasets/nlp/qualia-catchment-notes-v0/documents/north-spring-fixture-001.txt` (**99 bytes** UTF-8). Same string as charter UC-A / `nlp/mod.rs` fixture. | No weights. Source 99 B ≪ 256 KiB transport ≪ 42 MiB Sentinel. Peak RSS **not measured**. Native p95 **UNEVALUATED** (charter engineering target &lt; 10 ms is not a receipt). | Unit test `nlp::tests::year_one_pipeline_on_catchment_paragraph`: `token_count >= 10`, `sentence_count >= 2`, `plans.len() >= 4`, `source_hash != 0`. Gold file `…/gold/north-spring-fixture-001.json` lists gazetteer/date/quantity spans — **not scored here**. **No F1.** | **UNEVALUATED** quality and latency; **smoke-capable** on unit tests |
| F1 symbolic-lite | Same as B0, Alpha profile | UC-A / UC-B | Weights = 0 | Token/sentence/gazetteer/ISO vs v0 gold: wait for NLP-006 `minimum_score` | Alpha **target**, not certified |
| F2 compact statistical | N/A (no NLP CRF) | — | — | — | **Gap** |
| F3 learned encoder | N/A (no NLP GGUF loaded) | — | Resident bytes **UNEVALUATED**; cannot claim &lt; 42 MiB | — | **Blocked** on NLP-100 DONE + NLP-006 + NLP-306 + license |

Gold tokenisation quirk (do not “fix” in this package): `north-spring-fixture-001.json` records the last token as `"15."` kind `Number` (period glued). That is current tokenizer behavior, not a quality claim.

---

## 6. What this inventory does not authorize

- Download of UD, OntoNotes, GUM, FrameNet, WordNet TTL, GeoNames dumps, GGUF, P64, BERT, UDPipe, or Stanza.
- Claiming BERT / UDPipe / Stanza fit in 42 MiB.
- Starting NLP-306 or wiring `llm_agent` into `analyze_document`.
- Treating FNV-1a `TextEmbedder`, Jaccard “cross-encoder”, GeoNames OWL, or WordNet `.q42` as NER/POS.
- Marking NLP-009 `DONE` (implementer must not).
- Substituting WMI GPU names for a wgpu adapter receipt.

---

## 7. Recommended next package

**NLP-006** — freeze dataset-specific gates on `qualia-catchment-notes-v0` / `qualia-english-notes-v0` where gold coverage exists; leave UD/coref/SRL **UNEVALUATED**; no Stanford comparison claim.

Keep **symbolic-lite** as Alpha. Do **not** start NLP-306 until NLP-100 is independently `DONE` and those NLP-006 gates exist. Compact statistical remains a **later in-tree CRF** option, not a Python vendor.

**Handoff:** path `docs/work-in-progress/NLP_REUSE_INVENTORY_WIP.md`. Tracker row / `NOTICES.md` are supervisor-owned.
