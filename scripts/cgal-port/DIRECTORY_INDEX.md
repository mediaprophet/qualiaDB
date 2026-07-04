---
created: 2026-07-04
updated: 2026-07-04
update_scope: Comprehensive
---

# cgal-port Index

## Functionality Overview

Capability-reference workbench that fetches a pinned sparse CGAL 6.2 CC0
specification/test checkout, scans package metadata/documentation/tests/examples,
and emits a deterministic QualiaDB Rust + JSON coverage checklist for the native
implementation. CGAL is a reference and CC0 oracle only — never a source of
derived code (see `README.md`; authoritative build plan:
`docs/plans/native-computational-geometry.md`).

## File & Subdirectory Manifest

- `port_cgal.py`: Fetch, scan, normalize, and registry-generation program.
- `README.md`: Operator instructions, generated destinations, and status semantics.

## Changelog

- **2026-07-04**: Added the CGAL native-port workbench and comprehensive index.
