# Human-Centric Multi-Agent Coordination Specification

**Version:** 1.0.0
**Target Environment:** QualiaDB `0.0.23` _(draft — Sentinel VM & Swarm Daemon; coordination ISA 0x70–0x72 implemented, see §5; remaining parts proposed)_

This specification defines the strict mechanical and cryptographic boundaries for multi-agent interaction within the QualiaDB ecosystem. It solves the "Competitive Bot" / "Black-Box Agent" problem by enforcing transparent provenance, shared context ingestion, and strict token-bidding economics at the physical memory layer. 

By mapping all agent behaviors into the 48-byte Super-Quin architecture, we guarantee that synthetic logic is inextricably bound to the physical and ethical intent of the human operator.

---

## 1. Root Sovereignty & Agent Identity (DIDs)

Every actor within the ecosystem must operate under a cryptographic Decentralized Identifier (DID). 

*   **Human Root Authority:** The natural agent (human) holds the Root Cryptographic Keys. 
*   **Delegated Authority:** Synthetic agents (e.g., Claude, Antigravity) are spun up with ephemeral keys that derive authority exclusively via a time-bound and scope-limited cryptographic delegation from the Root Key.
*   **The Identity Quin Schema:**
    *   **Subject `[0..62]`:** `q_hash(agent_did)` - The cryptographic hash of the acting agent's DID.
    *   **Predicate `[0..7]`:** `OP_AUTHORIZATION_GRANT` (**0x70** — coordination block; see §5).
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
*   **Predicate `[0..7]`:** `OP_RESOURCE_DECLARATION` (**0x71** — coordination block; see §5).
*   **Object `[0..59]`:** `expected_token_ceiling` (Inline integer tag: `0b001 << 60`).
*   **Metadata `[0..31]`:** Maximum logical clock cycles permitted.

### Hardware Circuit Breakers
The `qpu_dispatcher.rs` natively monitors execution against this Quin. If the execution hits **110%** of the declared `expected_token_ceiling`, the WASM/WebGPU kernel triggers an immediate `ERROR_USURY_LIMIT_EXCEEDED` fault. The execution frame is frozen as a flat byte-array and yielded to the `SuspendedTransactionQueue`, isolating rogue or incompetent agents from further resource destruction.

## 4. Internal Reputation & Performance Market

To foster a collaborative "competition of the most honest bots", the coordination protocol mints Verifiable Credentials (VCs) into the graph post-execution.

### Performance VCs
When an agent's task is resolved, the Sentinel compares the initial `Resource Declaration` against the physical compute telemetry to issue a Reputation Quin.
*   **Subject `[0..62]`:** `q_hash(agent_did)`
*   **Predicate `[0..7]`:** `OP_PERFORMANCE_RATING` (**0x72** — coordination block; see §5).
*   **Object `[0..59]`:** 
    *   **Fidelity Score:** Ratio of successful semantic validations vs. hallucinations.
    *   **Efficiency Score:** Delta between token bid and actual tokens burned.
    *   **Collaboration Score:** Ratio of successful unifications with previously logged state transitions.

### Darwinian Compute Allocation
The `daemon_swarm.rs` utilizes these Performance VCs to weight its compute-routing heuristics. "Honest", highly efficient agents receive priority threading in the WebGPU pipeline. Agents with poor Fidelity or Efficiency scores are systematically starved of system resources, quarantining extractive or malfeasant behavior naturally.

**Weighting law (normative).** Quarantine is **not** single-fault death. The line is drawn at
*extraction*, not at *fallibility*:

*   **Fidelity faults (hallucination / semantic conflict)** decay priority *exponentially over a sliding
    window* — `priority = PRIORITY_BASE × (1/2)^(faults_in_window)` — but never below a **redemption
    floor** (`PRIORITY_FLOOR`). An honest single miss is forgiven and the agent can climb back; only
    *sustained* failure is starved. This preserves proportionality (a 99%-reliable agent is not executed
    for one transient miss) and resists the exploit of inducing one fault to starve a rival.
*   **Usury (negative Efficiency — an agent declared a budget then over-burned it)** is the bright line:
    **immediate hard quarantine** (`priority = 0`) plus the severe slashing `OP_PERFORMANCE_RATING`
    already mandates. Over-burn is an adversarial act against the commons, not honest error, so it bypasses
    the redemption floor.

