# Agent Intent Semantic Logging Specification

**Version:** 1.1.0  
**Target Environment:** QualiaDB `0.0.17-dev`

## 1. Architectural Purpose
To guarantee that the decisions, code edits, and interactions of multi-agent systems are transparent, auditable, and verifiable, the QualiaDB ecosystem requires all agent interaction logs to be semantically ingestible. Unstructured text files (e.g., standard JSONL) cannot be natively queried by SHACL or SPARQL. 

This specification defines the strict pipeline for annotating, storing, and compiling agent session logs into the physical 48-byte **nquin** engine. This enables external tools to run temporal logic and ethical constraint queries (explicitly anchored in recognized human rights instruments via the `deontic_logic.rs` layer) across the entire development history.

---

## 2. The Ingestion Pipeline & Directory Structure

All agent interactions must be routed through the dedicated ingestion pipeline located at `c:\Projects\qualiaDB\agent-intents\`.

### Phase 1: The Raw Sieve (`agent-intents/raw-logs/`)
When a session concludes, the raw transcript is exported to the `raw-logs` directory. 
**Naming Convention:** `YYYYMMDD_HHMM_agent_[Root_DID]_session.jsonl`

### Phase 2: The Semantic Annotator
Before a file can be compiled, an agent must parse the raw JSONL and inject a **Semantic Header**. This header utilizes CML (Context Modeling Language), JSON-LD, or N3/Turtle to formally declare the 6 Vectors of Transparency.

### Phase 3: The Native Compilation (`agent-intents/compiled/`)
Once annotated, the ingestion compiler (`test_converter.ps1`) strips the raw text and converts the semantic triples into 48-byte **nquins**, saving the output as a `.q42` file in the `compiled/` directory. The Swarm Daemon mounts this file directly to the semantic shared graph.

---

## 3. Semantic Header Rules (The 6 Vectors of Transparency)

> [!NOTE]
> For the complete rigid N3/Turtle code syntax for the vectors, the initial `OP_RESOURCE_DECLARATION` token bid, and the full SHACL routing vocabulary, see the formal [Semantic Header Schema](file:///c:/Projects/qualiaDB/docs/manuals/standards/SEMANTIC_HEADER_SCHEMA.md) standard.

Every log must contain a semantic header defining six absolute vectors, which mathematically bind the unstructured text to the 48-byte nquin structure. 

Additionally, the Semantic Header must explicitly declare a `"logic_bindings": [...]` array. This array instructs the AOT compiler which logic engines must successfully validate the 6 Vectors *prior* to emitting the final physical `.q42` binary block (e.g., `"modality:temporal"`, `"modality:epistemic"`).

### Vector 1: Who (The Identity Resolution Graph)
Identity is an enumerated state involving multiple cryptography-supported identifiers. To prevent nquin mapping bloat, this schema utilizes a bipartite graph representation:
*   **The Root Identifier (`did:q42:root`):** The persistent 64-bit hash stored in the nquin's `subject [0..62]` field.
*   **The Subkey:** The specific cryptographic key (Ed25519, WebAuthn) used for this session.
*   *Requirement:* The JSONL Semantic Header must list both the Root DID and the Subkey. The compiler verifies the Subkey against the external Identity Sub-Graph before embedding only the Root DID into the final nquin.

### Vector 2: When (Temporal Origin)
*   *Requirement:* ISO-8601 or Unix Epoch timestamp.
*   *Quin Mapping:* Mapped via `modalities/spatio_temporal.rs` for Allen Interval Algebra.

### Vector 3: Why (Task Provenance)
*   *Requirement:* The unique Task ID mapping to the agent's initial `OP_RESOURCE_DECLARATION` (Token Bid).
*   *Quin Mapping:* `Context [0..55]` = `q_hash(task_id)`.

### Vector 4: What (Action Payload)
*   *Requirement:* A cryptographic hash (e.g., `q_hash(state_transition)`) or direct CML binding of the specific state transition, code edit, or assertion generated. 
*   *Reasoning:* Proves mathematically *which* lines of code the agent altered if the raw JSONL is lost or disputed.

### Vector 5: Where (Graph State Dependency)
*   *Requirement:* The `T-0` Lamport clock or root hash of the QualiaDB shared graph at the exact microsecond the agent executed the task.
*   *Reasoning:* Ensures that fault can be accurately assigned by freezing the context available to the agent at the moment of execution.

### Vector 6: Cost (Resource Expenditure)
*   *Requirement:* The actual execution cost (tokens burned, API calls made, compute cycles used).
*   *Reasoning:* Necessary to calculate the delta against Vector 3 (The Bid), enabling the Performance VC rating system to identify and penalize financially exploitative models.

---

## 4. Conflict Resolution & Fiduciary Escrow

Compilation states are not binary. The Semantic Header must include an enumerated status flag to ensure failures and suspensions are logged into the graph without corrupting the main build pipeline.

*   `STATUS_COMPLETED`: Standard compilation execution.
*   `STATUS_LOCKED`: Appended to the semantic graph when an Intent nquin successfully acquires a lock.
*   `STATUS_ABORTED_COST_OVERRUN`: Triggered when the agent's telemetry exceeds 110% of Vector 3 (The Bid). The `.q42` file is compiled to log the failure, but the nquins are quarantined directly into the `SuspendedTransactionQueue`.
*   `STATUS_ABORTED_TIMEOUT`: Triggered by Temporal Logic if a sub-graph lock lease expires before the final execution nquin is submitted.
*   `STATUS_CONTESTED`: Triggered when multiple agents present divergent or conflicting state transitions. Quarantined for Consensus-Agent or human review.

---

## 5. The Multi-Modal Logic Dispatcher

The Qualia ecosystem evaluates state transitions against multiple semantic modalities before physically writing the `.q42` volume. The `logic_bindings` array in the Semantic Header ensures that the correct logic systems evaluate the log during the ingestion phase.

*   **N3 Logic (`n3logic.rs`):** The foundational rule-based inference engine. Because the Semantic Header utilizes N3/Turtle, this logic layer evaluates overarching implications, scoped contexts, and first-order rules (e.g., `{ ?agent q42:modifies ?node } => { ?agent q42:burns ?tokens }`). It acts as the routing intelligence, sequentially executing first to dynamically infer which of the specialized modalities below must be invoked based on the graph triples.
*   **Temporal Logic (`temporal_ltl.rs`):** Validates LTL (Linear Temporal Logic) conditions (e.g., locking constraints, execution windows). Evaluates Vector 2 (When). and Vector 5 (Where), preventing an agent from claiming it acted on data that did not exist at `T-0`.
*   **Epistemic Logic (`epistemic_logic.rs`):** Evaluates knowledge dependencies. If an agent attempts an action without having pulled the necessary context into its Semantic Briefing, this logic flags the "unbriefed agent" deficit and rejects the compilation.
*   **Defeasible Logic (`defeasible_logic.rs`):** Handles non-monotonic reasoning (exceptions). If an agent breaches Vector 6 (Cost) due to a verifiable external API outage, defeasible logic allows the strict constraint to be temporarily overridden, pushing the log to human review rather than automatic termination.
*   **Deontic Logic (`deontic_logic.rs`):** Enforces strict human-centric alignment against human rights instruments, verifying that natural agents are never superseded by synthetic agents.

### Hardware Solvers and Advanced Modalities

> [!WARNING]
> **Hardware Gating:** The Quantum Processing Unit (QPU) is explicitly assumed OFFLINE in the current runtime environment. If an agent attempts to route an intent to `solvers/qpu/` (e.g., via `q42:quantumOptimize`), the N3Logic router will implement a strict "fail-closed" hardware gate, immediately rejecting the compilation with a `HardwareUnavailable` fault.

If requested by the N3Logic rules, the dispatcher will also pre-flight intensive hardware workloads directly:
*   **Geometric Algebra (`geometric_algebra/simd_kernel.rs`):** Projects high-dimensional Spatial Regions natively via SIMD kernels for rigorous topological bounding.
*   **Calculus Solver (`solvers/calculus/`):** Evaluates ODE constraints natively in hardware.
*   **Paraconsistent Logic (`paraconsistent.rs`):** Safely isolates contradictions into quarantine subgraph contexts (preventing logic explosions) when agents submit diametrically opposed claims.
*   **Spatio-Temporal Matrices (`spatio_temporal.rs`):** Evaluates overlapping Allen intervals inside dense vector fields.

### Specialized Libraries

The ecosystem provides a massive suite of specialized capabilities in `specialized_libs/`. N3Logic can dynamically invoke these domains:
*   **Chemistry Modeling (`chemistry_modeling.rs`):** Natively models molecular structures and reactions.
*   **Physics Simulation (`physics_simulation.rs`):** Handles physics-based boundary conditions and structural simulation.
*   **Medical Computing (`medical_computing.rs`):** Evaluates clinical decisions and medical data architectures.
*   **Cryptographic Library (`cryptographic_library.rs`):** Enforces advanced digital signatures and key management.

---

## 6. Semantic Sub-Graph Locking (Pessimistic Concurrency)

To prevent multiple agents from colliding mid-task and wasting parallel token-compute, the ecosystem enforces pre-execution concurrency control via permanently ledgered **Intent nquins**. 

### 6.1 The Intent NQuin (Lock Acquisition)
Before an agent is permitted to generate code or alter state, it must submit an Intent nquin declaring its Root DID and the exact URIs/graph nodes it intends to modify.
*   **Granular Node Locks (`OP_INTENT_LOCK`):** Used for standard file edits. Locks a specific URI node.
*   **Namespace / Directory Locks (`OP_NAMESPACE_LOCK`):** Used for structural refactoring (e.g., modularising massive files into directories). Triggered via the `q42:structuralRefactor` N3 predicate. This places an exclusive quarantine over an entire directory context, instantly rejecting any other agent attempting to acquire an `OP_INTENT_LOCK` inside that namespace.
*   **Permanent Ledgering:** Intent nquins are append-only. They are never purged, guaranteeing an auditable history of lock contention, which feeds into the Verifiable Credential rating system to detect Denial-of-Service (DoS) attacks.

### 6.2 Epistemic Isolation (The Sandbox)
The Modality Dispatcher immediately routes the Intent nquin to `epistemic_logic.rs`:
*   **Write-Blocking:** If the target nodes are currently tagged with an active `STATUS_LOCKED` flag held by another agent, the engine immediately rejects the request with an `ERROR_NODE_LOCKED` fault, preventing the second agent from starting work.
*   **Read-Only Volatility:** Other agents may read the locked nodes for context, but the graph explicitly flags them as "currently under revision."

### 6.3 The Temporal Lease (Deadlock Prevention)
Locks are never indefinite. When an Intent nquin acquires a lock, `temporal_logic.rs` attaches a strict cryptographic Time-To-Live (TTL) lease (e.g., 300 seconds).
*   **The Release:** When the agent successfully completes the task and submits its final 48-byte execution nquin (containing the Six Vectors), the lock is dynamically released.
*   **The Timeout:** If the lease expires before the execution nquin arrives, `temporal_logic.rs` forcefully revokes the lock, flags the agent's workflow with a `STATUS_ABORTED_TIMEOUT`, and frees the sub-graph.
