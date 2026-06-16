# QualiaDB Next Steps: Zero-Heap Execution And 64-Slot VM Scaling

Date: 2026-06-16

This document turns the current zero-heap and 64-slot VM considerations into a concrete refactor plan. The recommended path is to keep the Webizen VM small, fixed, and deterministic while moving large medical and diagnostic complexity into the graph state and Rust orchestration layer.

## Recommendation

Use a blackboard-style architecture with a streamed visitor at the rule layer.

The Webizen VM should remain a tiny proof kernel with a fixed 64-opcode working set. It should not grow into a paging engine for large bytecode programs. Complex medical evaluation should scale through the Q42 graph, clinical engines, resident model inference, and Rust orchestration that compiles one small proof clause at a time.

In short:

- Keep the VM limit at 64 instructions.
- Treat the graph and Q42 volume as the large state container.
- Treat the Rust orchestrator as the rule walker and attention mechanism.
- Treat each VM run as a complete, bounded symbolic proof step.
- Route irreducible rules that need more than 64 opcodes into native Rust clinical routines, not into a larger VM.

## Current Anchors

The codebase already has most of the right boundaries:

- `crates/qualia-core-db/src/modalities/logic/core.rs` keeps `WebizenVM::bytecode_buffer` at `[Option<WebizenOpcode>; 64]`.
- `WebizenVM::load_bytecode()` clears stale slots before loading a bounded instruction slice.
- `execute_differential_diagnostics()` already follows caller-supplied output buffers and hard overflow errors.
- `prune_defeasible_claims()` already works over caller-owned slices and scrubs the pruned tail.
- `crates/qualia-core-db/src/daemon_graph.rs` keeps daemon graph loading behind `MAX_GRAPH_QUINS`.
- `crates/qualia-core-db/src/modalities/graph_theory.rs` is explicitly quarantined as heap-backed batch analysis with `MAX_HEAP_GRAPH_ANALYSIS_QUINS`.

These are good foundations. The remaining work is to make the orchestration layer first-class.

## Zero-Heap Refactor Rules

Use these rules for any path that can run inside evaluator loops, edge-node daemon loops, or medical validation flows.

### 1. Replace Owned Output Collections

Avoid returning `Vec<T>` from hot-path logic. Prefer:

```rust
pub fn evaluate_something(
    input: &[NQuin],
    out: &mut [NQuin],
) -> Result<usize, EvalError>
```

The caller owns memory. The callee returns the valid prefix length. On overflow, return a hard error such as `OutputBufferFull` or `BufferOverflow`.

Do not silently truncate medical, deontic, diagnostic, or access-control results.

### 2. Replace Hash Sets In Hot Paths

For small bounded sets, use sorted stack arrays plus binary search:

```rust
fn insert_sorted_unique(buf: &mut [u64], len: &mut usize, value: u64) -> Result<(), BufferOverflow> {
    match buf[..*len].binary_search(&value) {
        Ok(_) => Ok(()),
        Err(pos) => {
            if *len >= buf.len() {
                return Err(BufferOverflow);
            }
            buf[pos..=*len].rotate_right(1);
            buf[pos] = value;
            *len += 1;
            Ok(())
        }
    }
}
```

For ergonomics, consider adding `heapless` where a fixed-capacity `Vec`, `String`, or `FnvIndexMap` meaningfully reduces hand-rolled slice code. Treat `heapless` capacity errors as normal control-flow errors, not panics.

### 3. Scrub Caller-Owned Buffers

When a function partitions or filters in place, clear the invalid tail before returning:

```rust
for slot in &mut out[valid_count..] {
    *slot = NQuin::default();
}
```

This prevents stale sensitive inferences from remaining readable after the valid slice boundary.

## 64-Slot VM Strategy

Do not raise the VM instruction buffer to 128, 512, or 1024 as the default answer. Larger buffers make the VM less predictable and move complexity into the wrong layer.

The VM should execute only complete leaf proofs. The orchestrator should handle large rules.

### Proposed AST

Add a small rule AST beside the existing `WebizenOpcode` compiler path:

```rust
pub enum LogicNode<'a> {
    And(&'a LogicNode<'a>, &'a LogicNode<'a>),
    Or(&'a LogicNode<'a>, &'a LogicNode<'a>),
    Not(&'a LogicNode<'a>),
    Leaf(ConstraintRef),
}
```

`ConstraintRef` should be compact and hash-based. Avoid heap-owned strings inside the evaluator-facing form. If parser layers start with strings, hash them before constructing the runtime rule tree.

### Proposed Visitor

Add a streamed visitor that owns reusable scratch state:

```rust
pub struct StreamingRuleVisitor<'a> {
    vm: &'a mut WebizenVM,
    op_buffer: [WebizenOpcode; 64],
}
```

The visitor walks the AST, compiles a single `Leaf` into `op_buffer`, loads only the used prefix into the VM, and executes. `And` and `Or` short-circuit at the Rust layer so expensive branches are never compiled when the result is already known.

