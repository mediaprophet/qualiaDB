# Semantic Header Schema
**Version:** 1.0.0
**Target Environment:** QualiaDB `0.0.17-dev`

## 1. Overview
The Semantic Header is a mandatory block of N3/Turtle code prepended to every Agent Intent JSONL log. It explicitly binds the unstructured conversation log to the rigid 48-byte nquin graph, enabling the `n3logic.rs` AOT compiler to dynamically enforce execution constraints (epistemic, temporal, defeasible, and deontic) prior to graph entry.

This specification rigidly defines the syntax for the 6 Vectors of Transparency, the `OP_RESOURCE_DECLARATION` token bid, and the advanced SHACL routing vocabulary.

---

## 2. The 6 Vectors of Transparency (N3 Syntax)

Every Semantic Header MUST contain the following `PREFIX` declarations and declare exactly six vectors.

```turtle
@prefix q42: <http://webizen.org/ns/core#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix agent: <did:q42:root:> .
@prefix task: <urn:uuid:> .

# Vector 1: Who (Identity Resolution)
agent:e30d6469-77fb q42:usesSubkey "ed25519:abcd1234efgh" ;
    q42:intentStatus q42:STATUS_COMPLETED .

# Vector 2: When (Temporal Origin)
agent:e30d6469-77fb q42:executedAt "2026-06-21T10:30:00Z"^^xsd:dateTime .

# Vector 3: Why (Task Provenance & Lock Acquisition)
agent:e30d6469-77fb q42:acquiresLock task:123e4567-e89b-12d3-a456-426614174000 .
# Alternatively, for Structural Modularisation Tasks:
# agent:e30d6469-77fb q42:structuralRefactor "crates/qualia-core-db/src/specialized_libs/" .

# Vector 4: What (Action Payload Hash)
agent:e30d6469-77fb q42:actionPayload "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" .

# Vector 5: Where (Graph State Dependency)
agent:e30d6469-77fb q42:executesOnGraphState "lamport_root:1A2B3C4D5E" .

# Vector 6: Cost (Resource Expenditure)
agent:e30d6469-77fb q42:burnsTokens "2500"^^xsd:integer .
```

---

## 3. OP_RESOURCE_DECLARATION (The Token Bid)

Before acquiring a temporal lock, the agent must declare its expected resource usage via `OP_RESOURCE_DECLARATION`. This establishes the baseline for Vector 6. If the agent's actual telemetry at the end of the session exceeds this bid by 110%, the Defeasible Logic engine will automatically flag the transition with `STATUS_ABORTED_COST_OVERRUN`.

```turtle
# The Token Bid (Pre-flight Resource Declaration)
task:123e4567-e89b-12d3-a456-426614174000 a q42:ResourceDeclaration ;
    q42:maxTokens "3000"^^xsd:integer ;
    q42:estimatedComputeCycles "5000000"^^xsd:integer ;
    q42:apiCostCap "0.05"^^xsd:decimal .
```

If the final `q42:burnsTokens` (Vector 6) is `"3500"^^xsd:integer` (which is > 3000 * 1.10), the ingestion compiler will quarantine the agent's work.

---

## 4. Multi-Modal Logic & Hardware Routing Triggers

The `n3logic.rs` routing intelligence dynamically parses the header to trigger specialized subsystems. Agents may append these specific predicates to spin up hardware compute layers or invoke advanced logic checks:

### Hardware Accelerators & Constraints
*   **Geometric Algebra:** 
    `agent:e30d6469 q42:usesPGA task:123 .` 
    *(Triggers SIMD topology kernels).*
*   **Calculus Solver:** 
    `agent:e30d6469 q42:solveCalculus "ode_params" .` 
    *(Triggers ODE native solvers).*
*   **QPU Operations:** 
    `agent:e30d6469 q42:quantumOptimize "graph_state" .` 
    > **WARNING:** The QPU is offline. Declaring this predicate will immediately trigger a `HardwareUnavailable` fault, hard-crashing the compilation.

### Specialized SHACL Libraries
*   **Chemistry Modeling:** `agent:e30d6469 q42:chemistryModeling "molecule_hash" .`
*   **Physics Simulation:** `agent:e30d6469 q42:physicsSimulation "boundary_cond" .`
*   **Medical Computing:** `agent:e30d6469 q42:medicalComputing "clinical_data" .`
*   **Cryptography:** `agent:e30d6469 q42:cryptography "key_sig" .`

### Advanced Logic Modalities
*   **Paraconsistent Logic:** `agent:e30d6469 q42:paraconsistentCheck "contradiction_hash" .` *(Triggers safe isolation of paradoxes).*
*   **Spatio-Temporal Matrices:** `agent:e30d6469 q42:spatialRegion "geo_bounds" .`

---

## 5. Conflict Resolution Status Codes

The Semantic Header MUST terminate with an explicit `q42:intentStatus` indicating the final disposition of the intent log.

*   `q42:STATUS_COMPLETED`
*   `q42:STATUS_LOCKED`
*   `q42:STATUS_ABORTED_COST_OVERRUN`
*   `q42:STATUS_ABORTED_TIMEOUT`
*   `q42:STATUS_CONTESTED`
