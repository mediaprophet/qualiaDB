# SPARQL-Star Architectural Integration: Context, Sentinel, Webizen

**Date:** 2026-06-10  
**Branch:** 0.0.10-dev

---

## Overview

SPARQL-Star implementation must integrate natively with QualiaDB's core architectural components:
- **Context Interface** → Execution scope / graph isolation
- **Sentinel** → Policy validation and structural gatekeeping
- **Webizen** → Agent identity and assertion provenance

These cannot remain as high-level abstractions. They must map to deterministic, indexed structures within the Q42 binary format.

---

## 1. Component Semantics to Q42 Storage Mapping

### Context Interface

**Semantic:** Graph (G) parameter in SPARQL, localized sub-graph scope  
**Q42 Mapping:** Dedicated 64-bit ContextID/GraphID slot in QualiaQuin  
**Current Status:** QualiaQuin already has `context` field (bits 0-55: Contract/graph/world DID hash)

**Indexing Requirements:**
- GSPO (Graph-Subject-Predicate-Object) index for context-isolated scans
- GPOS (Graph-Predicate-Object-Subject) index for context-optimized predicate queries
- ContextID must be leading element in index trees for O(log n) context filtering

**SPARQL-Star Integration:**
```sparql
SELECT ?claim ?value
WHERE {
  GRAPH ?context {
    <<?claim q:hasValue ?value>> q:verifiedBy ?verifier .
  }
}
```
→ Physical operator seeks on GSPO where G = resolved ContextID

---

### Sentinel

**Semantic:** Automated structural gatekeeper, policy validation, cryptographic monitor  
**Q42 Mapping:** Stored as internal structural schemas (SHACL shape graphs, specialized rulesets)

**Storage Location:**
- Sentinel configurations stored in dedicated Q42 blocks with special metadata flags
- Sentinel rules indexed by ContextID for fast lookup during ingestion
- Sentinel validation integrated into write pipeline before disk commit

**Ingestion Pipeline Integration:**
```
[Inbound Stream] → [Sentinel Validation] → [Dictionary Encoding] → [Q42 Write]
```

**Query Execution Integration:**
- Sentinel acts as query-rewriting or filtering node in Logical Query Plan
- Unauthorized/structurally invalid embedded triples pruned before index scan
- Sentinel rules applied at physical operator level

**SPARQL-Star Integration:**
```sparql
<<:claim :status :verified>> :assertedBy ?agent .
```
→ Sentinel validates before Virtual ID generation and lexicon write

**Metadata Field Usage:**
- Use `metadata` bits [32-60] for Sentinel rule IDs
- Use `metadata` bits [0-31] for confidence weights from Sentinel evaluation

---

### Webizen

**Semantic:** Natural person or agent, decentralized identity anchor  
**Q42 Mapping:** Persistent WebID/URI with permanent high-priority ID in global lexicon

**Lexicon Priority:**
- Webizen IDs assigned high-priority slots in lexicon (lower hash ranges)
- WebID URIs pre-registered in lexicon during system initialization
- Webizen lookups optimized with dedicated index

**Ontological Continuity:**
```sparql
<<:claim :status :verified>> :assertedBy <webizen:timothy> .
```
→ Q42 parser normalizes `<webizen:timothy>` to single 64-bit WebizenID
→ Embedded triple assigned VirtualID
→ Quad written: `[VirtualID, assertedByID, WebizenID, ContextID]`

**Identity Verification:**
- Webizen Ed25519 signatures stored in separate Q42 blocks
- Cryptographic verification integrated with Sentinel validation
- Webizen DID resolution integrated with identifier module

---

## 2. Updated Ingestion Pipeline

