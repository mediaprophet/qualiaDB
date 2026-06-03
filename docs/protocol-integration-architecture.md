---
layout: null
---

# Protocol Integration Architecture
**Cooperative Projects + Wellfair + Qualia Engine**

**Version:** 0.1 (June 2026)  
**Status:** Draft for review and iteration

---

## 1. Executive Summary

This document defines how decentralized protocols (GUN, WebTorrent, WebRTC, Git + git-mark) are integrated with the Qualia engine to power Cooperative Projects, while being deeply embedded into the **Wellfair** personal vault.

Wellfair serves a **dual purpose**:
- As a sovereign personal data vault (health, life events, consent, privacy).
- As a "fair dealings" layer that manages a person's project activities without violating personal boundaries, appointments, or wellbeing.

The Qualia engine (Rust core, Sentinel logic, provenance DAGs, Q42 model) acts as the unifying trust and logic layer across all protocols and applications.

---

## 2. Core Principles

1. **Provenance First** — Every significant action, contract, claim, share allocation, and protocol session must carry verifiable provenance.
2. **Consent & Agency as First-Class** — No data flow, scheduling, or obligation can override explicit consent or personal boundaries.
3. **Offline-First & Edge-Native** — Systems must function meaningfully without constant connectivity.
4. **Personal vs Project Boundary Protection** — Project demands must not automatically override personal life events, appointments, health needs, or rest.
5. **Human Dignity & Fairness** — Economic and collaboration models must treat unpaid contributors fairly and protect vulnerable participants.

---

## 3. Key Protocols & Their Roles

| Protocol       | Primary Strength                     | Integration Role in This System                                                                 | Qualia Engine Contribution                          |
|----------------|--------------------------------------|--------------------------------------------------------------------------------------------------|-----------------------------------------------------|
| **GUN**        | Decentralized graph sync, offline-first | Real-time / eventual sync of share state, claims, obligations, and live project data            | Sentinel logic for conflict resolution & consent   |
| **WebTorrent** | P2P file & media distribution        | Distribution of ontology files, Q42 datasets, claim bundles, large project artifacts            | Provenance tracking of distributed objects         |
| **WebRTC**     | Real-time video, voice, data channels| Contract negotiation calls, claim verification sessions, live collaboration                     | Session provenance + consent enforcement           |
| **Git + git-mark** | Versioned, signed history       | Contract versioning, claim history, audit trails, signed provenance of changes                  | Deep provenance DAG linking commits to ontology entities |
| **Qualia Engine** | Core logic, provenance, consent   | Unifying layer that all protocols feed into or are governed by                                  | Sentinel VM, Q42 model, provenance, N3Logic/SHACL  |

---

## 4. Qualia Engine as the Unifying Layer

The Qualia engine provides:
- **Q42 Graph Model** — Canonical representation of entities, relations, obligations, shares, and claims.
- **Sentinel VM** — Defeasible logic, SHACL validation, consent enforcement, and dynamic rule evaluation.
- **Provenance DAGs** — Cryptographically strong history of every meaningful change and event.
- **Edge-Native Design** — Low memory footprint suitable for mobile and constrained devices.

All protocol activity should ultimately produce or reference Q42 entities with proper provenance.

---

## 5. Wellfair Mobile Vault Requirements (Guidance for AI Agents)

Wellfair must support **two tightly integrated layers** while strongly protecting personal boundaries.

### 5.1 Personal Layer (Core Vault)
- Store and manage personal health data, life events, appointments, tasks, and preferences.
- Strong privacy modes (Sanctuary Mode, duress support, volatile memory where appropriate).
- Consent management for any data sharing with projects or other agents.
- Life-event awareness (family obligations, health appointments, rest periods, etc.).

### 5.2 Project / "Fair Dealings" Layer
- Manage contracts, verifiable claims, Dynamic Equity / Stewardship Shares, obligations, and payments.
- Allow the user to participate in multiple Cooperative Projects fairly.
- Track personal contributions (time, AI agents, infrastructure, etc.) with proper provenance.
- Support optional tokenization and cash-out flows (governed by project policy).

### 5.3 Boundary & Life-Event Awareness (Critical)
Wellfair must actively protect the user from project demands overriding personal life:
- Do **not** auto-schedule or pressure work during known personal appointments, family events, health needs, or rest periods without explicit consent.
- Surface conflicts clearly and require explicit opt-in before allowing project obligations to impact personal calendar.
- Support "Do Not Disturb / Personal Priority" modes that projects must respect.
- Record any override attempts with provenance for accountability.

