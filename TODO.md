# QualiaDB TODO / Remaining Work

**Last Updated:** 2026-06-06
**Current Branch:** `feat/resource-catalog`

This file tracks work that is **not yet complete**.

## High Priority

- [ ] Wire Resource Catalog into application startup (auto-load on launch)
- [ ] Implement full `qualia resources` CLI commands (currently stubs)
- [ ] Connect `download` command to the existing persistence/download system
- [ ] Add provenance recording when importing resources
- [ ] Create basic Tauri/React UI for browsing resources

## UI Overhaul (Desktop App)

The desktop UI (Tauri + React/Vite) needs significant work:

- [ ] Design new layout for Resource Catalog browser
- [ ] Add tabs/sections: LLMs | Ontologies | SPARQL Endpoints
- [ ] Implement search + filtering by tags, size, license, domain
- [ ] Add "Download / Import" buttons with progress indicators
- [ ] Show resource details in a side panel or modal
- [ ] Add health indicators (last verified, reliability for SPARQL)
- [ ] Support user-added custom resources
- [ ] Dark mode / accessibility improvements

## Resource Catalog Enhancements

- [ ] Support hot-reloading of YAML files
- [ ] Add checksum verification on download
- [ ] Implement optional catalog refresh from external sources (HF, BioPortal)
- [ ] Export catalog as RDF so it can be queried inside QualiaDB
- [ ] Add license policy enforcement

## Other Remaining Work

- [ ] Improve error handling in the loader
- [ ] Add unit tests for Resource Catalog
- [ ] Document how to add new resource types
- [ ] Consider making `config/resources.yaml` use TOML instead of YAML for consistency

## Notes

- Keep sovereignty as the default (no automatic external calls).
- The Resource Catalog should feel like a first-class citizen in the UI, similar to the existing download/persistence features.

---

**Next Milestone:** Complete CLI integration + basic UI browser on `feat/resource-catalog`.