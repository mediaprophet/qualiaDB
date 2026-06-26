# qualia-semantic-library

A Rust-native semantic library for turning a large, messy document corpus
(Timothy's ~60 GB of math / physics / logic / CS PDFs) into an organised,
searchable knowledge base that feeds QualiaDB.

This is an **offline developer tool**. The only model dependency is reached over
HTTP behind a swappable trait (`LlmBackend`) — Ollama today, a hosted API or the
native QualiaDB engine later. It is **never compiled into QualiaDB's core
inference path**; the project's native-wgpu rule is preserved.

## The unit of storage: the `.hmc` hypermedia container

Each document becomes one `.hmc` file — a **ZIP** (fast compression, random
access, readable by every tool) that bundles the original source *and every
derived asset* behind one self-describing `manifest.json`:

```text
manifest.json              # the index (HmcManifest): ids, hashes, assets, status
source/<original>          # verbatim source bytes (never modified)
derived/document.html      # canonical structured HTML
derived/document.txt       # plain text
derived/chunks.jsonl       # structural chunks (heading-aware)
derived/document.cml.ttl   # CML / RDF layer (QualiaDB-ingestable)
embeddings/vectors.f32     # row-major f32 chunk embeddings (after `embed`)
```

The original and its provenance never get separated; re-running the pipeline
updates derived assets in place without touching the source.

## Pipeline

| Stage | What | LLM? |
|-------|------|------|
| ingest | hash + dedup, extract structured text/HTML, chunk, emit CML, pack `.hmc` | no |
| embed  | embed chunks via the external LLM, store the matrix in the container | yes (HTTP) |
| analyze| ask the LLM for topical tags, write them into the manifest | yes (HTTP) |
| library| catalog a directory, dedup (exact + near, on the embedding manifold) | no |
| search | rank chunks across the library by cosine to a query embedding | yes (query embed) |
| reorganize | derive a clean, browsable tree (`<category>/<title>__<id>.hmc`) | no |

## CLI

```bash
qsl ingest "C:\path\to\corpus" --out library          # build containers
qsl info     library\<id>.hmc                          # inspect a container
qsl verify   library\<id>.hmc                          # BLAKE3 integrity check
qsl embed    library --embed-model qwen3-embedding:0.6b  # SOTA retrieval embedder
qsl analyze  library --gen-model gemma4:e4b            # topical tags
qsl library  library                                   # catalog + dedup report
qsl search   library --query "defeasible logic"        # semantic search
qsl reorganize library --out library-organized --apply # tidy the tree
```

## Status (2026-06-26, verified)

- **Working & tested:** the `.hmc` format (round-trip, integrity, idempotent
  re-write), the deterministic ingest chain (acquire → extract → chunk → CML →
  pack), the LLM HTTP seam (`generate` proven live; `embed` correct), library
  dedup/search/novelty on the embedding manifold, and reorganize. `cargo test`
  green; verified end-to-end on a real PDF.
- **Needs server config:** embeddings require an Ollama started with
  `--embeddings` (or a dedicated embed endpoint/model). The seam surfaces this
  cleanly and the library degrades gracefully without vectors.
- **The quality knob:** the default extractor is pure-Rust `pdf-extract` (text
  only — no per-page markers, MathML, or OCR). Heavier extractors
  (PyMuPDF / nougat for formulas, OCR for scans) plug in behind the `extractor`
  field; each container records honestly which extractor produced it.
- **Next seam:** pushing the produced CML into the live QualiaDB graph (the TTL
  is already emitted in the acquisition vocabulary QualiaDB ingests).
