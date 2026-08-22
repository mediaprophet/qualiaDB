# Workstream A — Collaborative/Cooperative ERP & PM: Extended Implementation Plan

**Date:** 2026-08-18
**Principal / copyright holder:** Timothy Charles Holborn <timothy.holborn@gmail.com>
**Source requirements:** `consult/20260818_qualia-collaborative-ui-requirements.md`
**Companion:** `IMPLEMENTATION_TRACKER.md` § Workstream A

> This document is the authoritative implementation plan for the full collaborative
> project ecosystem. It extends the original requirements doc with surfaces identified
> during review. Each item has a status: `[done]`, `[wip]`, `[todo]`, or `[blocked]`.
> Update as work progresses.

---

## 1. Current state (completed Phase 1 containers)

The following 9 project container types are implemented as mock UI views with
honest labelling, dispatched from `containers.rs`, seeded in `projects.rs`, and
registered in the command palette:

| # | Container type | File | Status |
|---|---------------|------|--------|
| 1 | `kanban` | `project_views/kanban.rs` | [done] |
| 2 | `project_sheet` | `project_views/project_sheet.rs` | [done] |
| 3 | `budget` | `project_views/budget.rs` | [done] |
| 4 | `cost_base` | `project_views/cost_base.rs` | [done] |
| 5 | `deliverable` | `project_views/deliverable.rs` | [done] |
| 6 | `review` | `project_views/review.rs` | [done] |
| 7 | `discussion` | `project_views/discussion.rs` | [done] |
| 8 | `roadmap` | `project_views/roadmap.rs` | [done] |
| 9 | `commons` | `project_views/commons.rs` | [done] |

Rights & Wallet manifold enhanced with tabbed views:

| # | Container type | File | Status |
|---|---------------|------|--------|
| 10 | `rights` (5 tabs) | `rights_views/mod.rs` + `rights_views/rights_tabs.rs` | [done] |
| 11 | `wallet` (4 tabs) | `rights_views/mod.rs` + `rights_views/wallet_tabs.rs` | [done] |

---

## 2. Extended container inventory — all proposed surfaces

### 2.1 Planning & Visualization

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.1.1 | `gantt` | Interactive Gantt chart: task bars, dependencies (FS/SS/FF/SF), critical path, phase swimlanes, drag-to-reschedule | P1 | COP-P4 dependencies, COP-P3 phases | [todo] |
| 2.1.2 | `dashboard` | Project KPI dashboard: health status, budget burn, task completion %, team velocity, risk count, upcoming milestones, recent activity feed. Widget-grid layout | P0 | Aggregate of all project data | [todo] |
| 2.1.3 | `timeline` | Zoomable interactive temporal canvas: phases, milestones, tasks, events, decisions on a single time axis. Click any node for detail. Navigational (distinct from Roadmap's structured view and Gantt's bar chart) | P1 | COP-P3/P5, COP-X4 events | [todo] |
| 2.1.4 | `calendar` | Month/week/day views: deadlines, meetings, milestones, review due dates, funding events, obligation shifts. Drag to reschedule. iCal export. **Human-centric**: can aggregate events across multiple projects and personal commitments | P1 | COP-X4, COP-P5 milestones | [todo] |

### 2.2 Knowledge & Documentation

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.2.1 | `wiki` | Structured document tree: tags, categories, agent attribution, version history (append-only, predecessor chain), provenance per page. Pages link to work items, deliverables, decisions. Search + category filter sidebar | P0 | COP-X1 extended, provenance chain | [todo] |
| 2.2.2 | `doc_mgmt` | Centralized document registry: artifact hash, kind (contract, spec, report, legal), licensing terms per document, sensitivity class, version chain, access control. Distinct from Deliverables (outputs) — this covers all project documents including inputs and references | P1 | COP-X5 ArtifactAttachment, COP-R3 | [todo] |

### 2.3 Resource Management

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.3.1 | `resource_report` | Unified resource view: human resources (contributors, roles, capacity, availability), material, capital, compute. Allocation vs availability heatmap. Includes financial cost projections and non-financial resources (time, equipment, skills) | P1 | COP-C1, LOG-10 capacity | [todo] |
| 2.3.2 | `time_tracking` | Timesheet entries per contributor per work item: start/end, duration, billable flag, rate, provenance. Aggregation by phase, contributor, task type. Replay-safe merge | P1 | COP-X1, replay-safe merge | [todo] |

### 2.4 Governance & Policy

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.4.1 | `governance` | Project-level policy configuration: decision thresholds (M-of-N), role taxonomy editor, deontic norm compilation settings, values anchor editor, sensitivity class policy, consensus rules, escalation procedures. Distinct from Rights & Agreements (legal surface) — this is project policy config | P0 | COP-R4, LOG-1/2/5, COP-A3 | [todo] |
| 2.4.2 | `voting` | Active and historical votes: proposal, eligible voters, cast votes, threshold, result, timeout. Supports M-of-N, ranked-choice, consensus protocols. Links to Decision Log | P1 | LOG-5 consensus, COP-X2 | [todo] |
| 2.4.3 | `risk` | Risk register: description, probability, impact, severity, owner, mitigation, status (open/mitigated/closed), trigger conditions. Risk heatmap matrix. Links to work items and decisions | P1 | Risk ontology extension | [todo] |

### 2.5 Task & Issue Management

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.5.1 | `task_list` | Flat/filterable task list (complementary to Kanban board): sortable columns, bulk actions, filter by assignee/phase/priority/status. Export to CSV. **Human-centric**: can aggregate tasks across multiple projects | P1 | `wellfair_work_item_board` | [todo] |
| 2.5.2 | `issues` | Bug/incident/report tracking: severity, reproducibility, affected version, linked deliverable, resolution status. Distinct from Kanban tasks — issues have repro steps and version impact | P1 | COP-P2 + issue extension | [todo] |

### 2.6 Asset & Licensing Management

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.6.1 | `asset_mgr` | Digital Asset Manager (DAM): asset registry (images, videos, 3D models, datasets, ontologies, code), licensing terms per asset (CC-BY, permissive, restricted, selfhood), provenance, usage rights, expiry, consumer class bindings. Thumbnail grid + detail panel. Drag-to-attach to deliverables | P1 | COP-X5, COP-R3, COP-M1/M3 | [todo] |
| 2.6.2 | `credentials` | Project-scoped credential manager: membership credentials (COP-A2), role credentials, professional licenses/certifications, skill credentials. Verify/revoke/status. DID-signed. Links to Members tab in Project Sheet | P0 | COP-A2, DID signing | [todo] |

### 2.7 Community & Communication

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.7.1 | `events` | Meeting scheduler + minutes: agenda, attendees, minutes (append-only), action items, decisions recorded, linked work items. Calendar integration. Recurring meeting support | P1 | COP-X1, COP-X4 | [todo] |
| 2.7.2 | `news` | Announcements feed: project updates, milestone reached, funding received, new members, obligation shifts, commons publications. Public/private toggle. RSS/magnet export for open projects | P2 | COP-X4 notifications | [todo] |
| 2.7.3 | `bounties` | Bounty board: task description, reward (sats/XEC/USDC), sponsor, claim status, claimant, completion evidence, payout status. Links to work items and wallet. Escrow support | P1 | COP-C5 funding, wallet, COP-P2 | [todo] |

### 2.8 Portfolio & Cross-Project

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.8.1 | `portfolio` | Cross-project dashboard: all projects for the principal, status summary, shared resources, cross-project dependencies, aggregate budget/obligation, resource conflicts | P2 | Cross-project aggregate | [todo] |

### 2.9 Automation & Integration

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.9.1 | `automation` | VibeScript trigger rules: event → condition → action. E.g. "when work item moves to InReview → assign reviewer + notify". Rule editor with capability gating. Execution log | P1 | VibeScript, IntentBus | [todo] |
| 2.9.2 | `integrations` | External service connections: Git repos, CI/CD pipelines, external APIs, webhooks, import/export pipelines. Status per integration. Credential vault reference | P2 | External transport | [todo] |

### 2.10 Analytics & Retrospective

| # | Container type | Purpose | Priority | Engine binding | Status |
|---|---------------|---------|----------|----------------|--------|
| 2.10.1 | `analytics` | Metrics & KPIs: burndown chart, velocity over time, cycle time distribution, budget variance trend, contribution distribution, obligation growth curve, review turnaround time. Chart grid | P1 | Derived from all project data | [todo] |
| 2.10.2 | `retrospective` | Sprint/phase retrospective: what went well, what didn't, action items. Append-only (like discussion). Links to decisions and work items for evidence | P2 | COP-X1, append-only | [todo] |

---

## 3. Project Lifecycle Framework

### 3.1 Lifecycle stages