### Error Model

Use explicit errors:

```rust
pub enum RuleEvalError {
    OpcodeCapacityExceeded,
    OutputBufferFull,
    UnsupportedIrreducibleConstraint,
    VmError,
}
```

If one leaf cannot fit into 64 opcodes, that is not a reason to expand the VM. It means the leaf is not actually irreducible for this architecture. Split it into smaller rule nodes or route it to a native clinical engine.

## Medical Evaluation Model

Medical evaluation can be extraordinarily large without requiring giant bytecode payloads. Separate state from execution.

### State Container

Store high-dimensional patient context in the graph and domain stores:

- biomarkers
- medications
- allergies
- temporal events
- imaging findings
- SNOMED, LOINC, RadLex, FHIR, and local ontology quins
- LLM-proposed defeasible claims
- deterministic validation outcomes

This layer may be large. It should be bounded by storage and graph-level policy, not by VM bytecode size.

### Orchestrator

The Rust layer should select one narrow question at a time:

- Does this medication conflict with this condition?
- Does this proposed diagnosis satisfy the strict evidence rules?
- Is this inference contradicted by a hard fact?
- Does this action violate a deontic or sanctuary-lane constraint?

The orchestrator compiles each question into a small VM leaf proof or routes it to native Rust code.

### Execution Kernel

The Webizen VM executes the bounded proof. It should read exactly the graph pointers and inline values needed for that proof, then emit a boolean or inferred `NQuin`.

This gives medical evaluation large-scale context without making symbolic execution unbounded.

## Native Clinical Escape Hatch

Some rules should not be bytecode at all.

If a single medical rule needs a long, indivisible chain of calculations, route it to native Rust modules such as `clinical_engine.rs`, comorbidity evaluation paths, or specialized medical libraries. The VM can validate the result afterward by checking a compact proof quin, confidence marker, or provenance edge.

Use this rule:

- If it decomposes into independent symbolic predicates, use the streamed visitor.
- If it is a dense numerical or clinical scoring routine, use native Rust.
- If it is probabilistic pattern recognition, use the local model path and then validate the resulting defeasible claims symbolically.

## Implementation Plan

### Phase 1: Guardrails

- Add a `MAX_WEBIZEN_OPCODES: usize = 64` constant next to `WebizenVM`.
- Change `load_bytecode()` to return `Result<(), VmError>` if input exceeds 64 opcodes.
- Update existing call sites to handle the result.
- Add a regression test proving oversized bytecode is rejected rather than truncated.

### Phase 2: Leaf Compiler

- Introduce `compile_single_constraint(constraint, out: &mut [WebizenOpcode]) -> Result<usize, RuleEvalError>`.
- Keep each compiler function caller-buffer based.
- Prohibit `Vec<WebizenOpcode>` in hot-path compiler APIs.
- Add tests for exact-fit 64-opcode leaves and overflow at 65 opcodes.

### Phase 3: Streaming Rule Visitor

- Add `LogicNode` and `StreamingRuleVisitor`.
- Implement `And`, `Or`, and `Not` with short-circuit behavior.
- Compile and execute only leaf nodes.
- Add tests proving short-circuiting skips unneeded leaf compilation.

### Phase 4: Medical Blackboard Integration

- Introduce a diagnostic orchestration layer that pulls candidate facts from the Q42 graph into fixed scratch buffers.
- Emit inferred diagnostic quins through caller-owned output buffers or a streaming callback.
- Route complex clinical scoring to native Rust, then validate the emitted result quin through the VM.

### Phase 5: Audit Remaining Heap Usage

- Search for hot-path `Vec`, `String`, `HashMap`, `HashSet`, and `Box`.
- Classify each occurrence as one of:
  - hot path, must refactor
  - batch path, must guard and document
  - parser/setup path, acceptable with boundary conversion
- Update `20260616_audit-report.md` or a follow-up audit note with this classification.

## Acceptance Criteria

The next refactor round is complete when:

- No VM bytecode load silently truncates instructions.
- No hot-path compiler returns `Vec<WebizenOpcode>`.
- Complex rule evaluation can exceed 64 total logical operations by streaming leaves, while each VM invocation stays at or below 64 opcodes.
- Medical diagnostic evaluation can process large graph state without requiring large bytecode buffers.
- Buffer overflow, opcode overflow, and unsupported irreducible constraints are explicit errors.
- Tests cover oversized bytecode, oversized output buffers, short-circuiting, and native clinical escape routing.

## Final Direction

The architecture should scale complexity in data and orchestration, not in the VM.

The Q42 graph can be huge. The medical model can be high-dimensional. The Rust orchestrator can be smart. The VM should stay tiny, deterministic, and boring in the best possible sense: a fixed-size proof kernel that never allocates, never pages instructions, and never has to understand the whole medical universe to verify one claim.
