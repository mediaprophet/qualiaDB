//! Orchestration invoke seams — roster agents → DAG execution → governance → blackboards.
//!
//! Exposes the `agent_runtime::orchestration` module through VibeScript invoke
//! IDs. The orchestration layer connects roster agents, the symbolic planner,
//! the DAG executor, blackboard I/O, and deontic governance into a single
//! lifecycle that can be driven from VibeScript.
//!
//! Invoke IDs exposed:
//! - `Orchestration.session_create` — create a new orchestration session.
//! - `Orchestration.session_plan` — plan the session's task.
//! - `Orchestration.session_execute` — execute the planned DAG.
//! - `Orchestration.session_status` — query session status and results.
//! - `Orchestration.roster_register` — register an agent in the roster.
//! - `Orchestration.roster_list` — list agents in the roster.
//! - `Orchestration.roster_capabilities` — list all capabilities in the roster.
//! - `Orchestration.assign_agents` — assign agents to planned steps.
//!
//! Note: sessions are tracked in a process-local registry. Each VibeScript
//! invocation creates or looks up a session by ID. This is the persistence
//! layer for orchestration state across invoke calls.

use super::super::args;
use crate::agent_runtime::orchestration::{
    assign_agents, create_session, execute_session, plan_session, session_summary, AgentRole,
    OrchestrationSession, RosterAgent,
};
use poet_vibe::{Diagnostic, Span, Value};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Process-local registry of orchestration sessions.
static SESSIONS: Mutex<Option<BTreeMap<String, OrchestrationSession>>> = Mutex::new(None);

fn with_sessions<F, R>(f: F) -> R
where
    F: FnOnce(&mut BTreeMap<String, OrchestrationSession>) -> R,
{
    let mut guard = SESSIONS
        .lock()
        .expect("orchestration sessions mutex poisoned");
    if guard.is_none() {
        *guard = Some(BTreeMap::new());
    }
    f(guard.as_mut().expect("sessions map"))
}

fn parse_role(s: &str) -> AgentRole {
    match s.to_ascii_lowercase().as_str() {
        "researcher" => AgentRole::Researcher,
        "analyst" => AgentRole::Analyst,
        "synthesiser" | "synthesizer" => AgentRole::Synthesiser,
        "verifier" => AgentRole::Verifier,
        "reporter" => AgentRole::Reporter,
        "orchestrator" => AgentRole::Orchestrator,
        _ => AgentRole::Custom,
    }
}

/// `Orchestration.session_create` — create a new orchestration session.
pub fn session_create(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Orchestration.session_create needs id"))?;
    let task = args::rec_str(args, "task")
        .ok_or_else(|| args::bad(span, "Orchestration.session_create needs task"))?;
    let session = create_session(id, task);
    with_sessions(|sessions| {
        sessions.insert(id.to_string(), session);
    });
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("status", Value::String("created".into())),
    ]))
}

/// `Orchestration.session_plan` — plan the session's task using the planner.
pub fn session_plan(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Orchestration.session_plan needs id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| args::bad(span, format!("session {id} not found")))?;
        plan_session(session).map_err(|e| args::bad(span, format!("plan_session: {e}")))?;
        let step_count = session.plan.as_ref().map(|p| p.steps.len()).unwrap_or(0);
        Ok(args::record([
            ("id", Value::String(id.to_string())),
            ("status", Value::String("planned".into())),
            ("step_count", Value::U64(step_count as u64)),
        ]))
    })
}

/// `Orchestration.session_execute` — execute the planned DAG.
pub fn session_execute(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Orchestration.session_execute needs id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| args::bad(span, format!("session {id} not found")))?;
        execute_session(session).map_err(|e| args::bad(span, format!("execute_session: {e}")))?;
        let results: Vec<Value> = session
            .results
            .iter()
            .map(|r| {
                let mut rec = BTreeMap::new();
                rec.insert("node_id".into(), Value::U64(r.node_id as u64));
                rec.insert("node_name".into(), Value::String(r.node_name.clone()));
                rec.insert("success".into(), Value::Bool(r.success));
                rec.insert(
                    "agent_id".into(),
                    match &r.agent_id {
                        Some(aid) => Value::String(aid.clone()),
                        None => Value::Null,
                    },
                );
                Value::Record(rec)
            })
            .collect();
        Ok(args::record([
            ("id", Value::String(id.to_string())),
            ("status", Value::String(session.status.as_str().into())),
            ("results", Value::List(results)),
        ]))
    })
}

