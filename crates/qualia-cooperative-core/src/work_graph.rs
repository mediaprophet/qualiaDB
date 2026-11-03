//! Enriched WBS / PDM Work Graph: Dependencies (`FS`/`SS`/`FF`/`SF`), `Lag`, `TaskRef`,
//! `TaskConstraint`, `ThreePointEstimate`, `TimeEntry` (producing Stage 2 `ActivityAndResources`
//! records), and post-merge `SemanticConflict` detection.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contribution_lifecycle::ActivityAndResources;
use crate::record::{q_hash_str, InstantBridge, NQuin, SensitivityClass};
use crate::work_item::WorkItem;

/// Precedence Diagramming Method (PDM) relationship kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DepKind {
    /// Finish-to-Start (default).
    #[default]
    FS,
    /// Start-to-Start.
    SS,
    /// Finish-to-Finish.
    FF,
    /// Start-to-Finish.
    SF,
}

/// Lead/lag offset on a PDM dependency link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Lag {
    #[default]
    None,
    WorkingMinutes(i64),
    ElapsedMinutes(i64),
    Percent(i32),
}

/// Reference to a predecessor or successor task — either within the current project
/// or across projects (`External`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", content = "ref", rename_all = "snake_case")]
pub enum TaskRef {
    Local(String),
    External {
        project_id: String,
        work_item_id: String,
    },
}

impl TaskRef {
    /// Returns the local `work_item_id` when `Local`, or the target `work_item_id` when `External`.
    pub fn work_item_id(&self) -> &str {
        match self {
            Self::Local(id) => id,
            Self::External { work_item_id, .. } => work_item_id,
        }
    }
}

/// An immutable PDM dependency edge between two tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDependency {
    pub id: String,
    pub project_id: String,
    pub predecessor: TaskRef,
    pub successor: TaskRef,
    pub kind: DepKind,
    pub lag: Lag,
    pub created_at_unix: u32,
}

impl WorkDependency {
    /// Construct a new local `FS` dependency with optional lag.
    pub fn new_local(
        project_id: impl Into<String>,
        predecessor_id: impl Into<String>,
        successor_id: impl Into<String>,
        kind: DepKind,
        lag: Lag,
        created_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            predecessor: TaskRef::Local(predecessor_id.into()),
            successor: TaskRef::Local(successor_id.into()),
            kind,
            lag,
            created_at_unix,
        }
    }

    /// Zero-heap compilation into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        if out.is_empty() {
            return 0;
        }
        let s = q_hash_str(self.predecessor.work_item_id());
        let p = q_hash_str("https://vocab.Burst.net.au/coop#precedesTask");
        let o = q_hash_str(self.successor.work_item_id());
        let ctx = q_hash_str(&self.project_id);
        let meta = SensitivityClass::Restricted.to_metadata_mask() | (self.created_at_unix as u64);
        out[0] = NQuin::new(s, p, o, ctx, meta);
        1
    }
}

