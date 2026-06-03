---
layout: null
---

# Protocol Integration Architecture
**Cooperative Projects + Wellfair + Qualia Engine**

**Version:** 0.2 (June 2026)  
**Status:** Draft — significantly expanded

---

## 1. Purpose of This Document

This architecture document provides a clear, comprehensive blueprint for integrating decentralized protocols (GUN, WebTorrent, WebRTC, Git + git-mark) with the Qualia engine to support Cooperative Projects, while deeply embedding everything into the **Wellfair** personal vault.

Wellfair has a dual role:
- As the individual’s personal data vault for health, life events, consent, and privacy.
- As the layer that enables **fair dealings** in project activities — ensuring people are treated justly in collaborative work without their personal lives, appointments, health needs, or boundaries being overridden.

The Qualia engine (Rust core, Sentinel logic, provenance, Q42 model) serves as the trust, logic, and provenance layer that makes these integrations reliable and consent-respecting.

This document is written to be directly usable by AI coding agents (such as Claude) when implementing features in the Wellfair mobile app and related systems.

---

## 2. Core Principles

- **Agency and Personhood First** — Every person remains the center of agency. AI agents, tools, infrastructure, and project structures exist to serve the person, not the other way around.
- **Relational and Social** — Human life and collaboration are fundamentally relational and social. Systems must reflect real relationships, mutual obligations, consent, and fairness between people rather than isolated individual "sovereignty".
- **Consent as Foundational** — No data flow, scheduling decision, obligation, or protocol session should occur without explicit, revocable, purpose-bound consent.
- **Protection of Personal Boundaries** — Project demands must not automatically override personal appointments, family responsibilities, health needs, or rest. The system must actively help protect these boundaries.
- **Provenance and Accountability** — Significant actions, especially those that affect others or cross personal/project boundaries, must carry clear, verifiable provenance.
- **Fairness in Dealings** — Economic models, obligation tracking, and contribution recognition must treat all contributors (especially unpaid ones) with dignity and fairness.
- **Offline-First and Practical Resilience** — Core functionality should continue to work when connectivity is poor or absent.

---

## 3. Key Protocols and Their Roles

| Protocol          | Main Strength                              | Role in This System                                                                 | How Qualia Adds Value                              |
|-------------------|--------------------------------------------|-------------------------------------------------------------------------------------|----------------------------------------------------|
| **GUN**           | Decentralized graph sync, offline-first   | Real-time or eventual consistency for share state, active claims, obligations, and live project data | Sentinel logic for conflict resolution and consent enforcement |
| **WebTorrent**    | Peer-to-peer file and media distribution  | Distributing ontology files, Q42 datasets, claim packages, and larger project artifacts | Provenance tracking of distributed objects        |
| **WebRTC**        | Real-time video, voice, and data channels | Contract negotiation, claim verification calls, live collaboration sessions        | Session-level provenance + consent gating         |
| **Git + git-mark**| Versioned history and signed objects      | Contract versioning, claim history, audit trails, and strong provenance of changes | Deep provenance linking commits to ontology entities |
| **Qualia Engine** | Core logic, provenance, consent engine    | The unifying layer that all other protocols ultimately report to or are governed by | Sentinel VM, Q42 model, provenance DAGs, N3Logic/SHACL rules |

---

## 4. Qualia Engine as the Unifying Layer

The Qualia engine provides the semantic and logical foundation:

- **Q42 Graph Model** — The canonical way to represent entities, relations, obligations, Dynamic Equity / Stewardship Shares, contracts, and claims.
- **Sentinel VM** — Executes defeasible logic, SHACL shapes, consent rules, and dynamic policy evaluation.
- **Provenance DAGs** — Cryptographically grounded history of actions and state changes.
- **Edge and Mobile Suitability** — Designed for low-memory, offline-capable environments.

All protocol activity should ultimately create or reference Q42 entities with proper provenance.

See:
- `ontology/cooperative-projects.ttl` (main ontology)
- `ontology/example-project-qualia-wellfair.ttl` (concrete example)

---

## 5. Wellfair Mobile Vault — Requirements and Guidance for AI Agents

Wellfair must support the person across both personal life and project activities while actively protecting boundaries between them.

### 5.1 Personal Layer
- Store and manage health data, life events, appointments, tasks, preferences, and consent records.
- Support strong privacy features (including Sanctuary Mode and duress considerations).
- Maintain awareness of personal calendar, family responsibilities, health needs, and rest periods.

### 5.2 Project / Fair Dealings Layer
- Handle contracts, verifiable claims, Dynamic Equity / Stewardship Shares, obligations, payments, and contribution tracking.
- Allow the person to participate in multiple projects fairly and transparently.
- Support optional tokenization and cash-out flows when permitted by project governance.

