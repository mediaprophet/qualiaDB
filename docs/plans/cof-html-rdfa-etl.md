# COF (HTML+RDFa) + CML ETL for legislation / PDF corpora

**Status:** tools landed 2026-07-11  
**Primary code:** `tools/legislation-etl/`  
**Corpus:** `C:\Users\Admin\Downloads\20260630_AU-FED-LEGISLATION` (188 PDFs)  
**Publish target (ns.webcivics style):** `webcivics/ns/.../institutions/au-fed-legislation/`

## Decisions (from principal + Gemini thread)

1. **RDF first** — Turtle/N3 for the graph; not flat MD as the system of record.
2. **CML stays the concept model** — TEXT → CONCEPT → LOGIC; Proposed vs Attested.
3. **COF is not a second ontology world** — it is an **HTML+RDFa profile** for *agent context*:
   attributes bind claims/entities to graph nodes without Markdown’s missing attribute system
   and without presentation-HTML token bloat.
4. **Ollama for volume** — page-at-a-time classification; resumable; Qualia native GGUF remains primary for product inference when ready.
5. **No images in agent docs** — waste tokens; this plan text-only.

## Dual surface per instrument

```
source.pdf
  → structure parse (deterministic)
  → optional Ollama deontic classify (per page, JSON)
  → package:
       .cml.n3     graph (cml:Proposed)
       .cml.html   HTML+RDFa COF + human CSS
       .jsonld
       manifest.json
```

## Pipeline map

| Stage | Tool | LLM? |
|-------|------|------|
| PDF text + page index | `legis2cml` / PyMuPDF | no |
| Provision boundaries | regex structure parse | no |
| Deontic classify | Ollama `/api/chat` JSON | yes |
| Graph emit | N3 + JSON-LD | no |
| Agent/human surface | HTML+RDFa COF | no |
| Later: embeddings / search | `qualia-semantic-library` (qsl) | yes |
| Later: Solid LDP + notifications | Solid-OutPost + pod delta | no |

## Batch command

```powershell
cd C:\Projects\qualia-27062026\tools\legislation-etl
python batch_au_legislation.py --limit 3 --no-llm          # smoke
python batch_au_legislation.py --resume --model llama3.2   # full
```

## Follow-ups (principal)

- [ ] Title/slug improve from Federal Register filename → short Act name  
- [ ] PDF xywh coords when using a layout-aware extractor (Docling/pymupdf blocks)  
- [ ] Ingest `.cml.n3` into QualiaDB / Solid pod containers  
- [ ] Solid Notifications → delta graph partition (as discussed)  
- [ ] Wire qsl `embed` over same Ollama host for legislation search  

## Solid-OutPost docs

Hackathon primer/protocol live under Downloads; generation scripts
(`create_hackathon_proposal*.js`) are separate from the legislation ETL.
Update those when the proposal narrative should cite COF/CML packages explicitly.