Projects progress through stages where rules, terms, and rewards may change.
The UI must surface the current stage and its applicable ruleset, and provide
a mechanism for stage transitions (governed by the project's consensus protocol).

| # | Stage | Typical rules/terms changes | UI surface |
|---|-------|-----------------------------|------------|
| 3.1.1 | `initiation` | Project charter, initial members, values anchor, sensitivity class | Project Sheet + Governance |
| 3.1.2 | `planning` | Roadmap, budget, cost base, role assignments, deontic norms compiled | All planning containers |
| 3.1.3 | `execution` | Work items active, contributions accumulating, obligations growing, reviews ongoing | Kanban + Budget + Cost Base + Time Tracking |
| 3.1.4 | `review` | Phase reviews, deliverable acceptance, decision logs, retrospective | Reviews + Deliverables + Retrospective |
| 3.1.5 | `transition` | TSL shift evaluation (State A → State B), commons publication, obligation payoff | Cost Base (TSL tab) + Commons |
| 3.1.6 | `operation` | Product/service live, customer support, distribution, billing, feedback | Product/Service containers (§4) |
| 3.1.7 | `maintenance` | Ongoing support, updates, patches, community management | Support + Issues + News |
| 3.1.8 | `archival` | Project archived, artefacts preserved in commons, audit trail sealed | Commons + Asset Manager |

**Implementation:**
- `lifecycle_stage` field on Project Sheet (selectable, drives visible containers)
- Stage transition is a governance action (requires consensus per project policy)
- Each stage has a default container set (shown/hidden based on current stage)
- Stage history is append-only (provenance chain)

| # | Item | Priority | Status |
|---|------|----------|--------|
| 3.1.9 | Lifecycle stage field on Project Sheet | P0 | [todo] |
| 3.1.10 | Stage-driven container visibility | P1 | [todo] |
| 3.1.11 | Stage transition governance action | P1 | [todo] |
| 3.1.12 | Stage history provenance chain | P1 | [todo] |

### 3.2 Stage-dependent rules engine

In later stages, the project's rules, terms, and reward distributions may change:

| # | Mechanism | Description | Priority | Status |
|---|-----------|-------------|----------|--------|
| 3.2.1 | Reward schedule | Royalty multipliers, bounty rates, contributor shares may change per stage | P1 | [todo] |
| 3.2.2 | Obligation terms | TSL parameters may shift at stage boundaries (e.g. commercial → commons) | P1 | [todo] |
| 3.2.3 | Access policy | Visibility/permissions may change (e.g. planning = restricted, operation = public) | P1 | [todo] |
| 3.2.4 | Deontic norm set | Active norms may change per stage (e.g. execution: OBLIGATE contribution; archival: FORBID modification) | P1 | [todo] |
| 3.2.5 | Consensus threshold | Decision thresholds may change (e.g. initiation: 1-of-1; operation: 3-of-5) | P1 | [todo] |

---

## 4. Product, Service & Operations Surfaces

For projects that produce released products or services (software, physical goods,
content, services), the following surfaces are needed:

### 4.1 Product/Service Management

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 4.1.1 | `product_catalog` | Product/service registry: name, version, description, pricing, licensing, availability status, release notes. Links to deliverables and assets | P1 | [todo] |
| 4.1.2 | `release_manager` | Release tracking: version, changelog, artefact hash, release channel (stable/beta/nightly), download count, rollback capability | P1 | [todo] |
| 4.1.3 | `customer_feedback` | Feedback inbox: ratings, reviews, bug reports, feature requests, testimonials. Linked to issues and deliverables. Sentiment indicator | P1 | [todo] |
| 4.1.4 | `customer_support` | Support ticket system: ticket ID, customer DID (or anonymous), subject, status (open/in-progress/resolved/closed), priority, assignee, SLA timer, linked issue/deliverable | P1 | [todo] |

### 4.2 Distribution & Logistics

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 4.2.1 | `distribution` | Distribution channels: physical (shipping, warehouse), digital (download, magnet, streaming), hybrid. Tracking per shipment/download. Inventory levels for physical goods | P2 | [todo] |
| 4.2.2 | `logistics` | Logistics coordination: suppliers, shipping addresses, tracking numbers, delivery status, customs/duty for international. Links to budget (shipping costs) and asset manager | P2 | [todo] |

### 4.3 Sales & Marketing

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 4.3.1 | `coupons` | Coupon/discount code manager: code, discount type (% / fixed / bundle), valid period, usage limit, remaining uses, linked product, consumer class restrictions | P2 | [todo] |
| 4.3.2 | `sponsorship` | Sponsorship manager: sponsor DID, sponsorship tier, amount, duration, benefits, logo placement, exclusivity terms. Links to budget (funding flows) | P2 | [todo] |
| 4.3.3 | `eval_license` | Evaluation license issuer: license key, duration, feature restrictions, usage limits, expiry, conversion to full license. Primarily for software projects | P2 | [todo] |

### 4.4 Billing & Payments

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 4.4.1 | `billing` | Billing system: invoices, recurring subscriptions, payment status, payment method (ILP/Lightning/XEC/fiat), tax calculation, dunning (failed payment retry). Links to wallet and budget | P1 | [todo] |
| 4.4.2 | `subscription_mgr` | Subscription manager: subscriber DID, plan, billing cycle, renewal date, cancellation, upgrade/downgrade. Usage tracking per subscriber | P2 | [todo] |

### 4.5 Project Infrastructure

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 4.5.1 | `infrastructure` | Project infrastructure registry: websites (URLs, hosting, DNS), online accounts (services, credentials reference), servers, domains, SSL certificates, API keys. Status per item. Access control metadata | P1 | [todo] |

---

## 5. Group / Organisation Layer

Groups can have multiple projects, their own community, suppliers, partners,
and collaborators. This requires a layer above individual projects.

### 5.1 Group manifold

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 5.1.1 | `group_profile` | Group identity: name, DID, description, members, governance model, values anchor, licensing profile. Analogous to Project Sheet but for the group | P1 | [todo] |
| 5.1.2 | `group_portfolio` | All projects under the group: status summary, shared resources, cross-project dependencies, aggregate financials, resource allocation | P1 | [todo] |
| 5.1.3 | `group_community` | Community surface: members, discussion, events, news, shared wiki. Cross-project community space | P1 | [todo] |
| 5.1.4 | `group_suppliers` | Supplier/partner registry: organization, contact, contracts, SLAs, pricing, linked budget items. Distinct from project-level cost base | P2 | [todo] |
| 5.1.5 | `group_governance` | Group-level governance: charter, decision thresholds, role taxonomy, deontic norms applicable across all group projects | P1 | [todo] |

### 5.2 Group–project relationships

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 5.2.1 | Group → project ownership | Projects belong to a group; group governance applies unless overridden | P1 | [todo] |
| 5.2.2 | Shared resources | Group-level resources (contributors, equipment, budget pool) assignable to projects | P1 | [todo] |
| 5.2.3 | Cross-project dependencies | Work items / deliverables can depend on items in other group projects | P2 | [todo] |
| 5.2.4 | Group-level analytics | Aggregate metrics across all group projects | P2 | [todo] |

---

## 6. Currency & Multi-Sig Wallet Support

### 6.1 Multi-currency support

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 6.1.1 | Currency registry | Supported currencies: XEC, sats (Lightning), USDC, fiat (AUD, USD, EUR, etc.), Q42 (internal), custom tokens. Each has metadata (symbol, precision, network) | P0 | [todo] |
| 6.1.2 | Per-currency balances | Wallet shows balances per currency, with conversion display (optional rate feed) | P0 | [todo] |
| 6.1.3 | Per-currency ledger entries | Budget and ledger entries specify currency; aggregation respects currency boundaries | P0 | [todo] |
| 6.1.4 | Currency conversion | Optional conversion display using rate oracle or manual rate entry | P2 | [todo] |

### 6.2 Crypto account assignment

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 6.2.1 | Account registry | Multiple crypto accounts (ILP pointers, Lightning channels, XEC addresses, on-chain addresses) assignable to different purposes | P0 | [todo] |
| 6.2.2 | Purpose assignment | Each account tagged with purpose: project_funding, royalty_distribution, tax_escrow, bounty_escrow, operational, savings, custom | P0 | [todo] |
| 6.2.3 | Project-scoped accounts | Accounts can be scoped to a specific project or shared across projects (group-level) | P1 | [todo] |
| 6.2.4 | Account health | Per-account status: balance, last activity, channel capacity (Lightning), connectivity | P1 | [todo] |

### 6.3 Multi-sig support

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 6.3.1 | Multi-sig wallet configuration | Define M-of-N multi-sig: required signers (DIDs), threshold (M), total signers (N), timeout, fallback | P0 | [todo] |
| 6.3.2 | Transaction proposal | Propose transaction: amount, currency, destination, purpose, attached evidence. Visible to all required signers | P0 | [todo] |
| 6.3.3 | Signature collection | Signers approve/reject; threshold reached → execute; timeout → auto-reject | P0 | [todo] |
| 6.3.4 | Multi-sig history | Append-only log of proposals, signatures, executions, rejections, timeouts | P1 | [todo] |
| 6.3.5 | Purpose-specific multi-sig | Different multi-sig configs for different purposes (e.g. royalty distribution: 2-of-3; infrastructure costs: 1-of-2) | P1 | [todo] |

### 6.4 Wallet container enhancement

The existing Wallet container (4 tabs) needs enhancement:

| # | Tab | Addition | Priority | Status |
|---|-----|----------|----------|--------|
| 6.4.1 | Balances | Multi-currency, per-account breakdown, conversion display | P0 | [todo] |
| 6.4.2 | Accounts | New tab: account registry with purpose assignment, health status, project scoping | P0 | [todo] |
| 6.4.3 | Multi-sig | New tab: multi-sig config, pending proposals, signature queue, history | P0 | [todo] |
| 6.4.4 | ILP / Lightning / XEC | Payment execution with account selection, multi-sig awareness | P1 | [todo] |
| 6.4.5 | Tax Suite | Multi-currency tax routing, per-jurisdiction | P1 | [todo] |
| 6.4.6 | Compute Costs | Per-project compute cost allocation | P1 | [todo] |

---

## 7. Human-Centric Systems Considerations

The QualiaDB ecosystem is human-centric: some surfaces serve the person across
all their commitments, not just a single project. This requires distinguishing
project-scoped views from personal cross-project views.

### 7.1 Personal aggregate views

| # | Surface | Description | Priority | Status |
|---|---------|-------------|----------|--------|
| 7.1.1 | Personal Calendar | Aggregates events, deadlines, meetings from all projects + personal commitments. Color-coded by source. Filter by project/category/agent | P1 | [todo] |
| 7.1.2 | Personal Task List | Aggregates tasks assigned to the user across all projects + personal tasks. Unified priority sorting. Filter by project/phase/deadline | P1 | [todo] |
| 7.1.3 | Personal Dashboard | Cross-project KPIs for the individual: active tasks, upcoming deadlines, pending reviews, unread discussions, wallet balance, obligation status | P1 | [todo] |
| 7.1.4 | Notification Center | Unified notification inbox from all projects + system + community. Priority, category, source project. Mark read/unread. Archive | P1 | [todo] |

### 7.2 Agent-aware views

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 7.2.1 | Agent-filtered views | Wiki, task list, discussion, etc. can be filtered by agent attribution (who created/modified/owns) | P1 | [todo] |
| 7.2.2 | Agent contribution summary | Per-agent contribution metrics across project surfaces (tasks completed, hours logged, decisions made, documents authored) | P2 | [todo] |
| 7.2.3 | Agent capacity view | Per-agent available capacity (time, skills) across all project commitments — avoids over-allocation | P1 | [todo] |

### 7.3 Versioning & provenance

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 7.3.1 | Version history on all appendable records | Wiki pages, decisions, discussion threads, deliverables, documents — all show version chain with predecessor_id, author DID, timestamp | P0 | [todo] |
| 7.3.2 | Provenance display | Every record shows: author DID, asserted time, valid time, blob_hash, epistemic_status, evidence_type, sensitivity class | P0 | [todo] |
| 7.3.3 | Replay-safe indicators | Where derivations are replay-safe (Contribution, Ledger, WorkItemStatus, CostBaseEntry), show badge | P1 | [todo] |
| 7.3.4 | Sensitivity class badges | Every record shows sensitivity class (Public / Restricted / Classified / Selfhood). UI prevents Selfhood records from entering commons publication | P0 | [todo] |

---

## 8. Permissive Commons Integration

### 8.1 Commons as project asset/system support

Projects may produce or consume Permissive Commons assets. The commons
relationship is bidirectional:

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8.1.1 | Commons consumption | Project depends on commons assets (ontologies, datasets, models, tools). Registry of consumed commons assets with version, license terms, obligation status | P1 | [todo] |
| 8.1.2 | Commons production | Project publishes assets to commons (existing `commons` container). Enhanced with lifecycle stage awareness (only publish after review stage) | P1 | [todo] |
| 8.1.3 | Commons obligation tracking | Per-asset obligation status: total cost, paydown progress, consumer class bindings, TSL state (A/B), shift indicator | P1 | [todo] |
| 8.1.4 | Commons seed status | Post-shift (State B) assets: share-alike seed obligation. Show seeding status per asset (seeding/not seeding, ratio, uptime) | P2 | [todo] |
| 8.1.5 | Commons artefact versioning | Published artefacts have version chains. New versions supersede old (with tombstone). Consumer notification on new version | P2 | [todo] |

---

## 8a. Agreement Framework & Instruments

### 8a.1 Instrument-based agreement definition

Agreements are defined using high-level legal and ethical instruments rather than
ad-hoc clauses. The system provides a library of instruments that can be composed
into project-specific agreements.

| # | Instrument class | Examples | Description | Priority | Status |
|---|-----------------|----------|-------------|----------|--------|
| 8a.1.1 | Human rights instruments | UDHR, ICCPR, ICESCR, UNDRIP | Foundational rights anchors that all agreements inherit. Non-negotiable baseline | P0 | [todo] |
| 8a.1.2 | Creative Commons instruments | CC-BY, CC-BY-SA, CC-BY-NC, CC0 | Licensing terms for creative/intellectual outputs | P0 | [todo] |
| 8a.1.3 | Permissive Commons instruments | Permissive Commons Protocol (COP) | The QualiaDB-specific instrument family: COP-R4 (deontic), COP-M1/M3 (selfhood/membership), COP-A2/A3 (agency/authority), COP-C1/C5 (contribution/funding), COP-P1-P5 (project taxonomy), COP-X1-X5 (extended artefacts) | P0 | [todo] |
| 8a.1.4 | Fiduciary instruments | Fiduciary duty, stewardship, escrow terms | Legal obligations of custodians, guardians, and stewards | P1 | [todo] |
| 8a.1.5 | Data governance instruments | Data sharing, consent management, data sovereignty | Terms governing data flows between parties | P1 | [todo] |
| 8a.1.6 | Labour instruments | Fair labour standards, contributor covenants, codes of conduct | Terms governing contributor participation | P1 | [todo] |
| 8a.1.7 | Peace & humanitarian instruments | Geneva Conventions principles, humanitarian access, do-no-harm | For peace infrastructure and humanitarian ICT projects | P1 | [todo] |
| 8a.1.8 | Custom instruments | User-defined instrument templates | Projects may define their own instruments with deontic norm compilation | P2 | [todo] |

### 8a.2 Agreement composition

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8a.2.1 | Instrument selection | When creating an agreement, select one or more instruments from the library. Each instrument contributes its normative terms | P0 | [todo] |
| 8a.2.2 | Clause overlay | Projects may add custom clauses that override or extend instrument defaults. Overrides must be explicitly justified and are logged in the provenance chain | P0 | [todo] |
| 8a.2.3 | Conflict detection | When multiple instruments are composed, detect normative conflicts (e.g. CC-BY-NC vs commercial use) and surface them for resolution | P1 | [todo] |
| 8a.2.4 | Instrument versioning | Instruments have versions; agreements reference specific versions. Updates to instruments do not retroactively change existing agreements | P1 | [todo] |
| 8a.2.5 | Agreement template registry | Pre-configured agreement templates for common project types (e.g. "Humanitarian ICT Commons Agreement", "Software Contributor Accord") | P1 | [todo] |

### 8a.3 Agreement lifecycle

| # | Stage | Description | Priority | Status |
|---|-------|-------------|----------|--------|
| 8a.3.1 | Draft | Agreement authored, not yet signed | P0 | [todo] |
| 8a.3.2 | Review | Eligible signers review terms; comments append-only | P0 | [todo] |
| 8a.3.3 | Sign | Signers sign with DID; threshold (M-of-N) required for activation | P0 | [todo] |
| 8a.3.4 | Active | Agreement in force; deontic norms compiled and enforced | P0 | [todo] |
| 8a.3.5 | Amend | Append-only amendments with re-signature from required parties | P1 | [todo] |
| 8a.3.6 | Expire | Agreement reaches end date or condition; obligations wind down | P1 | [todo] |
| 8a.3.7 | Terminate | Early termination with breach log entry and obligation settlement | P1 | [todo] |

### 8a.4 Agreement container

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8a.4.1 | `agreement_builder` | Instrument selection, clause overlay, conflict detection, signatory management, threshold setting. Wizard-style interface for composing agreements from instruments | P0 | [todo] |

---

## 8b. Fair Value, Compensation & Obligation Cost

### 8b.1 Fair value evaluation framework

The system provides a structured framework for evaluating the fair value of
contributor labour, accounting for local economic context, skill levels, and
project-specific circumstances.

| # | Factor | Description | Priority | Status |
|---|--------|-------------|----------|--------|
| 8b.1.1 | Base rate | Minimum compensation rate per unit of contribution (hour/task/deliverable). Set per project or per role. May be zero (voluntary) or non-zero | P0 | [todo] |
| 8b.1.2 | PPP (Purchasing Power Parity) | Adjust base rate by contributor's local PPP index. A contributor in a lower-PPP region receives a higher nominal rate to achieve parity in real terms. PPP data sourced from registered data sources (§8l.4.1: OECD, World Bank, IMF — mocked). Source is selectable per project; comparison between sources available | P0 | [todo] |
| 8b.1.3 | Skill level multiplier | Skill tiers (entry, intermediate, advanced, expert, specialist) each with a multiplier on the base rate (e.g. 1.0x, 1.5x, 2.0x, 3.0x, 5.0x) | P0 | [todo] |
| 8b.1.4 | Expertise premium | For specialised expertise (rare skills, domain knowledge), an additional premium multiplier can be applied. Justified and logged | P1 | [todo] |
| 8b.1.5 | Resource contribution | Non-labour contributions (equipment, compute, facilities, data) valued at fair market rate or project-agreed rate | P1 | [todo] |
| 8b.1.6 | Circumstantial premium | When a contributor deserves more than base rate due to circumstances (urgency, risk, hardship, social impact), a circumstantial premium is applied. Must be justified and approved per governance | P1 | [todo] |
| 8b.1.7 | Market rate comparison | Optional: compare computed fair value against external market rates for similar work. Informational only | P2 | [todo] |

### 8b.2 Compensation model

| # | Mechanism | Description | Priority | Status |
|---|-----------|-------------|----------|--------|
| 8b.2.1 | Compensation status | Each contributor has a status: `fully_compensated`, `partially_compensated`, `uncompensated`. Drives obligation cost calculation | P0 | [todo] |
| 8b.2.2 | Compensation multiplier | Multiplier applied to fair value for contributors who are partially or fully uncompensated. Range: 1.0x (fully compensated) to 10x (fully uncompensated). Default schedule: uncompensated = 3x, partially = 1.5x. Project-configurable | P0 | [todo] |
| 8b.2.3 | Stage-specific multipliers | Multiplier may vary by project lifecycle stage. E.g. planning stage: 1.2x; execution: 2x; review: 1.5x; operation: 1x. Configurable per project | P0 | [todo] |
| 8b.2.4 | Contributor-specific multipliers | Instead of (or in addition to) stage-specific, specific contributors may have individual multipliers based on negotiation or governance decision | P1 | [todo] |
| 8b.2.5 | Compensation window (time-based) | Earlier contributions may cost less than later ones. A time-decay or time-escalation function can be applied. E.g. contributions during first 30 days: 0.8x; days 31-90: 1.0x; days 90+: 1.2x. Encourages early participation | P1 | [todo] |
| 8b.2.6 | Retroactive adjustment | When compensation status changes (e.g. project receives funding), past contributions may be retroactively adjusted. Append-only adjustment records with provenance | P1 | [todo] |
| 8b.2.7 | Royalty share | Long-term royalty distribution based on contribution weight. Contribution weight = fair_value × multiplier × time_factor. Expressed as percentage of royalty pool | P0 | [todo] |
| 8b.2.8 | Obligation cost | The total uncompensated value (fair_value × multiplier - actual_compensation) becomes the obligation cost. This is what legal persons must pay to access derivatives (see §8c) | P0 | [todo] |

### 8b.3 Compensation container

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8b.3.1 | `compensation_model` | Fair value calculator: base rate × PPP × skill × expertise × circumstantial = fair_value. Then: fair_value × compensation_multiplier × stage_multiplier × time_factor = obligation_cost. Per-contributor breakdown. Per-stage summary. Total obligation cost for project | P0 | [todo] |
| 8b.3.2 | `contribution_ledger` | Append-only ledger of all contributions: contributor DID, contribution type (time/expertise/skill/resource), quantity, fair_value, multiplier applied, obligation cost, actual_compensation, balance_owing. Replay-safe merge | P0 | [todo] |

---

## 8c. Differential Licensing & Obligation Recovery

### 8c.1 Agent-type-based pricing

For peace infrastructure and humanitarian ICT projects, derivatives are sought to
be freely available to natural persons (human beings) while legal persons
(incorporated entities, commercial users) are charged a rate that recovers the
obligation cost of unpaid contributor labour.

| # | Mechanism | Description | Priority | Status |
|---|-----------|-------------|----------|--------|
| 8c.1.1 | Agent-type classification | License terms specify different rates for: `natural_person` (human), `legal_person` (corporation, institution), `commercial_use` (any agent using for commercial gain), `non_commercial` (any agent using non-commercially), `humanitarian_use` (emergency response, aid), `research_use` (academic, non-commercial research) | P0 | [todo] |
| 8c.1.2 | Free for natural persons | Derivatives freely available to individual human beings for personal use, education, and non-commercial purposes. Enforced via license terms + consumer class binding | P0 | [todo] |
| 8c.1.3 | Obligation recovery rate for legal persons | Legal persons pay a rate calculated to recover obligation cost. Rate = obligation_cost / projected_legal_person_usage × margin. Updated as obligation cost grows and usage data accumulates | P0 | [todo] |
| 8c.1.4 | Tiered pricing for legal persons | Different tiers for legal persons: small enterprise, medium enterprise, large enterprise, government, humanitarian organization. Each tier has a different rate relative to obligation recovery | P1 | [todo] |
| 8c.1.5 | Sliding scale based on ability to pay | For organizations in lower-PPP regions, a reduced rate. For organizations in higher-PPP regions, full rate. Mirrors the PPP adjustment for contributors | P1 | [todo] |
| 8c.1.6 | Waiver mechanism | Humanitarian organizations and peace infrastructure projects may apply for waiver or reduced rate. Approved via governance. Logged with provenance | P1 | [todo] |
| 8c.1.7 | Obligation recovery tracking | Per-asset: total obligation cost, amount recovered, amount outstanding, recovery percentage. Visualised as progress bar | P0 | [todo] |
| 8c.1.8 | Obligation satisfaction event | When obligation cost is fully recovered for an asset, a `TSL_State_A_to_B` shift may be triggered (asset moves from obligation-bearing to share-alike seed) | P0 | [todo] |

### 8c.2 License definition & scope

Licenses may be applied at multiple levels of granularity — not only to the
entire project, but to individual constituents: deliverables, assets,
contributions, datasets, wiki pages, source code modules, publications, or any
publishable artefact. This enables mixed-licensing models within a single
project (e.g. source code under COP, datasets under CC-BY, hardware designs
under a commercial license with obligation recovery).

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8c.2.1 | License composition | A license is composed from: instrument(s) + agent-type pricing + obligation recovery terms + TSL parameters + waiver policy | P0 | [todo] |
| 8c.2.2 | License preview | Before publishing, preview the license terms in human-readable form with all agent-type rates, obligation recovery schedule, and TSL parameters | P1 | [todo] |
| 8c.2.3 | License attachment to assets | Each published asset (in asset_mgr or commons) has a license attached. License is immutable once published (new versions require new publication) | P0 | [todo] |
| 8c.2.4 | License verification | Consumers can verify license terms before use. System checks consumer's agent type and displays applicable rate | P1 | [todo] |
| 8c.2.5 | Per-constituent licensing | A license may be applied to any individual project constituent: deliverable, asset, contribution, dataset, wiki page, source code module, publication, IP item, or any publishable artefact. Each constituent may have a different license | P0 | [todo] |
| 8c.2.6 | Project default license | A project may define a default license that is automatically applied to new constituents unless overridden. Override requires explicit selection of an alternative license | P0 | [todo] |
| 8c.2.7 | Constituent license override | Any constituent may override the project default with a specific license. Override is logged with provenance (who, when, why). Multiple constituents may share a license or each have a unique one | P0 | [todo] |
| 8c.2.8 | License inheritance | Constituents may inherit license from parent: a deliverable inherits from its parent milestone, a source file inherits from its module, a dataset inherits from its collection. Inheritance may be overridden at any level | P1 | [todo] |
| 8c.2.9 | Mixed-licensing view | Project-level view showing all licenses in use across constituents: which constituents use which license, obligation recovery status per license, TSL state per license. Highlights where constituents have divergent licensing | P1 | [todo] |
| 8c.2.10 | License conflict detection | When constituents with different licenses are combined (e.g. a deliverable that includes a CC-BY dataset and a COP-licensed source module), detect and flag license conflicts (incompatible copyleft, conflicting obligation terms, incompatible TSL states) | P1 | [todo] |
| 8c.2.11 | License scope selector | When creating or editing any constituent, a license scope selector is available: project default, inherited, or specific. If specific, the license builder wizard opens to compose or select a license | P0 | [todo] |
| 8c.2.12 | Bulk license assignment | Assign a license to multiple constituents at once (e.g. all datasets, all source files in a module). Bulk assignment is logged with provenance and is reversible (constituents revert to project default or inherited) | P1 | [todo] |
| 8c.2.13 | License provenance chain | Each license assignment is append-only: constituent, license, assigned by, date, reason (default/override/inherited). Full history visible per constituent | P0 | [todo] |
| 8c.2.14 | Per-contributor licensing | A contributor may specify license terms for their own contributions (e.g. "my contributions are CC0" or "my contributions require attribution"). Project governance may accept, reject, or negotiate. Links to contribution ledger and agreement builder | P1 | [todo] |

### 8c.3 License & obligation containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8c.3.1 | `license_builder` | License composition wizard: select instruments, define agent-type pricing, set obligation recovery terms, configure TSL parameters, set waiver policy. Preview before publish. **Per-constituent scope selector**: apply to project default, specific constituent(s), or bulk assign. Mixed-licensing view showing all licenses across constituents. License conflict detection. License provenance chain per constituent | P0 | [todo] |
| 8c.3.2 | `obligation_tracker` | Per-asset obligation recovery dashboard: total obligation, recovered, outstanding, recovery rate, projected satisfaction date, TSL state indicator. Links to contribution_ledger and billing | P0 | [todo] |

---

## 8d. Awards, Tokens & Recognition

### 8d.1 Awards system

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8d.1.1 | Award definition | Project/group defines awards: name, criteria, icon, description, issuer, eligibility (contributors, community members, external) | P1 | [todo] |
| 8d.1.2 | Award issuance | Issue award to a DID with evidence (linked work items, contributions, peer nominations). DID-signed by issuer. Stored in credential manager | P1 | [todo] |
| 8d.1.3 | Award display | Awards shown on contributor profile, project sheet members tab, and group community. Badge-style display with provenance | P1 | [todo] |
| 8d.1.4 | Award categories | Suggested categories: contribution excellence, community building, innovation, humanitarian impact, peace infrastructure, mentorship, lifetime achievement | P2 | [todo] |
| 8d.1.5 | Peer nomination | Community members can nominate each other for awards. Nominations are append-only with evidence links | P2 | [todo] |

### 8d.2 Token system

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8d.2.1 | Token definition | Define project/group tokens: name, symbol, supply (fixed or mintable), decimals, purpose (governance, reward, utility, payment), initial distribution | P0 | [todo] |
| 8d.2.2 | Token issuance | Mint tokens to contributor DIDs as rewards for contributions, bounties, awards, or governance participation. Each issuance has provenance (reason, linked contribution, authorizer) | P0 | [todo] |
| 8d.2.3 | Token distribution rules | Rules for how tokens are distributed: per-contribution (proportional to fair_value), per-milestone, per-vote, per-award. Configurable per project | P1 | [todo] |
| 8d.2.4 | Token transfer | Token holders can transfer to other DIDs. Transfer log is append-only with provenance | P1 | [todo] |
| 8d.2.5 | Token vesting | Tokens may vest over time or upon conditions (milestone reached, stage transition, TSL shift). Vesting schedule per token class | P1 | [todo] |
| 8d.2.6 | Token governance | If tokens carry governance rights, display voting weight per holder. Link to voting container | P1 | [todo] |
| 8d.2.7 | Token-benefit conversion | Tokens may be convertible to: royalty shares, obligation recovery credits, commons access rights, compute credits, or fiat/crypto via external exchange | P2 | [todo] |
| 8d.2.8 | Token registry | Cross-project token registry: all tokens held by a DID across all projects/groups. Balances, vesting status, governance weight | P1 | [todo] |

### 8d.3 Awards & token containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8d.3.1 | `awards` | Award registry: defined awards, issued awards, nomination queue. Award definition editor. Issue/nominate actions | P1 | [todo] |
| 8d.3.2 | `token_mgr` | Token manager: define tokens, mint/transfer/vest, distribution rules, holder registry, governance weight. Links to wallet and contribution_ledger | P0 | [todo] |

---

## 8e. Disputes, Complaints & Corrections

### 8e.1 Dispute resolution

Disputes arise when agents disagree about facts, obligations, contributions,
rights, or decisions. The system provides a structured dispute lifecycle that
integrates with the governance, rights, and agreement surfaces.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8e.1.1 | Dispute filing | Any agent (natural, legal, software via proxy) may file a dispute: subject, description, disputed record(s), evidence links, desired outcome, urgency (low/normal/high/critical) | P0 | [todo] |
| 8e.1.2 | Dispute parties | All parties to the dispute are identified by DID. Each party can submit statements (append-only). Third-party mediators/arbitrators can be added by governance | P0 | [todo] |
| 8e.1.3 | Dispute categories | Categories: factual (disputed claim), contribution (disputed fair value or attribution), rights (disputed agreement terms), decision (disputed governance outcome), conduct (disputed agent behaviour), resource (disputed allocation) | P0 | [todo] |
| 8e.1.4 | Dispute lifecycle | open → under_review → mediation → resolution → closed. Each transition is append-only with provenance. Reopening is possible with new evidence | P0 | [todo] |
| 8e.1.5 | Resolution types | resolved_by_agreement (parties agree), resolved_by_mediation (mediator proposes), resolved_by_arbitration (binding), resolved_by_governance (consensus vote), withdrawn (filer retracts), stale (timeout) | P0 | [todo] |
| 8e.1.6 | Resolution enforcement | Resolution may trigger: correction (§8e.3), compensation adjustment, obligation update, agreement amendment, credential revocation, or breach log entry | P1 | [todo] |
| 8e.1.7 | Escalation | If mediation fails, dispute escalates to arbitration or governance vote per project policy. Escalation path configurable | P1 | [todo] |
| 8e.1.8 | Dispute history | All disputes are permanently retained (append-only). Past disputes inform future decisions. Dispute history visible on agent profiles and project records | P1 | [todo] |

### 8e.2 Complaints

Complaints are a subset of disputes focused on agent conduct, policy violations,
or harm — without necessarily disputing a specific record.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8e.2.1 | Complaint filing | File complaint: subject, description, accused agent(s), evidence, category (conduct, policy_violation, harm, harassment, discrimination, breach_of_trust), urgency | P0 | [todo] |
| 8e.2.2 | Anonymous complaints | Support anonymous filing where safety is a concern. Anonymity is cryptographically enforced. Investigator access controlled by governance | P1 | [todo] |
| 8e.2.3 | Complaint investigation | Assigned investigator(s) review evidence, interview parties, and produce findings. Findings are append-only with provenance | P1 | [todo] |
| 8e.2.4 | Complaint outcomes | Outcomes: substantiated (with sanctions), unsubstantiated, inconclusive, referred_to_external (legal/regulatory). Sanctions may include: warning, suspension, removal, credential revocation, obligation penalty | P1 | [todo] |
| 8e.2.5 | Sanction tracking | Active sanctions per agent: type, duration, scope, enforcing authority, appeal status. Links to credentials and governance | P1 | [todo] |
| 8e.2.6 | Appeal process | Sanctioned agent may appeal. Appeal reviewed by separate body (governance-configured). Appeal outcomes: upheld, overturned, modified | P2 | [todo] |
| 8e.2.7 | Whistleblower protection | Complaints filed in good faith are protected from retaliation. Retaliation is itself a complaint category. Protected status enforced via governance policy | P1 | [todo] |

### 8e.3 Corrections

Corrections are amendments to previously asserted records. The append-only
provenance model means corrections do not delete the original — they supersede
it with a new record that references the predecessor.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8e.3.1 | Correction by original maker | The agent who made the original claim may issue a correction: corrected record, reason, evidence. Original record is marked superseded (not deleted). Correction chain is visible | P0 | [todo] |
| 8e.3.2 | Correction by other agent | An agent other than the original maker may propose a correction. Proposed correction enters review: original maker is notified, can accept/reject/counter. If rejected, becomes a dispute (§8e.1) | P0 | [todo] |
| 8e.3.3 | Correction by dispute resolution | A dispute resolution may mandate a correction. The correction is applied with the dispute resolution as authority. Original maker cannot reject | P0 | [todo] |
| 8e.3.4 | Correction by clarification | An agent may issue a clarification (not a correction) that adds context to an existing record without superseding it. Clarifications are linked but do not change validity | P1 | [todo] |
| 8e.3.5 | Correction scope | Corrections may apply to: claims, contributions, decisions, agreement terms, credentials, obligation entries, license terms, asset metadata, wiki pages, any appendable record | P0 | [todo] |
| 8e.3.6 | Correction chain display | Any record that has been corrected shows a "Correction history" expander: chain of corrections with author, reason, evidence, timestamp. Original is always visible | P0 | [todo] |
| 8e.3.7 | Cascade corrections | When a record is corrected, dependent records (e.g. contribution value → royalty share → obligation cost) may need recalculation. System flags dependent records for review | P1 | [todo] |
| 8e.3.8 | Correction notification | All agents who referenced or depended on a corrected record are notified. Notification includes: what changed, why, impact on their records | P1 | [todo] |
| 8e.3.9 | Retraction vs correction | Retraction (full withdrawal) is a special correction type where the maker asserts the record should not have been made. Retracted records remain visible with a retraction marker. Dependencies flagged for review | P1 | [todo] |

### 8e.4 Disputes, complaints & corrections containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8e.4.1 | `disputes` | Dispute registry: open disputes, dispute history, filing form, party statements, mediator panel, resolution display. Links to disputed records, agreements, governance | P0 | [todo] |
| 8e.4.2 | `complaints` | Complaint registry: filed complaints, investigation status, findings, sanctions, appeals. Anonymous filing support. Links to accused agent profiles, credentials | P0 | [todo] |
| 8e.4.3 | `corrections` | Correction log: all corrections across the project, filterable by record type, author, status. Correction chain viewer for any record. Cascade impact flags | P0 | [todo] |

---

## 8f. Onboarding & Bulk Administration

Many users (especially early adopters and project founders) need to fill in
significant amounts of historical data: past contributors, costs, agreements,
people who helped — some of whom may be pseudo-anonymous to the project or
public. This process needs to be efficient and structured.

### 8f.1 Bulk import & data entry

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8f.1.1 | CSV/JSON import | Import contributors, costs, contributions, agreements, assets from CSV or JSON files. Column mapping wizard. Validation with error report before commit | P0 | [todo] |
| 8f.1.2 | Bulk contributor entry | Add multiple contributors at once: DID (or pseudo-anonymous handle), role, skill level, compensation status, join date. No DID required for pseudo-anonymous entries (assigned a project-scoped placeholder DID) | P0 | [todo] |
| 8f.1.3 | Bulk contribution entry | Add multiple historical contributions at once: contributor, date, type, quantity, fair value (auto-calculated or manual), compensation status. Links to work items (optional) | P0 | [todo] |
| 8f.1.4 | Bulk agreement entry | Add multiple agreements at once: title, parties, instrument, status, date signed. For historical agreements that predate the system | P1 | [todo] |
| 8f.1.5 | Bulk asset entry | Add multiple assets at once: name, type, license, provenance, value. For existing project assets not yet in the system | P1 | [todo] |
| 8f.1.6 | Retroactive date entry | All bulk entries support retroactive dates (asserted time vs valid time). System marks these as `retroactive` with provenance noting the entry was made after the fact | P0 | [todo] |
| 8f.1.7 | Draft mode | Bulk entries go into a draft state before commit. Review, edit, then commit all at once. Draft is savable/resumable | P0 | [todo] |
| 8f.1.8 | Import preview & validation | Before commit, preview all entries with validation: missing fields, duplicate DIDs, invalid dates, unknown instruments. Error report with row-level issues | P0 | [todo] |

### 8f.2 Pseudo-anonymous contributors

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8f.2.1 | Pseudo-anonymous handle | Contributors can be entered with a handle/alias instead of a DID. System assigns a project-scoped placeholder DID (e.g. `did:qualia:project_42:anon_07`) | P0 | [todo] |
| 8f.2.2 | Identity claim | A pseudo-anonymous contributor (or their delegate) may later claim their identity by proving control of the placeholder DID. Claim links the handle to a real DID | P1 | [todo] |
| 8f.2.3 | Visibility control | Each contributor has a visibility setting: `public` (DID visible to all), `project` (DID visible to project members only), `pseudo_anonymous` (handle only, DID hidden) | P0 | [todo] |
| 8f.2.4 | Delegate entry | A project founder may enter contributors on their behalf. The entry is marked as `entered_by: <founder DID>` with provenance. The contributor can later claim and verify | P0 | [todo] |
| 8f.2.5 | Anonymous contribution attribution | Contributions can be attributed to a pseudo-anonymous handle without revealing the real DID. The real DID is cryptographically linked but not publicly visible | P1 | [todo] |

### 8f.3 Onboarding wizard

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8f.3.1 | Step-by-step project setup | Wizard for new projects: basic info → project type → governance settings → invite members → import existing data → define agreements → set compensation model → review & activate | P0 | [todo] |
| 8f.3.2 | Template-based setup | Pre-configured templates per project type (welfare-support, software, humanitarian ICT, etc.) that pre-fill container sets, governance defaults, compensation defaults | P1 | [todo] |
| 8f.3.3 | Progress tracking | Wizard tracks completion: which steps are done, which are skipped, which are deferred. Resumable at any point | P1 | [todo] |
| 8f.3.4 | Founder onboarding mode | Special mode for project founders who are entering historical data: emphasizes bulk import, retroactive dates, pseudo-anonymous contributors, and compensation status for past unpaid work | P0 | [todo] |

### 8f.4 Onboarding containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8f.4.1 | `onboarding` | Onboarding wizard container: step-by-step setup, bulk import, template selection, progress tracking. Founder mode for historical data entry | P0 | [todo] |
| 8f.4.2 | `bulk_import` | Bulk data entry: CSV/JSON upload, column mapping, validation, draft review, commit. Supports contributors, contributions, agreements, assets | P0 | [todo] |

---

## 8g. Governance Meetings, Minutes & Resolutions

### 8g.1 Meeting management

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8g.1.1 | Meeting scheduling | Schedule governance meetings: date, time, duration, location (physical/virtual/hybrid), agenda items, expected attendees, quorum requirement | P0 | [todo] |
| 8g.1.2 | Agenda management | Agenda items are submitted, ordered, and published before the meeting. Each item has: title, description, proposed resolution, time allocation, presenter | P0 | [todo] |
| 8g.1.3 | Attendance recording | Record attendees (DID), proxies (who voted on behalf of whom), apologies, and absences. Quorum verification at meeting start | P0 | [todo] |
| 8g.1.4 | Minutes recording | Minutes are recorded in real-time or entered after the meeting. Minutes are append-only: discussion summary, per-item outcomes, action items, resolutions, dissenting views | P0 | [todo] |
| 8g.1.5 | Action items | Each meeting produces action items: assignee, task, deadline, linked work item. Action items tracked to completion. Overdue items flagged | P0 | [todo] |
| 8g.1.6 | Resolutions | Formal resolutions: text, moved by, seconded by, vote result (for/against/abstain), binding/non-binding, effective date. Resolutions are append-only with provenance | P0 | [todo] |
| 8g.1.7 | Meeting types | Standing (recurring), special (called for specific purpose), emergency (urgent), annual (yearly review). Each type may have different quorum and notice requirements | P1 | [todo] |
| 8g.1.8 | Proxy voting | Members may designate a proxy to vote on their behalf. Proxy designation is DID-signed, revocable, and may be limited to specific meeting or agenda items | P1 | [todo] |

### 8g.2 Resolution lifecycle

| # | Stage | Description | Priority | Status |
|---|-------|-------------|----------|--------|
| 8g.2.1 | Proposed | Resolution proposed by eligible member, added to agenda | P0 | [todo] |
| 8g.2.2 | Discussed | Discussed at meeting; minutes record discussion | P0 | [todo] |
| 8g.2.3 | Voted | Vote taken: for/against/abstain per member. Result determined by voting rules (majority, supermajority, consensus) | P0 | [todo] |
| 8g.2.4 | Carried/Lost | Resolution is carried (adopted) or lost (rejected). Result is append-only | P0 | [todo] |
| 8g.2.5 | Enacted | Carried resolutions are enacted: may trigger governance changes, agreement amendments, obligation updates, container modifications | P1 | [todo] |
| 8g.2.6 | Reviewed | Resolutions can be reviewed at subsequent meetings. Review may confirm, amend, or rescind (new resolution required to rescind) | P1 | [todo] |

### 8g.3 Governance settings

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8g.3.1 | Voting system configuration | Define voting system: simple majority, supermajority (e.g. 2/3), consensus, ranked-choice, approval voting. Per decision type (may differ for different categories) | P0 | [todo] |
| 8g.3.2 | Eligibility rules | Who can vote: all members, contributors only, credential holders only, role-based. Quorum: minimum number or percentage for valid vote | P0 | [todo] |
| 8g.3.3 | Decision categories | Categories of decisions with different rules: routine (simple majority), significant (supermajority), constitutional (unanimous or near-unanimous), emergency (expedited process) | P0 | [todo] |
| 8g.3.4 | Transparency setting | Per decision type: `public` (all members see votes), `recorded` (votes recorded but not publicly attributed), `anonymous` (votes anonymous), `zk` (zero-knowledge proof of valid vote without revealing voter or vote) | P0 | [todo] |
| 8g.3.5 | Notice periods | Minimum notice before a vote: e.g. routine 24h, significant 7 days, constitutional 30 days. Emergency provisions for shorter notice | P1 | [todo] |
| 8g.3.6 | Cooling-off period | Period after a vote during which the decision can be revisited: e.g. 7 days for significant decisions. Allows for second thoughts or new information | P2 | [todo] |
| 8g.3.7 | Delegation chains | Member may delegate their vote to another, who may in turn delegate. Delegation chains are visible and limited in depth (configurable) | P1 | [todo] |

### 8g.4 Governance containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8g.4.1 | `governance_meetings` | Meeting scheduler, agenda, attendance, minutes, action items, resolutions. Meeting history. Links to voting container and governance settings | P0 | [todo] |
| 8g.4.2 | `governance_settings` | (Previously defined in §2.4.1 — enhanced) Voting system config, eligibility, decision categories, transparency settings, notice periods, delegation chains, cooling-off | P0 | [todo] |

---

## 8h. Conflict of Interest

### 8h.1 COI declaration

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8h.1.1 | COI declaration | Any agent may declare a conflict of interest: nature of conflict, affected area (project, decision, agreement, contributor), severity (potential/perceived/actual), related parties, proposed management (recusal, disclosure-only, managed participation) | P0 | [todo] |
| 8h.1.2 | Mandatory declarations | Certain roles or actions require COI declaration before participation: e.g. voting on a decision where the voter has a financial interest, reviewing a deliverable they contributed to, approving a budget they benefit from | P0 | [todo] |
| 8h.1.3 | Standing declarations | Agents may file standing COI declarations that apply to all future decisions in a category (e.g. "I am employed by Org X, which is a supplier to this project — declare for all supplier-selection decisions") | P1 | [todo] |
| 8h.1.4 | Annual COI review | Members in governance roles complete an annual COI review: confirm existing declarations, add new ones, retire stale ones | P2 | [todo] |

### 8h.2 COI management

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8h.2.1 | Recusal | An agent with a COI may recuse themselves from a decision. Recusal is recorded: agent, decision, reason. Recused agents cannot vote, comment, or influence the decision | P0 | [todo] |
| 8h.2.2 | Automatic recusal | System may automatically recuse an agent based on declared COI and decision category. Agent is notified and can contest the automatic recusal | P1 | [todo] |
| 8h.2.3 | Disclosure-only | For minor or perceived conflicts, agent discloses but continues to participate. Disclosure is visible to all decision participants | P0 | [todo] |
| 8h.2.4 | Managed participation | For moderate conflicts, agent participates under conditions: e.g. may comment but not vote, or may vote but not chair. Conditions set by governance | P1 | [todo] |
| 8h.2.5 | COI register | Project-wide COI register: all declarations, status (active/resolved/retired), affected areas. Visible to governance body. Visibility to all members configurable | P0 | [todo] |
| 8h.2.6 | COI in meetings | Meeting agenda items show declared COIs for each item. Recused members are listed. Minutes record recusals and managed participation arrangements | P0 | [todo] |
| 8h.2.7 | Undeclared COI | If an undeclared COI is discovered (via complaint or audit), it may trigger: voiding of the affected decision, complaint filing (§8e.2), sanction, or mandatory re-vote | P1 | [todo] |

### 8h.3 COI container

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8h.3.1 | `conflict_of_interest` | COI register: declarations, management status, affected areas. Declaration form. Recusal tracking. Links to governance meetings, voting, decisions | P0 | [todo] |

---

## 8i. Zero-Knowledge Privacy Controls

### 8i.1 ZK in governance

Some governance decisions involve information that should not be fully public
(e.g. an accusation of wrongdoing that can only be resolved by disclosing
information breaching others' rights or privacy). Zero-knowledge proofs allow
certain facts to be verified without revealing underlying data.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8i.1.1 | ZK vote verification | Prove that a vote was cast by an eligible voter and counted correctly, without revealing which way they voted or who they are. Vote count is verifiable, individual votes are private | P1 | [todo] |
| 8i.1.2 | ZK eligibility proof | Prove eligibility to vote (e.g. holds required credential, is a contributor) without revealing identity. Useful for anonymous or pseudo-anonymous members | P1 | [todo] |
| 8i.1.3 | ZK quorum proof | Prove quorum was met without revealing who attended. Only the count is public | P1 | [todo] |
| 8i.1.4 | ZK conflict disclosure | In a dispute or complaint, prove that a conflict of interest exists (or does not exist) without revealing the specific nature of the conflict. Only the existence/absence is verified | P1 | [todo] |
| 8i.1.5 | ZK sanction proof | Prove that a sanction was applied and is in effect, without revealing the sanctioned agent's identity or the specific complaint details | P2 | [todo] |

### 8i.2 ZK in disputes & complaints

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8i.2.1 | ZK evidence verification | Prove that evidence exists and meets certain criteria (e.g. timestamped, signed by a specific DID, contains specific keywords) without revealing the evidence content. Used when evidence contains sensitive information | P1 | [todo] |
| 8i.2.2 | Private dispute resolution | Dispute details are visible only to parties, mediators, and governance body. A ZK proof is published proving the dispute was resolved according to process, without revealing the details. Public record shows: dispute existed, was resolved, resolution type | P1 | [todo] |
| 8i.2.3 | Accusation privacy | An accusation of wrongdoing may require disclosure of information that breaches others' rights. The process: accusation is filed privately, evidence is reviewed by a restricted panel, ZK proof of due process is published. Full disclosure only if the panel determines it is necessary and proportionate | P1 | [todo] |
| 8i.2.4 | Whistleblower ZK | Whistleblower identity is protected via ZK: prove the complaint was filed by a legitimate agent (not fabricated) without revealing who. Only a designated authority can unmask in cases of verified bad-faith filing | P1 | [todo] |

### 8i.3 ZK in compensation & obligations

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8i.3.1 | ZK compensation proof | Prove that a contributor was compensated (or not) at a certain level, without revealing the exact amount. Useful for public transparency without privacy breach | P2 | [todo] |
| 8i.3.2 | ZK obligation proof | Prove that an obligation cost exists and is being recovered, without revealing individual contributor compensation details. Aggregate proof only | P2 | [todo] |
| 8i.3.3 | ZK contribution proof | Prove that a contribution was made (type, quantity, date) without revealing the contributor's identity. For pseudo-anonymous contributors who want their contribution verified but not attributed | P2 | [todo] |

### 8i.4 Transparency vs privacy settings

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8i.4.1 | Transparency default | Project governance sets a transparency default: `full` (all records public to members), `partial` (some records restricted), `minimal` (only aggregate/proof data public) | P0 | [todo] |
| 8i.4.2 | Per-record sensitivity | Each record has a sensitivity class (existing: Public / Restricted / Classified / Selfhood). Records with higher sensitivity use ZK proofs for public-facing summaries | P0 | [todo] |
| 8i.4.3 | Member transparency override | Members can request greater transparency on specific records via governance process. If approved, restricted records become visible to the requesting member | P1 | [todo] |
| 8i.4.4 | Public transparency mode | For open/civic projects, a public transparency mode publishes non-sensitive records to non-members. ZK proofs replace sensitive data in the public view | P1 | [todo] |

---

## 8j. Provenance Studies, Innovation, Research & IP Creation

### 8j.1 Provenance studies

Tracking the history of works in a particular area — who did what, when, building
on whose prior work — helps understand the lineage of ideas, artefacts, and
innovations. This is distinct from per-record provenance (which tracks authorship
of individual records); provenance studies trace the broader intellectual and
creative genealogy.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8j.1.1 | Work lineage graph | Visual graph showing how works relate: work A influenced work B, work B built upon work C, work D is a derivative of work A. Nodes are works (deliverables, publications, artefacts), edges are typed relationships (influenced_by, derived_from, extended, replicated, refuted, corroborated) | P1 | [todo] |
| 8j.1.2 | People & agent timeline | Timeline of people and agents involved in a domain: who contributed what, when, in what capacity. Cross-references contribution ledger, credentials, and wiki. Shows the human and institutional history of a pursuit | P1 | [todo] |
| 8j.1.3 | Citation chain | Track citation/reference chains: which works cite which, building a citation graph. Useful for research projects and for establishing priority (who first asserted a concept) | P1 | [todo] |
| 8j.1.4 | Influence map | Beyond formal citations, track informal influence: "this idea came from a conversation with X", "this approach was inspired by Y's work in domain Z". Softer than citations, still valuable for understanding intellectual history | P2 | [todo] |
| 8j.1.5 | Provenance study report | Generate a provenance study report for a domain: summary of key works, people, timeline, influence map, citation graph. Exportable as a wiki page or standalone document | P2 | [todo] |
| 8j.1.6 | External work registry | Register works outside the project that are relevant to the domain: external papers, books, datasets, tools, standards. Each with metadata (authors, date, DOI/URL, license). Links to internal works via influence/citation edges | P1 | [todo] |
| 8j.1.7 | Priority assertion | Formally assert priority: "this work first asserted concept X on date Y". Priority assertions are append-only with provenance. Disputable via §8e.1. Links to evidence (work record, timestamp, witness attestations) | P1 | [todo] |

### 8j.2 Innovation tracking

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8j.2.1 | Innovation log | Append-only log of innovations: description, category (process, product, method, tool, concept), novelty assessment (incremental, significant, breakthrough), evidence, date of conception, date of reduction to practice | P1 | [todo] |
| 8j.2.2 | Innovation pipeline | Track innovations from conception → development → validation → adoption. Status at each stage. Links to work items, deliverables, and wiki pages | P1 | [todo] |
| 8j.2.3 | Prior art search | Before asserting novelty, search prior art: internal provenance studies, external work registry, commons artefact registry. Results logged with provenance | P2 | [todo] |
| 8j.2.4 | Innovation attribution | Attribute innovations to contributors: who conceived, who developed, who validated. Attribution is append-only. Disputable via §8e.1. Links to contribution ledger | P1 | [todo] |
| 8j.2.5 | Innovation awards | Link innovations to the awards system (§8d.1): innovations may be nominated for innovation awards. Evidence chain from innovation log → award nomination → award issuance | P2 | [todo] |

### 8j.3 Research tools

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8j.3.1 | Research question tracker | Define research questions: question, hypothesis, status (open/investigated/answered/abandoned), linked experiments/studies, answer with evidence | P1 | [todo] |
| 8j.3.2 | Experiment log | Log experiments: hypothesis, method, parameters, results, conclusion, reproducibility data. Append-only. Links to research questions and datasets | P1 | [todo] |
| 8j.3.3 | Literature review | Structured literature review: papers reviewed, key findings, relevance to project, quality assessment. Links to external work registry | P1 | [todo] |
| 8j.3.4 | Replication tracker | Track replication attempts: original study, replication method, results, consistency assessment. Supports the reproducibility of research findings | P2 | [todo] |
| 8j.3.5 | Research ethics log | Log ethics considerations: IRB approval, consent records, data sensitivity, potential harms, mitigation measures. Links to credentials and complaints | P1 | [todo] |

### 8j.4 IP creation & management

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8j.4.1 | IP registry | Registry of intellectual property created by the project: type (patent, copyright, trade secret, design right, plant variety, traditional knowledge), title, inventors/authors, date of creation, status (draft/filed/granted/expired), jurisdiction, linked deliverables | P0 | [todo] |
| 8j.4.2 | IP creation workflow | Workflow: conception (innovation log) → disclosure (IP registry entry) → review (prior art, novelty assessment) → filing decision (file/keep secret/publish to commons) → filing (patent application, copyright registration, commons publication) → maintenance (renewals, enforcement) | P1 | [todo] |
| 8j.4.3 | Inventor attribution | Formally attribute inventors/authors per IP item. Attribution is append-only with provenance. Disputable via §8e.1. Links to contribution ledger and credentials | P0 | [todo] |
| 8j.4.4 | IP licensing | Each IP item has licensing terms (links to §8c license builder). IP may be licensed differently from project deliverables (e.g. patent filed but software source published to commons) | P1 | [todo] |
| 8j.4.5 | IP enforcement log | Log enforcement actions: cease and desist, litigation, settlement, licensing offer. Append-only with provenance. Links to disputes | P2 | [todo] |
| 8j.4.6 | Defensive publications | Publish defensive disclosures to commons to prevent patenting by others (prior art establishment). Links to commons publication and external work registry | P1 | [todo] |
| 8j.4.7 | Traditional knowledge protection | For projects involving traditional knowledge: record source community, consent for use, benefit-sharing terms, cultural sensitivity class. Links to agreements and rights | P1 | [todo] |

### 8j.5 Containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8j.5.1 | `provenance_studies` | Work lineage graph, people timeline, citation chain, influence map, external work registry, priority assertions. Provenance study report generator | P1 | [todo] |
| 8j.5.2 | `innovation_log` | Innovation log, pipeline, prior art search, attribution. Links to awards and IP registry | P1 | [todo] |
| 8j.5.3 | `research_tools` | Research questions, experiment log, literature review, replication tracker, ethics log. Links to wiki and datasets | P1 | [todo] |
| 8j.5.4 | `ip_registry` | IP registry, creation workflow, inventor attribution, licensing, enforcement log, defensive publications, traditional knowledge protection | P0 | [todo] |

---

## 8k. Knowledge Base & Project Specialist Agents

### 8k.1 Project knowledge base

A structured knowledge base that aggregates information from across the project
into a queryable, context-rich corpus. This knowledge base can be used by
software agents (LLM-powered) to provide project-specialist assistance.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8k.1.1 | Knowledge aggregation | Automatically aggregate from: wiki pages, discussion threads, decisions, meeting minutes, deliverables, research findings, innovation log, provenance studies, contribution ledger, agreements, governance settings. Each item indexed with metadata (type, author, date, tags, sensitivity class) | P1 | [todo] |
| 8k.1.2 | Knowledge graph | Build a knowledge graph from aggregated data: entities (people, organisations, concepts, works, technologies), relationships (contributed_to, authored, influenced, depends_on, implements, refutes). Graph is queryable (SPARQL) and visualisable | P1 | [todo] |
| 8k.1.3 | Tag & category system | Unified tag and category system across all project surfaces: tags on wiki pages, tasks, deliverables, innovations, research questions, meeting items. Tags are hierarchical and cross-cutting. Categories are project-defined | P1 | [todo] |
| 8k.1.4 | Version-aware indexing | Knowledge base is version-aware: queries can target current state or historical state at a given point in time. Uses the provenance chain to reconstruct past states | P2 | [todo] |
| 8k.1.5 | Sensitivity-aware access | Knowledge base respects sensitivity classes: Selfhood records excluded from aggregation, Classified records included but access-gated, Public/Restricted records included per transparency settings | P0 | [todo] |
| 8k.1.6 | External knowledge sources | Incorporate external knowledge: linked commons artefacts, external work registry, domain ontologies, standards documents. Each with provenance and license terms | P1 | [todo] |
| 8k.1.7 | Knowledge base search | Full-text search + structured query over the knowledge base. Faceted by type, tag, category, author, date range, sensitivity class. Results link back to source records | P0 | [todo] |

### 8k.2 Project specialist agents

Software agents that use the project knowledge base to provide specialised
assistance. These are computational systems — no mind, no intent (per agent
nomenclature rules). They produce model assertions and ungrounded generations
that must be verified by a natural agent before acting upon.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8k.2.1 | Agent definition | Define project specialist agents: name, purpose, knowledge base scope (which tags/categories/surfaces), capabilities (answer questions, summarise, draft documents, suggest actions, analyse patterns), model requirements (local/remote, size, domain fine-tuning) | P1 | [todo] |
| 8k.2.2 | Agent context window | The agent's context window is populated from the knowledge base: relevant wiki pages, recent decisions, active tasks, contributor profiles, project type defaults. Context is assembled based on the query and the agent's scope | P1 | [todo] |
| 8k.2.3 | Agent query interface | Chat-style interface for querying project specialist agents. Queries are logged with provenance. Responses include citations to source records in the knowledge base. Responses are marked as model assertions requiring verification | P1 | [todo] |
| 8k.2.4 | Agent grounding | Agent responses are grounded in the knowledge base: every factual assertion in a response must cite a source record. Ungrounded assertions are flagged. This is the neuro-symbolic bridge — the knowledge base provides the symbolic substrate, the LLM provides the natural language interface | P0 | [todo] |
| 8k.2.5 | Agent capability gating | Agents are capability-gated: an agent cannot perform actions (modify records, send payments, make decisions) unless explicitly granted the capability. By default, agents are read-only (query and summarise) | P0 | [todo] |
| 8k.2.6 | Agent provenance | Every agent response is logged: agent ID, query, response, source citations, context window contents, model version, timestamp. Append-only. This is the audit trail for agent-assisted work | P1 | [todo] |
| 8k.2.7 | Agent fine-tuning corpus | Build a fine-tuning corpus from the knowledge base: Q&A pairs generated from wiki content, decision summaries, task descriptions. Corpus is exportable for model fine-tuning. Sensitivity-aware (Selfhood records excluded) | P2 | [todo] |
| 8k.2.8 | Agent templates | Pre-configured agent templates per project type: "Welfare Support Specialist" (knows welfare protocols, participant rights, safeguard procedures), "Humanitarian ICT Specialist" (knows commons licensing, obligation recovery, peace infrastructure instruments), "Software Project Specialist" (knows codebase, release process, contributor model) | P2 | [todo] |
| 8k.2.9 | Agent collaboration | Multiple specialist agents may collaborate: one answers the query, another verifies grounding, a third checks for sensitivity violations. Multi-agent orchestration via VibeScript | P2 | [todo] |
| 8k.2.10 | Agent feedback loop | Natural agents (humans) can provide feedback on agent responses: helpful/not helpful, accurate/inaccurate, well-grounded/poorly-grounded. Feedback is logged and used to improve the knowledge base and fine-tuning corpus | P1 | [todo] |

### 8k.3 Knowledge base containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8k.3.1 | `knowledge_base` | Knowledge aggregation dashboard: indexed sources, knowledge graph visualisation, tag/category system, search interface. Shows coverage (which surfaces are indexed, gaps) | P1 | [todo] |
| 8k.3.2 | `agent_console` | Project specialist agent query interface: select agent, ask question, view response with citations, provide feedback. Agent definition editor. Agent provenance log | P1 | [todo] |

---

## 8l. External Data Sources & Datasets

### 8l.1 Data source registry

Projects need to reference external data sources for evaluations (economic
indicators, scientific datasets, standards, reference data). Each source has
metadata, access terms, and provenance. The registry makes sources citeable,
verifiable, and swapable (e.g. PPP from OECD vs World Bank).

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8l.1.1 | Source registration | Register an external data source: name, publisher (OECD, World Bank, IMF, UN, WHO, NOAA, etc.), URL/API endpoint, data format (CSV, JSON, XML, RDF, CBOR-LD), update frequency (real-time, daily, monthly, quarterly, annual, irregular), license terms, access method (public, API key, subscription, purchased), sensitivity class, provenance | P0 | [todo] |
| 8l.1.2 | Source categorisation | Categories: economic (PPP, GDP, inflation, exchange rates), demographic (population, census, migration), health (epidemiology, WHO data), scientific (climate, biodiversity, genomics), geographic (GIS, satellite, cadastral), standards (ISO, IEEE, W3C), legal (regulations, case law), social (inequality indices, human development), other | P0 | [todo] |
| 8l.1.3 | Source versioning | Data sources have versions (e.g. "World Bank PPP 2024 release"). Evaluations reference specific versions. When a source updates, dependent evaluations are flagged for review. Version history retained | P0 | [todo] |
| 8l.1.4 | Source evaluation | Each source has a quality assessment: authority (official/academic/commercial/community), methodology transparency, peer review status, update reliability, historical accuracy. Helps users choose between competing sources | P1 | [todo] |
| 8l.1.5 | Source comparison | When multiple sources cover the same metric (e.g. PPP from OECD vs World Bank vs IMF), show side-by-side comparison: methodology differences, coverage, update frequency, last sync, values for key indicators | P1 | [todo] |
| 8l.1.6 | Source dependency tracking | Track which project evaluations depend on which sources. When a source updates or is deprecated, all dependent evaluations are flagged. Cascade impact assessment | P1 | [todo] |
| 8l.1.7 | Source deprecation | Mark a source as deprecated (no longer maintained, superseded, unreliable). Dependent evaluations flagged for migration to alternative source | P1 | [todo] |

### 8l.2 Dataset management

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8l.2.1 | Dataset registration | Register a dataset: name, source, description, schema (fields, types, units), size, format, date range, geographic coverage, license, sensitivity class, provenance. Datasets may be external (referenced) or internal (project-produced) | P0 | [todo] |
| 8l.2.2 | Dataset preview | Preview dataset contents: first N rows, schema, summary statistics, date range, missing data indicators. For large datasets, preview is sampled | P1 | [todo] |
| 8l.2.3 | Dataset citation | Citeable reference for each dataset: standard citation format (DataCite, Dublin Core). Citations are used in provenance studies (§8j), research tools (§8j.3), and knowledge base (§8k) | P1 | [todo] |
| 8l.2.4 | Dataset linking | Link datasets to: research questions, experiments, evaluations, deliverables, wiki pages, knowledge base entries. Bi-directional links (dataset shows what uses it; consumer shows which dataset) | P1 | [todo] |
| 8l.2.5 | Dataset versioning | Datasets have versions. New versions supersede old (with tombstone). Version diff (what changed). Dependent evaluations flagged on new version | P1 | [todo] |
| 8l.2.6 | Dataset transformation log | Log transformations applied to a dataset: filter, aggregate, join, normalize, anonymize. Each transformation is append-only with provenance. Enables reproducibility | P2 | [todo] |
| 8l.2.7 | Dataset sensitivity | Datasets have sensitivity class. Datasets containing personal data (Selfhood) require consent records and access controls. Anonymization status tracked | P0 | [todo] |

### 8l.3 Evaluation framework

Evaluations use data sources and datasets to compute project-relevant metrics.
Each evaluation is reproducible: it references the exact source versions and
methodology used.

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 8l.3.1 | Evaluation definition | Define an evaluation: name, purpose, input sources/datasets (with versions), methodology (formula, algorithm, model), output (metric, indicator, report), update frequency, responsible agent | P0 | [todo] |
| 8l.3.2 | Evaluation execution | Execute evaluation: fetch data from sources, apply methodology, produce output. Execution is logged with provenance (source versions, timestamp, executor). Results are append-only | P1 | [todo] |
| 8l.3.3 | Evaluation reproducibility | Any evaluation can be re-run with the same source versions to verify results. Reproducibility is a core principle — no black-box evaluations | P1 | [todo] |
| 8l.3.4 | Evaluation comparison | Compare evaluation results across time or across methodologies. E.g. PPP-adjusted compensation using OECD vs World Bank data — show the difference | P2 | [todo] |
| 8l.3.5 | Pre-built evaluations | Pre-built evaluation templates: PPP-adjusted compensation (§8b), obligation recovery rate (§8c), budget variance, resource allocation efficiency, contribution distribution equity, risk probability assessment | P1 | [todo] |
| 8l.3.6 | Custom evaluations | Projects may define custom evaluations using VibeScript: access registered data sources, apply custom logic, produce project-specific metrics. Capability-gated | P2 | [todo] |

### 8l.4 Pre-configured data sources

The system ships with a library of commonly used data sources (mocked for UI,
wired when daemon is available):

| # | Source | Publisher | Category | Use case | Status |
|---|--------|-----------|----------|----------|--------|
| 8l.4.1 | PPP indices | OECD, World Bank, IMF | Economic | Fair value compensation (§8b.1.2) | [todo] |
| 8l.4.2 | Exchange rates | BIS, ECB, Federal Reserve | Economic | Multi-currency wallet (§6.1) | [todo] |
| 8l.4.3 | GDP & economic indicators | World Bank, UN | Economic | Project economic context | [todo] |
| 8l.4.4 | Human Development Index | UNDP | Social | Welfare-support project context | [todo] |
| 8l.4.5 | Inequality indices (Gini) | World Bank | Social | Compensation equity evaluation | [todo] |
| 8l.4.6 | Climate data | NOAA, IPCC | Scientific | Environmental impact assessment | [todo] |
| 8l.4.7 | Health indicators | WHO | Health | Welfare-support project context | [todo] |
| 8l.4.8 | Population/demographic | UN Population Division | Demographic | Resource planning, market sizing | [todo] |
| 8l.4.9 | Standards references | ISO, IEEE, W3C, IETF | Standards | Compliance, interoperability | [todo] |
| 8l.4.10 | Legal/regulatory | World Legal Information Institute | Legal | Compliance, rights | [todo] |
| 8l.4.11 | Geospatial | OpenStreetMap, cadastral | Geographic | GIS, spatial planning | [todo] |
| 8l.4.12 | Commons artefact registry | QualiaDB commons | Permissive Commons | Provenance studies, obligation tracking | [todo] |
| 8l.4.13 | Crypto price feeds | CoinGecko, Kraken, Bitfinex | Economic | Wallet valuation, billing | [todo] |
| 8l.4.14 | Lightning Network stats | 1ML, Amboss | Economic | Wallet health, routing | [todo] |
| 8l.4.15 | Custom (user-defined) | Any | Any | Project-specific data source | [todo] |

### 8l.5 Data source containers

| # | Container type | Purpose | Priority | Status |
|---|---------------|---------|----------|--------|
| 8l.5.1 | `data_sources` | Data source registry: registered sources, categorisation, versioning, quality assessment, comparison view, dependency tracking, deprecation. Pre-configured source library. Dataset registry with preview, citation, linking, transformation log | P0 | [todo] |
| 8l.5.2 | `evaluations` | Evaluation framework: defined evaluations, execution log, reproducibility verification, comparison view, pre-built templates, custom evaluation editor | P1 | [todo] |

---

## 9. Per-Project-Type Variations

### 9.1 Default container sets per project type

Each project type (COP-P1 taxonomy) has a default set of containers:

| Project type | Required containers | Optional containers |
|--------------|---------------------|---------------------|
| Welfare-support | Project Sheet, Kanban, Budget, Cost Base, Discussion, Pulse, Calendar, Credentials, Governance, Governance Meetings, Disputes, Complaints, Corrections, COI, Onboarding, Data Sources | Health vitals, Clinical risk, Rights & Agreements, Agency & Delegation, Risk Register, Time Tracking, Bulk Import, Evaluations |
| Professional (house-build) | Project Sheet, Kanban, Budget, Deliverables, Reviews, Roadmap, Pulse, Gantt, Resource Report, Asset Manager | 3D model, GIS, Finance, Portal, Calendar, Events |
| Professional / open (software) | Project Sheet, Kanban, Cost Base, Obligation, Discussion, Pulse, Wiki, Task List, Issues, Automation, Integrations | VibeScript, Graph, Portal, Reviews, Analytics, Release Manager, Customer Feedback, Billing |
| Civic / open | Project Sheet, Kanban, Funding, Royalties, Portal, Pulse, News, Bounties, Sponsorship, Governance Meetings, COI | Discussion, Reviews, Events, Voting, Disputes, Complaints, Corrections, Onboarding |
| Research | Project Sheet, Kanban, Deliverables, Reviews, Roadmap, Discussion, Pulse, Wiki, Calendar, Credentials, Provenance Studies, Innovation Log, Research Tools, IP Registry, Knowledge Base, Data Sources, Evaluations | Library, Latex, Graph, Portal, Analytics, Retrospective, Agent Console |
| Humanitarian ICT / commons | Project Sheet, Kanban, Cost Base, Obligation Tracker, TSL State, Commons Publication, Commons Artefacts, Pulse, Asset Manager, Credentials, Agreement Builder, Compensation Model, Contribution Ledger, License Builder, Disputes, Complaints, Corrections, COI, Governance Meetings, Onboarding, Provenance Studies, IP Registry, Knowledge Base, Data Sources | Ontology Builder, Chemistry, Bioinformatics, Library, Distribution, Token Manager, Awards, Bounties, Governance, Voting, Bulk Import, Innovation Log, Research Tools, Agent Console, Evaluations |

### 9.2 Implementation

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 9.2.1 | Project type selector on Project Sheet | Drives default container set | P0 | [todo] |
| 9.2.2 | Container set function | `fn default_containers_for_type(project_type: &str) -> Vec<SeedContainer>` | P0 | [todo] |
| 9.2.3 | Lifecycle stage awareness | Container visibility changes with stage | P1 | [todo] |
| 9.2.4 | Container template registry | Pre-configured containers drag-drop from toolbox dock | P2 | [todo] |

---

## 10. Cross-Cutting Concerns

### 10.1 Inter-container linking

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 10.1.1 | Cross-container reference system | Wiki → task, task → deliverable, risk → decision, bounty → work item, event → task. Typed links with provenance | P1 | [todo] |
| 10.1.2 | Backlink display | When viewing a record, show all containers that reference it | P2 | [todo] |
| 10.1.3 | Drag-to-link | Drag a record from one container onto another to create a typed link | P2 | [todo] |

### 10.2 Honesty labels

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 10.2.1 | Every container shows honesty badge | live / present / partial / missing / planned | P0 | [todo] |
| 10.2.2 | Every tool shows honesty badge | | P0 | [todo] |
| 10.2.3 | Mock data clearly labeled | Footer or banner: "Mock data — engine wiring pending" | P0 | [done] (existing containers) |

### 10.3 Capability gating

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 10.3.1 | Per-action capability check | Every tool/container action checks capability at call time | P0 | [todo] |
| 10.3.2 | Disabled state with explanation | Show greyed-out + tooltip when capability unavailable | P1 | [todo] |

### 10.4 Accessibility

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 10.4.1 | WCAG contrast on all project surfaces | | P1 | [todo] |
| 10.4.2 | Keyboard navigation | Tab through containers, enter to focus, escape to exit | P1 | [todo] |
| 10.4.3 | Screen reader support | ARIA roles, live regions for dynamic updates | P2 | [todo] |
| 10.4.4 | Sanctuary mode respect | Selfhood records hidden, commons disabled, minimal UI | P1 | [todo] |

### 10.5 Multi-language (i18n)

| # | Item | Description | Priority | Status |
|---|------|-------------|----------|--------|
| 10.5.1 | String extraction | All visible strings extracted to a translation table | P2 | [todo] |
| 10.5.2 | Language selector | In settings, per-user preference | P2 | [todo] |
| 10.5.3 | Per-language wiki pages | Wiki pages can have translations linked by provenance | P2 | [todo] |

---

## 11. Implementation Priority & Sequencing

### Phase 1 — Core project surfaces (P0, partially done)

Already completed: kanban, project_sheet, budget, cost_base, deliverable, review, discussion, roadmap, commons, rights (5 tabs), wallet (4 tabs).

Remaining P0:

| # | Item | Dependencies | Status |
|---|------|-------------|--------|
| 11.1 | `dashboard` | All other containers exist (they do) | [todo] |
| 11.2 | `wiki` | Provenance chain infrastructure | [todo] |
| 11.3 | `governance` | Links to Rights & Agreements | [todo] |
| 11.4 | `credentials` | DID signing (mocked) | [todo] |
| 11.5 | Multi-currency wallet enhancement | Existing wallet container | [todo] |
| 11.6 | Multi-sig wallet support | Multi-currency | [todo] |
| 11.7 | Lifecycle stage field on Project Sheet | Project Sheet exists | [todo] |
| 11.8 | Honesty badges on all containers | | [todo] |
| 11.9 | Provenance display | | [todo] |
| 11.10 | Sensitivity class badges | | [todo] |
| 11.11 | `agreement_builder` — instrument-based agreement composition | Instrument library | [todo] |
| 11.12 | `compensation_model` — fair value calculator | Contribution ledger | [todo] |
| 11.13 | `contribution_ledger` — append-only contribution records | Kanban work items | [todo] |
| 11.14 | `license_builder` — differential licensing wizard | Agreement builder | [todo] |
| 11.15 | `obligation_tracker` — obligation recovery dashboard | Contribution ledger, billing | [todo] |
| 11.16 | `token_mgr` — token definition, minting, vesting | Wallet, contribution ledger | [todo] |
| 11.17 | Instrument library (human rights, CC, COP, fiduciary, labour, peace) | | [todo] |
| 11.18 | Fair value evaluation framework (base rate, PPP, skill, expertise) | | [todo] |
| 11.19 | Compensation model (multipliers, stage-specific, time-based) | Fair value framework | [todo] |
| 11.20 | Differential licensing (agent-type pricing, obligation recovery) | Compensation model | [todo] |
| 11.21 | Obligation cost calculation & tracking | Contribution ledger | [todo] |
| 11.22 | Royalty share calculation | Contribution ledger | [todo] |
| 11.23 | `disputes` — dispute filing, lifecycle, resolution | Governance, rights | [todo] |
| 11.24 | `complaints` — complaint filing, investigation, sanctions | Credentials, governance | [todo] |
| 11.25 | `corrections` — correction log, chain display, cascade | All appendable records | [todo] |
| 11.26 | Correction by original maker | Provenance chain | [todo] |
| 11.27 | Correction by other agent (propose → review) | Provenance chain | [todo] |
| 11.28 | Correction by dispute resolution (mandated) | Disputes | [todo] |
| 11.29 | Correction chain display | Provenance chain | [todo] |
| 11.30 | Dispute lifecycle (open → review → mediation → resolution → closed) | Governance | [todo] |
| 11.31 | Complaint investigation & findings | Governance | [todo] |
| 11.32 | `onboarding` — onboarding wizard, founder mode | Project Sheet | [todo] |
| 11.33 | `bulk_import` — CSV/JSON import, validation, draft mode | Onboarding | [todo] |
| 11.34 | `governance_meetings` — meetings, agenda, minutes, resolutions | Governance | [todo] |
| 11.35 | `conflict_of_interest` — COI register, declarations, recusal | Governance meetings | [todo] |
| 11.36 | Pseudo-anonymous contributor support (placeholder DID, visibility) | Contribution ledger | [todo] |
| 11.37 | Retroactive date entry (asserted vs valid time) | Bulk import | [todo] |
| 11.38 | Resolution lifecycle (proposed → discussed → voted → carried/lost → enacted) | Governance meetings | [todo] |
| 11.39 | Voting system configuration (majority, supermajority, consensus, ranked-choice) | Governance settings | [todo] |
| 11.40 | Transparency setting per decision type (public/recorded/anonymous/zk) | Governance settings | [todo] |
| 11.41 | COI declaration & recusal | Governance meetings, voting | [todo] |
| 11.42 | COI in meetings (agenda items show declared COIs) | Governance meetings | [todo] |
| 11.43 | Transparency default setting (full/partial/minimal) | Governance settings | [todo] |
| 11.44 | Per-record sensitivity class with ZK for public summaries | Provenance | [todo] |
| 11.45 | `ip_registry` — IP creation, inventor attribution, licensing, enforcement | Agreement builder, license builder | [todo] |
| 11.46 | Knowledge base sensitivity-aware access | Provenance, sensitivity classes | [todo] |
| 11.47 | Knowledge base search (full-text + structured) | Knowledge aggregation | [todo] |
| 11.48 | Agent grounding (every assertion cites source record) | Knowledge base | [todo] |
| 11.49 | Agent capability gating (read-only by default) | Capability system | [todo] |
| 11.50 | Per-constituent licensing (apply license to any individual constituent) | License builder | [todo] |
| 11.51 | Project default license + constituent override | License builder | [todo] |
| 11.52 | License scope selector (project default / inherited / specific) | License builder | [todo] |
| 11.53 | License provenance chain per constituent | License builder | [todo] |
| 11.54 | `data_sources` — source registry, categorisation, versioning, comparison | | [todo] |
| 11.55 | Dataset registration with schema, sensitivity, provenance | Data sources | [todo] |
| 11.56 | Source versioning & dependency tracking | Data sources | [todo] |
| 11.57 | Evaluation definition (sources, methodology, output) | Data sources | [todo] |
| 11.58 | Pre-configured source library (PPP, exchange rates, HDI, etc.) | Data sources | [todo] |

### Phase 2 — Extended project surfaces (P1)

| # | Item | Dependencies | Status |
|---|------|-------------|--------|
| 11.11 | `gantt` | Roadmap, Kanban | [todo] |
| 11.12 | `timeline` | Roadmap, events | [todo] |
| 11.13 | `calendar` | Events, milestones | [todo] |
| 11.14 | `doc_mgmt` | Asset manager | [todo] |
| 11.15 | `resource_report` | Cost base, time tracking | [todo] |
| 11.16 | `time_tracking` | Kanban work items | [todo] |
| 11.17 | `voting` | Governance | [todo] |
| 11.18 | `risk` | Governance | [todo] |
| 11.19 | `task_list` | Kanban | [todo] |
| 11.20 | `issues` | Kanban, deliverables | [todo] |
| 11.21 | `asset_mgr` | Deliverables, commons | [todo] |
| 11.22 | `events` | Calendar | [todo] |
| 11.23 | `bounties` | Wallet, kanban | [todo] |
| 11.24 | `automation` | VibeScript, IntentBus | [todo] |
| 11.25 | `analytics` | All project data | [todo] |
| 11.26 | `product_catalog` | Deliverables | [todo] |
| 11.27 | `release_manager` | Product catalog | [todo] |
| 11.28 | `customer_feedback` | Issues, product catalog | [todo] |
| 11.29 | `customer_support` | Issues | [todo] |
| 11.30 | `billing` | Wallet, budget | [todo] |
| 11.31 | `infrastructure` | | [todo] |
| 11.32 | Group manifold (5 containers) | Project containers exist | [todo] |
| 11.33 | Personal aggregate views (4 surfaces) | Project containers exist | [todo] |
| 11.34 | Stage-dependent rules engine | Lifecycle stages | [todo] |
| 11.35 | Inter-container linking | | [todo] |
| 11.36 | Per-project-type default container sets | | [todo] |
| 11.39 | Commons consumption tracking | | [todo] |
| 11.40 | `awards` — award definition, issuance, nomination | Credentials | [todo] |
| 11.41 | Token distribution rules | Token manager | [todo] |
| 11.42 | Token vesting & governance rights | Token manager, voting | [todo] |
| 11.43 | Agreement template registry | Agreement builder | [todo] |
| 11.44 | Conflict detection (instrument composition) | Agreement builder | [todo] |
| 11.45 | Tiered pricing for legal persons | License builder | [todo] |
| 11.46 | Sliding scale (PPP for legal persons) | License builder | [todo] |
| 11.47 | Waiver mechanism (humanitarian/org) | License builder, governance | [todo] |
| 11.48 | Retroactive compensation adjustment | Contribution ledger | [todo] |
| 11.49 | Token registry (cross-project) | Token manager | [todo] |
| 11.50 | Peer nomination for awards | Awards | [todo] |
| 11.51 | Dispute resolution enforcement | Disputes, corrections | [todo] |
| 11.52 | Escalation (mediation → arbitration → governance) | Disputes, governance | [todo] |
| 11.53 | Anonymous complaints | Complaints | [todo] |
| 11.54 | Sanction tracking | Complaints, credentials | [todo] |
| 11.55 | Whistleblower protection | Complaints, governance | [todo] |
| 11.56 | Correction by clarification | Corrections | [todo] |
| 11.57 | Cascade corrections (dependent recalculation) | Corrections | [todo] |
| 11.58 | Correction notification | Corrections | [todo] |
| 11.59 | Retraction (special correction type) | Corrections | [todo] |
| 11.60 | Dispute history (agent profiles, project records) | Disputes | [todo] |
| 11.61 | Identity claim (pseudo-anonymous → real DID) | Pseudo-anonymous support | [todo] |
| 11.62 | Delegate entry (founder enters contributors on their behalf) | Pseudo-anonymous support | [todo] |
| 11.63 | Template-based setup (per project type) | Onboarding | [todo] |
| 11.64 | Meeting types (standing, special, emergency, annual) | Governance meetings | [todo] |
| 11.65 | Proxy voting | Governance meetings, voting | [todo] |
| 11.66 | Notice periods per decision category | Governance settings | [todo] |
| 11.67 | Delegation chains (vote delegation) | Governance settings | [todo] |
| 11.68 | Standing COI declarations | COI | [todo] |
| 11.69 | Automatic recusal based on declared COI | COI, voting | [todo] |
| 11.70 | Managed participation (conditions for moderate conflicts) | COI | [todo] |
| 11.71 | Undeclared COI discovery & voiding | COI, complaints | [todo] |
| 11.72 | ZK vote verification | Voting, governance | [todo] |
| 11.73 | ZK eligibility proof | Voting | [todo] |
| 11.74 | ZK quorum proof | Governance meetings | [todo] |
| 11.75 | ZK conflict disclosure | COI, disputes | [todo] |
| 11.76 | Private dispute resolution (ZK proof of due process) | Disputes | [todo] |
| 11.77 | Accusation privacy (restricted panel, ZK proof) | Complaints, disputes | [todo] |
| 11.78 | Whistleblower ZK protection | Complaints | [todo] |
| 11.79 | Member transparency override | Governance settings | [todo] |
| 11.80 | Public transparency mode (open/civic projects) | Governance settings | [todo] |
| 11.81 | `provenance_studies` — work lineage, people timeline, citation chain | Wiki, contribution ledger | [todo] |
| 11.82 | `innovation_log` — innovation tracking, pipeline, attribution | Provenance studies | [todo] |
| 11.83 | `research_tools` — research questions, experiments, literature review | Wiki, external work registry | [todo] |
| 11.84 | `knowledge_base` — aggregation, knowledge graph, tag system, search | All project surfaces | [todo] |
| 11.85 | `agent_console` — specialist agent query interface, citations, feedback | Knowledge base | [todo] |
| 11.86 | Agent definition & context window assembly | Knowledge base | [todo] |
| 11.87 | Agent provenance logging | Agent console | [todo] |
| 11.88 | Agent feedback loop (human feedback on agent responses) | Agent console | [todo] |
| 11.89 | External work registry | Provenance studies | [todo] |
| 11.90 | Priority assertion (formal priority claim with evidence) | Provenance studies | [todo] |
| 11.91 | IP creation workflow (conception → disclosure → filing → maintenance) | IP registry | [todo] |
| 11.92 | Defensive publications (prior art establishment via commons) | IP registry, commons | [todo] |
| 11.93 | Traditional knowledge protection | IP registry, agreements | [todo] |
| 11.94 | Knowledge graph (entities, relationships, SPARQL queryable) | Knowledge base | [todo] |
| 11.95 | Tag & category system (unified across all surfaces) | Knowledge base | [todo] |
| 11.96 | `evaluations` — execution, reproducibility, comparison, templates | Data sources | [todo] |
| 11.97 | Source quality assessment & comparison | Data sources | [todo] |
| 11.98 | Source deprecation & migration | Data sources | [todo] |
| 11.99 | Dataset preview, citation, linking | Data sources | [todo] |
| 11.100 | Dataset versioning & transformation log | Data sources | [todo] |
| 11.101 | Evaluation execution & reproducibility | Evaluations | [todo] |
| 11.102 | Pre-built evaluation templates (PPP, obligation, budget variance) | Evaluations | [todo] |
| 11.103 | Custom evaluations via VibeScript | Evaluations, VibeScript | [todo] |
| 11.104 | Evaluation comparison across methodologies/sources | Evaluations | [todo] |
| 11.105 | License inheritance (parent → child constituents) | License builder | [todo] |
| 11.106 | Mixed-licensing view (all licenses across constituents) | License builder | [todo] |
| 11.107 | License conflict detection (incompatible licenses combined) | License builder | [todo] |
| 11.108 | Bulk license assignment (multiple constituents at once) | License builder | [todo] |
| 11.109 | Per-contributor licensing (contributor specifies terms for own contributions) | License builder, contribution ledger | [todo] |

### Phase 3 — Polish (P2)

| # | Item | Status |
|---|------|--------|
| 11.51 | `news` | [todo] |
| 11.52 | `portfolio` | [todo] |
| 11.53 | `integrations` | [todo] |
| 11.54 | `retrospective` | [todo] |
| 11.55 | `distribution` | [todo] |
| 11.56 | `logistics` | [todo] |
| 11.57 | `coupons` | [todo] |
| 11.58 | `sponsorship` | [todo] |
| 11.59 | `eval_license` | [todo] |
| 11.60 | `subscription_mgr` | [todo] |
| 11.61 | Commons seed status | [todo] |
| 11.62 | Commons artefact versioning | [todo] |
| 11.63 | Container template registry | [todo] |
| 11.64 | i18n | [todo] |
| 11.65 | Backlink display | [todo] |
| 11.66 | Drag-to-link | [todo] |
| 11.67 | Group-level analytics | [todo] |
| 11.68 | Group suppliers | [todo] |
| 11.69 | Cross-project dependencies | [todo] |
| 11.70 | Custom instrument definition | [todo] |
| 11.71 | Market rate comparison (informational) | [todo] |
| 11.72 | Token-benefit conversion | [todo] |
| 11.73 | Award categories (suggested set) | [todo] |
| 11.74 | License verification (consumer-side) | [todo] |
| 11.75 | License preview (human-readable) | [todo] |

---

## 12. File Structure (proposed)

```
qualia-ui/src/browser/
├── project_views/
│   ├── mod.rs                    (existing — add new modules)
│   ├── kanban.rs                 (existing)
│   ├── project_sheet.rs          (existing — add lifecycle stage)
│   ├── budget.rs                 (existing)
│   ├── cost_base.rs              (existing)
│   ├── deliverable.rs            (existing)
│   ├── review.rs                 (existing)
│   ├── discussion.rs             (existing)
│   ├── roadmap.rs                (existing)
│   ├── commons.rs                (existing)
│   ├── dashboard.rs              (new — P0)
│   ├── wiki.rs                   (new — P0)
│   ├── governance.rs             (new — P0)
│   ├── credentials.rs            (new — P0)
│   ├── gantt.rs                  (new — P1)
│   ├── timeline.rs               (new — P1)
│   ├── calendar.rs               (new — P1)
│   ├── doc_mgmt.rs               (new — P1)
│   ├── resource_report.rs        (new — P1)
│   ├── time_tracking.rs          (new — P1)
│   ├── voting.rs                 (new — P1)
│   ├── risk.rs                   (new — P1)
│   ├── task_list.rs              (new — P1)
│   ├── issues.rs                 (new — P1)
│   ├── asset_mgr.rs              (new — P1)
│   ├── events.rs                 (new — P1)
│   ├── bounties.rs               (new — P1)
│   ├── automation.rs             (new — P1)
│   ├── analytics.rs              (new — P1)
│   ├── product_catalog.rs        (new — P1)
│   ├── release_manager.rs        (new — P1)
│   ├── customer_feedback.rs      (new — P1)
│   ├── customer_support.rs       (new — P1)
│   ├── billing.rs                (new — P1)
│   ├── infrastructure.rs         (new — P1)
│   ├── news.rs                   (new — P2)
│   ├── portfolio.rs              (new — P2)
│   ├── integrations.rs           (new — P2)
│   ├── retrospective.rs          (new — P2)
│   ├── distribution.rs           (new — P2)
│   ├── logistics.rs              (new — P2)
│   ├── coupons.rs                (new — P2)
│   ├── sponsorship.rs            (new — P2)
│   ├── eval_license.rs           (new — P2)
│   ├── subscription_mgr.rs       (new — P2)
│   ├── commons_consumption.rs    (new — P1)
│   ├── agreement_builder.rs      (new — P0: instrument-based agreement composition)
│   ├── compensation_model.rs     (new — P0: fair value calculator, multipliers)
│   ├── contribution_ledger.rs    (new — P0: append-only contribution records)
│   ├── license_builder.rs        (new — P0: differential licensing wizard)
│   ├── obligation_tracker.rs     (new — P0: obligation recovery dashboard)
│   ├── token_mgr.rs              (new — P0: token definition, minting, vesting)
│   ├── awards.rs                 (new — P1: award registry, issuance, nomination)
│   ├── disputes.rs               (new — P0: dispute filing, lifecycle, resolution)
│   ├── complaints.rs             (new — P0: complaint filing, investigation, sanctions)
│   ├── corrections.rs            (new — P0: correction log, chain display, cascade)
│   ├── onboarding.rs             (new — P0: onboarding wizard, founder mode)
│   ├── bulk_import.rs            (new — P0: CSV/JSON import, validation, draft mode)
│   ├── governance_meetings.rs    (new — P0: meetings, agenda, minutes, resolutions)
│   ├── conflict_of_interest.rs   (new — P0: COI register, declarations, recusal)
│   ├── provenance_studies.rs     (new — P1: work lineage, people timeline, citation chain)
│   ├── innovation_log.rs         (new — P1: innovation tracking, pipeline, attribution)
│   ├── research_tools.rs         (new — P1: research questions, experiments, literature review)
│   ├── ip_registry.rs            (new — P0: IP creation, inventor attribution, licensing)
│   ├── knowledge_base.rs         (new — P1: aggregation, knowledge graph, search)
│   ├── agent_console.rs          (new — P1: specialist agent query, citations, feedback)
│   ├── data_sources.rs           (new — P0: source registry, datasets, versioning, comparison)
│   └── evaluations.rs            (new — P1: evaluation framework, execution, templates)
├── rights_views/
│   ├── mod.rs                    (existing — enhance with accounts + multi-sig tabs)
│   ├── rights_tabs.rs            (existing)
│   ├── wallet_tabs.rs            (existing — enhance for multi-currency)
│   ├── wallet_accounts.rs        (new — P0: account registry, purpose assignment)
│   └── wallet_multisig.rs        (new — P0: multi-sig config, proposals, signatures)
├── group_views/
│   ├── mod.rs                    (new — P1: group manifold containers)
│   ├── group_profile.rs          (new — P1)
│   ├── group_portfolio.rs        (new — P1)
│   ├── group_community.rs        (new — P1)
│   ├── group_suppliers.rs        (new — P2)
│   └── group_governance.rs       (new — P1)
├── personal_views/
│   ├── mod.rs                    (new — P1: personal aggregate surfaces)
│   ├── personal_calendar.rs      (new — P1)
│   ├── personal_tasks.rs         (new — P1)
│   ├── personal_dashboard.rs     (new — P1)
│   └── notification_center.rs    (new — P1)
└── ... (existing modules)
```

---

## 13. Container count summary

| Category | Count |
|----------|-------|
| Existing project containers | 9 |
| Existing rights/wallet containers | 2 |
| New planning & visualization | 4 |
| New knowledge & documentation | 2 |
| New resource management | 2 |
| New governance & policy | 3 |
| New task & issue management | 2 |
| New asset & licensing | 2 |
| New community & communication | 3 |
| New portfolio & cross-project | 1 |
| New automation & integration | 2 |
| New analytics & retrospective | 2 |
| New product/service/operations | 7 |
| New group containers | 5 |
| New personal aggregate surfaces | 4 |
| New agreement framework (§8a) | 1 |
| New compensation & obligation (§8b) | 2 |
| New differential licensing (§8c) | 2 |
| New awards & tokens (§8d) | 2 |
| New disputes, complaints & corrections (§8e) | 3 |
| New onboarding & bulk admin (§8f) | 2 |
| New governance meetings (§8g) | 2 |
| New conflict of interest (§8h) | 1 |
| New ZK privacy controls (§8i) | 0 (cross-cutting) |
| New provenance, innovation, research & IP (§8j) | 4 |
| New knowledge base & specialist agents (§8k) | 2 |
| New external data sources & evaluations (§8l) | 2 |
| **Total** | **74** |

---

## 14. Open questions for the principal

1. **Group manifold**: Should groups be a separate manifold (switchable via the manifold tab bar) or a container type within the Projects manifold? Separate manifold seems cleaner for multi-project groups.
2. **Personal views manifold**: Should personal aggregate views (calendar, tasks, dashboard, notifications) be a separate "Personal" manifold, or part of an existing manifold (e.g. Sanctuary or Settings)?
3. **Product/operations containers**: Should these live in the Projects manifold or a separate "Operations" manifold for projects that have reached the operation stage?
4. **Lifecycle stage transitions**: Should stage transitions be manual (governance action) or automatic (triggered by conditions like all deliverables accepted)? Manual seems safer for accountability.
5. **Multi-sig execution**: Should multi-sig be a wallet-level feature (configurable per transaction) or a project-level governance feature (configurable per purpose)? Both?
6. **Commons consumption**: Should consumed commons assets be tracked in the project's cost base (as a negative cost) or in a separate commons consumption container?
7. **Calendar scope**: Should the project-level calendar be a separate container from the personal calendar, or should there be one calendar with project/personal filters? One calendar with filters is more human-centric.
8. **Bounty escrow**: Should bounty escrow be held in the project wallet (multi-sig) or in a separate escrow account? Project wallet with multi-sig seems safer.
9. **Compensation multiplier scope**: Should the compensation multiplier (1.2x–10x) be applied uniformly to all resource contributors for a given stage, or should it be configurable per contributor? The plan supports both — is the default stage-level with per-contributor overrides acceptable?
10. **Obligation recovery rate**: Should the rate for legal persons be automatically calculated (obligation_cost / projected_usage × margin) or manually set by project governance? Automatic with manual override?
11. **TSL shift trigger**: Should the TSL State A → B shift happen automatically when obligation is fully recovered, or require a governance decision? Automatic seems aligned with the protocol but governance override may be needed.
12. **Token supply model**: Should project tokens be fixed supply (minted once at project inception) or mintable (new tokens minted as contributions are made)? Mintable with a cap?
13. **Instrument library provenance**: Should the instrument library (UDHR, CC, COP, etc.) be stored locally as CBOR-LD artefacts or loaded from an external commons registry? Local with commons sync?
14. **Peace infrastructure project type**: Should "peace infrastructure" be a distinct project type in the COP-P1 taxonomy, or a modifier that can be applied to any project type (e.g. humanitarian_ict + peace_modifier)?
15. **Fair value data sources**: For PPP adjustment, should the system use a specific data feed (World Bank, IMF) or allow manual entry? Mocked for now, but the interface should accommodate either.
16. **Governance meeting integration**: Should governance meetings be a standalone container or a tab within the existing governance container? Standalone seems cleaner for meeting history and minutes.
17. **COI enforcement strictness**: Should the system prevent (block) voting by recused agents, or merely flag it? Blocking is safer but may conflict with edge cases where recusal was automatic and the agent contests it.
18. **ZK implementation scope**: For the initial UI mock, should ZK controls be presented as labeled mock surfaces (\"ZK proof — engine wiring pending\") or should the UI hide ZK options entirely until the backend supports them? Labeled mock seems consistent with the existing honesty approach.
19. **Onboarding wizard vs. individual containers**: Should the onboarding wizard be a modal overlay (like search workbench) or a container on the canvas? Modal is better for first-run; container is better for ongoing bulk admin. Both?
20. **IP filing scope**: Should the UI support actual patent filing workflows ( interfacing with patent offices) or only track IP status manually? Manual status tracking with document attachment seems sufficient for the UI layer.
21. **Knowledge base storage**: Should the project knowledge base be stored as a CBOR-LD artefact (portable, commons-publishable) or as an index over existing records (no duplication)? Index-over-records is more efficient but less portable.
22. **Specialist agent model hosting**: Should project specialist agents run locally (WASM-side, small models) or require a daemon-side model server? Local small models for common queries, daemon-side for complex reasoning? UI should accommodate both via capability detection.
23. **Data source access**: Should the UI attempt to fetch live data from external sources (CORS-permitting) or always proxy through the daemon? Proxy is safer and avoids CORS issues, but live fetch gives immediate feedback for public APIs.
24. **Evaluation engine**: Should evaluations be computed client-side (WASM, for simple formulas) or always server-side (daemon, for complex models)? Client-side for simple formulas (PPP adjustment, budget variance), server-side for complex models (risk probability, resource optimisation).

---

_End of extended implementation plan. Update statuses as work progresses._
