# Full SPARQL implementation — closing the parser gap

**Author:** working instrument (allocated by Timothy), 2026-07-12
**Status:** active — executing in verified slices
**Scope goal (Timothy, 2026-07-12):** the SPARQL sibling libraries fully implemented and
reachable end-to-end, *with the single exception of SPARQL-MM*, which needs prior work on the
vision libraries to have any media to match against.

## The key finding that makes this tractable

The audit framed UPDATE / SERVICE / FILTER / extension functions as "implemented but unwired
libraries the executor never calls." A full-file trace showed the real situation is narrower and
much better:

- **The AST is complete.** `sparql_ast.rs` models the whole SPARQL algebra: `Pattern::{Optional,
  Union, Minus, Filter, Graph, Group, PropertyPath, Service, AsOf, StarTriple, Triple}` and
  `Expression::{Variable, Literal, Iri, UnaryOp, BinaryOp, Function, Subquery, EmbeddedTriple}`
  with the full `BinaryOp`/`UnaryOp` sets and a complete SPARQL 1.1 `Function` enum (+ `Custom`).
- **The planner is complete.** `QueryPlanner::plan_pattern` already lowers every one of those
  `Pattern` variants to a physical operator.
- **The executor is complete.** `QueryExecutor` dispatches `PhysicalOperatorType::{Filter, Union,
  Optional, Minus(via plan), GroupBy, Having, Project, Distinct, Sort, Limit, PropertyPath, Graph,
  Service, AsOf, StarTripleScan, …}`, and `ExpressionEvaluator` already evaluates the full
  `Expression` tree including function calls.

**The only gap is the parser.** `sparql_parser.rs` is a 665-line string-slicer that recognises
`SELECT/ASK/CONSTRUCT/DESCRIBE` + `PREFIX` + a flat WHERE of basic triple patterns. It never
produces a `Filter`, `Optional`, `Union`, `Minus`, `Bind`, `Service`, or `Expression::Function`
node, and it has no UPDATE path. So the engine can *run* the full algebra; nothing *feeds* it.

Therefore this work is almost entirely: **write a proper recursive-descent SPARQL parser that emits
the AST the engine already consumes**, plus (a) thread the extension registry into the evaluator and
(b) give UPDATE a real, governed mutation path.

## Structure

`sparql_parser.rs` becomes `sparql_parser/` (per CLAUDE.md §11) with focused submodules:
`tokenizer.rs`, `expr.rs` (expression grammar), `pattern.rs` (group-graph-pattern grammar),
`update.rs` (UPDATE), and `mod.rs` (the `parse_sparql` entry + SELECT/ASK/CONSTRUCT/DESCRIBE shells,
preserving the public path). The `ctx` arena (`alloc_pattern`/`alloc_expression`/`register_variable`/
`function_args`) is the build target for every producer.

## Slices (each independently committed, gated on `cargo test`)

1. **Tokenizer + expression parser + `FILTER`.** A SPARQL tokenizer (keywords, IRIs `<…>`,
   prefixed names, variables `?x`, string/numeric/boolean literals, `<<`/`>>`, operators,
   punctuation) and a precedence-climbing expression parser producing `Expression` nodes
   (`|| < && < comparison < additive < multiplicative < unary < primary`; primary = literal /
   var / IRI / `(expr)` / builtin-function call / `<<s p o>>`). Wire `FILTER(expr)` inside the
   WHERE group. This alone makes the whole `ExpressionEvaluator` reachable from a real query
   (including the Wave-1 `TRIPLE()` fix). Golden-query tests: numeric/string filters, `&&`/`||`,
   `regex`, `BOUND`, comparison against IRIs/literals.

2. **Full group-graph-pattern grammar.** `{ … }` groups with `OPTIONAL {}`, `{} UNION {}`,
   `MINUS {}`, `BIND(expr AS ?v)`, `GRAPH <g> {}`, nested groups, and correct `FILTER` scoping.
   Each lowers to the existing `Pattern` node. Tests per operator against the existing executor.

3. **`SERVICE` + local federation.** Parse `SERVICE <ep> { … }` → `Pattern::Service`; the executor
   already runs the inner pattern. Route `local:`/`qualia:` endpoints through
   `FederatedQueryEngine`. Remote-HTTP SERVICE stays an honest, non-fabricating placeholder —
   network egress is a governance decision (documented, not silently enabled).

4. **Extension-function registry.** Thread `ExtensionRegistry` + `&[NQuin]` through
   `ExpressionEvaluator`; resolve `Function::Custom(q_hash(iri))` via the registry (re-key the
   builtin registry from ASCII placeholders to real `q_hash(iri)` so parser and registry agree);
   the FILTER catch-all stops silently returning `true` for unknown functions. Implement the
   GeoSPARQL functions that are in-process-computable now (real `geof:distance` haversine; `sf*`
   topological predicates over parsed WKT) — the ones that need external data stay honest stubs.

5. **UPDATE.** Parse `INSERT DATA` / `DELETE DATA` / `DELETE … INSERT … WHERE` / `LOAD` / `CREATE`
   / `DROP` / `CLEAR` → `UpdateOperation`. Give it a **governed** mutation path: add
   `DaemonGraphStore::{insert, retain}`, run under one write guard, mirror each insert/delete to
   the WAL via `commit_semantic_mutation` (ed25519-signed, classification-checked) so mutations are
   auditable and not a signing bypass. `DeleteData` must respect graph/context scope. Expose via a
   guarded path (CLI first; a daemon `/update` endpoint only with the egress/auth checks the read
   path already has). `LOAD` (needs an HTTP fetch) and remote effects stay honest stubs.
   **Open architectural note:** the daemon graph and the WAL are currently decoupled (no replay of
   WAL into the in-memory graph on restart); durable UPDATE needs that link, tracked here.

6. **WebSocket subscribe.** With real mutations existing (slice 5), call
   `SparqlWebSocketHandler::notify_subscribers` from the mutation path and attach the handler to the
   live `/qualia-bridge` transport (shared graph state rather than a borrowed `&[NQuin]`).

## Explicitly out of scope (with reason)

- **SPARQL-MM** (`sparql_mm.rs`): its media-fragment matching needs the vision libraries to ingest
  any media in the first place; its predicate-hash constants can be corrected once there is a live
  consumer. Deferred by Timothy's instruction.
- **Remote-HTTP `SERVICE`** and **`LOAD`**: real network egress, a governance decision — kept as
  honest placeholders behind the existing egress boundary.

## Honesty bar

Every slice lands with tests that would fail without it, `cargo test` green before commit, and the
functionality manual's SPARQL bullet updated to match reality. No slice claims "done" on a path a
real query can't reach.
