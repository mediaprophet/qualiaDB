# Unintegrated M64 v3 prototypes

Moved from the production source tree during the quota-limited build handoff on 2026-11-13. These files were never routed in parent modules. They are incomplete, unreviewed drafts, not supported functionality. No feature was removed from the compiled module graph.

Read ../../qbs12-m64-v3-partial-map.md before reuse. Factor shared whole-reader validation instead of duplicating it; resolve cross-chunk RepeatNearest padding; finish actual authenticated-range tests. Restore deliberately, review and route only when acceptance is satisfied.

```json
[
  {
    "original": "q42/m64_matter/georaster/range.rs",
    "sha256": "3c4f148b63cef512c475794238c597a1d654dc3d553b7a2832891a9f9c4f9089",
    "bytes": 5598
  },
  {
    "original": "bundle/stream/formats/m64_v3.rs",
    "sha256": "720459b5fbf3480b399698fa210a5401aadb3750c8769046dd77adcde4b27d59",
    "bytes": 5110
  },
  {
    "original": "bundle/stream/formats/m64_v3_tests.rs",
    "sha256": "0a640305e599e73294d14ed90da682da7771acaa2f8f5c45ccd12b670bcae594",
    "bytes": 11298
  }
]
```
