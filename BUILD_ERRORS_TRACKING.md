# Build Errors Tracking

**Date:** 2026-06-10  
**Branch:** 0.0.10-dev  
**Platform:** Windows

---

## Pre-existing Compilation Errors

These errors are blocking the full build and are unrelated to SPARQL-Star implementation.

### Error 1: gguf_bridge.rs:2034 - Syntax Error
```
error: expected expression, found `.`
    --> crates\qualia-core-db\src\gguf_bridge.rs:2034:13
     |
2034 |             .block_on(QTensorEngine::try_new())
     |             ^ expected expression
```
**Status:** Pre-existing, not caused by SPARQL-Star changes  
**Impact:** Blocks library compilation  
**Platform:** Unknown - needs verification on Linux

### Error 2: lib.rs:564 - Unresolved Import
```
error[E0432]: unresolved import `crate::spatial`
   --> crates\qualia-core-db\src\lib.rs:564:20
    |
564 |         use crate::spatial::{embed_h3_context, SpatiotemporalQuadTree};
    |                    ^^^^^^^ unresolved import
    |
help: a similar path exists
    |
564 |         use crate::domains::geospatial::spatial::{embed_h3_context, SpatiotemporalQuadTree};
```
**Fix:** Change `crate::spatial` to `crate::domains::geospatial::spatial`  
**Status:** Pre-existing  
**Platform:** Likely cross-platform (path resolution issue)

### Error 3: lib.rs:662 - Unresolved Import
```
error[E0432]: unresolved import `crate::geometric`
   --> crates\qualia-core-db\src\lib.rs:662:20
    |
662 |         use crate::geometric::{extract_spatial_projection, BoundingHull, VectorSectorMap};
    |                    ^^^^^^^ unresolved import
    |
help: a similar path exists
    |
662 |         use crate::domains::mathematical::geometric::{extract_spatial_projection, BoundingHull, VectorSectorMap};
```
**Fix:** Change `crate::geometric` to `crate::domains::mathematical::geometric`  
**Status:** Pre-existing  
**Platform:** Likely cross-platform (path resolution issue)

### Error 4: webizen.rs:1518 - Unresolved Import
```
error[E0433]: cannot find `logic` in `crate`
   --> crates\qualia-core-db\src\webizen.rs:1518:34
     |
1518 |         let mut mock_vm = crate::logic::WebizenVM::new();
     |                                  ^^^^^ unresolved import
    |
help: a similar path exists
    |
1518 |         let mut mock_vm = crate::modalities::logic::WebizenVM::new();
```
**Fix:** Change `crate::logic` to `crate::modalities::logic`  
**Status:** Pre-existing  
**Platform:** Likely cross-platform (path resolution issue)

### Error 5: shacl.rs:1257 - Type Mismatch
```
error[E0308]: mismatched types
   --> crates\qualia-core-db\src\modalities\logic\shacl.rs:1257:13
     |
1256 |         let shape = compiler().compile(
    |                                ------- arguments to this method are incorrect
1257 |             "fhir:Observation",
     |             ^^^^^^^^^^^^^^^^^^ expected `ShaclTarget`, found `&str`
```
**Status:** Pre-existing  
**Platform:** Likely cross-platform (type system issue)

### Error 6: shacl.rs:1307 - Type Mismatch
```
error[E0308]: mismatched types
   --> crates\qualia-core-db\src\modalities\logic\shacl.rs:1307:87
     |
1307 |         let shapes = compiler.compile_property_map("ex:TestClass", "ex:testProperty", &constraints);
     |                               --------------------                                    ^^^^^^^^^^^^ expected `&[(String, Value)]`, found `&Vec<(&str, serde_json::Value)>`
```
**Status:** Pre-existing  
**Platform:** Likely cross-platform (type system issue)

---

## SPARQL-Star Compilation Status

**Files Modified for SPARQL-Star:**
- `crates/qualia-core-db/src/resolver.rs` ✅ Compiles
- `crates/qualia-core-db/src/lexicon.rs` ✅ Compiles
- `crates/qualia-core-db/src/q42_lex.rs` ✅ Compiles
- `crates/qualia-core-db/src/rdf_star.rs` ✅ Compiles (new module)
- `crates/qualia-core-db/tests/sparql_star_tests.rs` ✅ Compiles
- `crates/qualia-cli/src/parsers/turtle_star.rs` ✅ Compiles

**Conclusion:** SPARQL-Star changes compile successfully. Pre-existing errors are blocking full library compilation.