```
[Inbound Stream]
│ (Contains Webizen Assertions, Sentinel Rules, & Context Scopes)
│
▼
[Context & Policy Layer]
│ (Resolves Active Context Interface & Sentinel Rules)
│
▼
[Sentinel Validation]
│ (Validates embedded triples against Sentinel rules)
│ (Checks Webizen identity and authorization)
│
▼
[Dictionary Encoding]
│ (Maps Webizen WebIDs → 64-bit Internal IDs)
│ (Maps Context URIs → 64-bit ContextIDs)
│ (Generates Virtual IDs for embedded triples)
│
▼
[Q42 Binary Serialization]
│ (Writes uniform, optimized fixed-width slots to disk)
│ (Writes 24-byte [u64; 3] tuples to lexicon for embedded triples)
│ (Writes Sentinel metadata to metadata field)
```

---

## 3. Query Engine Adjustments

### Context-Aware Query Execution

**SPARQL Query:**
```sparql
PREFIX q: <http://qualia.org/schema/>

SELECT ?claim ?value ?verifier
WHERE {
  GRAPH ?context {
    <<?claim q:hasValue ?value>> q:verifiedBy ?verifier .
  }
  ?verifier a q:Webizen .
  ?context a q:SentinelValidatedContext .
}
```

**Physical Operator Blueprint:**
```rust
struct PhysicalContextualScan {
    raw_storage: Q42Engine,
    context_id: u64,        // Pre-resolved 64-bit ContextID
    sentinel_id: u64,       // Pre-resolved 64-bit Sentinel rule ID
    webizen_pattern: u64,   // Concrete WebizenID or variable pointer
}

impl PhysicalContextualScan {
    fn next(&mut self, ctx: &mut ExecutionContext) -> Option<Binding> {
        // 1. Direct seek on GSPO index where G = self.context_id
        // 2. Intersect results with virtual embedded triples mapping table
        // 3. Filter by Sentinel validation rules
        // 4. Stream matching, normalized binding tuples forward
    }
}
```

---

## 4. SPARQL-Star Integration Requirements

### Virtual ID Generation with Context

**Current:** `generate_embedded_triple_id(subject, predicate, object)`  
**Required:** Context-aware Virtual ID generation

```rust
pub fn generate_embedded_triple_id_with_context(
    subject: u64,
    predicate: u64,
    object: u64,
    context_id: u64,
) -> u64 {
    // Option 1: Include context in hash
    let mut bytes = [0u8; 32]; // 24 bytes for triple + 8 bytes for context
    bytes[0..8].copy_from_slice(&subject.to_le_bytes());
    bytes[8..16].copy_from_slice(&predicate.to_le_bytes());
    bytes[16..24].copy_from_slice(&object.to_le_bytes());
    bytes[24..32].copy_from_slice(&context_id.to_le_bytes());
    
    generate_60bit_token(&bytes) | TAG_EMBEDDED
    
    // Option 2: Keep Virtual ID context-independent, use QualiaQuin context field
    // This is probably better for cross-context queries
}
```

### Lexicon Extension for Sentinel Metadata

**Current:** Lexicon stores [u64; 3] for embedded triples  
**Required:** Also store Sentinel validation metadata

```rust
pub enum Lexeme<'a> {
    String(&'a str),
    EmbeddedTriple(&'a [u64; 3]),
    EmbeddedTripleWithSentinel {
        triple: &'a [u64; 3],
        sentinel_id: u64,
        confidence: f32,
    },
}
```

### Parser Integration with Sentinel

**Required:** Turtle-Star parser must check Sentinel rules before generating Virtual IDs

```rust
impl TurtleStarParser {
    pub fn parse_with_sentinel_validation(
        &mut self,
        input: &[u8],
        sentinel: &SentinelRule,
    ) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        let (virtual_id, components) = self.parse_embedded_triple_internal(input)?;
        
        // Validate against Sentinel rules
        if !sentinel.validate_embedded_triple(&components)? {
            return Err(RdfStarParseError::UnsupportedFeature); // Sentinel rejected
        }
        
        Ok((virtual_id, components))
    }
}
```

---

## 5. Implementation Priority

