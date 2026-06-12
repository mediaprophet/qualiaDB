# XML & XSD Support in QualiaDB: Review and Recommendations

This document reviews the current state of XML, XSD, and related serialization support within the QualiaDB ecosystem, particularly focusing on the CLI ingest pipelines and SPARQL engine. It also provides strategic recommendations for maintaining the project's strict zero-allocation architecture.

## 1. Current State of XML & XSD Support

### 1.1 Ingest Pipelines & Conversion to Q42
- **RDF/XML Parsing:** The ingest pipeline currently supports RDF/XML through the `rio_xml::RdfXmlParser`. It is integrated into both the legacy streaming path (`crates/qualia-core-db/src/ingest.rs`) and the new unified CLI pipeline (`crates/qualia-cli/src/ingest/mod.rs` via `ingest_rdf_xml`).
- **Zero-Allocation Architecture:** The unified CLI pipeline utilizes an `ExternalSorter` to manage out-of-core K-way merges, which successfully keeps the peak memory usage bounded to ~48 MB regardless of the input file size. The parsed triples are hashed via `hash_token()` into the 64-bit `NQuin` format.
- **XSD Datatypes:** XSD types (e.g., `xsd:integer`, `xsd:decimal`, `xsd:dateTime`, `xsd:string`) are currently handled in `crates/qualia-cli/src/ingest/mapper.rs`. The `compile_shacl_mapping` function maps these to a `TargetDatatype` enum, which is used to guide tabular data (CSV/JSON) into the correct inline 60-bit payload tags defined in `resolver.rs`.

### 1.2 Serialization & SPARQL Engine
- **SPARQL XML Serialization:** The SPARQL engine supports the W3C SPARQL Query Results XML Format. `crates/qualia-core-db/src/sparql_library/serialisers/sparql_results.rs` implements `format_xml`, which can serialize `BindingRow` results natively.
- **Embedded Triples (RDF-Star):** The XML serializer correctly handles embedded triples (values where the MSB tag indicates a triple) by reconstructing the `<triple>` XML node recursively.

### 1.3 CLI Usage
- The CLI (`qualia-cli`) automatically detects `.xml` or `.rdf` extensions via `ingest_auto` and routes them to the `ingest_rdf_xml` pipeline, making it seamless for users to convert XML graphs into native `.q42` v2 volumes.

---

## 2. Implementation Strategy Recommendations

To align XML/XSD support strictly with QualiaDB's immovable rules (specifically **Zero heap in hot paths**), the following implementations are recommended:

### Recommendation A: True Zero-Allocation XML Streaming
**Observation:** While `ExternalSorter` bounds peak memory, `rio_xml::RdfXmlParser` currently yields string allocations (`t.subject.to_string()`, etc.) in the parser callback before they are hashed into `NQuin`s.
**Strategy:**
- Implement a custom, zero-copy XML/RDF lexer that operates directly over memory-mapped files (`mmap`). 
- Instead of allocating intermediate `String`s for subject/predicate/object, the parser should yield byte slices (`&[u8]`).
- These byte slices should be hashed directly using a byte-compatible `q_hash()` variant to produce the 64-bit DIDs, completely eliminating heap allocations (`String`) during the hot path of XML ingestion.

### Recommendation B: Strict Inline XSD Tagging during XML Ingest
**Observation:** Currently, `mapper.rs` enforces XSD types for tabular inputs, but raw RDF/XML ingest relies on standard string hashing unless explicitly mapped.
**Strategy:**
- During the RDF/XML parsing phase, detect typed literals (e.g., `"123"^^xsd:integer`).
- Directly utilize the authoritative inline tags from `resolver.rs`:
  - `INLINE_TAG_INTEGER` (0b001 << 60)
  - `INLINE_TAG_DECIMAL` (0b010 << 60)
  - `INLINE_TAG_BOOLEAN` (0b011 << 60)
- Pack the parsed values directly into the 60-bit `object` payload of the `NQuin` instead of hashing the string representation. This significantly reduces lexicon bloat and improves SPARQL range query performance.

### Recommendation C: Apply SHACL Profiles to XML Ingest
**Observation:** Tabular ingest uses SHACL profiles (`.shacl.ttl`) for structural hints, but XML ingest is currently unconstrained.
**Strategy:**
- Extend the `ingest_rdf_xml` pipeline to optionally accept a `MappingProfile` or SHACL target class. 
- As triples stream in, validate them against the `sh:datatype` expectations and coerce the resulting Quins into the appropriate inline payload formats.

### Recommendation D: Streaming XML Serializer Output
**Observation:** `format_xml` in `sparql_results.rs` currently formats results from a buffered slice of `BindingRow` (`results: &[BindingRow]`).
**Strategy:**
- For massive SPARQL queries, buffer the results locally and stream the XML output in chunks to the `Write` trait. 
- Ensure that the HTTP server (`sparql_endpoint.rs`) honors HTTP chunked transfer encoding when returning `application/sparql-results+xml`, allowing gigabyte-scale results to be served without blowing up the 512MB RAM floor.
