//! Symbolic agent planner.
//!
//! Given a task description and a set of available capabilities, produces a
//! DAG plan by matching task verbs to capability names. This is a
//! deterministic, rule-based planner — no LLM inference is used.
//!
//! The planner recognises a fixed set of task verbs (research, analyse,
//! synthesise, verify, report) and maps them to capability patterns. Each
//! planned node carries an effect label and a budget derived from the task
//! complexity heuristic.

use std::collections::BTreeMap;

/// A single planned step in the agent plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedStep {
    /// Node ID within the plan DAG.
    pub id: u32,
    /// Human-readable step name.
    pub name: String,
    /// Capability invoke-ID this step will call.
    pub capability: String,
    /// Effect class: "pure", "hot", "cold", "async", or "external".
    pub effect: &'static str,
    /// Token/cycle budget for this step.
    pub budget: u32,
    /// Input channel names this step consumes.
    pub inputs: Vec<String>,
    /// Output channel names this step produces.
    pub outputs: Vec<String>,
    /// IDs of steps this step depends on.
    pub depends_on: Vec<u32>,
}

/// A complete agent plan — a topologically-ordered sequence of steps.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentPlan {
    pub steps: Vec<PlannedStep>,
    pub task: String,
    pub total_budget: u32,
}

/// Task verb recognised by the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskVerb {
    Research,
    Analyse,
    Synthesise,
    Verify,
    Report,
}

impl TaskVerb {
    /// Classify a task description's primary verb.
    pub fn classify(task: &str) -> Option<TaskVerb> {
        let lower = task.to_ascii_lowercase();
        if lower.contains("research") || lower.contains("investigate") || lower.contains("explore")
        {
            Some(TaskVerb::Research)
        } else if lower.contains("analy") || lower.contains("exam") || lower.contains("inspect") {
            Some(TaskVerb::Analyse)
        } else if lower.contains("synth") || lower.contains("compos") || lower.contains("creat") {
            Some(TaskVerb::Synthesise)
        } else if lower.contains("verif") || lower.contains("valid") || lower.contains("check") {
            Some(TaskVerb::Verify)
        } else if lower.contains("report") || lower.contains("summary") || lower.contains("output")
        {
            Some(TaskVerb::Report)
        } else {
            None
        }
    }
}

/// Match a capability name from the available set for a given verb.
fn match_capability(verb: TaskVerb, available: &[String]) -> Option<String> {
    let patterns: &[&str] = match verb {
        TaskVerb::Research => &[
            "NLP.substrate_extract",
            "NLP.graphrag_query",
            "GraphReasoning",
        ],
        TaskVerb::Analyse => &["NLP.analyze", "NLP.frame_extract", "NLP.relation_extract"],
        TaskVerb::Synthesise => &["NLP.coref_resolve", "NLP.fst_lookup"],
        TaskVerb::Verify => &[
            "Inference.verify_turn",
            "Inference.grounding",
            "Sentinel.gate",
        ],
        TaskVerb::Report => &["Asset.create", "Render.scene", "Scene.create"],
    };
    for pat in patterns {
        for cap in available {
            if cap.starts_with(*pat) || cap == *pat {
                return Some(cap.clone());
            }
        }
    }
    // Fallback: if no specific match, use the first available capability.
    available.first().cloned()
}

/// Heuristic budget based on task length and verb complexity.
fn budget_for(verb: TaskVerb, task: &str) -> u32 {
    let base = match verb {
        TaskVerb::Research => 500,
        TaskVerb::Analyse => 300,
        TaskVerb::Synthesise => 400,
        TaskVerb::Verify => 200,
        TaskVerb::Report => 150,
    };
    // Add 1 token per 10 chars of task description, capped at 200.
    let length_bonus = (task.len() / 10).min(200) as u32;
    base + length_bonus
}

