# SPARQL 1.1 & 1.2 Implementation Plan

**Status:** In Progress  
**Date:** 2025-01-XX  
**Target:** Full SPARQL 1.1 compliance with SPARQL-Star (1.2) extensions

---

## Architecture Overview

### Zero-Allocation Constraints

Given QualiaDB's strict zero-allocation requirements from AGENTS.md:
- No `Vec`, `String`, or `Box` in hot paths
- Stack-allocated AST using fixed-size arrays
- Caller-supplied output buffers
- Maximum 42MB SlgArena for execution

### Layered Architecture

```
┌─────────────────────────────────────────┐
│  SPARQL Protocol Layer (HTTP)            │
│  - /sparql endpoint                      │
│  - Content negotiation (XML/JSON/TSV)    │
└─────────────────────────────────────────┘
                 │
┌─────────────────────────────────────────┐
│  SPARQL Parser (pest)                    │
│  - Zero-allocation token stream          │
│  - Stack-allocated AST                   │
└─────────────────────────────────────────┘
                 │
┌─────────────────────────────────────────┐
│  Logical Query Planner                   │
│  - Pattern normalization                │
│  - Join order optimization               │
│  - Filter pushdown                       │
└─────────────────────────────────────────┘
                 │
┌─────────────────────────────────────────┐
│  Physical Query Executor                 │
│  - Index scans (subject/predicate/object)│
│  - Hash joins (stack-allocated)          │
│  - Nested loop joins                     │
└─────────────────────────────────────────┘
                 │
┌─────────────────────────────────────────┐
│  Storage Engine (Q42)                    │
│  - QualiaQuin array                      │
│  - Lexicon lookup                        │
│  - Context filtering                    │
└─────────────────────────────────────────┘
```

---

## Module Structure

### New Modules to Create

1. **`sparql_parser.rs`** - PEST-based SPARQL grammar and parser
2. **`sparql_ast.rs`** - Stack-allocated AST structures
3. **`sparql_planner.rs`** - Logical query planner
4. **`sparql_executor.rs`** - Physical query executor
5. **`sparql_filter.rs`** - FILTER expression evaluator
6. **`sparql_aggregates.rs`** - Aggregate functions
7. **`sparql_results.rs`** - Result formatters (XML/JSON/TSV/CSV)

### Existing Modules to Extend

- **`query_engine.rs`** - Add SPARQL execution entry point
- **`query_compiler.rs`** - Integrate with new SPARQL parser
- **`sentinel.rs`** - Add SPARQL query validation
- **`rdf_star.rs`** - Integrate embedded triple support

---

## Data Structures

### AST Representation (Stack-Allocated)

```rust
#[repr(C)]
pub enum SparqlQuery {
    Select(SelectQuery),
    Ask(AskQuery),
    Construct(ConstructQuery),
    Describe(DescribeQuery),
}

#[repr(C)]
pub struct SelectQuery {
    pub distinct: bool,
    pub variables: [Option<Var>; 16], // Max 16 variables
    pub var_count: u8,
    pub dataset: DatasetClause,
    pub where_clause: WhereClause,
    pub group_by: [Var; 16],
    pub group_by_count: u8,
    pub having: Option<Expr>,
    pub order_by: [OrderCondition; 16],
    pub order_by_count: u8,
    pub limit: Option<u64>,
    pub offset: u64,
}

#[repr(C)]
pub enum Pattern {
    Triple(TriplePattern),
    Optional(Box<Pattern>),      // Requires careful handling
    Union([Pattern; 2]),
    Graph(Var, Box<Pattern>),
    Filter(Expr),
    Subquery(SelectQuery),
}

#[repr(C)]
pub enum Expr {
    Var(Var),
    Literal(Literal),
    IRI(IRI),
    UnaryOp(UnaryOp, Box<Expr>),
    BinaryOp(BinaryOp, Box<Expr>, Box<Expr>),
    Function(Function, Vec<Expr>), // May need stack array
}
```

### Execution Context

```rust
#[repr(C)]
pub struct ExecutionContext {
    pub quins: &[QualiaQuin],
    pub lexicon: &Q42LexMmap,
    pub bindings: [Binding; 256], // Max 256 bindings
    pub binding_count: usize,
    pub arena: SlgArena,           // For temporary allocations
}
```

---

## Implementation Phases

### Phase 1: Core Parsing (Week 1)
- [ ] PEST grammar for SPARQL 1.1
- [ ] Parser implementation
- [ ] AST structures
- [ ] Basic SELECT queries (single triple pattern)
- [ ] Simple FILTER expressions

### Phase 2: Pattern Matching (Week 2)
- [ ] Triple pattern matching
- [ ] Variable binding
- [ ] Basic joins (nested loop)
- [ ] OPTIONAL patterns
- [ ] UNION patterns

