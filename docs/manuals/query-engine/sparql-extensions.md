# SPARQL Extensions Reference

> **QualiaDB Implementation Status — 2026-06-11**
>
> | Extension | Status | Module |
> |-----------|--------|--------|
> | SPARQL 1.1 UPDATE | ✅ Implemented | `sparql_update.rs` |
> | SPARQL-Star / RDF-Star (`<< >>`) | ✅ Implemented | `sparql_library/parsers/`, `sparql_executor.rs` |
> | SPARQL 1.1 Federated Query (`SERVICE`) | ✅ Implemented | `sparql_executor.rs::execute_service()` |
> | GeoSPARQL (OGC) | ✅ Implemented | `kml_bridge.rs`, `spatial_sieve.rs` |
> | Property Paths | ✅ Implemented | `sparql_executor.rs::execute_property_path()` |
> | SHACL-SPARQL | ✅ Implemented | `sparql_shacl.rs` |
> | SPARQL-MM (multimedia / time-series) | ✅ Implemented | `sparql_mm.rs` |
> | Aggregates (COUNT/SUM/AVG/MIN/MAX) | ✅ Implemented | `sparql_aggregates.rs` |
> | **`AS OF` / `AT TIME`** (Qualia extension) | ✅ Implemented (Phase 4) | `sparql_parser.rs`, `Pattern::AsOf`, `PhysicalOperatorType::AsOf` |
> | Entailment Regimes (RDFS/OWL) | ⏳ Future | — |
>
> **`AS OF` / `AT TIME`** is a Qualia-specific extension not in SPARQL 1.1/1.2.
> Syntax: `SELECT ... WHERE { ... } AS OF "YYYY-MM-DD"^^xsd:dateTime` or
> `... AT TIME <ms-integer>`. See `sparql-temporal-extension.md` (pending) for
> the standards track note.
>
> This document is a reference used during initial design.

---

To map out a clear frame of reference for your execution engine layout, SPARQL extensions fall into three distinct architectural categories: **W3C Standards Extensions**, **Vendor-Specific Engine Extensions**, and **Specialized Query/Reasoning Frameworks**.

When building a high-performance database with zero-allocation constraints, tracking these extensions helps you plan your AST indexing pipelines and identify which features map directly down to raw integer-based index scans versus which features require complex custom expression evaluation.

---

## 1. W3C Official Extensions (The Core Baseline)

These extensions evolved from the initial SPARQL 1.0 specification into full W3C Recommendations, or form the core of the current **SPARQL 1.2** Working Draft.

* **SPARQL 1.1 Update Language:** Extends read-only graph queries into an imperative database mutation tool using operations like `INSERT DATA`, `DELETE DATA`, `DELETE/INSERT WHERE`, `LOAD`, `CLEAR`, `CREATE`, and `DROP`.
* **SPARQL-Star (SPARQL 1.2 RDF-Star Integration):** Introduces the ability to treat a triple contextually as a term using the nested `<< :s :p :o >>` syntax. This allows metadata annotations directly on edges without triggering heavy standard reification patterns.
* **SPARQL 1.1 Federated Query (`SERVICE`):** Extends graph matching across networks. The `SERVICE` keyword directs the execution engine to split the query block and delegate specific subqueries to remote HTTP endpoints, merging bindings locally.
* **SPARQL 1.1 Entailment Regimes:** Modifies basic graph pattern matching to incorporate semantic reasoning. It dictates how queries evaluate data using implicit knowledge derived from **RDFS**, **OWL**, or **RIF** rules, rather than relying strictly on simple triple pattern matching.

---

## 2. Advanced Query & Reasoning Extensions

These specifications build on top of SPARQL syntax to enable data validation, path manipulation, and full text search.

* **SHACL-SPARQL (Shapes Constraint Language Extensions):** Part of the SHACL validation framework. It allows developers to write highly complex data-validation logic using explicit SPARQL query blocks inside shapes via `sh:sparql`. The engine binds the node being validated to the pre-bound variable `$this`.
* **GeoSPARQL (OGC Standard):** An open geospatial extension defining standard property shapes and filter functions for geographic information. It provides OGC-compliant data types (like `geo:wktLiteral`) and functions to calculate topological relations (e.g., `geof:sfContains`, `geof:distance`).
* **Property Paths (SPARQL 1.1 Native Extension):** While baked into 1.1, this acts as a graph-traversal extension. It enables regular-expression-style paths over predicates, such as arbitrary-length paths (`foaf:knows+`) and inverse paths (`^rdfs:subClassOf`).

---

## 3. Vendor & Engine Specific Extensions

Commercial and open-source triple stores frequently implement custom extension frameworks to expose hardware optimizations or specialized data structures directly through the SPARQL surface syntax.

| Extension Class | Common Examples | Architectural Purpose |
| --- | --- | --- |
| **Time-Series / Windowing** | **SPARQL-MM** (Multimedia), Stream Processing Extensions | Introduces windowing clauses (like sliding time intervals) to evaluate continuous streaming RDF data feeds rather than static, point-in-time snapshots. |

---

## Architectural Impact on Zero-Allocation Engines

If your engine enforces a zero-allocation, index-based layout using flat arrays and `u16` pattern IDs, handling these extensions requires isolating them into specific compilation pathways:

### Custom Property Functions (Magic Predicates)

Extensions like GeoSPARQL or Full-Text search often express themselves as "Magic Predicates":

```sparql
?place geo:hasGeometry ?geo .
?geo geof:within :boundingInstance . # Magic Predicate/Property function

```

Instead of lowering `geof:within` to a standard B-Tree lookup matching a predicate identifier slot, your Logical Query Planner must catch this specific dictionary ID during optimization and rewrite it into a **Specialized Functional Physical Operator**. This operator pulls candidates from your spatial or geometric indexes directly instead of querying your core Hexastore quad tables.

### Tracking Registry

To support user-defined function expansions without polluting your core execution loop memory, consider using a static function dispatch table:

```rust
pub type ExtensionFn = fn(args: &[u64], results: &mut BindingRow) -> bool;

pub struct ExtensionRegistry {
    // Maps a dictionary ID to a statically compiled function pointer
    pub functions: [(u64, ExtensionFn); 32], 
    pub count: usize,
}

```

This structural configuration keeps your memory footprint bounded, ensures lookups remain deterministic, and allows you to support advanced analytic or validation extensions while fully respecting your zero-allocation layout targets.