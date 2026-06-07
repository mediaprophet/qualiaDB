# Docs Consistency Issues Log

**Status:** Working backlog  
**Date:** 2026-06-08  
**Purpose:** Track older non-SDO documentation that still needs alignment with
current QualiaDB terminology, format semantics, shipped targets, and sync-path
reality.

## What This Log Tracks

This backlog focuses on inconsistencies outside `docs/sdo-info/`, especially:

- `human identity` vs `identifier` / `auth` / `credential` terminology
- legacy `sovereign` wording where `agency` or `human-centric` language now
  fits better
- outdated desktop-target claims after the Flutter-first transition
- outdated sync-path claims where the repo now has a documented libp2p daemon
  sync implementation
- unresolved `.q42` / `.c.q42` / `.qchk` semantics in older prose
- places where generic `CBOR` wording should be distinguished from `CBOR-LD`
  semantic payload intent

## Addressed In This Pass

These files were updated directly in this pass:

- [ARCHITECTURE.md](/C:/Projects/qualiaDB/ARCHITECTURE.md:98)
- [docs/manuals/ARCHITECTURE.md](/C:/Projects/qualiaDB/docs/manuals/ARCHITECTURE.md:35)
- [docs/manuals/flutter-api-reference.md](/C:/Projects/qualiaDB/docs/manuals/flutter-api-reference.md:225)
- [docs/manuals/webizen-protocol-rfc.md](/C:/Projects/qualiaDB/docs/manuals/webizen-protocol-rfc.md:1)
- [docs/api.html](/C:/Projects/qualiaDB/docs/api.html:278)
- [docs/protocol-integration-architecture.md](/C:/Projects/qualiaDB/docs/protocol-integration-architecture.md:1)

## Open Issues

### Priority 1

- [ARCHITECTURE.md](/C:/Projects/qualiaDB/ARCHITECTURE.md:88)
  BIDX prose still says subject-hash range coverage, while the internal SDO
  draft records an implementation/docs mismatch around object-hash indexing.
  This needs one canonical repo-wide statement before further cleanup.

- [docs/manuals/ARCHITECTURE.md](/C:/Projects/qualiaDB/docs/manuals/ARCHITECTURE.md:13)
  The manual still compresses several unresolved storage and sync semantics into
  a simplified story. It needs a second pass once `.q42` and sync profiles are
  frozen more formally.

- [docs/manuals/webizen-protocol-rfc.md](/C:/Projects/qualiaDB/docs/manuals/webizen-protocol-rfc.md:1)
  The opening terminology is now improved, but the rest of the draft still
  needs a deeper rewrite so `human identity`, `identifier nyms`, credentials,
  and transport profiles stay distinct throughout.

- [docs/manuals/developing-qapps.md](/C:/Projects/qualiaDB/docs/manuals/developing-qapps.md:13)
  The first four sections are still Tauri-heavy historical material. The file
  should either be split into `legacy-tauri-qapps.md` plus a Flutter/Qapp-Vault
  guide, or rewritten so the active Flutter-first path leads.

- [docs/manuals/DEVELOPMENT.md](/C:/Projects/qualiaDB/docs/manuals/DEVELOPMENT.md:28)
  Still front-loads legacy Tauri desktop build flow. Needs a Flutter-first
  development path and clearer partitioning of legacy instructions.

### Priority 2

- [docs/manuals/glossary.md](/C:/Projects/qualiaDB/docs/manuals/glossary.md:108)
  Several glossary entries still use `identity` in places that should probably
  become `identifier`, `identifier material`, or `credential` depending on the
  exact concept.

- [docs/manuals/adr/004-sentinel-to-webizen-rebrand.md](/C:/Projects/qualiaDB/docs/manuals/adr/004-sentinel-to-webizen-rebrand.md:12)
  Still uses `sovereign` language and would benefit from a terminology pass to
  align with `agency` and `human-centric` wording.

- [docs/manuals/adr/005-dns-frontdoor-and-hcai-agreements.md](/C:/Projects/qualiaDB/docs/manuals/adr/005-dns-frontdoor-and-hcai-agreements.md:7)
  Still describes QualiaDB as a `sovereign vault`. Should be reframed as a
  human-centric, agency-preserving vault environment.

- [docs/manuals/adr/0004-bilateral-guardianship-scrubbing.md](/C:/Projects/qualiaDB/docs/manuals/adr/0004-bilateral-guardianship-scrubbing.md:26)
  `health/identity solutions` wording should be checked against the new
  identity-vs-identifier distinction.

- [docs/api.html](/C:/Projects/qualiaDB/docs/api.html:278)
  The highest-signal tiles are fixed, but the rest of the page should get a UI
  copy pass for `identity`, `credentials`, and legacy Tauri references.

- [docs/api-explorer/catalog.js](/C:/Projects/qualiaDB/docs/api-explorer/catalog.js:1138)
  Many generated/help summaries still use older terminology around identity,
  credentials, and transport. Needs a targeted terminology sweep.

- [docs/RESOURCE_CATALOG.md](/C:/Projects/qualiaDB/docs/RESOURCE_CATALOG.md:69)
  Contains older `sovereignty` wording that should be reviewed against the
  current human-centric / agency language.

- [docs/protocol-integration-architecture.md](/C:/Projects/qualiaDB/docs/protocol-integration-architecture.md:31)
  Still positions GUN/WebRTC as core integration surfaces without a full pass
  reconciling them with the implemented libp2p daemon sync profile.

- [docs/manuals/tax-oracle-spec.md](/C:/Projects/qualiaDB/docs/manuals/tax-oracle-spec.md:16)
  and broader network/security docs will likely need a later pass if the
  WireGuard / TLS SAN / IPv6 semantic-enrichment direction becomes formalized.
  That layer distinction is not yet documented outside the newer protocol work.

### Priority 3

- [docs/index.html](/C:/Projects/qualiaDB/docs/index.html:122)
  Marketing language still uses `sovereignty`. Needs a product-copy pass rather
  than a purely technical edit.

- [docs/cost_model.html](/C:/Projects/qualiaDB/docs/cost_model.html:249)
  Product copy includes `sovereignty requirements` and `KeyVault identity`
  wording that may need alignment with `agency` and `identifier`.

- [docs/docuquin-pipeline.html](/C:/Projects/qualiaDB/docs/docuquin-pipeline.html:112)
  Marketing copy uses `sovereign knowledge`.

- [docs/hard-sciences-showcase.md](/C:/Projects/qualiaDB/docs/hard-sciences-showcase.md:58)
  Still describes the Tauri desktop as the main environment.

- [docs/UI_IMPROVEMENTS.md](/C:/Projects/qualiaDB/docs/UI_IMPROVEMENTS.md:4)
  Still framed around the Tauri + React desktop application rather than the
  current Flutter-first desktop direction.

## Suggested Next Order

1. Finish the legacy-target cleanup:
   - `docs/manuals/developing-qapps.md`
   - `docs/manuals/DEVELOPMENT.md`
   - `docs/hard-sciences-showcase.md`
   - `docs/UI_IMPROVEMENTS.md`
2. Do a repo-wide terminology pass for:
   - `human identity`
   - `identifier`
   - `auth`
   - `verifiable claim`
   - `verifiable credential`
3. Reconcile older sync prose with:
   - [docs/sdo-info/qualia-sync-protocol.md](/C:/Projects/qualiaDB/docs/sdo-info/qualia-sync-protocol.md:1)
4. Reconcile older storage prose with:
   - [docs/sdo-info/q42-format-internal-draft.md](/C:/Projects/qualiaDB/docs/sdo-info/q42-format-internal-draft.md:1)
