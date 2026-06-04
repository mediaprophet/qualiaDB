# Cooperative Projects + Qualia Ecosystem — Project State

**Date:** June 2026  
**Purpose:** Context export for new chat sessions

---

## 1. Overall Direction

The goal is to build a human-centric, relational, logic-driven system for cooperative work that properly supports both humans and software agents while keeping legal and moral responsibility with human Principals.

Key themes:
- **Agency & Personhood First**
- **Relational & Social** modeling (not isolated "self-sovereign" individuals)
- **Explicit, opt-in** inheritance and propagation
- **CBOR-LD** as the primary runtime serialization format
- **Sentinel logic** (N3Logic + SHACL) as the enforcement layer
- Strong protection of personal boundaries and consent

---

## 2. Current Ontology State (cooperative-projects.ttl)

### Core Concepts
- `qp:Project`, `qp:Subproject` relationships
- `qp:subProjectOf` / `qp:hasSubproject`
- `qp:inheritsGovernanceFrom` (explicit, opt-in)
- `qp:propagatesObligationToParent` (boolean, defaults to protecting agency)
- `qp:graduatedFrom` — allows a subproject to become independent
- `qp:dependsOn` — general many-to-many dependency (not just hierarchy)
- `qp:ContextualConsent`, `qp:RelationshipContext`, `qp:RelationshipRole`
- Dynamic Equity / Stewardship Shares (`qp:Slice`)
- Contracts, Verifiable Claims, Tokenized Shares, Cash-Out logic

### Key Logic Patterns
- Inheritance and obligation roll-up are **explicit and conditional**
- Personal data and `ContextualConsent` are **never automatically lifted** to parent projects
- Subprojects can maintain independent Dynamic Equity / Stewardship Shares
- Credential requirements can cascade when governance is inherited
- Governance inheritance is additive (subprojects can add local rules)

### Rich Examples Present
- Multi-subproject scenarios with different strategies (tight integration, partial inheritance, loose affiliation)
- Relationship contexts (doctor, project teammate, sibling) with scoped consent

---

## 3. UI Progress

### Updated Pages
- `docs/project-detail.html` — Subprojects section + Create Subproject modal (with inheritance and propagation checkboxes)
- `docs/cooperative.html` — Hierarchy badges and parent/subproject indicators on project cards
- `docs/kanban.html` — Hierarchy filter, color-coded badges, dynamic filtering by parent/subproject
- `docs/roadmap.html` — Hierarchy-aware phases with filtering and badges

### Style
- Consistent glassmorphism (Tailwind + backdrop blur)
- Unified navigation across Cooperative Projects pages

---

## 4. Agent Framework Direction (Latest Focus)

### Core Requirements
- Software agents should be able to perform defined tasks within projects
- Agents must be **anchored under a human Principal**
- Agents **cannot bear legal responsibility** (decision-making and accountability resolve to humans)
- Distinction needed between:
  - Software agents
  - Legal representatives of companies (different rights/obligations)

### Needed Capabilities
- Structured **Planning Environment** that includes:
  - Features, design, specs, functionality
  - Philosophy and values
  - Business / Product / Marketing plans
  - Team plans (human + agent roles)
- Agents should be able to operate within scoped authority (time-bound, purpose-bound, project-bound)
- All agent actions must carry clear **provenance** back to the authorizing human

### Open Questions
- Scope of agent capabilities (can they create tasks, update plans, negotiate?)
- How should software agents be identified and authorized? (did:key + scoped credentials?)
- Should the Planning Environment be a top-level concept or part of every project?

---

## 5. Icon Work

- **QualiaDB**: Q-Star + 42 concept (primary direction)
- **Webizen**: Quantum glowing orb / guardian style (calm, protective, intelligent)
- Assets prepared in `assets/icons/`

---

## 6. Key Files

- `ontology/cooperative-projects.ttl` — Main ontology + logic
- `docs/project-detail.html`
- `docs/cooperative.html`
- `docs/kanban.html`
- `docs/roadmap.html`
- `assets/icons/` — Icon assets
- `docs/PROJECT_STATE.md` — This file

---

## 7. Design Principles to Maintain

- Everything is **Principal-centered**
- Inheritance and propagation are **explicit and reversible**
- Personal boundaries and consent are protected by default
- Logic (Sentinel) enforces boundaries rather than UI-level checks
- CBOR-LD is the runtime format; Turtle/JSON-LD for ontology and documentation

---

## 8. Recommended Next Steps

1. Finalize Agent Framework ontology (`qp:SoftwareAgent`, capabilities, planning structures)
2. Design the **Planning Environment** concept
3. Continue UI refinement (especially agent-aware views)
4. Icon finalization and asset cleanup

---

*This document is intended to be updated as the project evolves.*