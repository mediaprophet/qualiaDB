# Resource Catalog Implementation Plan

**Branch:** `feat/resource-catalog`

This document outlines the phased implementation of the external Resource Catalog system for LLMs, Ontologies, and SPARQL endpoints.

## Goals

- Allow dynamic management of external resources without recompiling QualiaDB.
- Keep the system sovereign and offline-first by default.
- Integrate cleanly with the existing download/persistence system.
- Make the catalog itself queryable inside QualiaDB where possible.

## Current State (as of 2026-06-06)

- `resources/` directory with YAML registries created.
- Basic Rust module skeleton added.
- Configuration example in `config/`.

## Phased Implementation

### Phase 1: Foundation (Current)
- [x] Define YAML schemas and starter data
- [x] Create master `catalog.yaml`
- [x] Add Rust types matching the YAML
- [ ] Implement YAML loader using `serde` + `serde_yaml`
- [ ] Add basic CLI commands (`qualia resources list`, `qualia resources show`)

### Phase 2: Core Integration
- [ ] Wire loader into application startup
- [ ] Integrate LLM downloads with existing persistence layer
- [ ] Add ontology import path (with optional SHACL validation)
- [ ] Store resource metadata + provenance in the Super-Quin graph
- [ ] Support user-added custom resources via override file

### Phase 3: Usability & UI
- [ ] Expose resource browser in React/Vite UI
- [ ] Add filtering, search, and tagging
- [ ] "Download / Import" actions in the interface
- [ ] Show resource health/status (last verified, size, license warnings)

### Phase 4: Advanced Sovereign Features
- [ ] Optional catalog refresh tool (pulls from HF, BioPortal, LOV)
- [ ] Checksum + signature verification on download
- [ ] License policy enforcement
- [ ] Export catalog as RDF/JSON-LD (so it becomes part of the semantic graph)
- [ ] Support for local mirrors and IPFS references

## Technical Decisions

- **Format**: YAML for human editability + git friendliness. Optional RDF export later.
- **Loading**: `serde` + `serde_yaml` (or `serde_yml`).
- **Storage**: Resource metadata lives in a dedicated management graph inside QualiaDB.
- **Configuration**: `config/resources.yaml` (or TOML if preferred).

## Open Questions

- Should we support hot-reloading of the catalog while the app is running?
- How should large ontology imports be chunked / modularized?
- Do we want built-in support for content negotiation when downloading ontologies?

## Next Actions

1. Complete the YAML loader implementation.
2. Add CLI commands.
3. Integrate with the download system on `0.0.6-dev`.