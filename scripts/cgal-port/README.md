# CGAL capability-reference workbench for QualiaDB native computational geometry

This workbench tracks CGAL's package **surface as a public-domain coverage
reference** — a completeness checklist for the substantially different native
Rust implementation under
`crates/qualia-core-db/src/specialized_libs/computational_geometry`. The native
library is built on the QualiaDB / Webizen engine (10-D tensor, `.10d` container,
`wgpu`/Forge, WASM, renderer); it is **not a transliteration of CGAL**. See the
authoritative build plan: `docs/plans/native-computational-geometry.md`.

It does not embed C++, generate an FFI wrapper, or copy headers. The generated
registry records every upstream package, dependencies, documentation/test
coverage, upstream licence metadata, and native implementation status. Geometry
kernels use caller-owned slices and POD layouts so one implementation can serve
native, WASM, the 10-D manifold, graph reasoning, renderer, and qapp tools.

**Licence discipline (load-bearing):** CGAL's algorithm *source* (the release
tarballs' `include/`/`src/`) is **GPL/LGPL and is never copied or derived from**.
CGAL's `doc/` and `test/` are **CC0** — used as the specification and as
golden-output oracles for the native (clean-room) implementation. Reference and
validation only, never derivation.

```powershell
python scripts/cgal-port/port_cgal.py --fetch
cargo test -p qualia-core-db computational_geometry
```

The pinned input is CGAL `v6.2`. `--fetch` creates a blob-filtered sparse
checkout in the operating-system temporary directory containing package
metadata, documentation, tests, and examples. Generated outputs live in:

- `crates/qualia-core-db/src/specialized_libs/computational_geometry/generated/`
- `resources/cgal-port/cgal-6.2-packages.json`

To mark a package complete, add its native module and tests, then update
`PORTED_PACKAGES` in `port_cgal.py` and regenerate. `Foundation` means the
shared capability exists but the complete named upstream API surface is not yet
ported; `Ported` means its documented operations and conformance cases are
covered.
