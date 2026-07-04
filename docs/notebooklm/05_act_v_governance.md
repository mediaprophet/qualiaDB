# Act V — Governance

> *The engine refuses. With a reason. In a language the caller can read.*

---

## Thesis

> **Every action in the engine is gated by a norm. The norm is a Quin. The
> Quin is readable. The refusal is a Quin. The audit log is a Quin. The
> governance is not a layer above the engine; it is the engine.**

---

## Voice-over script

### Shot 1 — A request enters the engine. It is wrapped in an NQuin. [SLOW]

> Every request enters as an NQuin. [PAUSE]
> The subject is the caller. The predicate is the action. The object is
> the target. The context is the contract. [PAUSE]

### Shot 2 — The request passes through `validate_intent`. The N3Logic Rights Ontology is consulted. [SLOW]

> The first gate is `validate_intent`. [PAUSE]
> It reads the N3Logic Rights Ontology — a set of N3 rules compiled into
> deontic Quins. [PAUSE]
> The rules say who may do what, under which contract, in which context. [PAUSE]
> If the rules say `Deny`, the request is refused. [PAUSE]
> The refusal is written to the write-ahead log, signed with Ed25519,
> with the caller's DID, with the timestamp. [PAUSE]

### Shot 3 — The request passes the pre-flight. It enters the inference loop. [SLOW]

> If the rules say `Permit`, the request enters the inference loop. [PAUSE]
> The Sentinel watches the logits. [PAUSE]
> If the Sentinel detects an anomaly, the token is rolled back. [PAUSE]

### Shot 4 — The output is produced. It is validated. [SLOW]

> The output is then validated. [PAUSE]
> It must have at least one provenance NQuin citation. [PAUSE]
> The citation must point to a fact in the graph. [PAUSE]
> The fact must be in the contract's accessible layers. [PAUSE]

### Shot 5 — A deontic contract is shown. It has obligations, permissions, prohibitions. Some are defeasible. [ITEM]

> The contract is a set of deontic Quins. [PAUSE] [ITEM]
> Obligate — the agent must do this. [PAUSE] [ITEM]
> Permit — the agent may do this. [PAUSE] [ITEM]
> Forbid — the agent must not do this. [PAUSE] [ITEM]
> Defeater — this rule may be overridden by a higher-priority rule. [END LIST] [PAUSE]

### Shot 6 — A defeater is encountered. The defeasible rule is overridden. A new verdict is emitted. [SLOW]

> A defeater is a rule with the high bit of the predicate set. [PAUSE]
> When a defeater matches, the defeasible rule is removed. [PAUSE]
> The remaining rules are re-evaluated. [PAUSE]
> The verdict is emitted. [PAUSE]

### Shot 7 — A contradiction is detected. Two Quins disagree about the same fact. [SLOW]

> Sometimes two Quins disagree. [PAUSE]
> The same subject. The same predicate. Different objects. [PAUSE]
> Classical logic would say: everything is now provable. [PAUSE]
> Paraconsistent logic says: isolate the second-arriving Quin into a
> quarantine context. The rest of the system keeps running. [PAUSE]

### Shot 8 — The quarantine context is shown as a separate node graph, sealed off from the main graph. [SLOW]

> The quarantine context is a separate node graph. [PAUSE]
> It is sealed off from the main graph. [PAUSE]
> The main graph does not see it. The quarantine graph does not see the
> main graph. [PAUSE]
> A merge requires external authority. [PAUSE]

### Shot 9 — A multi-party contract is ratified. Three guardians must consent. [SLOW]

> Some contracts require multi-party ratification. [PAUSE]
> Three guardians must each apply a consent token. [PAUSE]
> The transaction is suspended until two of three have consented. [PAUSE]
> The suspended transaction queue holds the frame. [PAUSE]
> When the threshold is met, the frame is executed. [PAUSE]

### Shot 10 — The MCP cooperation gate. A caller without a verified DID is denied. [SLOW]

> The MCP server has a cooperation gate. [PAUSE]
> A caller without a verified DID is denied. [PAUSE]
> A caller with a verified DID but no grounding principal is denied. [PAUSE]
> A caller with a verified, grounded DID may proceed — but only if the
> request does not violate a non-derogable norm. [PAUSE]