/// Replay-safe, order-independent union of `WorkDependency` records by stable `id`
/// (also deduplicating identical `(predecessor, successor, kind, lag)` tuples).
pub fn merge_dependencies(
    existing: &[WorkDependency],
    incoming: &[WorkDependency],
) -> Vec<WorkDependency> {
    let mut merged: Vec<WorkDependency> = Vec::with_capacity(existing.len() + incoming.len());
    for dep in existing.iter().chain(incoming.iter()) {
        let duplicate = merged.iter().any(|d| {
            d.id == dep.id
                || (d.project_id == dep.project_id
                    && d.predecessor == dep.predecessor
                    && d.successor == dep.successor
                    && d.kind == dep.kind
                    && d.lag == dep.lag)
        });
        if !duplicate {
            merged.push(dep.clone());
        }
    }
    merged.sort_by(|a, b| {
        a.created_at_unix
            .cmp(&b.created_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

/// Scheduling constraint type for CPM/PDM planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskConstraint {
    /// As Soon As Possible (default).
    #[default]
    Asap,
    /// As Late As Possible.
    Alap,
    /// Start No Earlier Than.
    Snet,
    /// Start No Later Than.
    Snlt,
    /// Finish No Earlier Than.
    Fnet,
    /// Finish No Later Than.
    Fnlt,
    /// Must Start On.
    Mso,
    /// Must Finish On.
    Mfo,
}

impl TaskConstraint {
    /// True when the constraint requires a companion `constraint_date_unix` timestamp.
    pub const fn requires_anchor_date(&self) -> bool {
        matches!(
            self,
            Self::Snet | Self::Snlt | Self::Fnet | Self::Fnlt | Self::Mso | Self::Mfo
        )
    }
}

/// Three-point PERT duration/effort estimate in integer minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreePointEstimate {
    pub optimistic_m: u32,
    pub most_likely_m: u32,
    pub pessimistic_m: u32,
}

impl ThreePointEstimate {
    /// Construct a new three-point estimate.
    pub const fn new(optimistic_m: u32, most_likely_m: u32, pessimistic_m: u32) -> Self {
        Self {
            optimistic_m,
            most_likely_m,
            pessimistic_m,
        }
    }

    /// True if `optimistic_m <= most_likely_m <= pessimistic_m`.
    pub const fn is_valid(&self) -> bool {
        self.optimistic_m <= self.most_likely_m && self.most_likely_m <= self.pessimistic_m
    }

    /// Expected PERT duration `(O + 4M + P) / 6` in integer minutes.
    pub const fn pert_expected_minutes(&self) -> u32 {
        let weighted = (self.optimistic_m as u64)
            + 4 * (self.most_likely_m as u64)
            + (self.pessimistic_m as u64);
        ((weighted + 3) / 6) as u32
    }

    /// Standard deviation `(P - O) / 6` in integer minutes.
    pub const fn pert_std_dev_minutes(&self) -> u32 {
        if self.pessimistic_m <= self.optimistic_m {
            0
        } else {
            (self.pessimistic_m - self.optimistic_m) / 6
        }
    }
}

/// Logged time/expense against a work item. Produces a Stage 2 `ActivityAndResources` record
/// (and NEVER directly mints a payable `LifecycleObligation` without the 9-stage lifecycle).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub project_id: String,
    pub work_item_id: String,
    pub contributor_did: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<String>,
    pub minutes: u32,
    pub reimbursable_expense_cents: i64,
    pub note: String,
    pub logged_at_unix: u32,
}

impl TimeEntry {
    /// Convert this `TimeEntry` into a Stage 2 `ActivityAndResources` record.
    pub fn to_activity_record(&self) -> ActivityAndResources {
        ActivityAndResources {
            id: format!("act:{}", self.id),
            project_id: self.project_id.clone(),
            contributor_did: self.contributor_did.clone(),
            agreement_id: self.agreement_id.clone(),
            work_item_id: Some(self.work_item_id.clone()),
            effort_minutes: self.minutes,
            reimbursable_expense_cents: self.reimbursable_expense_cents,
            capital_advanced_cents: 0,
            occurred_at_unix: self.logged_at_unix,
            occurred_at_instant: Some(InstantBridge::from_coarse(self.logged_at_unix)),
        }
    }
}

/// Post-merge semantic conflicts detected across a project's WBS/PDM graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticConflict {
    DependencyCycle {
        cycle_task_ids: Vec<String>,
    },
    ConstraintViolation {
        work_item_id: String,
        constraint: TaskConstraint,
        reason: String,
    },
    Overallocation {
        assignee_did: String,
        total_assigned_minutes: u32,
        capacity_minutes: u32,
    },
}

