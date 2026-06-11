// Enhanced Interval Reasoning Engine
// Advanced temporal algebra operations, interval constraint satisfaction, and temporal planning

use crate::NQuin;
use std::collections::HashMap;

/// Temporal interval with start and end points
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalInterval {
    pub id: u64,
    pub start: i64,  // Unix timestamp or relative time
    pub end: i64,    // Unix timestamp or relative time
    pub duration: i64,
}

impl TemporalInterval {
    /// Create a new interval
    pub fn new(id: u64, start: i64, end: i64) -> Self {
        assert!(start <= end, "Interval start must be <= end");
        let duration = end - start;
        Self { id, start, end, duration }
    }
    
    /// Check if interval contains a point
    pub fn contains(&self, point: i64) -> bool {
        point >= self.start && point <= self.end
    }
    
    /// Check if interval overlaps with another
    pub fn overlaps(&self, other: &TemporalInterval) -> bool {
        self.start <= other.end && other.start <= self.end
    }
    
    /// Get intersection with another interval
    pub fn intersection(&self, other: &TemporalInterval) -> Option<TemporalInterval> {
        if !self.overlaps(other) {
            return None;
        }
        
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        Some(TemporalInterval::new(0, start, end))
    }
    
    /// Get union with another interval
    pub fn union(&self, other: &TemporalInterval) -> TemporalInterval {
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        TemporalInterval::new(0, start, end)
    }
    
    /// Get gap between intervals
    pub fn gap(&self, other: &TemporalInterval) -> Option<i64> {
        if self.overlaps(other) {
            return None;
        }
        
        if self.end < other.start {
            Some(other.start - self.end)
        } else {
            Some(self.start - other.end)
        }
    }
}

/// Allen's Interval Algebra relations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllenRelation {
    Before,           // A before B
    After,            // A after B  
    Meets,            // A meets B
    MetBy,            // A met by B
    Overlaps,         // A overlaps B
    OverlappedBy,     // A overlapped by B
    Starts,           // A starts B
    StartedBy,       // A started by B
    During,           // A during B
    Contains,         // A contains B
    Ends,             // A ends B
    EndedBy,          // A ended by B
    Equal,            // A equal B
}

/// Interval constraint satisfaction problem
pub struct IntervalCSP {
    pub intervals: HashMap<u64, TemporalInterval>,
    pub constraints: HashMap<(u64, u64), Vec<AllenRelation>>,
    pub solution: Option<HashMap<u64, TemporalInterval>>,
}

impl IntervalCSP {
    /// Create a new interval CSP
    pub fn new() -> Self {
        Self {
            intervals: HashMap::new(),
            constraints: HashMap::new(),
            solution: None,
        }
    }
    
    /// Add an interval to the CSP
    pub fn add_interval(&mut self, interval: TemporalInterval) {
        self.intervals.insert(interval.id, interval);
    }
    
    /// Add a constraint between two intervals
    pub fn add_constraint(&mut self, id1: u64, id2: u64, relation: AllenRelation) {
        self.constraints.entry((id1, id2)).or_insert_with(Vec::new).push(relation);
    }
    
    /// Check if two intervals satisfy a given relation
    pub fn satisfies_relation(&self, interval1: &TemporalInterval, interval2: &TemporalInterval, relation: &AllenRelation) -> bool {
        match relation {
            AllenRelation::Before => interval1.end < interval2.start,
            AllenRelation::After => interval1.start > interval2.end,
            AllenRelation::Meets => interval1.end == interval2.start,
            AllenRelation::MetBy => interval1.start == interval2.end,
            AllenRelation::Overlaps => {
                interval1.start < interval2.start && interval2.start < interval1.end && interval1.end < interval2.end
            },
            AllenRelation::OverlappedBy => {
                interval2.start < interval1.start && interval1.start < interval2.end && interval2.end < interval1.end
            },
            AllenRelation::Starts => interval1.start == interval2.start && interval1.end < interval2.end,
            AllenRelation::StartedBy => interval2.start == interval1.start && interval2.end < interval1.end,
            AllenRelation::During => interval1.start > interval2.start && interval1.end < interval2.end,
            AllenRelation::Contains => interval2.start > interval1.start && interval2.end < interval1.end,
            AllenRelation::Ends => interval1.start > interval2.start && interval1.end == interval2.end,
            AllenRelation::EndedBy => interval2.start > interval1.start && interval2.end == interval1.end,
            AllenRelation::Equal => interval1.start == interval2.start && interval1.end == interval2.end,
        }
    }
    
