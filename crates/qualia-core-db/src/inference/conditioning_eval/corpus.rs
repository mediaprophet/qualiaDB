//! Versioned task manifests and split enforcement for held-out evaluation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitType {
    Train,
    Dev,
    HeldOutTest,
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub task_id: String,
    pub split: SplitType,
    pub prompt: String,
    pub expected_outcome: String,
    pub domain: String,
}

#[derive(Debug, Clone)]
pub struct TaskManifest {
    pub manifest_id: String,
    pub version: u32,
    pub tasks: Vec<TaskItem>,
}

impl TaskManifest {
    pub fn new(manifest_id: impl Into<String>, version: u32) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            version,
            tasks: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: TaskItem) {
        self.tasks.push(task);
    }

    pub fn held_out_tasks(&self) -> impl Iterator<Item = &TaskItem> {
        self.tasks.iter().filter(|t| t.split == SplitType::HeldOutTest)
    }
}
