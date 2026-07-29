---
created: 2026-07-29
updated: 2026-07-29
branch: 0.0.28
status: Implemented and browser-verified
---

# Webizen Semantic Library implementation

## Outcome

Hypermedia is now a first-class application surface rather than a feature hidden inside
“Lived Memory”. Naturalised mode opens a semantic file explorer; Advanced Technical mode
retains the existing CML, COF, observer, spatial, legislation, facet, export, and sharing
workbench.

The implementation reuses the existing `HypermediaStore`, ontology processors, ingestion
commands, graph representation, QApp dispatcher, and comorbidity evaluator. It does not
introduce a second storage system or illustrative medical results.

## Naturalised information architecture

The Semantic Library uses three persistent regions:

1. Collections: Overview, All items, Documents, Ontologies, AI models, Health records,
   Finance, QApps, and Images & audio.
2. Working area: semantic search, import, collection contents, and overview cards.
3. Inspector: selected item metadata or the semantic-processing pipeline.

Collections are meaning-derived. A pathology PDF mapped with LOINC concepts appears under
Health records even when its stored processing receipt is Markdown. This is deliberately
different from a folder tree and closer to how the underlying graph works.

## Import and processing contract

The main “Import files” action opens the native file picker and routes known formats:

| Source | Existing processor invoked | Library result |
|---|---|---|
| RDF, OWL, Turtle, N3, N-Triples, JSON-LD, TriG | `ingest_ontology` | Ontology graph + provenance receipt |
| PDF | `ingest_pdf` | Extracted graph + provenance receipt |
| PNG/JPEG/WebP/GIF/TIFF | `ingest_image` | Image-derived graph + provenance receipt |
| Text/Markdown/HTML/EPUB/DOC/DOCX/ODT | `ingest_literature` | Text graph + provenance receipt |
| GGUF/P64 | Model setup workflow | Recognised without pretending the weights were activated |
| CSV/TSV/XLS/XLSX/ODS | Domain mapping required | Recognised; conversion waits for a health/finance/general profile |

Every completed processor call registers a Hypermedia provenance receipt with source URI,
semantic role, processor result, sensitivity, and section. Unknown formats fail explicitly.

## Ontology-to-Anatomy path

The Anatomy page now contains a real ontology-inference panel:

```text
Imported record → FHIR / LOINC / ICD → Qualia graph
                → comorbidity rules → Anatomy
```

The desktop command obtains the principal DID hash inside the host and calls
`eval_comorbidity_json_from_daemon`. The UI can focus the evaluation on the whole body,
Heart, Liver, Kidney, Brain, or Lungs. It renders only returned verdicts. An empty result
states that the graph has no supported interaction and directs the person to Health records;
it does not claim absence of disease.

The previous hard-coded Type 2 Diabetes, hypertension, CKD, contraindication, and trajectory
example has been removed.

## QApp launch

QApps now provide:

- task/domain/capability search;
- a Choose → Check → Run explanation;
- visible dependency labels such as Semantic Library, health ontologies, financial ontology,
  ontology/graph, or AI model;
- “Run QApp” for catalogue entries that open a working Studio canvas.

The existing `QAppDispatcher` and per-app pane definitions remain the execution layer.

## Setup context

Setup state version 3 persists an optional `SetupProfile`:

- preferred name;
- locale and timezone;
- accessibility/presentation needs;
- interests;
- preferred ontologies;
- care priorities;
- QApp goals.

The data remains local, can be left blank, and is described as a mapping/presentation
preference rather than a clinical assertion.

## Verification

- `cargo check -p webizen-studio` — passed.
- `cargo check -p webizen-desktop` — passed.
- Agent QA scoped formatting (39 files) — passed.
- Browser contract suite — 8/8 passed with console-error capture.
- Visual inspection performed for Semantic Library, filtered QApp discovery, and Anatomy
  comorbidity context.

The first UI-run failure was a test-only 404 caused by direct navigation through a static
server without SPA fallback. The contract was corrected to enter QApps through Webizen’s
navigation; the rerun passed.

## Captures

- `webizen-0.0.28-semantic-library.png`
- `webizen-0.0.28-qapps-discovery.png`
- `webizen-0.0.28-qapps-discovery-full.png`
- `webizen-0.0.28-anatomy-comorbidity-panel.png`
- `webizen-0.0.28-anatomy-inference.png`

## Remaining bounded work

This implementation exposes and connects the existing processors. Domain-specific tabular
profile authoring (for example, choosing which invoice column maps to which finance concept)
still requires a dedicated mapping dialogue; the UI therefore refuses to imply that an
unmapped spreadsheet has been converted. Model files are similarly recognised and routed to
the existing model setup/activation system rather than marked active merely because a path was
selected.
