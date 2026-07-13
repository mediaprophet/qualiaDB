//! Task scheduling: policy-driven task queue and execution history.

use super::*;
use std::time::Instant;

/// Task scheduler
pub struct TaskScheduler {
    scheduling_policy: SchedulingPolicy,
    task_queue: TaskQueue,
    execution_history: Vec<TaskExecutionRecord>,
}

/// Task queue
pub struct TaskQueue {
    pending_tasks: Vec<Task>,
    running_tasks: Vec<Task>,
    completed_tasks: Vec<Task>,
}

impl TaskScheduler {
    /// Create new task scheduler
    pub fn new() -> Self {
        Self {
            scheduling_policy: SchedulingPolicy::Adaptive,
            task_queue: TaskQueue::new(),
            execution_history: Vec::new(),
        }
    }

    /// Submit task
    pub fn submit_task(&mut self, task: Task) -> Result<(), AmbientError> {
        self.task_queue.pending_tasks.push(task);
        // Sort pending tasks according to the scheduling policy.
        self.sort_pending();
        Ok(())
    }

    /// Sort pending tasks according to the current scheduling policy.
    fn sort_pending(&mut self) {
        match self.scheduling_policy {
            SchedulingPolicy::Fifo => {
                // FIFO: keep insertion order (no sort needed).
            }
            SchedulingPolicy::Priority => {
                // Priority: highest priority first.
                self.task_queue
                    .pending_tasks
                    .sort_by(|a, b| b.priority.cmp(&a.priority));
            }
            SchedulingPolicy::ShortestJobFirst => {
                // SJF: shortest estimated duration first.
                self.task_queue
                    .pending_tasks
                    .sort_by(|a, b| a.estimated_duration.cmp(&b.estimated_duration));
            }
            SchedulingPolicy::Deadline => {
                // Deadline: earliest deadline first (tasks without deadlines go last).
                self.task_queue
                    .pending_tasks
                    .sort_by(|a, b| match (a.deadline, b.deadline) {
                        (Some(da), Some(db)) => da.cmp(&db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    });
            }
            SchedulingPolicy::Adaptive => {
                // Adaptive: priority first, then shortest job as tiebreaker.
                self.task_queue.pending_tasks.sort_by(|a, b| {
                    b.priority
                        .cmp(&a.priority)
                        .then_with(|| a.estimated_duration.cmp(&b.estimated_duration))
                });
            }
        }
    }

    /// Get pending tasks
    pub fn get_pending_tasks(&self) -> Vec<Task> {
        self.task_queue.pending_tasks.clone()
    }

    pub fn get_pending_tasks_into(&self, out: &mut [TaskHandle]) -> Result<usize, AmbientError> {
        if out.len() < self.task_queue.pending_tasks.len() {
            return Err(AmbientError::InsufficientResources(
                "task output buffer full".to_string(),
            ));
        }

        for (index, task) in self.task_queue.pending_tasks.iter().enumerate() {
            out[index] = TaskHandle {
                task_id_hash: crate::q_hash(&task.task_id),
                task_type: task.task_type.clone(),
                priority: task.priority.clone(),
                compute_units: task.resource_requirements.compute_units,
                memory: task.resource_requirements.memory,
            };
        }

        Ok(self.task_queue.pending_tasks.len())
    }

    /// Dispatch the next pending task to a device, moving it to running.
    /// Returns the dispatched task, or `None` if no tasks are pending.
    pub fn dispatch_next(&mut self) -> Option<Task> {
        let task = self.task_queue.pending_tasks.pop()?;
        self.task_queue.running_tasks.push(task.clone());
        Some(task)
    }

    /// Mark a running task as completed, recording it in execution history.
    pub fn complete_task(
        &mut self,
        task_id: &str,
        device_id: &str,
        success: bool,
        usage: ResourceUsage,
    ) {
        // Remove from running tasks.
        if let Some(pos) = self
            .task_queue
            .running_tasks
            .iter()
            .position(|t| t.task_id == task_id)
        {
            let task = self.task_queue.running_tasks.remove(pos);
            self.task_queue.completed_tasks.push(task.clone());

            // Record in execution history.
            self.execution_history.push(TaskExecutionRecord {
                task_id: task.task_id.clone(),
                device_id: device_id.to_string(),
                start_time: Instant::now() - task.estimated_duration,
                end_time: Instant::now(),
                actual_duration: task.estimated_duration,
                success,
                resource_usage: usage,
            });

            // Trim history.
            if self.execution_history.len() > 500 {
                let drop = self.execution_history.len() - 500;
                self.execution_history.drain(0..drop);
            }
            // Trim completed tasks.
            if self.task_queue.completed_tasks.len() > 200 {
                let drop = self.task_queue.completed_tasks.len() - 200;
                self.task_queue.completed_tasks.drain(0..drop);
            }
        }
    }

    /// Get the number of currently running tasks.
    pub fn running_count(&self) -> usize {
        self.task_queue.running_tasks.len()
    }

    /// Get the number of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.task_queue.completed_tasks.len()
    }

    /// Get recent execution history records.
    pub fn recent_history(&self, n: usize) -> &[TaskExecutionRecord] {
        let start = self.execution_history.len().saturating_sub(n);
        &self.execution_history[start..]
    }

    /// Set the scheduling policy.
    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.scheduling_policy = policy;
        self.sort_pending();
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            pending_tasks: Vec::new(),
            running_tasks: Vec::new(),
            completed_tasks: Vec::new(),
        }
    }
}
