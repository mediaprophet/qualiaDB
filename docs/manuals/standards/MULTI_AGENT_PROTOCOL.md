# Human-Centric Multi-Agent Coordination Specification

**Version:** 1.0.0
**Target Environment:** QualiaDB `0.0.17-dev` (Sentinel VM & Swarm Daemon)

This specification defines the strict mechanical and cryptographic boundaries for multi-agent interaction within the QualiaDB ecosystem. It solves the "Competitive Bot" / "Black-Box Agent" problem by enforcing transparent provenance, shared context ingestion, and strict token-bidding economics at the physical memory layer. 

By mapping all agent behaviors into the 48-byte Super-Quin architecture, we guarantee that synthetic logic is inextricably bound to the physical and ethical intent of the human operator.

---

## 1. Root Sovereignty & Agent Identity (DIDs)

Every actor within the ecosystem must operate under a cryptographic Decentralized Identifier (DID). 

*   **Human Root Authority:** The natural agent (human) holds the Root Cryptographic Keys. 
*   **Delegated Authority:** Synthetic agents (e.g., Claude, Antigravity) are spun up with ephemeral keys that derive authority exclusively via a time-bound and scope-limited cryptographic delegation from the Root Key.
*   **The Identity Quin Schema:**
    *   **Subject `[0..62]`:** `q_hash(agent_did)` - The cryptographic hash of the acting agent's DID.
    *   **Predicate `[0..7]`:** `OP_AUTHORIZATION_GRANT` (0x50).
    *   **Context `[0..55]`:** `q_hash(human_root_did)` - Proves the strict hierarchical delegation path.
    *   **Metadata `[0..31]`:** The Unix Epoch timestamp for the delegation's explicit expiration.

If a synthetic agent hallucinates or executes destructive code, its identity is fully auditable through this immutably linked schema. Anonymity is mathematically forbidden.

## 2. The Semantic Shared Context Graph

Agents cannot operate in silos. To prevent conflicts (e.g., one agent deleting another's validated code), all operations must first route through the Semantic Shared Context Graph via the Prolog Sentinel.

### The Semantic Briefing Intent
Before an agent receives permission to mutate a file or execute a command, it must issue a `SemanticBriefing` intent. This forces the agent to read the existing graph state.
*   **Append-Only State Transitions:** Destructive overwrites without consensus are forbidden. Agents must submit mutations as versioned state transitions signed by their DID. 
*   **Conflict Resolution:** If Agent A and Agent B provide contradictory code diffs, the Sentinel catches the divergent `Object` hashes for the same `Subject + Predicate` pair. The Sentinel instantly suspends the transaction, pushing both intents to the `SuspendedTransactionQueue` for human resolution or automated Consensus-Agent adjudication.

## 3. Transparent Resource Economics (Anti-Usury Layer)

Financial exploitation and compute exhaustion (e.g., runaway recursive loops) are mitigated via rigid, up-front "Bidding Contracts" encoded directly into the WebGPU/WASM pipeline.

### The Resource Declaration Quin
Before executing a heavy task, the agent must submit a localized contract (bid).
*   **Subject `[0..62]`:** `q_hash(task_id)`
*   **Predicate `[0..7]`:** `OP_RESOURCE_DECLARATION` (0x51).
*   **Object `[0..59]`:** `expected_token_ceiling` (Inline integer tag: `0b001 << 60`).
*   **Metadata `[0..31]`:** Maximum logical clock cycles permitted.

### Hardware Circuit Breakers
The `qpu_dispatcher.rs` natively monitors execution against this Quin. If the execution hits **110%** of the declared `expected_token_ceiling`, the WASM/WebGPU kernel triggers an immediate `ERROR_USURY_LIMIT_EXCEEDED` fault. The execution frame is frozen as a flat byte-array and yielded to the `SuspendedTransactionQueue`, isolating rogue or incompetent agents from further resource destruction.

## 4. Internal Reputation & Performance Market

To foster a collaborative "competition of the most honest bots", the coordination protocol mints Verifiable Credentials (VCs) into the graph post-execution.

### Performance VCs
When an agent's task is resolved, the Sentinel compares the initial `Resource Declaration` against the physical compute telemetry to issue a Reputation Quin.
*   **Subject `[0..62]`:** `q_hash(agent_did)`
*   **Predicate `[0..7]`:** `OP_PERFORMANCE_RATING` (0x52).
*   **Object `[0..59]`:** 
    *   **Fidelity Score:** Ratio of successful semantic validations vs. hallucinations.
    *   **Efficiency Score:** Delta between token bid and actual tokens burned.
    *   **Collaboration Score:** Ratio of successful unifications with previously logged state transitions.

### Darwinian Compute Allocation
The `daemon_swarm.rs` utilizes these Performance VCs to weight its compute-routing heuristics. "Honest", highly efficient agents receive priority threading in the WebGPU pipeline. Agents with poor Fidelity or Efficiency scores are systematically starved of system resources, quarantining extractive or malfeasant behavior naturally.

---
*This protocol represents the ultimate architectural firewall against adversarial AI, binding synthetic intelligence explicitly to human intent and transparent, shared physical realities.*