/// `Orchestration.session_status` — query session status and summary.
pub fn session_status(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Orchestration.session_status needs id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get(id)
            .ok_or_else(|| args::bad(span, format!("session {id} not found")))?;
        let summary = session_summary(session);
        let mut rec = BTreeMap::new();
        for (k, v) in summary {
            rec.insert(k, Value::String(v));
        }
        Ok(Value::Record(rec))
    })
}

/// `Orchestration.roster_register` — register an agent in the session's roster.
pub fn roster_register(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let session_id = args::rec_str(args, "session_id")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_register needs session_id"))?;
    let agent_id = args::rec_str(args, "agent_id")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_register needs agent_id"))?;
    let did = args::rec_str(args, "did")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_register needs did"))?;
    let name = args::rec_str(args, "name")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_register needs name"))?;
    let role_str = args::rec_str(args, "role").unwrap_or("custom");
    let role = parse_role(role_str);
    let mut agent = RosterAgent::new(agent_id, did, name, role);
    // Add capabilities from a list.
    if let Value::Record(map) = args {
        if let Some(Value::List(caps)) = map.get("capabilities") {
            for cap in caps {
                if let Value::String(s) = cap {
                    agent.add_capability(s);
                }
            }
        }
    }
    with_sessions(|sessions| {
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| args::bad(span, format!("session {session_id} not found")))?;
        session.roster.register(agent);
        Ok(args::record([
            ("session_id", Value::String(session_id.to_string())),
            ("agent_id", Value::String(agent_id.to_string())),
            ("registered", Value::Bool(true)),
        ]))
    })
}

/// `Orchestration.roster_list` — list agents in the session's roster.
pub fn roster_list(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let session_id = args::rec_str(args, "session_id")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_list needs session_id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get(session_id)
            .ok_or_else(|| args::bad(span, format!("session {session_id} not found")))?;
        let agents: Vec<Value> = session
            .roster
            .agents
            .values()
            .map(|a| {
                let mut rec = BTreeMap::new();
                rec.insert("id".into(), Value::String(a.id.clone()));
                rec.insert("did".into(), Value::String(a.did.clone()));
                rec.insert("name".into(), Value::String(a.name.clone()));
                rec.insert("active".into(), Value::Bool(a.active));
                rec.insert(
                    "capabilities".into(),
                    Value::List(
                        a.capabilities
                            .iter()
                            .map(|c| Value::String(c.clone()))
                            .collect(),
                    ),
                );
                Value::Record(rec)
            })
            .collect();
        Ok(args::record([
            ("session_id", Value::String(session_id.to_string())),
            ("agents", Value::List(agents)),
            ("count", Value::U64(session.roster.count() as u64)),
        ]))
    })
}

/// `Orchestration.roster_capabilities` — list all capabilities in the roster.
pub fn roster_capabilities(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let session_id = args::rec_str(args, "session_id")
        .ok_or_else(|| args::bad(span, "Orchestration.roster_capabilities needs session_id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get(session_id)
            .ok_or_else(|| args::bad(span, format!("session {session_id} not found")))?;
        let caps: Vec<Value> = session
            .roster
            .all_capabilities()
            .into_iter()
            .map(Value::String)
            .collect();
        Ok(args::record([
            ("session_id", Value::String(session_id.to_string())),
            ("capabilities", Value::List(caps)),
        ]))
    })
}

