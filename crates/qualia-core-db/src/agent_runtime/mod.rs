//! Agent runtime — planning, corpus management, and evaluation.
//!
//! This module provides the build-new agent runtime functionality needed
//! by the Poet/NLP interface:
//!
//! - `planner`: Symbolic rule-based agent task planner.
//! - `corpus`: Golden corpus loader and parser.
//! - `evaluator`: Agent output evaluation against golden corpora.

pub mod corpus;
pub mod evaluator;
pub mod orchestration;
pub mod planner;

pub use corpus::{GoldenCase, GoldenCorpus};
pub use evaluator::{
    compute_metrics, eval_case, evaluate_corpus, score_case, CaseResult, EvalMetrics, MatchMethod,
};
pub use orchestration::{
    assign_agents, create_blackboard, create_phase_leaser, create_session, execute_session,
    plan_session, session_summary, session_to_pipeline, AgentRole, AgentRoster,
    NodeExecutionResult, OrchestrationSession, OrchestrationStatus, RosterAgent,
};
pub use planner::{plan_task, AgentPlan, PlannedStep, TaskVerb};