1. **Context ID Integration** (High Priority)
   - Verify QualiaQuin context field is properly used
   - Add GSPO/GPOS index support for context-aware queries
   - Update query engine to support GRAPH clauses

2. **Sentinel Validation Integration** (High Priority)
   - Define Sentinel rule storage format in Q42
   - Integrate Sentinel validation into ingestion pipeline
   - Add Sentinel metadata to lexicon for embedded triples

3. **Webizen Identity Integration** (Medium Priority)
   - Ensure Webizen IDs have high-priority lexicon slots
   - Add Webizen signature verification
   - Integrate Webizen provenance tracking

4. **Query Engine Context Support** (Medium Priority)
   - Implement PhysicalContextualScan operator
   - Add GRAPH clause support to query compiler
   - Optimize context-isolated index scans

---

## 6. Architectural Decisions (RESOLVED)

### Q1: Context in Virtual ID Hash vs. QualiaQuin Context Field
**Decision:** Keep them separate.  
**Rationale:** An embedded triple `<<s p o>>` represents an abstract structural semantic claim, independent of where or by whom it is uttered. If Context ID is included in the Virtual ID hash, identical claims across different contexts mint different Virtual IDs, fracturing the index and breaking global deduplication.  
**Implementation:** Virtual ID is pure cryptographic hash of 24-byte payload (s, p, o) only. Context layer applied via QualiaQuin container (bits 0-55) wrapping the record.

### Q2: Sentinel Storage Format
**Decision:** Store Sentinel rules as separate, dedicated Q42 blocks within main database file.  
**Rationale:** Sentinel rules are graph data (shapes/structural schemas). Storing as native Q42 blocks allows query engine to use existing B-Tree index scans. Sidecar files introduce sync risks and IO overhead; polluting lexicon with complex schemas is suboptimal.

### Q3: Webizen Lexicon Priority & Collision Mitigation
**Decision:** Allocate dedicated high-priority prefix/range for authoritative identities (Webizens), use Chained Bucketing for collision mitigation.  
**Rationale:** Webizens are sovereign actors. Reserved range allows low-level operators to identify agents without dictionary lookup. For hash collisions on 24-byte reverse lookups, chained bucketing inside Q42 storage preserves zero-allocation via fixed-offset pointers to collision overflow rows.

### Q4: Preventing Sentinel Performance Bottlenecks
**Decision:** Implement Asynchronous Pipeline Stage with Fast-Path Cache Filters.  
**Rationale:** Synchronous SHACL validation during file writes kills ingestion throughput.  
**Implementation:**
1. Maintain bitmask/Bloom filter of active Sentinel rule IDs in memory
2. As quads stream, check against Bloom filter. If quad doesn't touch protected context/predicate namespace, fast-path directly to Q42 append buffer
3. If rule triggered, hand off to dedicated worker thread pool for async validation before final block commit

---

## 7. Open Questions (REMAINING)

1. **Nesting Depth Limits:** What is the maximum nesting depth for embedded triples (e.g., `<<<s p o>> p2 o2>>`)? This determines stack array size.

2. **Webizen ID Range:** What specific 64-bit prefix range should be reserved for Webizen IDs?

---

## 8. Context ID Integration Status (COMPLETED 2025-01-XX)

### Implementation Details

**QualiaQuin Context Field:**
- Field: context: u64 (bits 0-55: Contract/graph/world DID hash, bits 56-63: Sensitivity class)
- Already exists in QualiaQuin structure (48-byte Super-Quin)
- Properly used in N-Quads-Star parser (graph → context mapping)

**Context Filtering Functions Added:**
Location: crates/qualia-core-db/src/query_engine.rs

1. ilter_by_context(quins, context_hash) - Filter by single context hash (0 = wildcard)
2. ilter_by_contexts(quins, context_hashes) - Filter by multiple context hashes
3. count_by_context(quins) - Count Quins per context hash
4. unique_contexts(quins) - Get unique context hashes from Quins
5. ilter_by_context_and_subject(quins, context_hash, subject) - Combined context+subject filter
6. ilter_by_context_and_predicate(quins, context_hash, predicate) - Combined context+predicate filter
7. ilter_by_context_and_object(quins, context_hash, object) - Combined context+object filter

