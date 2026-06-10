# QualiaDB TODO / Remaining Work

**Last Updated:** 2026-06-10  
**Current Branch:** `0.0.10-dev`

This file tracks remaining work after the build error resolution phase.

---

## Completed Tasks

### Resource Catalog
- [x] Wire Resource Catalog into application startup
- [x] Implement full `qualia resources` CLI commands
- [x] Connect `download` command to persistence system
- [x] Add provenance recording when importing resources
- [x] Create UI for browsing resources (Flutter LLM Hub, Ontology Hub implemented)

### Build System
- [x] Resolve all 82 build errors
- [x] Fix tokio runtime nesting issues
- [x] Complete module reorganization

---

## Remaining Implementation Tasks

See [to-do/](to-do/) for detailed implementation tasks:

### Security (Critical Priority)
- [ ] Fix zk_proofs.rs stub (verify_proof returns true) - see to-do/001
- [ ] Fix fiduciary_crypto.rs signature verification - see to-do/002
- [ ] Fix ML-DSA hand-rolled crypto (use FIPS 204) - see to-do/003
- [ ] Fix verify_ecc_parity mock - see to-do/004

### Functionality (High Priority)
- [ ] Fix query layer stubs (mmap_query_subject, lazy_superblock_query, indexing) - see to-do/005
- [ ] Fix wgpu/Vulkan mock pipeline (use real fused_transformer shader) - see to-do/006

### Documentation
- [ ] Update remaining documentation files to match current state - see to-do/009

---

## UI Enhancements (Future Work)

The desktop UI (Flutter) is functional but could be enhanced:

- [ ] Improve Resource Catalog UI with search and filtering
- [ ] Add progress indicators for downloads
- [ ] Add health indicators for SPARQL endpoints
- [ ] Support user-added custom resources
- [ ] Dark mode improvements

---

## Notes

- Keep sovereignty as the default (no automatic external calls)
- The Resource Catalog is now a first-class citizen in the system
- Build status: 0 errors, 539 test functions in qualia-core-db
- Version: 0.0.10-dev