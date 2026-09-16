//! Cold, bounded evaluation API for prompt precision (Prompt Precision P5).

pub mod corpus;
pub mod report;
pub mod scoring;

pub use corpus::{SplitType, TaskItem, TaskManifest};
pub use report::{EvaluationReport, PairedComparison};
pub use scoring::{evaluate_task_output, TaskScore};
