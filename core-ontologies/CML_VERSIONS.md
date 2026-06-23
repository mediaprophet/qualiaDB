# CML schema changelog

The generated CML layer is reproducible from SOURCE at a stamped `SCHEMA_VERSION`
(see [`CML_UPGRADE.md`](CML_UPGRADE.md)). Each bump records what changed, what regenerates,
and any breaking concern. Bump `SCHEMA_VERSION`, run `tools/reprocess_library.py`, and add a
row here in the same commit.

| Version | Date | Change | Regenerates | Breaking? |
|---------|------|--------|-------------|-----------|
| 2 (ns) | 2026-06-23 | **Namespace migration** `https://ns.webcivics.org/` → `https://ns.webcivics.net/` (now live) across SOURCE (`*.n3`, `un-instruments/`, tools, SHACL) + engine (`crates/qualia-core-db` string literals) + the generated layer. Pure host change; **no schema-structure change** (`SCHEMA_VERSION` stays `2`). Verified: `reprocess_library.py` 221/221 volumes, 74 557 triples round-trip, counts unchanged (101 / 3 518 / 3 619); Rust gate + lib (1215) green. `sdo.webcivics.org` and website links left unchanged. | All `concepts/*.n3` + `dist/q42/*` (every `cml:`/`values:` URI `q_hash` changes). | **Hash-space**: any externally-distributed `.q42` keyed to the old-namespace hashes must be rebuilt. |
| **2** | 2026-06-22 | Introduced the library-upgrade protocol: every generated concept now carries `cml:schemaVersion`; added `reprocess_library.py` (full + `--check` staleness gate) and the SOURCE/GENERATED separation that preserves human curation across regenerations. | `concepts/*.n3` (adds the version stamp), then `dist/q42`, demo data, INDEX. | No — additive (a new triple per concept); content otherwise unchanged. |
| 1 | 2026-06-22 | Initial machine-derived CML concept layer (101 instruments → 3,518 concepts / 3,619 norms, all `cml:Proposed`). | (baseline) | — |