### 5.3 Protecting Personal Boundaries (Critical Requirement)

Wellfair must actively help the person maintain boundaries between personal life and project demands:

- Do **not** auto-schedule, pressure, or default to project work during known personal appointments, family events, health needs, or protected rest time.
- Clearly surface conflicts and require explicit consent before allowing project obligations to impact personal time.
- Provide "Personal Priority" or "Do Not Disturb for Life Events" modes that projects must respect.
- Record any attempts to override personal boundaries with provenance so the person can review them.

This is a core part of "fair dealings" — treating the whole person with dignity, not just their productive capacity.

### 5.4 Consent and Data Flow Controls
- All data leaving the personal vault toward projects or other agents must pass through explicit, revocable consent relations (see `qp:hasConsentRelation` in the ontology).
- Support fine-grained, purpose-bound, and time-limited sharing.
- Allow the person to see and revoke consents easily.

### 5.5 Protocol Integration Requirements (for Mobile Wellfair)

When implementing protocol support in the Wellfair mobile app, an AI agent should:

- **WebRTC**: Enable secure video and voice calls for contract negotiation and claim verification. Every session should generate provenance events that can be stored in Q42 format.
- **GUN**: Use for real-time or eventual synchronization of share state, active obligations, and claim updates when the device is online.
- **WebTorrent**: Support P2P download/upload of project assets, ontology snapshots, or claim bundles (especially useful for larger files or resilience).
- **Git / git-mark**: Allow viewing and creating signed, versioned contract documents or provenance trails (particularly valuable for auditability and legal contexts).

All protocol usage must be consent-gated and produce proper provenance records.

### 5.6 Concrete Rules for AI Agents Implementing Wellfair Mobile

1. Always keep the natural person (Principal) as the center of agency.
2. Treat AI agents, infrastructure, and contributions as tools of the person.
3. Default to protecting personal time and boundaries — require explicit consent for any override.
4. Align major entities with the `qp:` ontology classes and properties.
5. Generate Q42-compatible provenance for significant actions.
6. Make consent visible, revocable, and purpose-bound.
7. Clearly separate Personal Life view from Project Activities view while allowing controlled, consent-based interaction between them.
8. Log and surface boundary conflicts with provenance.
9. Prefer offline-first design for core functions (viewing obligations, shares, claims, and personal calendar).
10. Support the relational and social nature of collaboration — reflect real relationships and mutual obligations rather than purely transactional models.

**Priority areas for early implementation in Wellfair mobile:**
- WebRTC session handling with provenance output
- Basic GUN sync for share and claim state
- Consent UI flows for project data sharing and scheduling
- Personal calendar + project obligation conflict detection and consent gating
- Display of Dynamic Equity / Stewardship Shares and obligations pulled from the shared graph

---

## 6. Cross-References to Existing Work

- Main ontology: `ontology/cooperative-projects.ttl`
- Detailed example project: `ontology/example-project-qualia-wellfair.ttl`
- Contracts & Claims UI: `docs/contracts.html`
- Economics and visualizations: `docs/economics.html`
- Project Assets / Library: `docs/project-assets.html`
- Strategic Canvases: `docs/canvases.html`
- Protocol Integration Architecture (this document): `docs/protocol-integration-architecture.md`

---

## 7. Phased Implementation Roadmap

**Phase 1 – Foundations (largely complete)**
- Ontology for contracts, claims, shares, consent, obligations, and provenance
- Core UI pages (Contracts, Economics, Project Assets, etc.)
- Basic provenance concepts in Qualia engine

**Phase 2 – Protocol Wiring (Current Focus)**
- Model protocol events in the ontology (WebRTC sessions, GUN sync, WebTorrent distribution, git commits)
- Implement provenance capture for WebRTC and git activity
- Add GUN sync capability in Wellfair mobile for share/claim state
- Strengthen consent and boundary enforcement between personal and project layers

**Phase 3 – Deeper Integration**
- WebTorrent distribution of assets and claim packages
- Optional tokenization and governed cash-out flows
- Full Sentinel logic enforcement across protocol events
- Richer relational and social modeling of collaboration

**Phase 4 – Advanced Capabilities**
- Hybrid connectivity and fallback strategies
- Stronger git-mark style signed provenance for high-assurance audit trails
- Cross-project obligation propagation with consent
- Advanced personal boundary intelligence (learning and suggesting protections)

---

## 8. Open Questions

- Preferred mobile libraries for GUN and WebRTC (React Native, Flutter, or native modules).
- Standardized way to represent WebRTC sessions and GUN sync events as Q42 provenance.
- Governance rules for cash-out across different project types and risk profiles.
- How deeply to surface relational and social context (e.g., showing how one person’s shares or obligations relate to others in the project).

---

**End of Document**

This is a living architecture. Update as implementation and real-world use reveal new needs.