# Legislation ETL → CML + HTML+RDFa COF

Turn Commonwealth PDF Acts into **ns.webcivics.net-style** packages:

| Artefact | Role |
|----------|------|
| `*.cml.n3` | CML concept graph (TEXT → CONCEPT → LOGIC), all `cml:Proposed` |
| `*.cml.html` | Human + agent surface: **HTML+RDFa COF profile** over the same graph |
| `*.jsonld` | Same graph as JSON-LD |
| `manifest.json` | Source hash, counts, model, schema version |
| `*.progress.json` | Per-page Ollama resume state |

## CML vs COF (do not conflate)

- **CML** (Context Markup Language, `https://ns.webcivics.net/cml/`) — the **graph model**: concepts, realizations, deontic norms, curation (Proposed/Attested). Lives as N3/TTL/JSON-LD and as NQuin contexts in QualiaDB.
- **COF** (Context Optimisation Format, `https://ns.webcivics.net/cof/`) — an **HTML+RDFa serialisation profile** for agent context windows: structural scaffolding (`cof:Document`, `cof:Section`, `cof:Block`, `cof:Claim`) with attributes, almost no layout bloat. Domain meaning stays on `cml:` / `values:` / `skos:`.

Markdown alone cannot attach `ref` / provenance / confidence to a span. Full presentation HTML wastes tokens. COF = constrained HTML technology + RDFa, not pretty pages.

## Prerequisites

```text
pip install pymupdf requests
ollama pull llama3.2          # or llama3.1:8b
# optional: rdflib for --emit-ttl
```

Qualia side (optional later): set desktop **Inference Backend = Ollama** so chat can use the same daemon while native GGUF is offline. ETL here talks to Ollama **directly** (batch-friendly).

## Single instrument

```powershell
cd C:\Projects\qualia-27062026\tools\legislation-etl
python legis2cml.py `
  --input "C:\Users\Admin\Downloads\20260630_AU-FED-LEGISLATION\C2004A00601.pdf" `
  --jurisdiction AU `
  --model llama3.2 `
  --out-dir "C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation\C2004A00601"
```

Structure only (no Ollama, fast smoke):

```powershell
python legis2cml.py --input path\to\act.pdf --no-llm --out-dir .\smoke-out
```

## Full corpus (188 PDFs)

```powershell
python batch_au_legislation.py --resume --model llama3.2
# smoke first 3 without LLM:
python batch_au_legislation.py --limit 3 --no-llm
```

Default out root:  
`C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation\`

## Curation prime directive

Machines **propose** only. Regeneration never writes `cml:Attested` or `skos:exactMatch`. Human attestation is a separate, signed step (as on ns.webcivics core concepts).

## Related Qualia pieces

| Piece | Path |
|-------|------|
| Optional Ollama chat harness | `crates/qualia-client-core/src/ollama_harness.rs` |
| Semantic library (.hmc) | `crates/qualia-semantic-library` |
| Plan | `docs/plans/cof-html-rdfa-etl.md` |
| COF vocab | `tools/legislation-etl/cof.n3` |
| CML vocab | `https://ns.webcivics.net/cml/` (webcivics `public/core/cml.n3`) |
