# Task 005: Fix Query Layer Stubs - mmap_query_subject and lazy_superblock_query

## Problem
The query layer has critical stubs that prevent real database functionality:

1. `query_engine.rs:8` - `mmap_query_subject()` prints a line and returns `Ok(vec![])` - does nothing
2. `query_engine.rs:29` - `lazy_superblock_query()` fabricates results with fake relevance
3. `indexing.rs` is empty - no index exists despite "microsecond memory-mapped queries" claims

**Files**:
- `crates/qualia-core-db/src/query_engine.rs`
- `crates/qualia-core-db/src/indexing.rs`
- **Severity**: 🔴 HIGH

## Current State

### mmap_query_subject (Line 8)
```rust
pub fn mmap_query_subject(subject: u64) -> Result<Vec<QualiaQuin>, Error> {
    println!("Querying subject: {}", subject);
    return Ok(vec![]); // Returns empty - does nothing
}
```

### lazy_superblock_query (Line 29)
```rust
pub fn lazy_superblock_query(
    query: &Query,
    target_percent: u8,
) -> Result<QueryResult, Error> {
    // Fabricates relevance: block_index % 100 < target_percent
    let relevance = block_index % 100 < target_percent;
    // Fake streaming: "we'll skip the disk read and pretend we streamed it"
    // ...
}
```

### indexing.rs
```rust
// Empty file - no index implementation
```

## Implementation Plan

### Part 1: Implement mmap_query_subject
Implement actual memory-mapped query for subject ID:

```rust
pub fn mmap_query_subject(
    subject: u64,
    mmap: &[u8],
) -> Result<Vec<QualiaQuin>, Error> {
    let mut results = Vec::new();
    let quins = unsafe {
        std::slice::from_raw_parts(
            mmap.as_ptr() as *const QualiaQuin,
            mmap.len() / std::mem::size_of::<QualiaQuin>(),
        )
    };

    for quin in quins {
        if quin.subject == subject {
            results.push(*quin);
        }
    }

    Ok(results)
}
```

### Part 2: Implement lazy_superblock_query
Implement real query with actual relevance scoring:

```rust
pub fn lazy_superblock_query(
    query: &Query,
    mmap: &[u8],
) -> Result<QueryResult, Error> {
    let mut results = Vec::new();
    let quins = unsafe {
        std::slice::from_raw_parts(
            mmap.as_ptr() as *const QualiaQuin,
            mmap.len() / std::mem::size_of::<QualiaQuin>(),
        )
    };

    for quin in quins {
        // Actual relevance calculation based on query predicates
        let relevance = calculate_relevance(quin, query);
        if relevance > 0.0 {
            results.push(QueryHit {
                quin: *quin,
                relevance,
                block_index: get_block_index(quin),
            });
        }
    }

    // Sort by relevance
    results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

    Ok(QueryResult {
        hits: results,
        total_quins: quins.len(),
    })
}

fn calculate_relevance(quin: &QualiaQuin, query: &Query) -> f32 {
    // Real relevance scoring based on predicate/object matching
    // Use TF-IDF or BM25 if text, exact match for IRIs
    // ...
}
```

### Part 3: Implement Indexing
Build an index for fast subject/predicate/object lookups:

```rust
use std::collections::HashMap;

pub struct QuinIndex {
    subject_index: HashMap<u64, Vec<usize>>,
    predicate_index: HashMap<u64, Vec<usize>>,
    object_index: HashMap<u64, Vec<usize>>,
}

impl QuinIndex {
    pub fn new() -> Self {
        QuinIndex {
            subject_index: HashMap::new(),
            predicate_index: HashMap::new(),
            object_index: HashMap::new(),
        }
    }

    pub fn build(&mut self, quins: &[QualiaQuin]) {
        for (idx, quin) in quins.iter().enumerate() {
            self.subject_index.entry(quin.subject)
                .or_insert_with(Vec::new).push(idx);
            self.predicate_index.entry(quin.predicate)
                .or_insert_with(Vec::new).push(idx);
            self.object_index.entry(quin.object)
                .or_insert_with(Vec::new).push(idx);
        }
    }

    pub fn query_subject(&self, subject: u64) -> &[usize] {
        self.subject_index.get(&subject)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn query_predicate(&self, predicate: u64) -> &[usize] {
        self.predicate_index.get(&predicate)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
```

## Implementation Steps

1. **Implement mmap_query_subject**:
   - Accept mmap parameter
   - Iterate over Quins in memory
   - Match by subject
   - Return matching Quins

2. **Implement lazy_superblock_query**:
   - Remove fake relevance calculation
   - Implement real relevance scoring
   - Remove fake streaming telemetry
   - Return actual query results

3. **Implement indexing.rs**:
   - Create QuinIndex struct
   - Build subject/predicate/object indexes
   - Add query methods for fast lookups
   - Add index persistence to disk

4. **Integrate index with query_engine**:
   - Load index on startup
   - Use index for fast lookups
   - Fall back to mmap scan if index not available
   - Update index on writes

5. **Write comprehensive tests**:
   - mmap_query_subject returns correct results
   - lazy_superblock_query relevance is accurate
   - Index build and query operations
   - Performance benchmarks (microsecond queries)

## Success Criteria
- ✅ mmap_query_subject returns actual matching Quins
- ✅ lazy_superblock_query calculates real relevance
- ✅ No fake telemetry or fabricated results
- ✅ indexing.rs has working index implementation
- ✅ Queries complete in microseconds (as claimed)
- ✅ Tests verify correctness
- ✅ Benchmarks verify performance

## Related Files
- `crates/qualia-core-db/src/query_engine.rs` (main)
- `crates/qualia-core-db/src/indexing.rs` (to implement)
- `crates/qualia-core-db/src/storage.rs` (mmap integration)
- `crates/qualia-core-db/src/mini_parser.rs` (query parsing)
- `README.md` (claims to update)

## Estimated Complexity
- mmap_query_subject: 0.5-1 day
- lazy_superblock_query: 1-2 days
- indexing.rs: 2-3 days
- Integration and testing: 1-2 days
- **Total**: 5-8 days

## Dependencies
- Can be done independently
- May coordinate with storage.rs for mmap integration

## Notes
- This is critical for database functionality
- Memory-mapped queries should be fast (use of mmap)
- Index should be persisted and loaded on startup
- Consider using a more sophisticated index (B-tree, LSM-tree) for large datasets
- Benchmark to verify "microsecond" claims
- Update documentation to reflect real capabilities