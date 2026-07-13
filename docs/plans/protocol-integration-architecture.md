---
layout: null
---

# Protocol Integration Architecture
**Cooperative Projects + Wellfair + Qualia Engine**

**Version:** 0.3 (June 2026)  
**Status:** Updated with CBOR-LD as runtime serialization + fine-grained relational + logic-driven consent paradigm

---

## 1. Purpose of This Document

This architecture defines how decentralized protocols (GUN, WebTorrent, WebRTC, Git + git-mark) integrate with the Qualia engine to power Cooperative Projects, while being deeply embedded in the **Wellfair** personal vault.

Wellfair serves a dual purpose:
- As the individual's human-centric personal vault (health, life events,
  consent, privacy).
- As the layer enabling **fair dealings** in project activities, while actively protecting personal boundaries, appointments, health needs, and rest from being overridden by project demands.

The Qualia engine provides the core logic, provenance, and semantic foundation. **CBOR-LD** is adopted as the primary native serialization format for runtime, mobile, and protocol exchanges.

This document serves as a practical specification for AI coding agents
implementing features in Wellfair mobile and related systems.

Important implementation note:

- the currently implemented Qualia daemon sync path is libp2p request-response
  over TCP + Noise + Yamux with CBOR-framed messages
- the broader semantic payload direction remains CBOR-LD
- GUN and WebRTC in this document describe integration targets or adjacent
  protocol profiles, not the already-frozen engine sync grammar

---

## 2. Core Principles

- **Agency and Personhood First** — The natural person remains the center of agency. Tools, AI agents, and project structures serve the person.
- **Relational and Social** — Collaboration and data sharing are fundamentally relational and social. Systems must reflect real-world relationships, mutual obligations, and contextual consent rather than isolated individual models.
- **Fine-Grained Contextual Consent** — Sharing decisions are driven by specific relationship contexts, purposes, data categories, and time bounds, enforced by logic.
- **Logic Engine as Core Decision Maker** — Qualia Webizen evaluates contextual rules dynamically rather than relying solely on static permissions.
- **CBOR-LD as Runtime Format** — Efficient binary Linked Data serialization for mobile storage, sync, and protocol communication.
- **Provenance and Accountability** — All significant actions carry verifiable provenance.
- **Protection of Personal Boundaries** — Project obligations must not automatically override personal life without explicit consent.
- **Offline-First Resilience** — Core functionality continues with limited or no connectivity.

---

## 3. Key Protocols and Their Roles

| Protocol          | Primary Strength                        | Role in System                                                                 | Serialization & Logic Notes                                      |
|-------------------|-----------------------------------------|--------------------------------------------------------------------------------|------------------------------------------------------------------|
| **GUN**           | Decentralized graph sync, offline-first | Real-time/eventual sync of share state, claims, obligations, live data        | CBOR-LD payloads + Webizen logic for conflict resolution       |
| **WebTorrent**    | P2P file/media distribution             | Distribution of ontology files, Q42 datasets, claim bundles, large artifacts  | CBOR-LD metadata + provenance tracking                          |
| **WebRTC**        | Real-time video/voice/data              | Contract negotiation, claim verification, live collaboration                  | CBOR-LD session metadata + provenance; consent-gated channels   |
| **Git + git-mark**| Versioned signed history                | Contract versioning, claim history, strong audit trails                       | Git objects + CBOR-LD provenance overlays                       |
| **Qualia Engine** | Logic, provenance, semantics            | Unifying trust and decision layer across all protocols                        | Webizen VM evaluates rules; Q42 model; CBOR-LD native at edge  |

---

## 4. Serialization Strategy

- **Turtle / JSON-LD** — Primary formats for ontology definitions, documentation, and human-readable work.
- **CBOR-LD** — Primary native serialization for:
  - Wellfair mobile storage and local sync
  - Protocol exchanges (GUN messages, WebRTC data channels, WebTorrent metadata)
  - Verifiable claims and contract bundles
  - Q42-level internal representation where compactness is beneficial

CBOR-LD provides compact, semantically rich binary encoding that aligns with Qualia’s edge-native and low-memory design goals.

---

## 5. Qualia Engine as the Unifying Layer

The Qualia engine supplies:
- **Q42 Graph Model** — Canonical representation of entities, relations, obligations, Dynamic Equity / Stewardship Shares, contracts, claims, and relationship contexts.
- **Webizen VM** — Executes fine-grained contextual consent rules, SHACL validation, and dynamic policy evaluation.
- **Provenance DAGs** — Tamper-evident history of all meaningful actions and state changes.
- **Edge-Native Design** — Suitable for mobile and constrained environments, with CBOR-LD as the efficient serialization.

All protocol activity should ultimately produce or reference Q42 entities (serialized in CBOR-LD where appropriate) with proper provenance.

See:
- `ontology/cooperative-projects.ttl` (main ontology with relationship context and contextual consent extensions)
- `ontology/example-project-qualia-wellfair.ttl` (concrete examples)

---

