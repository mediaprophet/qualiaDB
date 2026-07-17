# Legislation ETL → CML + HTML+RDFa COF

Turn Australian Commonwealth and European Union legislation PDFs into
**ns.webcivics.net-style** packages:

| Artefact | Role |
|----------|------|
| `*.cml.n3` | CML concept graph (TEXT → CONCEPT → LOGIC), all `cml:Proposed` |
| `*.cml.html` | Human + agent surface: **HTML+RDFa COF profile** over the same graph |
| `*.jsonld` | Same graph as JSON-LD |
| `*.logic.shacl.ttl` | Per-instrument closed-world shapes for proposed logic routing |
| `*.cogai.chk` | CogAI chunks for applicable logic analyses |
| `manifest.json` | Source hash, counts, model, schema version |
| `*.progress.json` | Atomic, content-addressed Ollama segment checkpoints |
| `*.ttl` | Optional RDFLib-validated Turtle projection |
| `q42/*.q42` (+ optional `.lex`) | Bounded native QualiaDB volumes |

## CML vs COF (do not conflate)

- **CML** (Context Markup Language, `https://ns.webcivics.net/cml/`) — the **graph model**: concepts, realizations, deontic norms, curation (Proposed/Attested). Lives as N3/TTL/JSON-LD and as NQuin contexts in QualiaDB.
- **COF** (Context Optimisation Format, `https://ns.webcivics.net/cof/`) — an **HTML+RDFa serialisation profile** for agent context windows: structural scaffolding (`cof:Document`, `cof:Section`, `cof:Block`, `cof:Claim`) with attributes, almost no layout bloat. Domain meaning stays on `cml:` / `values:` / `skos:`.

Markdown alone cannot attach `ref` / provenance / confidence to a span. Full presentation HTML wastes tokens. COF = constrained HTML technology + RDFa, not pretty pages.

## Prerequisites

```text
pip install pymupdf requests
ollama pull llama3.2:3b-instruct-q4_K_M
# optional: rdflib for --emit-ttl
```

The ETL talks to Ollama directly (batch-friendly). With `--emit-q42`, it invokes the real
QualiaDB semantic ingest command, writes bounded 40-provision volumes under `q42/`, and verifies
every volume with a SPARQL round trip. The bound avoids feeding a large Act to one parser stack.

## Fidelity rules (section body text)

Every `cml:Concept` for a section or subsection **must** carry `values:originalText`
(the full provision body). Container sections that have been split into subsections also
keep `full_text` (pre-split body) so exports never look “section-empty” when only the
lead-in remains on the parent. The manifest reports:

```json
"coverage": {
  "concepts": 120,
  "conceptsWithText": 118,
  "emptyConcepts": 2,
  "emptyFrags": ["sch-1-sec-40a", "..."],
  "textCoverageRatio": 0.983,
  "ok": true
}
```

`ok` is true when every concept has text, or at least 85% do (amendment schedules sometimes
have heading-only repeal stubs). Re-run the ETL after upgrading — older flat `.n3` dumps
in the corpus that list concepts without `values:originalText` were produced before this
fix and should be regenerated with `--no-llm` (structure) or full Ollama enrichment.

## Native Qualia hypermedia path (Rust only — preferred product path)

Python remains a **batch ETL** for ns.webcivics packages. The **product Library** does not
call Python. Inside Qualia:

| Surface | Module / API |
|---------|----------------|
| Structure parse | `wellfair::legislation_ingest` |
| CML context graph (TEXT→CONCEPT→LOGIC) | `wellfair::cml_context` |
| Host | `wellfair_ingest_legislation_text`, `wellfair_ingest_legislation_pdf_hex`, `wellfair_build_cml_context`, `wellfair_enrich_library_cml` |

On ingest, every provision gets:

- full body text  
- deterministic **deontic** class → real deontic `NQuin` norms  
- **privacy / GDPR-family** signals (consent, erasure, DPIA, cross-border, …)  
- **rights** / **temporal** / cross-ref cues  
- proposed CML N3 (`cml:Proposed` only) + facet tags for Library search  
- **COF HTML+RDFa** (`cof/profile/html-rdfa-1`): agent-lean segments for token budgets  

### COF segmentation (token optimisation)

Large instruments are **not** one giant HTML dump. The Rust COF packer emits:

| Segment | Role |
|---------|------|
| `cof-seg-0` (index) | Token-cheap TOC: titles + deontic/privacy chips, no full bodies |
| `cof-seg-1…N` | Section-aligned body packs under ~24k chars (~6k tokens) each |

