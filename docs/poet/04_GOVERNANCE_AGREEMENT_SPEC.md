# POET Governance, Agreement & Deontic Logic Specification

**Document ID:** `POET-SPEC-004`  
**Status:** Canonical Domain Specification  
**Scope:** Digital contracts, multi-party consensus, Deontic logic evaluation, dispute resolution, and fiduciary governance in POET.

---

## 1. Overview & Deontic Governance Paradigm

Governance in the Webizen NOS is grounded in mathematically verifiable contracts, formal deontic logic, and Decentralized Identifiers (DIDs). The environment enables individuals, cooperatives, and organizations to formalize agreements, monitor compliance, and resolve disputes without relying on coercive centralized intermediaries.

```
+-----------------------------------------------------------------------------------+
|                        GOVERNANCE & AGREEMENT TOPOLOGY                            |
+-----------------------------------------------------------------------------------+
|  [Agreement & Contract Builder] <===> [Deontic Logic Inspector]                   |
|  - Multi-party DID selection          - Obligations (OP_OBLIGATE: 0x10)           |
|  - Clause & term composition          - Permissions (OP_PERMIT: 0x11)             |
|  - Effective dates & expiry           - Prohibitions (OP_FORBID: 0x12)            |
|                                       - Defeater tracking & condition evaluation  |
|                                                                                   |
|  [Consensus & Multi-Sig Flow]   <===> [Dispute Resolution & Remedy Timeline]      |
|  - M-of-N signature collection        - Claim & breach submission                 |
|  - SuspendedTransactionQueue          - Cryptographic evidence attachment         |
|  - DID signing ceremonies             - Remediation receipts & outcome ledger     |
+-----------------------------------------------------------------------------------+
```

---

## 2. Visual Agreement & Contract Builder

The Agreement Builder provides a guided, step-by-step authoring workflow:

### 2.1 Authoring Steps
1. **Parties & Roles:** Add participant DIDs, assign organizational roles (e.g., Fiduciary, Contributor, Auditor, Guardian), and define signing thresholds.
2. **Scope & Intent:** Define agreement domain scopes (e.g., Intellectual Property, Compensation, Resource Commons, Bilateral Service).
3. **Clauses & Terms:** Compose human-readable clauses paired with machine-executable N3 / Super-Quin semantic rules.
4. **Temporal Bounds:** Set effective start date, expiration timestamp, and optional renewal triggers.
5. **Signing Ceremony:** Collect Ed25519 / ML-DSA cryptographic signatures from all required party DIDs.

---

## 3. Deontic Logic Inspector & Rule Engine

The Deontic Logic Inspector visually surfaces the machine-verifiable normative state of the contract:

```
+-----------------------------------------------------------------------------------+
|                           DEONTIC NORM VISUALIZERS                                |
+-----------------------+-----------------------+-----------------------------------+
| 🟢 PERMISSIONS (0x11)  | 🔵 OBLIGATIONS (0x10)  | 🔴 PROHIBITIONS (0x12)            |
| Permitted actions and | Mandatory duties with | Forbidden actions; violations     |
| access rights granted | deadlines and fulfillment| trigger immediate breach alerts   |
| to specific DIDs.     | criteria.             | and quarantine routing.           |
+-----------------------+-----------------------+-----------------------------------+
```

- **Defeater Evaluation:** Visual indicators display when a defeater rule (`DEFEATER_BIT`) overrides a defeasible obligation or permission.
- **Expiry & Compliance Badges:** Active obligations count down remaining time; expired unfulfilled duties transition to `Breach` state.

---

## 4. Multi-Party Consensus & Suspended Execution

For agreements requiring multi-party approval:
- **Consensus Thresholds:** Configure M-of-N threshold rules (e.g., 3-of-5 guardians required).
- **Suspended Transaction Queue:** Pending actions remain safely suspended in the 32-slot `SuspendedTransactionQueue` until sufficient valid signatures are collected.
- **Signature Progress Bar:** Real-time visual progress showing collected signatures vs. required threshold.

---

## 5. Dispute Resolution & Remedy Timelines

When disputes arise:
- **Claim Submission:** Aggrieved parties submit a structured claim referencing specific agreement clauses and alleged breaches.
- **Evidence Timeline:** Chronological timeline linking cryptographic receipts, message logs, and asset hashes.
- **Remediation & Remedy:** The contract specifies formal remedy paths (e.g., rollback, compensation transfer, privilege revocation). As mandated by root `AGENTS.md`, apologies do not constitute remedy; concrete reversible actions and auditable records are enforced.

---

## 6. Governance Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-GOV-001` | **Visual Agreement Builder** | Step-by-step multi-party agreement creation with party DIDs, role assignment, and clause composition. | `agreement_views`, `governance_workflow.rs` |
| `POET-GOV-002` | **DID Signing Ceremony Flow** | Interactive cryptographic signing interface collecting signatures from required party DIDs. | `governance_views`, `crdt.rs` |
| `POET-GOV-003` | **Deontic Norm Visualizer** | Real-time visual display of active Obligations, Permissions, and Prohibitions with status badges. | `deontic_logic.rs`, `logic_workbench.rs` |
| `POET-GOV-004` | **Defeater & Expiry Engine** | Interactive tracking of defeater conditions and countdown timers for time-bounded obligations. | `deontic_logic.rs`, `governance_views` |
| `POET-GOV-005` | **M-of-N Consensus Tracker** | Visual tracking of M-of-N signature collection and `SuspendedTransactionQueue` execution state. | `crdt.rs`, `governance_workflow.rs` |
| `POET-GOV-006` | **Dispute Resolution Timeline** | Interactive timeline for claim filing, evidence presentation, counter-submissions, and resolution. | `governance_views`, `rights_views` |
| `POET-GOV-007` | **Fiduciary Remedy Ledger** | Audit log tracking concrete remediation actions, privilege adjustments, and compensation flows. | `governance_views`, `budget_workspace.rs` |
| `POET-GOV-008` | **Human Rights & Non-Adversarial Guard**| Structural audit rules enforcing non-adversarial conduct and international human rights baselines. | `qualia-core-db`, `AGENTS.md` |