/// `Orchestration.assign_agents` — assign agents to planned steps.
pub fn assign_agents_seam(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let session_id = args::rec_str(args, "session_id")
        .ok_or_else(|| args::bad(span, "Orchestration.assign_agents needs session_id"))?;
    with_sessions(|sessions| {
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| args::bad(span, format!("session {session_id} not found")))?;
        let assignments = assign_agents(session);
        let assignment_records: Vec<Value> = assignments
            .iter()
            .map(|(node_id, agent_id)| {
                let mut rec = BTreeMap::new();
                rec.insert("node_id".into(), Value::U64(*node_id as u64));
                rec.insert("agent_id".into(), Value::String(agent_id.clone()));
                Value::Record(rec)
            })
            .collect();
        Ok(args::record([
            ("session_id", Value::String(session_id.to_string())),
            ("assignments", Value::List(assignment_records)),
            ("count", Value::U64(assignments.len() as u64)),
        ]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn clear_sessions() {
        let mut guard = SESSIONS.lock().unwrap();
        *guard = None;
    }

    #[test]
    fn session_create_and_status() {
        clear_sessions();
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("s1".into()));
        m.insert("task".into(), Value::String("Research climate".into()));
        let result = session_create(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());

        let mut m2 = BTreeMap::new();
        m2.insert("id".into(), Value::String("s1".into()));
        let status = session_status(&Value::Record(m2), Span { start: 0, end: 0 });
        assert!(status.is_ok());
    }

    #[test]
    fn roster_register_and_list() {
        clear_sessions();
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("s2".into()));
        m.insert("task".into(), Value::String("Research".into()));
        session_create(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();

        let mut m2 = BTreeMap::new();
        m2.insert("session_id".into(), Value::String("s2".into()));
        m2.insert("agent_id".into(), Value::String("a1".into()));
        m2.insert("did".into(), Value::String("did:q42:a".into()));
        m2.insert("name".into(), Value::String("Alice".into()));
        m2.insert("role".into(), Value::String("researcher".into()));
        m2.insert(
            "capabilities".into(),
            Value::List(vec![Value::String("NLP.substrate_extract".into())]),
        );
        let result = roster_register(&Value::Record(m2), Span { start: 0, end: 0 });
        assert!(result.is_ok());

        let mut m3 = BTreeMap::new();
        m3.insert("session_id".into(), Value::String("s2".into()));
        let list = roster_list(&Value::Record(m3), Span { start: 0, end: 0 });
        assert!(list.is_ok());
    }

    #[test]
    fn full_lifecycle() {
        clear_sessions();
        // Create session.
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("s3".into()));
        m.insert(
            "task".into(),
            Value::String("Research climate impacts".into()),
        );
        session_create(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();

        // Register agent.
        let mut m2 = BTreeMap::new();
        m2.insert("session_id".into(), Value::String("s3".into()));
        m2.insert("agent_id".into(), Value::String("a1".into()));
        m2.insert("did".into(), Value::String("did:q42:a".into()));
        m2.insert("name".into(), Value::String("Alice".into()));
        m2.insert("role".into(), Value::String("researcher".into()));
        m2.insert(
            "capabilities".into(),
            Value::List(vec![Value::String("NLP.substrate_extract".into())]),
        );
        roster_register(&Value::Record(m2), Span { start: 0, end: 0 }).unwrap();

        // Plan.
        let mut m3 = BTreeMap::new();
        m3.insert("id".into(), Value::String("s3".into()));
        let plan_result = session_plan(&Value::Record(m3), Span { start: 0, end: 0 });
        assert!(plan_result.is_ok());

        // Assign agents.
        let mut m4 = BTreeMap::new();
        m4.insert("session_id".into(), Value::String("s3".into()));
        let assign_result = assign_agents_seam(&Value::Record(m4), Span { start: 0, end: 0 });
        assert!(assign_result.is_ok());

        // Execute.
        let mut m5 = BTreeMap::new();
        m5.insert("id".into(), Value::String("s3".into()));
        let exec_result = session_execute(&Value::Record(m5), Span { start: 0, end: 0 });
        assert!(exec_result.is_ok());
    }
}