### Shot 11 — Title card: **The governance is the engine.** [SLOW]

> The governance is not a layer above the engine. [PAUSE]
> The governance is the engine. [PAUSE]

---

## On-screen notes

- **Shot 1:** A single NQuin entering the engine from the left. The fields are labeled.
- **Shot 2:** The N3Logic Rights Ontology is shown as a small graph. The rule that fires is highlighted.
- **Shot 3:** The request passes the gate. The arrow turns green.
- **Shot 4:** The output box. The provenance citation is highlighted.
- **Shot 5:** A deontic contract. Four rules, color-coded by opcode.
- **Shot 6:** A defeater matches. The defeasible rule is struck through. A new verdict is emitted.
- **Shot 7:** Two Quins, same subject and predicate, different objects. They flash red.
- **Shot 8:** A second node graph appears, sealed off. The seal is a thick line.
- **Shot 9:** Three guardians. Two have consented (green check). One has not (red X). The transaction is suspended.
- **Shot 10:** The MCP server. A caller without a DID is shown being denied. The denial reason is readable.
- **Shot 11:** Title card.

---

## Source code anchors

- `crates/qualia-core-db/src/deontic_logic.rs` — `OP_OBLIGATE`, `OP_PERMIT`, `OP_FORBID`, `DEFEATER_BIT`, `evaluate_deontic_contract`, `compile_n3_rule_to_norm`, 10/10 tests.
- `crates/qualia-core-db/src/modalities/paraconsistent.rs` — `OP_ISOLATE`, `OP_CONTRADICTION_SCORE`, `OP_PARACONSISTENT_MERGE`, `route_paraconsistent`.
- `crates/qualia-core-db/src/modalities/defeasible.rs` — `evaluate_defeasible_frame`, `resolve_conflict`, `grounded_justified_rules`.
- `crates/qualia-core-db/src/modalities/epistemic.rs` — `OP_KNOWS`, `OP_BELIEVES`, `OP_COMMON_KNOWLEDGE`, `evaluate_epistemic_frame`.
- `crates/qualia-core-db/src/modalities/responsibility.rs` — `adjudicate`, `appraise`, `degree_of_responsibility`.
- `crates/qualia-core-db/src/modalities/capacity.rs` — `capacity_from_age`, `detect_duress`, `guardianship_authorized`.
- `crates/qualia-core-db/src/modalities/delegation.rs` — `delegation_in_force`, `attenuates`, `authority_after_crl`.
- `crates/qualia-core-db/src/modalities/jural.rs` — `jural_correlativity_holds`, `find_unmet_correlatives`, `resolve_collision`.
- `crates/qualia-core-db/src/modalities/interaction_governance.rs` — `govern_verdict`, `permits_execution`, `circuit_breaker`.
- `crates/qualia-core-db/src/modalities/illocution.rs` (in `governance/`) — `resolve_conflict`, `effective_weight`.
- `crates/qualia-core-db/src/modalities/consensus.rs` — `bft_quorum`, `bft_committed`, `vc_happens_before`.
- `crates/qualia-core-db/src/governance/webizen.rs` — `SlgArena`, `fire_registered_rules`, `SuspendedTransactionQueue`.
- `crates/qualia-core-db/src/foundation/crdt.rs` — `SuspendedTransactionQueue::apply_consensus_token`, `resolve_lww`.
- `crates/qualia-core-db/src/mcp/mcp_cooperation.rs` — `authorize_call`, `caller_grounded`, `unverified_caller_is_denied`.
- `crates/qualia-client-core/src/guardianship.rs` — `apply_guardian_token`, `deny_guardian_affirmation`, `is_agreement_ratified`.
- `AGENTS.md §4-F` — `DelegatedAccess` is now `[u8; 32]` DID hashes + `[u8; 64]` Ed25519 signatures, no `String` allocation.

---

## Duration

Approximately 120 seconds. This is the act where the viewer learns that the engine is not just fast — it is *accountable*.