### 5.4 Consent & Privacy Controls
- All cross-project or external data flows must go through explicit, revocable consent relations (aligned with `qp:hasConsentRelation`).
- Fine-grained control over what is shared (aggregated vs detailed, time-limited, purpose-bound).
- Integration with Nym or similar privacy networks where appropriate for anonymous participation.

### 5.5 Protocol Integration Points (Mobile Wellfair)
- **WebRTC**: Support secure video/voice calls for contract negotiation and claim verification. Sessions should generate provenance events in Q42.
- **GUN**: Use for real-time or eventual sync of share state, active obligations, and claim updates when online.
- **WebTorrent**: Allow downloading/uploading of project assets, ontology snapshots, or claim bundles via P2P when beneficial for resilience or large files.
- **Git / git-mark**: Support viewing and creating signed git-based contract versions or provenance trails (especially useful for audit and legal contexts).

All protocol usage must be consent-gated and produce proper provenance records.

---

## 6. Ontology Alignment Summary

Key ontology concepts that protocols must respect or produce:
- `qp:Contract` & `qp:VerifiableClaim`
- `qp:Slice` / Dynamic Equity / Stewardship Share + `qp:TokenizedShare`
- `qp:EffortObligation` and fulfillment status
- `qp:hasConsentRelation` and data flow rules
- `qp:ProjectGovernance` (including `allowsCashOut` and conditions)
- Provenance linking all of the above

Protocol events (WebRTC session started, GUN sync completed, WebTorrent swarm joined, git commit created) should be representable as provenance-tracked activities in the Q42 model.

---

## 7. Phased Implementation Roadmap

**Phase 1 – Foundations (Current)**
- Ontology for contracts, claims, shares, consent, and provenance (done)
- UI for Contracts & Claims, Economics, Project Assets (largely done)
- Basic provenance model in Qualia engine

**Phase 2 – Protocol Wiring (Next)**
- Model protocol sessions/events in ontology (WebRTC, GUN, WebTorrent, git)
- Implement provenance capture for WebRTC sessions and git commits
- Add basic GUN sync capability in Wellfair mobile (read/write share & claim state)

**Phase 3 – Full Integration & Boundary Protection**
- Consent-aware scheduling / boundary enforcement between personal calendar and project obligations
- WebTorrent distribution of assets and claim packages
- Tokenization and optional cash-out flows with governance checks
- Full Sentinel logic enforcement across protocols

**Phase 4 – Advanced & Resilience**
- Hybrid connectivity (GUN + WebRTC + WebTorrent fallback)
- Advanced git-mark style signed provenance for legal-grade audit trails
- Cross-project obligation propagation with consent

---

## 8. Specific Guidance for AI Agents Working on Wellfair Mobile

When implementing features in the Wellfair mobile app, an AI agent should:

1. **Always anchor agency to the natural person (Principal)** — AI agents, infrastructure costs, and contributions are tools of the person, not independent actors.
2. **Respect personal boundaries by default** — Never auto-schedule or pressure project work over known personal appointments, health needs, family events, or rest without explicit, recorded consent.
3. **Use the ontology** — All major entities (contracts, claims, shares, obligations, consent) should align with `qp:` classes and properties.
4. **Generate provenance** — Every significant action should produce Q42-compatible provenance records.
5. **Consent-gate all external flows** — Any data leaving the personal vault (to projects, other people, or protocols) must go through explicit consent relations.
6. **Support dual view** — The app should clearly distinguish between "Personal Life" and "Project Activities" while allowing controlled, consent-based interaction between them.
7. **Prefer offline-first** — Core functionality (viewing obligations, shares, claims, personal calendar) must work without network connectivity.
8. **Log boundary conflicts** — If a project tries to override personal time, record it with provenance for accountability and user review.

**Priority areas for initial wiring in Wellfair mobile:**
- WebRTC session handling with provenance output
- Basic GUN sync for share/claim state
- Consent UI for project data sharing and scheduling
- Personal calendar + project obligation conflict detection (with strong consent gates)
- Reading and displaying Dynamic Equity / Stewardship Shares and obligations from the shared graph

---

## 9. Open Questions & Future Work

- Exact library choices for GUN and WebRTC on mobile (React Native / Flutter / native modules).
- How deeply to integrate git signing on mobile vs server-side assisted flows.
- Standardized way to represent WebRTC sessions and GUN sync events in Q42 provenance.
- Governance rules for when cash-out of tokenized shares is permitted across different project types.

---

**End of Document**

*This architecture is intended to be living. Update as implementation reveals new requirements or better patterns.*