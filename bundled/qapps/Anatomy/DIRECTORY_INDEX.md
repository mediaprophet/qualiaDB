---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# Anatomy Index

## Functionality Overview
Comprehensive index of functionality for `Anatomy`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[Knowledge](Knowledge/DIRECTORY_INDEX.md)`
- 📁 `[lib](lib/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `API_CAPABILITIES_ASSESSMENT.md`
- 📄 `README.md`
- 📄 `TODO.md`
- 📄 `daemon-client.js`
  - `const QualiaDaemon`
  - `function readLaunchAuth`
  - `const params`
  - `function wsUrl`
  - `function connect`
  - `const url`
  - `const text`
  - `const msgId`
  - `function sendIntent`
  - `const intentFrame`
  - `const payload`
  - `const wasmApi`
  - `function safeSendIntent`
  - `function health`
  - `const res`
  - *(...and 5 more)*
- 📄 `dicom-overlay.js`
  - `const DicomOverlay`
  - `const DEFAULT_PLACEMENT`
  - `function ensureParser`
  - `function tagValue`
  - `function numericTag`
  - `const raw`
  - `const parsed`
  - `function normalizeToken`
  - `function loadOrganMap`
  - `const response`
  - `function inferOrganFromTags`
  - `const haystack`
  - `const tokens`
  - `function getWindowedPixelArray`
  - `const rows`
  - *(...and 59 more)*
- 📄 `index.html`
- 📄 `knowledge-parser.js`
  - `const KnowledgeParser`
  - `const SYSTEM_IRI_TO_LABEL`
  - `function expandPrefixes`
  - `function parsePrefixes`
  - `const prefixes`
  - `const lines`
  - `const match`
  - `function normalizeObject`
  - `function parseTurtleJs`
  - `const triples`
  - `const blocks`
  - `const headMatch`
  - `const subject`
  - `const statements`
  - `const parts`
  - *(...and 16 more)*
- 📄 `qapp.json`
- 📄 `qualia.js`
  - `const Qualia`
  - `function log`
  - `const timestamp`
  - `const fullMsg`
  - `function setLogCallback`
  - `function tryLoadWasm`
  - `const glueCandidates`
  - `const mod`
  - `const wasmUrl`
  - `function init`
  - `function parseConditionsTtl`
  - `function loadConditionsFromUrl`
  - `const response`
  - `const text`
  - `function insertSpatialEntity`
  - *(...and 3 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