### Phase 3: Advanced Features (Week 3)
- [ ] GRAPH patterns
- [ ] Subqueries
- [ ] FILTER expressions (full function library)
- [ ] Path expressions (property paths)

### Phase 4: Aggregation (Week 4)
- [ ] COUNT, SUM, AVG, MIN, MAX
- [ ] GROUP BY
- [ ] HAVING
- [ ] DISTINCT
- [ ] ORDER BY
- [ ] LIMIT/OFFSET

### Phase 5: Query Forms (Week 5)
- [ ] ASK queries
- [ ] CONSTRUCT queries
- [ ] DESCRIBE queries
- [ ] Update operations (INSERT DATA, DELETE DATA, etc.)

### Phase 6: Protocol & Results (Week 6)
- [ ] HTTP endpoint (/sparql)
- [ ] SPARQL XML result format
- [ ] SPARQL JSON result format
- [ ] SPARQL TSV/CSV result formats

### Phase 7: SPARQL-Star Integration (Week 7)
- [ ] Embedded triple patterns in WHERE
- [ ] BIND functions (SUBJECT, PREDICATE, OBJECT, TRIPLE)
- [ ] Annotation patterns
- [ ] Integration with existing RDF-Star parsers/serializers

---

## Zero-Allocation Strategies

### 1. Fixed-Size Arrays
- Variables: `[Option<Var>; 16]` (max 16 variables)
- Bindings: `[Binding; 256]` (max 256 bindings)
- Patterns: `[Pattern; 64]` (max 64 patterns per query)
- Order conditions: `[OrderCondition; 16]`

### 2. Arena Allocation
- Use SlgArena for temporary allocations
- Reset arena between queries
- Pool reusable buffers

### 3. Iterator-Based Processing
- Stream results instead of collecting into Vec
- Use callback-based result processing
- Lazy evaluation where possible

### 4. Stack Allocation
- Local variables on stack
- No heap allocation in hot paths
- Use `&mut [T]` for output buffers

---

## Performance Targets

- **Query compilation**: < 1ms for typical queries
- **Single triple pattern**: < 100μs
- **Join of 2 patterns**: < 1ms
- **Complex query (10+ patterns)**: < 10ms
- **Memory per query**: < 1MB (under 42MB SlgArena)

---

## Integration Points

### With Existing Code

1. **Query Engine** (`query_engine.rs`)
   - Add `execute_sparql()` function
   - Integrate with context filtering
   - Use existing mmap_query_subject where possible

2. **Sentinel** (`sentinel.rs`)
   - Validate SPARQL queries before execution
   - Check sensitivity labels in results
   - Apply routing metadata

3. **Webizen** (`webizen.rs`)
   - Integrate with WebizenVM for query execution
   - Use SlgArena for temporary allocations
   - Apply governance rules

4. **RDF-Star** (`rdf_star.rs`)
   - Use BIND functions for embedded triple access
   - Support embedded triple patterns in WHERE
   - Integrate with Virtual ID lookup

---

## Testing Strategy

### Unit Tests
- Parser tests for each query form
- AST structure validation
- Expression evaluator tests
- Join algorithm tests
- Aggregate function tests

### Integration Tests
- Full query execution tests
- Result format validation
- Performance benchmarks
- Memory usage verification

### Compliance Tests
- SPARQL 1.1 test suite (W3C)
- SPARQL-Star test suite (if available)
- Custom QualiaDB-specific tests

---

## Dependencies

### New Dependencies
- `pest` - Parser generator (zero-allocation friendly)
- `pest_derive` - Derive macros for pest

### Existing Dependencies
- `memmap2` - Memory-mapped file access
- `slg_arena` - Arena allocator (already in use)
- `q42_lex` - Lexicon lookup

---

## Known Challenges

1. **Recursive Structures**
   - OPTIONAL, UNION, GRAPH require careful handling
   - May need to use indices into pattern array instead of Box

2. **Variable Count Limits**
   - Fixed-size arrays limit variable count
   - Need to handle overflow gracefully

3. **Expression Evaluation**
   - SPARQL has many built-in functions
   - Need efficient function dispatch

4. **Join Ordering**
   - Optimal join order is NP-hard
   - Use heuristics (size estimation, selectivity)

5. **Subquery Correlation**
   - Correlated subqueries are complex
   - May need to defer implementation

---

## Success Criteria

- ✅ Pass SPARQL 1.1 W3C test suite (subset)
- ✅ Support all SPARQL 1.1 query forms
- ✅ Support all SPARQL-Star features
- ✅ Zero heap allocation in hot paths
- ✅ Under 42MB memory per query
- ✅ Sub-10ms execution for complex queries
- ✅ Full HTTP protocol support
- ✅ All result formats (XML, JSON, TSV, CSV)