/// Plan an agent task given a task description and available capabilities.
///
/// Produces a DAG of planned steps. The plan always starts with a research/
/// analysis step (if capabilities allow), followed by synthesis, verification,
/// and reporting steps as capabilities permit.
pub fn plan_task(task: &str, capabilities: &[String]) -> AgentPlan {
    let primary_verb = TaskVerb::classify(task).unwrap_or(TaskVerb::Research);
    let mut steps = Vec::new();
    let mut id = 0u32;
    let mut total_budget = 0u32;

    // Build a pipeline: research → analyse → synthesise → verify → report
    let pipeline_verbs = match primary_verb {
        TaskVerb::Research => vec![TaskVerb::Research, TaskVerb::Analyse, TaskVerb::Report],
        TaskVerb::Analyse => vec![TaskVerb::Analyse, TaskVerb::Report],
        TaskVerb::Synthesise => vec![TaskVerb::Research, TaskVerb::Synthesise, TaskVerb::Report],
        TaskVerb::Verify => vec![TaskVerb::Verify, TaskVerb::Report],
        TaskVerb::Report => vec![TaskVerb::Report],
    };

    let mut prev_output: Option<(u32, String)> = None;

    for (i, verb) in pipeline_verbs.iter().enumerate() {
        if let Some(cap) = match_capability(*verb, capabilities) {
            let step_name = format!("{verb:?}_step_{i}").to_lowercase();
            let budget = budget_for(*verb, task);
            let output_channel = format!("{step_name}_out");

            let inputs = if let Some((_, ref prev_ch)) = prev_output {
                vec![prev_ch.clone()]
            } else {
                vec![]
            };

            let depends_on = if let Some((prev_id, _)) = prev_output {
                vec![prev_id]
            } else {
                vec![]
            };

            let effect = match verb {
                TaskVerb::Research | TaskVerb::Analyse => "cold",
                TaskVerb::Synthesise => "hot",
                TaskVerb::Verify => "pure",
                TaskVerb::Report => "cold",
            };

            steps.push(PlannedStep {
                id,
                name: step_name.clone(),
                capability: cap,
                effect,
                budget,
                inputs,
                outputs: vec![output_channel.clone()],
                depends_on,
            });

            total_budget += budget;
            prev_output = Some((id, output_channel));
            id += 1;
        }
    }

    // If no steps were planned (no capabilities matched), create a minimal
    // noop step so the plan is never empty.
    if steps.is_empty() {
        steps.push(PlannedStep {
            id: 0,
            name: "noop".into(),
            capability: "noop".into(),
            effect: "pure",
            budget: 10,
            inputs: vec![],
            outputs: vec!["noop_out".into()],
            depends_on: vec![],
        });
        total_budget = 10;
    }

    AgentPlan {
        steps,
        task: task.to_string(),
        total_budget,
    }
}

/// Convert a plan to a VibeScript-compatible record value map.
pub fn plan_to_record_map(plan: &AgentPlan) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for step in &plan.steps {
        let mut rec = BTreeMap::new();
        rec.insert("capability".into(), step.capability.clone());
        rec.insert("effect".into(), step.effect.to_string());
        rec.insert("budget".into(), step.budget.to_string());
        rec.insert("inputs".into(), step.inputs.join(","));
        rec.insert("outputs".into(), step.outputs.join(","));
        rec.insert(
            "depends_on".into(),
            step.depends_on
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        map.insert(step.name.clone(), rec);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_research() {
        assert_eq!(
            TaskVerb::classify("research the topic"),
            Some(TaskVerb::Research)
        );
        assert_eq!(
            TaskVerb::classify("investigate the claim"),
            Some(TaskVerb::Research)
        );
    }

    #[test]
    fn classify_analyse() {
        assert_eq!(
            TaskVerb::classify("analyse the data"),
            Some(TaskVerb::Analyse)
        );
        assert_eq!(
            TaskVerb::classify("examine the results"),
            Some(TaskVerb::Analyse)
        );
    }

    #[test]
    fn classify_verify() {
        assert_eq!(
            TaskVerb::classify("verify the output"),
            Some(TaskVerb::Verify)
        );
        assert_eq!(
            TaskVerb::classify("validate the claim"),
            Some(TaskVerb::Verify)
        );
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(TaskVerb::classify("do something"), None);
    }

    #[test]
    fn plan_research_task() {
        let caps = vec![
            "NLP.substrate_extract".into(),
            "NLP.analyze".into(),
            "Asset.create".into(),
        ];
        let plan = plan_task("research the topic and report", &caps);
        assert!(!plan.steps.is_empty());
        assert!(plan.total_budget > 0);
        // Research task should have at least 2 steps (research + report)
        assert!(plan.steps.len() >= 2);
    }

    #[test]
    fn plan_with_matching_capabilities() {
        let caps = vec![
            "NLP.analyze".into(),
            "Inference.verify_turn".into(),
            "Asset.create".into(),
        ];
        let plan = plan_task("analyse the data and verify", &caps);
        assert!(!plan.steps.is_empty());
        // Each step should have a non-empty capability
        for step in &plan.steps {
            assert!(!step.capability.is_empty());
        }
    }

    #[test]
    fn plan_with_no_matching_capabilities() {
        let caps: Vec<String> = vec![];
        let plan = plan_task("unknown task", &caps);
        // Should produce a noop step
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].name, "noop");
    }

    #[test]
    fn plan_dag_has_topological_order() {
        let caps = vec![
            "NLP.substrate_extract".into(),
            "NLP.analyze".into(),
            "Asset.create".into(),
        ];
        let plan = plan_task("research and report", &caps);
        // Each step's depends_on should only reference earlier step IDs
        for (i, step) in plan.steps.iter().enumerate() {
            for dep in &step.depends_on {
                assert!(*dep < step.id, "step {i} depends on future step {dep}");
            }
            assert_eq!(step.id, i as u32);
        }
    }

    #[test]
    fn plan_budget_scales_with_task_length() {
        let caps = vec!["NLP.analyze".into()];
        let short = plan_task("analyse", &caps);
        let long = plan_task(&"analyse ".repeat(100), &caps);
        assert!(long.total_budget >= short.total_budget);
    }

    #[test]
    fn plan_to_record_map_roundtrip() {
        let caps = vec!["NLP.analyze".into()];
        let plan = plan_task("analyse the data", &caps);
        let map = plan_to_record_map(&plan);
        assert!(!map.is_empty());
        for (_, rec) in &map {
            assert!(rec.contains_key("capability"));
            assert!(rec.contains_key("budget"));
        }
    }
}