## 6. Wellfair Mobile Vault — Requirements for AI Agents

Wellfair must support both personal life and project activities while actively protecting boundaries between them, using fine-grained relational modeling and logic-enforced consent, serialized efficiently in CBOR-LD.

### 6.1 Personal Layer
- Health data, life events, appointments, tasks, preferences, and consent records.
- Strong privacy features (Sanctuary Mode, duress support).
- Awareness of personal calendar, family responsibilities, health needs, and rest periods.

### 6.2 Project / Fair Dealings Layer
- Contracts, verifiable claims, Dynamic Equity / Stewardship Shares, obligations, payments, and contribution tracking.
- Support for multiple concurrent projects with fair treatment of contributors.
- Optional tokenization and governed cash-out flows.

### 6.3 Fine-Grained Relational + Contextual Consent (Core Paradigm)

Wellfair must implement **fine-grained, context-aware sharing rules** driven by relationship context:

- Different data visibility depending on whether the other party is family, a project teammate, a treating physician, legal counsel, etc.
- Consent is scoped to relationship context + purpose + data category + optional time window.
- Logic engine (Webizen) evaluates rules dynamically rather than using static ACLs.
- All decisions and data flows produce provenance records (in Q42 / CBOR-LD).

**Example behaviors:**
- A treating physician may access relevant health data for medical purposes (time-limited).
- A project teammate may see obligation status and share summaries for coordination, but not detailed personal health or appointment data.
- A sibling may see general wellbeing status for family support, but not project financial details without further explicit consent.

### 6.4 Protection of Personal Boundaries

Wellfair must actively prevent project demands from overriding personal life:
- Do not auto-schedule or pressure work during known personal appointments, family events, health needs, or protected rest without explicit consent.
- Surface conflicts clearly and require explicit opt-in.
- Provide strong "Personal Priority" modes that projects must respect.
- Log boundary override attempts with provenance.

### 6.5 Protocol Integration Requirements (Mobile)

- **WebRTC**: Support secure video/voice calls for contract negotiation and claim verification. Sessions must generate CBOR-LD provenance events.
- **GUN**: Use for real-time or eventual sync of share state, obligations, and claims (CBOR-LD payloads).
- **WebTorrent**: Support P2P distribution of assets and claim bundles (CBOR-LD metadata + provenance).
- **Git / git-mark**: Support viewing and creating signed contract versions with CBOR-LD provenance overlays.

All protocol usage must be consent-gated via fine-grained contextual rules and produce proper provenance.

### 6.6 Rules for AI Agents Implementing Wellfair Mobile

1. Keep the natural person as the center of agency.
2. Model relationships using `qp:RelationshipContext` and `qp:ContextualConsent`.
3. Use the logic engine (Webizen) to evaluate sharing and scheduling decisions.
4. Serialize runtime data in CBOR-LD.
5. Generate Q42 provenance for significant actions.
6. Default to protecting personal boundaries; require explicit consent for overrides.
7. Clearly separate Personal Life and Project Activities views.
8. Log and surface boundary conflicts with provenance.
9. Prefer offline-first design.
10. Reflect the relational and social nature of human collaboration.

**Priority implementation areas:**
- WebRTC sessions with CBOR-LD provenance output
- GUN sync using CBOR-LD for share/claim state
- Contextual consent UI and logic enforcement
- Personal calendar + project obligation conflict detection with consent gating
- Display of Dynamic Equity / Stewardship Shares and obligations from the shared graph

---

## 7. Cross-References

- Main ontology (with RelationshipContext and ContextualConsent): `ontology/cooperative-projects.ttl`
- Concrete examples: `ontology/example-project-qualia-wellfair.ttl`
- Contracts & Claims UI: `docs/contracts.html`
- Economics & visualizations: `docs/economics.html`
- Project Assets: `docs/project-assets.html`
- This document: `docs/protocol-integration-architecture.md`

---

## 8. Phased Roadmap

**Phase 1** – Foundations (largely complete)
- Ontology for contracts, claims, shares, consent, relationship contexts
- Core UI pages
- Basic provenance model

**Phase 2** – CBOR-LD + Logic-Driven Consent (Current)
- Adopt CBOR-LD as primary runtime serialization
- Implement fine-grained contextual consent with Webizen logic
- Wire GUN + WebRTC with CBOR-LD payloads and provenance
- Strengthen personal boundary protection in Wellfair mobile

**Phase 3** – Full Integration
- WebTorrent distribution with CBOR-LD metadata
- Tokenization and governed cash-out flows
- Richer relational/social modeling across projects

**Phase 4** – Advanced
- Hybrid connectivity strategies
- High-assurance git-mark provenance
- Advanced personal boundary intelligence

---

## 9. Open Questions

- Specific CBOR-LD library choices for mobile (React Native / Flutter / native).
- Standardized mapping of WebRTC sessions and GUN sync events into Q42 provenance in CBOR-LD.
- Governance rules for cash-out across project types.
- Depth of relational/social context to surface in the UI.

---

**End of Document**