/// Detect post-merge semantic conflicts (`DependencyCycle`, `ConstraintViolation`, `Overallocation`)
/// across a set of work items and dependencies.
pub fn detect_semantic_conflicts(
    items: &[WorkItem],
    dependencies: &[WorkDependency],
    capacity_minutes_per_assignee: u32,
) -> Vec<SemanticConflict> {
    let mut conflicts = Vec::new();

    // 1. Cycle detection over local dependencies using Kahn's topological in-degree elimination.
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_deg: HashMap<&str, usize> = HashMap::new();
    let mut nodes: HashSet<&str> = HashSet::new();
    for dep in dependencies {
        if let (TaskRef::Local(u), TaskRef::Local(v)) = (&dep.predecessor, &dep.successor) {
            nodes.insert(u.as_str());
            nodes.insert(v.as_str());
            adj.entry(u.as_str()).or_default().push(v.as_str());
            *in_deg.entry(v.as_str()).or_insert(0) += 1;
            in_deg.entry(u.as_str()).or_insert(0);
        }
    }
    let mut queue: Vec<&str> = in_deg
        .iter()
        .filter_map(|(&node, &deg)| if deg == 0 { Some(node) } else { None })
        .collect();
    let mut visited = 0usize;
    while let Some(u) = queue.pop() {
        visited += 1;
        if let Some(neighbors) = adj.get(u) {
            for &v in neighbors {
                if let Some(d) = in_deg.get_mut(v) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push(v);
                    }
                }
            }
        }
    }
    if visited < nodes.len() {
        let mut cycle_nodes: Vec<String> = in_deg
            .into_iter()
            .filter_map(|(k, deg)| if deg > 0 { Some(k.to_string()) } else { None })
            .collect();
        cycle_nodes.sort();
        conflicts.push(SemanticConflict::DependencyCycle {
            cycle_task_ids: cycle_nodes,
        });
    }

    // 2. ConstraintViolation detection (missing anchor date on date-anchored constraint, or invalid PERT).
    for item in items {
        if item.constraint.requires_anchor_date() && item.constraint_date_unix.is_none() {
            conflicts.push(SemanticConflict::ConstraintViolation {
                work_item_id: item.id.clone(),
                constraint: item.constraint,
                reason: "date-anchored constraint is missing constraint_date_unix".to_string(),
            });
        } else if let Some(anchor) = item.constraint_date_unix {
            if matches!(item.constraint, TaskConstraint::Snlt | TaskConstraint::Fnlt | TaskConstraint::Mfo)
                && anchor < item.created_at_unix
            {
                conflicts.push(SemanticConflict::ConstraintViolation {
                    work_item_id: item.id.clone(),
                    constraint: item.constraint,
                    reason: format!(
                        "constraint anchor ({anchor}) precedes item creation ({})",
                        item.created_at_unix
                    ),
                });
            }
        }
        if let Some(est) = item.three_point_estimate {
            if !est.is_valid() {
                conflicts.push(SemanticConflict::ConstraintViolation {
                    work_item_id: item.id.clone(),
                    constraint: item.constraint,
                    reason: "three-point estimate violates optimistic <= most_likely <= pessimistic"
                        .to_string(),
                });
            }
        }
    }

    // 3. Overallocation detection per assignee_did.
    if capacity_minutes_per_assignee > 0 {
        let mut by_assignee: HashMap<&str, u32> = HashMap::new();
        for item in items {
            if let Some(did) = item.assignee_did.as_deref() {
                let mins = item
                    .estimate_minutes
                    .or_else(|| item.three_point_estimate.map(|e| e.pert_expected_minutes()))
                    .unwrap_or(0);
                *by_assignee.entry(did).or_insert(0) =
                    by_assignee.get(did).copied().unwrap_or(0).saturating_add(mins);
            }
        }
        let mut assignees: Vec<(&str, u32)> = by_assignee.into_iter().collect();
        assignees.sort_by(|a, b| a.0.cmp(b.0));
        for (did, total_m) in assignees {
            if total_m > capacity_minutes_per_assignee {
                conflicts.push(SemanticConflict::Overallocation {
                    assignee_did: did.to_string(),
                    total_assigned_minutes: total_m,
                    capacity_minutes: capacity_minutes_per_assignee,
                });
            }
        }
    }

    conflicts
}