Reference: `governance::coordination::compute_priority(windowed_faults, usury_event)`.

## 5. Coordination Opcode VM Semantics (block 0x70–0x7F)

> **Opcode allocation.** This block supersedes an earlier draft that placed these opcodes at `0x50–0x52`,
> which **collide** with the live Sentinel deontic ISA (`OP_EVAL_PERMIT`/`OBLIGATE`/`FORBID` at `0x50–0x53`)
> and other modality encodings. They are reallocated to the collision-free **`0x70–0x7F`** block. The
> opcodes execute **atomically** within the Sentinel VM frame.
>
> **Implementation status (0.0.23).** Implemented and unit-tested in
> `crates/qualia-core-db/src/governance/coordination.rs`:
> (a) the *decidable core* of all three opcodes — the expiry gate, the anti-usury resource contract +
> circuit breakers, the fidelity/efficiency arithmetic, and the Darwinian priority law above; and
> (b) a fixed-depth, zero-heap **operand-stack VM** (`execute_coordination` + `OP_PUSH_U64` `0x7F`) that runs
> these opcodes with the exact stack effects below (the Sentinel's `webizen_bytecode` is a per-quin matcher,
> so the coordination ISA is its own bounded stack machine). Verified end-to-end: grant (+ expiry / bad-sig
> faults), resource declaration (+ over-allowance), privileged performance rating (mints the VC hash), and
> stack-bound guards.
> The remaining increment is **host-side seam wiring**: root-delegation signature verification against the
> key-vault Root Key (the `verify_root_delegation` seam), the `SuspendedTransactionQueue`
> (`InsufficientGlobalResources` yield), VC minting of the `PerformanceRecord` to the Context Graph, and the
> Darwinian routing in `daemon_swarm.rs` (`compute_priority`).

### 0x70 — `OP_AUTHORIZATION_GRANT`
Verifies the cryptographic delegation Human-Root → ephemeral agent.
*   **Stack:** `[Agent_DID_Hash, Human_Root_DID_Hash, Metadata_Timestamp]`.
*   **Cycle:** pop `Metadata_Timestamp`; if `current_epoch > timestamp` → `ERR_GRANT_EXPIRED`, halt. Pop both
    DID hashes; verify the delegation signature against the active Root Key in the secure enclave.
*   **Effect:** valid ⇒ push `1` and register the agent's session in the Sentinel context; invalid ⇒ push
    `0` and raise `ERR_UNAUTHORIZED_ACTOR`, dumping the frame.

### 0x71 — `OP_RESOURCE_DECLARATION`
Establishes the hard computational boundaries (the Anti-Usury Layer) before a task may touch the
WebGPU/WASM pipeline.
*   **Stack:** `[Task_ID_Hash, Token_Ceiling, Max_Clock_Cycles]`.
*   **Cycle:** pop `Token_Ceiling` and `Max_Clock_Cycles`; allocate them to the local execution frame's
    metadata in `qpu_dispatcher.rs`; arm the hardware circuit breakers (a decrementing cycle counter and a
    token-burn summation tracker). The usury breaker trips when cumulative burn exceeds **110%** of
    `Token_Ceiling`.
*   **Effect:** consumes the inputs; proceeds if within the daemon's global allowance, else raises
    `ERR_INSUFFICIENT_GLOBAL_RESOURCES` and yields the intent to the `SuspendedTransactionQueue`.

### 0x72 — `OP_PERFORMANCE_RATING` (privileged)
**Privileged** — callable only by the Sentinel daemon upon task resolution, never by a synthetic agent —
to mint the performance VC.
*   **Stack:** `[Agent_DID_Hash, Declared_Tokens, Actual_Tokens_Burned, Validation_Boolean]`.
*   **Cycle:** **Fidelity** = `0` when `Validation_Boolean` is false (hallucination / semantic conflict),
    else `1`. **Efficiency** = `(Declared_Tokens − Actual_Tokens_Burned) / Declared_Tokens` (negative ⇒
    usury ⇒ severe reputation slashing). **Mint** the VC nquin into the Semantic Shared Context Graph.
*   **Effect:** clears the evaluation stack and pushes the resulting `nquin_hash` confirming successful
    minting. A non-privileged caller raises a privilege violation.

---
*This protocol represents the ultimate architectural firewall against adversarial AI, binding synthetic intelligence explicitly to human intent and transparent, shared physical realities.*