**Parser Integration:**
- N-Quads-Star parser maps graph parameter to QualiaQuin context field
- N-Triples-Star parser uses context_hash parameter for all Quins
- Turtle-Star parser accepts context_hash in constructor

**Usage Example:**
`
ust
// Filter Quins from a specific named graph
let filtered = filter_by_context(&all_quins, graph_context_hash);

// Filter Quins from multiple graphs
let filtered = filter_by_contexts(&all_quins, &[graph1, graph2, graph3]);

// Count Quins per graph
let counts = count_by_context(&all_quins);
// Result: HashMap<u64, usize> mapping context_hash → count

// Combined context + subject query
let results = filter_by_context_and_subject(&all_quins, context_hash, subject_id);
`

### GSPO/GPOS Index Status

**Current Status:** Not yet implemented  
**Reason:** Would require significant changes to Q42 volume format (new index blobs)

**Planned Implementation:**
- GSPO index: [Graph][Subject][Predicate][Object] → block ranges
- GPOS index: [Graph][Predicate][Object][Subject] → block ranges
- Similar structure to existing BIDX (Block Index)
- Context ID as leading key for O(log n) context filtering

**Workaround Until Indexes Implemented:**
- Use context filtering functions on loaded Quin slices
- BIDX currently indexes by object hash only
- For context-isolated queries: load blocks, then filter by context using filter_by_context()

### SPARQL GRAPH Clause Support

**Current Status:** Partial  
- Context field properly populated in parsers
- Context filtering functions available
- Query compiler does not yet parse GRAPH clauses

**Required for Full Support:**
1. Query compiler must parse GRAPH clauses in SPARQL
2. Resolve graph URIs to 64-bit ContextIDs via lexicon
3. Pass ContextID to physical operators
4. Physical operators use context filtering or GSPO index

**Example SPARQL Query:**
`sparql
SELECT ?s ?p ?o
WHERE {
  GRAPH <http://example.org/Graph1> {
    ?s ?p ?o .
  }
}
`

**Equivalent Rust:**
`
ust
let context_id = generate_60bit_token(b"http://example.org/Graph1");
let results = filter_by_context(&loaded_quins, context_id);
`

### Sensitivity Class in Context Field

**Context Field Layout:**
- Bits [0..55]: Contract/graph/world DID hash
- Bits [56..63]: Sensitivity class (SENSITIVITY_PUBLIC=0, RESTRICTED=1, CLASSIFIED=2)

**Fiduciary Mandate:**
All new code interacting with the QueryEngine must check the sensitivity byte.
Classified context data must NEVER egress the local node.

**Sensitivity Check Function:**
`
ust
pub const SENSITIVITY_PUBLIC: u8 = 0;
pub const SENSITIVITY_RESTRICTED: u8 = 1;
pub const SENSITIVITY_CLASSIFIED: u8 = 2;

pub fn get_sensitivity_class(context: u64) -> u8 {
    ((context >> 56) & 0xFF) as u8
}

pub fn filter_by_sensitivity(quins: &[QualiaQuin], max_sensitivity: u8) -> Vec<QualiaQuin> {
    quins.iter()
        .filter(|q| get_sensitivity_class(q.context) <= max_sensitivity)
        .copied()
        .collect()
}
`

### Summary

✅ Context field properly integrated in QualiaQuin  
✅ Context filtering functions implemented and tested  
✅ Parsers map graph/context to QualiaQuin context field  
⏳ GSPO/GPOS indexes not yet implemented (requires volume format changes)  
⏳ Query compiler GRAPH clause support not yet implemented  
⏳ Sensitivity checks not yet wired to network egress layer
