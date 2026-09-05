//! Dual-path Tool Chest honesty: standalone sketch vs live invoke vs denied.
//!
//! A local fallback must not turn a daemon rejection into success, and a
//! canvas SPARQL sketch must never be labelled as `GraphDatabase.sparql`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualPathReport {
    pub status_kind: &'static str,
    pub message: String,
}

/// Standalone result. Distinct from live success; names the live id it is not.
pub fn local_sketch(live_id: &str, body: &str) -> DualPathReport {
    DualPathReport {
        status_kind: "local",
        message: format!("Standalone (not {live_id}): {body}"),
    }
}

/// Daemon-backed success. The live `Family.method` is the source.
pub fn live_ok(live_id: &str, body: &str) -> DualPathReport {
    DualPathReport {
        status_kind: "success",
        message: format!("{live_id}: {body}"),
    }
}

/// Daemon rejected or failed. Do not attach a local sketch as if it were live.
pub fn live_denied(live_id: &str, diagnostic: &str) -> DualPathReport {
    DualPathReport {
        status_kind: "error",
        message: format!(
            "{live_id} did not succeed ({diagnostic}). A standalone sketch is not a live result."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dom_query_is_not_live_sparql() {
        let report = local_sketch(
            "GraphDatabase.sparql",
            r#"{"boolean":true,"source":"poet-local"}"#,
        );
        assert_eq!(report.status_kind, "local");
        assert!(report.message.contains("not GraphDatabase.sparql"));
        assert!(report.message.contains("poet-local"));
        assert_ne!(report.status_kind, "success");
    }

    #[test]
    fn live_success_names_the_capability() {
        let report = live_ok("GraphDatabase.sparql", r#"{"boolean":true}"#);
        assert_eq!(report.status_kind, "success");
        assert!(report.message.starts_with("GraphDatabase.sparql:"));
    }

    #[test]
    fn daemon_rejection_is_not_success() {
        let report = live_denied("GraphDatabase.sparql", "query failed");
        assert_eq!(report.status_kind, "error");
        assert!(report.message.contains("GraphDatabase.sparql"));
        assert!(report.message.contains("not a live result"));
        assert!(!report.message.contains("poet-local"));
    }

    #[test]
    fn sentinel_and_gazetteer_follow_the_same_rule() {
        assert_eq!(
            live_denied("Sentinel.inspect", "denied").status_kind,
            "error"
        );
        assert_eq!(
            local_sketch("NLP.gazetteer_run", "3 tokens").status_kind,
            "local"
        );
        assert_eq!(live_ok("Statistics.mean", "2.5").status_kind, "success");
    }

    #[test]
    fn dual_path_callers_do_not_promote_rejection_to_success() {
        for path in ["tool_actions.rs", "shapes_actions.rs", "chain_actions.rs"] {
            let src = match path {
                "tool_actions.rs" => include_str!("tool_actions.rs"),
                "shapes_actions.rs" => include_str!("shapes_actions.rs"),
                "chain_actions.rs" => include_str!("chain_actions.rs"),
                _ => unreachable!(),
            };
            assert!(
                !src.contains("fallback after daemon rejection"),
                "{path} still promotes a local fallback after daemon rejection"
            );
            assert!(
                !src.contains("Standalone fallback"),
                "{path} still treats a transport error as standalone success"
            );
        }
    }
}
