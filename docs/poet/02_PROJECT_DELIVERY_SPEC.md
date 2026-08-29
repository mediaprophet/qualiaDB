# POET Project Delivery & Economics Specification

**Document ID:** `POET-SPEC-002`  
**Status:** Canonical Domain Specification  
**Scope:** Interactive Kanban, Task Graphs, Gantt Roadmaps, Milestones, and Fiduciary Project Economics in POET.

---

## 1. Overview & Human Workflow

Project management in POET is an interactive, visual system designed to help distributed teams and contributors plan, track, and execute collaborative work while maintaining cryptographic evidence and economic transparency.

```
+-----------------------------------------------------------------------------------+
|                        PROJECT DELIVERY WORKSPACE TOPOLOGY                        |
+-----------------------------------------------------------------------------------+
|  [Kanban Board] <====> [Task Dependency Graph] <====> [Milestone / Gantt Ribbon]   |
|         |                                                        |                |
|         v                                                        v                |
|  [Task Assignment & DIDs]                                [Fiduciary Economics]    |
|  - Assignee DID & Avatar                                 - Plan vs Actual Ledgers |
|  - Estimated / Logged Hours                              - Variance Calculation   |
|  - Priority & Status Chips                               - Royalties & Tax Flows  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Interactive Kanban Board Architecture

The Kanban board replaces static database forms with an interactive, column-based workflow:

### 2.1 Columns & Lifecycle States
1. **Backlog:** Unscheduled ideas, raw requests, and initial proposals.
2. **To Do:** Prioritized work items committed to the active milestone.
3. **In Progress:** Actively claimed tasks with assigned contributor DIDs.
4. **Review / Verification:** Completed tasks undergoing peer review or test validation.
5. **Done:** Formally accepted deliverables with permanent audit receipts.

### 2.2 Task Card Capabilities
- **Visual Drag-and-Drop:** Intuitive card dragging across columns or quick-move dropdown actions with immediate optimistic column re-rendering.
- **Card Metadata:**
  - Title and rich description (Markdown-enabled).
  - Priority badge (`Low`, `Medium`, `High`, `Critical`) with distinct color coding.
  - Assigned Contributor DID with avatar and readable handle.
  - Due date badge with dynamic urgency highlighting (e.g., amber for `< 48h`, red for overdue).
  - Subtask checklist with live progress bar (`3/5 completed`).
  - Attached Semantic Library artifacts and evidence links.

---

## 3. Task Graph & Dependency Management

Complex engineering and creative endeavors require dependency modeling:
- **Visual Dependency Links:** Tasks can declare blocking dependencies (`blocked_by: [task_id]`).
- **Cycle Prevention:** Automatic graph cycle detection prevents circular dependencies (`A -> B -> A`).
- **Critical Path Highlighting:** Visual emphasis on the sequence of dependent tasks determining the minimum project duration.
- **Blocker Badges:** Tasks with unresolved prerequisites display an explicit "Blocked" badge with clickable links to the blocking items.

---

## 4. Roadmaps, Milestones & Gantt Ribbons

- **Milestone Ribbons:** Group tasks by delivery milestone with target completion dates and percentage progress bars.
- **Interactive Gantt Timeline:** Horizontal bar chart mapping task durations along a timeline ribbon, supporting zoom from days to quarters.
- **4D Temporal Scrubbing:** Connects with POET's 4D timeline ribbon in the top menubar to scrub through historical task states and view future projections.

---

## 5. Fiduciary Economics & Budgeting

Project economics uses fixed six-decimal arithmetic across five distinct ledgers:

```
+-----------------------------------------------------------------------------------+
|                           PROJECT ECONOMICS LEDGERS                               |
+-------------------+-------------------+-------------------+-----------------------+
| 1. Plan Ledger    | 2. Actual Ledger  | 3. Funding Ledger | 4. Royalty & Tax      |
| Estimated costs   | Incurred expenses | Received grants,  | Automated revenue     |
| and contributor   | and approved time | investments, and  | distributions and     |
| commitments.      | disbursements.    | escrow balances.  | statutory splits.     |
+-------------------+-------------------+-------------------+-----------------------+
                                        |
                                        v
+-----------------------------------------------------------------------------------+
| 5. Derived Summary & Variance: Real-time calculation of Plan vs. Actual Variance, |
| Burn Rate, and Runway Balance with multi-currency support (USD, EUR, ETH, SAT).   |
+-----------------------------------------------------------------------------------+
```

- **Lifecycle Honesty:** Records are marked explicitly as `Planned`, `Approved`, `Committed`, `Disbursed`, or `Settled`. Unverified estimates are never represented as completed payments.
- **Auditable Export:** One-click generation of machine-readable JSON/CBOR audit bundles for independent review.

---

## 6. Project Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-PROJ-001` | **Interactive Kanban Board** | Multi-column Kanban board supporting drag-and-drop card movement across 5 standard lifecycle stages. | `kanban.rs`, `task_list.rs` |
| `POET-PROJ-002` | **Task Creation & Editing Modal** | Rich modal dialog for creating/editing tasks with title, description, priority, assignee DID, and due date. | `kanban.rs`, `project_views` |
| `POET-PROJ-003` | **Task Dependency Graph** | Graph view linking tasks with blocking dependencies, cycle detection, and blocker badges. | `gantt.rs`, `timeline.rs` |
| `POET-PROJ-004` | **Gantt & Roadmap Timeline** | Interactive horizontal Gantt ribbon mapping milestones and task schedules with zoom controls. | `roadmap.rs`, `gantt.rs` |
| `POET-PROJ-005` | **Fiduciary Budget Ledgers** | Separate Plan, Actual, Funding, Royalty, and Tax ledgers with strict lifecycle status tracking. | `budget_workspace.rs`, `budget_model.rs` |
| `POET-PROJ-006` | **Variance & Burn Calculations** | Real-time calculation of budget variance and runway using fixed 6-decimal arithmetic. | `budget_model.rs` |
| `POET-PROJ-007` | **Risk & Mitigation Tracking** | Risk registry mapping identified risks to severity scores, mitigation strategies, and assigned owners. | `risk.rs` |
| `POET-PROJ-008` | **Deliverable Acceptance Flow** | Formal review step with cryptographic evidence attachments before marking tasks `Done`. | `deliverable.rs`, `review.rs` |
| `POET-PROJ-009` | **Economic Audit Bundle Export** | Export cryptographically verifiable JSON audit bundle containing full financial ledger history. | `budget_workspace.rs` |