Agents should load **index + only the body segment(s)** matching the query. RDFa
`typeof` / `property` / `resource` / `rel` carry the CML edges — do not strip them.
Host: `wellfair_build_cof_package`. Media type:
`text/html;profile="https://ns.webcivics.net/cof/profile/html-rdfa-1"`.

General text documents in the Library use the same `cml_context` + COF path.

## Units: metadata, intro, sections, subsections

A document is decomposed into its **natural** units, not fixed-size character slices:

| Unit | Graph output | Classified? |
|------|--------------|-------------|
| **Metadata** — title, `No. X of YYYY`, jurisdiction, in-force date | instrument node (`cof:Document` with `dc:title` / `dc:identifier` / `dc:date` / `dc:description`) | no — extracted |
| **Intro** — the long title (`An Act to …`) | `dc:description` on the instrument node | no |
| **Part / Division / Schedule / EU Chapter** | structural markers + hierarchy | no |
| **Section / EU Article** (no subsections) | `cml:Concept` + norm + logic | yes |
| **Section** (has subsections) | `cml:Concept` + `cml:hasPart` → its subsections (no norm of its own) | no |
| **Subsection** `(1)`,`(2)` | `cml:Concept` + norm + logic, `values:partOf` its section | yes |

Classifying at **subsection** granularity distinguishes a section's rule from its exception or
penalty (e.g. `s5(1)` Obligation vs `s5(2)` Permission) instead of collapsing them into one label.
Paragraph (`(a)`/`(b)`) granularity is intentionally not used (≈4× cost, little gain). Long sections
split at whole-subsection boundaries, never mid-clause; short sections stay whole. `num_ctx` is sized
to the segment budget (Ollama's 2048 default silently truncates the largest excerpts otherwise).

## Structure parsing (amending Acts and schedules)

- Front matter is skipped: for instruments with an enacting formula (*"The Parliament of Australia
  enacts:"*), parsing begins after it, so the table of contents / arrangement of sections is not
  read as body. That table otherwise produced phantom empty Parts/Divisions and collided the
  arrangement's section numbers with the real ones.
- `Schedule N—Title` headings are recognised as structural markers (the em-dash is required so
  commencement-table rows and cross-references that merely start with "Schedule N" are not matched;
  a repeated running page-header for the same schedule is de-duplicated). Fragments inside a schedule
  are namespaced `sch-N-…` so amendment-item numbers never collide with the principal instrument's.
- A `Part`/`Division` line whose title starts lowercase (e.g. *"Division 2 of Part III covers…"*,
  *"Part XI of the Crimes Act 1914"*) is prose or a cross-reference, not a heading, and stays body text.
- Older Acts with no enacting formula parse exactly as before.

## Segmentation and restart safety

- Sections are extracted across page boundaries before LLM work begins.
- Each reasoning request contains one provision by default (up to 8,000 characters); unusually
  long provisions are split into overlapping parts. This keeps the full-suite JSON Schema reliable
  on the bundled 3B local model. Larger models can raise `--max-segment-items`.
- Long sections are split at paragraph/sentence boundaries with 500 characters of overlap.
- Segment IDs hash the exact excerpt payload. Only schema-valid, complete Ollama responses are
  checkpointed; timeouts, malformed JSON, and missing provision keys remain pending. A
  *contentless* logic application (empty summary, e.g. the model's honest confidence-0.0 output on
  amendment-machinery fragments like `Insert:` / `Add:`) is dropped, not treated as a malformed
  response — the provision keeps its `Undertaking` classification instead of failing the whole file.
- Retries on a failed segment add a little sampling (attempt 0 stays deterministic at temp 0.0) so a
  genuinely hard segment is re-explored rather than re-failed identically.
- Checkpoints and batch manifests are written atomically. `--resume` reuses them only when the
  PDF hash, model, and segmentation settings still match.

## Full CML logic suite

Every bounded excerpt is explicitly considered for the registered suite below. The model returns
only subject-matter-applicable analyses; omission is preferred to forcing an irrelevant logic.
Every emitted `cml:LogicApplication` is source-backed, confidence-weighted, routed to a real
QualiaDB execution surface, and remains `cml:Proposed`.

The multimodal checkpoint contract is progress version 5. On the first `--resume` after this
upgrade, older deontic-only progress is archived as `*.progress.stale.json` and reprocessed so
the additional logic families are genuinely considered rather than silently reported as absent.

| CML modality | QualiaDB surface |
|---|---|
| Deontic | `modalities::logic::deontic` |
| Epistemic | `modalities::epistemic` |
| LTL | `modalities::temporal_ltl` |
| Paraconsistent | `modalities::paraconsistent` |
| Answer Set Programming | `modalities::asp` |
| Dialectical | `modalities::dialectical` |
| Linear logic | `modalities::linear` |
| Description logic | `modalities::dl` |
| Argumentation | `modalities::argumentation` |
| Allen intervals | `modalities::interval_reasoning` |
| Diffusion | `modalities::diffusion` |
| CogAI | `sparql_library::parsers::chk_parser` |
| SHACL | `modalities::logic::shacl` |
| N3Logic | `modalities::logic::n3_compiler` |

N3 remains the primary CML graph carrier. SHACL validates the routing envelope; CogAI chunks expose
the proposed premise/action view; native Q42 compilation makes the graph queryable. These artefacts
route analyses to evaluators—they do not misrepresent an LLM proposal as a completed formal proof.

## Single instrument

```powershell
cd C:\Projects\qualia-27062026\tools\legislation-etl
python legis2cml.py `
  --input "C:\Users\Admin\Downloads\20260630_AU-FED-LEGISLATION\C2004A00601.pdf" `
  --jurisdiction AU `
  --model llama3.2:3b-instruct-q4_K_M `
  --out-dir "C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation\C2004A00601" `
  --resume --emit-ttl
```

Structure only (no Ollama, fast smoke):

```powershell
python legis2cml.py --input path\to\act.pdf --no-llm --out-dir .\smoke-out
```

EU Regulations use their official ELI as provenance. Recitals remain in the source PDF while the
operative Chapter/Article structure is emitted for classification:

```powershell
python legis2cml.py `
  --input path\to\eli_reg_2016_679_oj_EN_TXT.pdf `
  --title "General Data Protection Regulation (EU) 2016/679" `
  --jurisdiction EU `
  --base-iri "https://ns.webcivics.net/values/eu/32016r0679" `
  --eli "http://data.europa.eu/eli/reg/2016/679/oj" `
  --resume --emit-ttl --emit-q42
```

## Full corpus (222 PDFs at 2026-07-15)

```powershell
python batch_au_legislation.py --resume
# Ollama + RDF validation + native QualiaDB volumes (build qualia-cli first):
cargo build -p qualia-cli
python batch_au_legislation.py --resume --emit-ttl --emit-q42
# smoke first 3 without LLM:
python batch_au_legislation.py --limit 3 --no-llm
# cap per-PDF wall-clock (resume-safe: checkpoints survive a kill); 0 = no limit:
python batch_au_legislation.py --resume --file-timeout 1800
```

The batch **streams each PDF's per-segment progress live** (the child runs unbuffered), so a large
Act no longer looks hung while it works. `Ctrl+C` stops the child cleanly and leaves the checkpoints
intact — rerun with `--resume`.

Default out root:  
`C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation\`

## Legislation-page legal information

Generated `*.cml.html` pages deliberately use a stronger information notice than the site's
human-rights instrument pages. Each page states that it is a technical alpha, is not legal advice or
an official/authorised version, may contain machine-transformation errors, and represents only the
identified point-in-time source. The page links its Federal Register ID or EU ELI directly to the official record
and separates three rights scopes:

1. source legislation and its Federal Register or EUR-Lex terms (subject to exceptions);
2. technical work copyright Timothy Charles Holborn under CC BY-NC-ND 4.0; and
3. affirmative permission for automated retrieval, indexing, semantic parsing, grounding, inference,
   and agent-assisted research, while model-training permission is not granted.

The site publishes the same distinction for agents at `/ai-use-policy.json` and in `llms.txt`.

After changing notice copy or styles, refresh already-generated HTML without reparsing PDFs or invoking
Ollama:

```powershell
python refresh_legal_notices.py
```

## Curation prime directive

Machines **propose** only. Regeneration never writes `cml:Attested` or `skos:exactMatch`. Human attestation is a separate, signed step (as on ns.webcivics core concepts).

## Related Qualia pieces

| Piece | Path |
|-------|------|
| Optional Ollama chat harness | `crates/qualia-client-core/src/ollama_harness.rs` |
| Native semantic ingest/query | `qualia-cli ingest semantic` / `qualia-cli query sparql` |
| Semantic library (.hmc) | `crates/qualia-semantic-library` |
| Plan | `docs/plans/cof-html-rdfa-etl.md` |
| COF vocab | `tools/legislation-etl/cof.n3` |
| CML vocab | `https://ns.webcivics.net/cml/` (webcivics `public/core/cml.n3`) |
