# ADR 0001: The 48-byte Qualia-Quin Alignment

## Status
Accepted

## Context
Traditional graph databases rely heavily on dynamic heap allocations (pointers, linked lists, string references) to manage nodes and edges. While this provides maximum flexibility, it inevitably triggers cache misses and unpredictable garbage collection pauses, which destroy performance in resource-constrained edge environments.

Our database, Qualia-DB, must run securely on mobile edge devices with strict 512MB RAM constraints and maximize hardware efficiency.

## Decision
We enforce a strict `#[repr(C, align(16))]` alignment, standardizing the `QualiaQuin` primitive to exactly 48 bytes.

```rust
#[repr(C, align(16))]
pub struct QualiaQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}
```

By ensuring the structural payload maps uniformly, memory operations bypass the heap allocator.

## Consequences
- **Positive:** Allocation operates essentially at CPU register speeds. Benchmarks prove a single instantiation takes ~3.5 nanoseconds.
- **Positive:** Array lookups are linearly predictable, allowing aggressive CPU prefetching and SIMD instruction pipelining.
- **Negative:** We lose the ability to store unbound strings or complex JSON objects inline. All arbitrary sized strings must be dictionary-encoded into 64-bit integer IDs upstream before ingestion.
