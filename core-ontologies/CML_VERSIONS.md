# CML schema changelog

The generated CML layer is reproducible from SOURCE at a stamped `SCHEMA_VERSION`
(see [`CML_UPGRADE.md`](CML_UPGRADE.md)). Each bump records what changed, what regenerates,
and any breaking concern. Bump `SCHEMA_VERSION`, run `tools/reprocess_library.py`, and add a
row here in the same commit.

| Version | Date | Change | Regenerates | Breaking? |
|---------|------|--------|-------------|-----------|
| **2** | 2026-06-22 | Introduced the library-upgrade protocol: every generated concept now carries `cml:schemaVersion`; added `reprocess_library.py` (full + `--check` staleness gate) and the SOURCE/GENERATED separation that preserves human curation across regenerations. | `concepts/*.n3` (adds the version stamp), then `dist/q42`, demo data, INDEX. | No — additive (a new triple per concept); content otherwise unchanged. |
| 1 | 2026-06-22 | Initial machine-derived CML concept layer (101 instruments → 3,518 concepts / 3,619 norms, all `cml:Proposed`). | (baseline) | — |
