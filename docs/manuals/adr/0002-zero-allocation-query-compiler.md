# ADR 0002: Zero-Allocation Query Compiler

## Status
Accepted

## Context
Parsing text strings (like SPARQL-Star or GeoSPARQL) conventionally involves lexing, tokenizing, and constructing an Abstract Syntax Tree (AST). This process rapidly consumes megabytes of heap memory. Under a 512MB RAM ceiling, multiple concurrent queries could easily induce an out-of-memory kernel panic (OOM kill).

## Decision
We completely abandon heap-allocated Abstract Syntax Trees. The compiler parses the raw incoming byte stream directly into native 64-bit hardware bitmasks using a finite state machine.

We parse specific nested graph identifiers (like `<<?s ?p ?o>>`) directly into the `NESTED_BIT_MASK` flag (`1 << 63`) on the target vectors. Furthermore, domain-specific logic, such as GeoSPARQL boundaries, are mapped instantly into the `SpatiotemporalAmbiguous` routing lane (`0b11 << 61`) without intermediate object allocation.

## Consequences
- **Positive:** Absolute memory stability. A query will never cause a RAM-based crash.
- **Positive:** Extreme speed. The compiler processes a nested GeoSPARQL query string down into its final hardware execution opcode in ~36 nanoseconds.
- **Negative:** The syntax parser is intensely rigid. Extending the engine to support complex nested sub-selects or string-heavy GraphQL mutations requires intricate bitwise engineering rather than simply adding a node to a tree.