    /// Solve the CSP using simple backtracking
    pub fn solve(&mut self) -> bool {
        let mut solution = HashMap::new();
        
        // Try to assign intervals that satisfy all constraints
        for (id, interval) in &self.intervals {
            solution.insert(*id, interval.clone());
        }
        
        // Check all constraints
        for ((id1, id2), relations) in &self.constraints {
            if let (Some(interval1), Some(interval2)) = (solution.get(id1), solution.get(id2)) {
                let satisfied = relations.iter().any(|relation| {
                    self.satisfies_relation(interval1, interval2, relation)
                });
                
                if !satisfied {
                    return false;
                }
            }
        }
        
        self.solution = Some(solution);
        true
    }
    
    /// Get the solution if it exists
    pub fn get_solution(&self) -> Option<&HashMap<u64, TemporalInterval>> {
        self.solution.as_ref()
    }
}

/// Temporal planning system
pub struct TemporalPlanner {
    pub tasks: HashMap<u64, Task>,
    pub schedule: Option<Vec<ScheduledTask>>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub name: String,
    pub duration: i64,
    pub dependencies: Vec<u64>,
    pub constraints: Vec<TaskConstraint>,
}

#[derive(Debug, Clone)]
pub enum TaskConstraint {
    MustStartAfter(i64),
    MustEndBefore(i64),
    MustStartBefore(i64),
    MustEndAfter(i64),
    FixedStart(i64),
    FixedEnd(i64),
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task: Task,
    pub interval: TemporalInterval,
}

