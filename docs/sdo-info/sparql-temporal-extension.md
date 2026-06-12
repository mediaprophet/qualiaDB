# SPARQL Temporal Extension: AS OF and AT TIME

**Status:** Editor's Draft | **Updated:** 2026-06-12

This document defines an extension to SPARQL 1.1/1.2 providing point-in-time and historical snapshot queries natively at the language level.

## 1. Introduction

Traditional SPARQL queries operate on the current state of a dataset. While approaches like GeoSPARQL or SPARQL-MM offer time-series windowing, Qualia-DB extends the core grammar to support native temporal traversal over a cryptographically linked Merkle-DAG Write-Ahead Log.

This extension introduces two query modifiers:
- `AS OF`: For assertion-time snapshots (what did the database know at time *t*?)
- `AT TIME`: For valid-time point queries (what was true in the world at time *t*?)

## 2. Syntax and Grammar

The extension adds two optional trailing modifiers to the standard SPARQL `WHERE` clause.

### 2.1. AS OF Modifier

```sparql
SELECT ?subject ?predicate ?object
WHERE {
  ?subject ?predicate ?object .
} AS OF "2024-06-01T00:00:00Z"^^xsd:dateTime
```
Alternatively, numeric unix timestamps (milliseconds) are supported:
```sparql
... AS OF 1717286400000
```

### 2.2. AT TIME Modifier

```sparql
SELECT ?subject ?predicate ?object
WHERE {
  ?subject ex:status ?status .
} AT TIME "2024-06-01T00:00:00Z"^^xsd:dateTime
```

## 3. Semantics and Execution Model

Temporal queries in Qualia-DB rely on `T_CONTEXT` overlay Quins using the W3C PROV-O vocabulary. 

### 3.1. AS OF (Assertion Time)
When `AS OF t` is supplied, the executor filters the underlying Merkle-DAG.
- A Quin is included if its `prov:generatedAtTime <= t`.
- If a Quin was superseded or tombstoned by a mutation where `prov:generatedAtTime <= t`, the original Quin is excluded from the result set.
- This provides an immutable cryptographic snapshot of the database at a prior state.

### 3.2. AT TIME (Valid Time)
When `AT TIME t` is supplied, the executor filters based on the entity's real-world validity.
- A Quin is included if its associated temporal context specifies `startedAtTime <= t` and `endedAtTime >= t` (or if it has no explicit ended time).
- This is an open-world default: if no temporal PROV-O annotations exist for a Quin, it is assumed to be eternally valid and is included.

## 4. Implementation Readiness

- **AST Representation**: Implemented as `TemporalMode` enum and `Pattern::AsOf` in `sparql_ast.rs`.
- **Query Planner**: Maps to `PhysicalOperatorType::AsOf` in `sparql_planner.rs`.
- **Executor**: Handled via `execute_as_of()` and `check_temporal_constraint()` in `sparql_executor.rs`.
- **Parser**: Recognized via `parse_temporal_literal()` in `sparql_parser.rs`.

## 5. Security and Access Control
Temporal traversal does not bypass standard Webizen Rights Ontology or deontic access controls. If a user did not have permission to view a Quin at time *t*, the query will still enforce the active capability profile restraints.