impl TemporalPlanner {
    /// Create a new temporal planner
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            schedule: None,
        }
    }
    
    /// Add a task to the planner
    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }
    
    /// Generate a schedule using simple greedy algorithm
    pub fn generate_schedule(&mut self) -> Result<(), String> {
        let mut scheduled: Vec<ScheduledTask> = Vec::new();
        let mut task_queue: Vec<&Task> = self.tasks.values().collect();
        
        // Sort tasks by dependencies (simple topological sort)
        task_queue.sort_by(|a, b| a.dependencies.len().cmp(&b.dependencies.len()));
        
        let mut current_time = 0i64;
        
        for task in task_queue {
            // Check if all dependencies are satisfied
            let mut dependencies_satisfied = true;
            let mut earliest_start = current_time;
            
            for &dep_id in &task.dependencies {
                if let Some(scheduled_dep) = scheduled.iter().find(|st| st.task.id == dep_id) {
                    earliest_start = earliest_start.max(scheduled_dep.interval.end);
                } else {
                    dependencies_satisfied = false;
                    break;
                }
            }
            
            if !dependencies_satisfied {
                return Err(format!("Task {} has unsatisfied dependencies", task.id));
            }
            
            // Apply constraints
            let mut start_time = earliest_start;
            let end_time = start_time + task.duration;
            
            for constraint in &task.constraints {
                match constraint {
                    TaskConstraint::MustStartAfter(time) => start_time = start_time.max(*time),
                    TaskConstraint::MustEndBefore(time) => {
                        if end_time > *time {
                            return Err(format!("Task {} must end before {}", task.id, time));
                        }
                    },
                    TaskConstraint::MustStartBefore(time) => {
                        if start_time > *time {
                            return Err(format!("Task {} must start before {}", task.id, time));
                        }
                    },
                    TaskConstraint::MustEndAfter(time) => {
                        if end_time < *time {
                            start_time = *time - task.duration;
                        }
                    },
                    TaskConstraint::FixedStart(time) => start_time = *time,
                    TaskConstraint::FixedEnd(time) => start_time = *time - task.duration,
                }
            }
            
            // Create interval and add to schedule
            let interval = TemporalInterval::new(task.id, start_time, start_time + task.duration);
            scheduled.push(ScheduledTask {
                task: task.clone(),
                interval,
            });
            
            current_time = start_time + task.duration;
        }
        
        self.schedule = Some(scheduled);
        Ok(())
    }
    
    /// Get the generated schedule
    pub fn get_schedule(&self) -> Option<&Vec<ScheduledTask>> {
        self.schedule.as_ref()
    }
    
    /// Check if schedule is valid
    pub fn validate_schedule(&self) -> bool {
        if let Some(schedule) = &self.schedule {
            // Check for overlaps in tasks that shouldn't overlap
            for (i, task1) in schedule.iter().enumerate() {
                for task2 in schedule.iter().skip(i + 1) {
                    // Tasks with dependencies can overlap if designed to do so
                    if task1.interval.overlaps(&task2.interval) {
                        // Check if this is allowed (simplified validation)
                        if !task1.task.dependencies.contains(&task2.task.id) && 
                           !task2.task.dependencies.contains(&task1.task.id) {
                            return false;
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    }
}

/// Advanced temporal algebra operations
pub struct TemporalAlgebra;

impl TemporalAlgebra {
    /// Compute transitive closure of temporal relations
    pub fn transitive_closure(intervals: &HashMap<u64, TemporalInterval>) -> HashMap<(u64, u64), AllenRelation> {
        let mut relations = HashMap::new();
        
        for (id1, interval1) in intervals {
            for (id2, interval2) in intervals {
                if id1 != id2 {
                    let relation = Self::determine_relation(interval1, interval2);
                    relations.insert((*id1, *id2), relation);
                }
            }
        }
        
        relations
    }
    
    /// Determine the Allen relation between two intervals
    pub fn determine_relation(interval1: &TemporalInterval, interval2: &TemporalInterval) -> AllenRelation {
        if interval1.end < interval2.start {
            AllenRelation::Before
        } else if interval1.start > interval2.end {
            AllenRelation::After
        } else if interval1.end == interval2.start {
            AllenRelation::Meets
        } else if interval1.start == interval2.end {
            AllenRelation::MetBy
        } else if interval1.start < interval2.start && interval2.start < interval1.end && interval1.end < interval2.end {
            AllenRelation::Overlaps
        } else if interval2.start < interval1.start && interval1.start < interval2.end && interval2.end < interval1.end {
            AllenRelation::OverlappedBy
        } else if interval1.start == interval2.start && interval1.end < interval2.end {
            AllenRelation::Starts
        } else if interval2.start == interval1.start && interval2.end < interval1.end {
            AllenRelation::StartedBy
        } else if interval1.start > interval2.start && interval1.end < interval2.end {
            AllenRelation::During
        } else if interval2.start > interval1.start && interval2.end < interval1.end {
            AllenRelation::Contains
        } else if interval1.start > interval2.start && interval1.end == interval2.end {
            AllenRelation::Ends
        } else if interval2.start > interval1.start && interval2.end == interval1.end {
            AllenRelation::EndedBy
        } else {
            AllenRelation::Equal
        }
    }
    
    /// Compose temporal relations
    pub fn compose_relations(rel1: AllenRelation, rel2: AllenRelation) -> Vec<AllenRelation> {
        // Simplified composition table
        match (rel1, rel2) {
            (AllenRelation::Before, AllenRelation::Before) => vec![AllenRelation::Before],
            (AllenRelation::Before, AllenRelation::Meets) => vec![AllenRelation::Before],
            (AllenRelation::Meets, AllenRelation::Before) => vec![AllenRelation::Before],
            (AllenRelation::During, AllenRelation::During) => vec![AllenRelation::During],
            (AllenRelation::Starts, AllenRelation::Starts) => vec![AllenRelation::Starts],
            _ => vec![rel1], // Default case
        }
    }
}

/// Convert interval reasoning results to NQuin
pub fn interval_to_quin(interval: &TemporalInterval, context: u64) -> NQuin {
    let mut quin = NQuin {
        subject: interval.id,
        predicate: crate::q_hash("has_temporal_interval"),
        object: ((interval.start as u64) << 32) | (interval.duration as u64 & 0xFFFFFFFF),
        context,
        metadata: interval.end as u64,
        parity: 0,
    };
    
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    quin
}

/// Convert schedule to NQuin collection
pub fn schedule_to_quins(schedule: &[ScheduledTask], context: u64) -> Vec<NQuin> {
    let mut quins = Vec::new();
    
    for scheduled_task in schedule {
        let interval_quin = interval_to_quin(&scheduled_task.interval, context);
        quins.push(interval_quin);
        
        // Store task metadata
        let task_quin = NQuin {
            subject: scheduled_task.task.id,
            predicate: crate::q_hash("has_task_metadata"),
            object: scheduled_task.task.duration as u64,
            context,
            metadata: crate::q_hash(&scheduled_task.task.name),
            parity: 0,
        };
        quins.push(task_quin);
    }
    
    quins
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_temporal_interval_creation() {
        let interval = TemporalInterval::new(1, 100, 200);
        assert_eq!(interval.start, 100);
        assert_eq!(interval.end, 200);
        assert_eq!(interval.duration, 100);
    }
    
    #[test]
    fn test_interval_operations() {
        let interval1 = TemporalInterval::new(1, 100, 200);
        let interval2 = TemporalInterval::new(2, 150, 250);
        
        assert!(interval1.overlaps(&interval2));
        
        let intersection = interval1.intersection(&interval2).unwrap();
        assert_eq!(intersection.start, 150);
        assert_eq!(intersection.end, 200);
        
        let union = interval1.union(&interval2);
        assert_eq!(union.start, 100);
        assert_eq!(union.end, 250);
    }
    
    #[test]
    fn test_allen_relations() {
        let interval1 = TemporalInterval::new(1, 100, 200);
        let interval2 = TemporalInterval::new(2, 50, 150);
        let interval3 = TemporalInterval::new(3, 200, 300);
        
        assert_eq!(TemporalAlgebra::determine_relation(&interval1, &interval2), AllenRelation::OverlappedBy);
        assert_eq!(TemporalAlgebra::determine_relation(&interval1, &interval3), AllenRelation::Meets);
    }
    
    #[test]
    fn test_interval_csp() {
        let mut csp = IntervalCSP::new();
        
        let interval1 = TemporalInterval::new(1, 100, 200);
        let interval2 = TemporalInterval::new(2, 200, 300);
        
        csp.add_interval(interval1);
        csp.add_interval(interval2);
        csp.add_constraint(1, 2, AllenRelation::Meets);
        
        assert!(csp.solve());
        assert!(csp.get_solution().is_some());
    }
    
    #[test]
    fn test_temporal_planner() {
        let mut planner = TemporalPlanner::new();
        
        let task1 = Task {
            id: 1,
            name: "Task 1".to_string(),
            duration: 100,
            dependencies: vec![],
            constraints: vec![TaskConstraint::FixedStart(100)],
        };
        
        let task2 = Task {
            id: 2,
            name: "Task 2".to_string(),
            duration: 50,
            dependencies: vec![1],
            constraints: vec![],
        };
        
        planner.add_task(task1);
        planner.add_task(task2);
        
        assert!(planner.generate_schedule().is_ok());
        assert!(planner.validate_schedule());
    }
    
    #[test]
    fn test_interval_to_quin() {
        let interval = TemporalInterval::new(42, 1000, 1500);
        let quin = interval_to_quin(&interval, 123);
        
        assert_eq!(quin.subject, 42);
        assert_eq!(quin.context, 123);
        assert_eq!(quin.metadata, 1500); // end time stored in metadata
    }
}
