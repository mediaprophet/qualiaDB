//! Selected runtime contracts. Fixed registry entries are not provider discovery.
use super::{EffectClass as E, HonestyLabel as H, SchemaArg as A, SchemaEntry as S};

pub(super) fn entries() -> Vec<S> {
    let mut entries = vec![
        S::new("task.spawn", E::Write, H::FailClosed, "TaskRecord")
            .with_arg(A::new("source", "string", true)).with_arg(A::new("metadata", "Value", false))
            .with_description("Registers a bounded local Pending task in the caller's runtime scope. Metadata grants no ownership. AST Engine only; no asynchronous or remote launch."),
        S::new("task.await", E::External, H::FailClosed, "TaskRecord")
            .with_arg(A::new("task", "TaskRecord", true))
            .with_description("Executes or reads a scoped local task synchronously on the admitted caller Host/Env/budget. No continuation suspension. Standalone/bytecode routes refuse."),
        S::new("task.status", E::Read, H::FailClosed, "TaskRecord")
            .with_arg(A::new("task", "TaskRecord", true))
            .with_description("Reads only caller-scope tasks; numeric-only IDs and foreign runtime scopes refuse."),
        S::new("task.cancel", E::Write, H::FailClosed, "TaskRecord")
            .with_arg(A::new("task", "TaskRecord", true))
            .with_description("Requests cancellation of a caller-scope task; acknowledgement follows a runtime checkpoint. Terminal outcomes are immutable; external effects are not rolled back."),
        S::new("task.with_timeout", E::Write, H::FailClosed, "TaskRecord")
            .with_arg(A::new("task", "TaskRecord", true)).with_arg(A::new("milliseconds", "i64", true))
            .with_description("Sets or tightens a monotonic deadline measured from task registration. Runtime checkpoints and post-call commit recheck it. Native calls cannot be preempted."),
        S::new("task.nursery", E::External, H::FailClosed, "Record")
            .with_arg(A::new("tasks", "List<TaskRecord>", true))
            .with_description("Validates one runtime scope and joins a bounded sequential group, cancelling unstarted siblings after failure. No concurrent nursery or suspension claim."),
        S::new("code.spawn_at", E::Write, H::FailClosed, "TaskRecord")
            .with_arg(A::new("node", "string", true)).with_arg(A::new("source_or_digest", "string", true)).with_arg(A::new("metadata", "Value", false))
            .with_description("Only did:q42:local_scheduler is admitted by this local profile. Other provider/remote identifiers refuse. AST Engine/runtime scope required."),
        S::new("code.hash_ast", E::Write, H::Always, "Record")
            .with_arg(A::new("source", "string", true))
            .with_description("SHA-256 of encoded AST, separately labelled FNV fast index; registers in-process content. Not package admission or source authority."),
        S::new("code.resolve", E::Read, H::FailClosed, "Record")
            .with_arg(A::new("digest", "string", true)).with_arg(A::new("candidate", "string", false))
            .with_description("Verifies available in-process AST bytes against SHA-256. Missing or changed bytes fail verification; not dependency/provider admission."),
        S::new("value.attach_provenance", E::Write, H::Always, "Record")
            .with_arg(A::new("value", "Value", true)).with_arg(A::new("provenance", "Record", true))
            .with_description("Attaches attributed data and registers local content integrity. Does not verify issuer signatures or adopt the statement."),
        S::new("value.verify_provenance", E::Read, H::FailClosed, "Record")
            .with_arg(A::new("wrapped", "Record", true))
            .with_description("Checks registered local integrity only; issuer_signature_verified remains false."),
        S::new("crypto.prove_zk_predicate", E::Pure, H::FailClosed, "Unsupported")
            .with_description("Legacy proof API refuses: no admitted cryptographic predicate prover connected."),
        S::new("crypto.verify_zk_predicate", E::Pure, H::FailClosed, "Unsupported")
            .with_description("Legacy proof API refuses: no admitted cryptographic predicate verifier connected."),
        S::new("crypto.commit_predicate", E::Pure, H::Always, "Record")
            .with_arg(A::new("predicate_text", "string", true)).with_arg(A::new("public_input", "Value", false))
            .with_description("Constructs a deterministic local SHA-256 binding to predicate text and public input (default Null). No registry or witness payload; predicate_truth_verified and issuer_authenticity_verified remain false."),
        S::new("crypto.verify_commitment", E::Pure, H::Always, "Record")
            .with_arg(A::new("commitment", "Record", true)).with_arg(A::new("public_input", "Value", false))
            .with_description("Returns integrity_verified by recomputing the deterministic binding with public input (default Null). Predicate truth and issuer authenticity remain unverified; no registry or cryptographic ZK verifier is involved."),
    ];
    for (id, description) in [
        (
            "effect.perform",
            "Checked effect descriptor only; no performed external effect.",
        ),
        (
            "effect.handle",
            "Checked handler descriptor only; does not invoke or resume a checked continuation.",
        ),
        (
            "effect.resume",
            "Checked resume descriptor only; no evaluator continuation resumption.",
        ),
        (
            "supervisor.start_link",
            "Checked supervisor plan descriptor only; no worker restart.",
        ),
        (
            "runtime.with_budget",
            "Checked budget descriptor only; does not modify the evaluator budget.",
        ),
        (
            "continuation.snapshot",
            "Checked data snapshot descriptor only; no evaluator checkpoint.",
        ),
        (
            "continuation.rollback_to",
            "Checked data restoration descriptor only; no execution or external-effect rollback.",
        ),
        (
            "memory.with_arena",
            "Checked arena descriptor only; no allocator ownership or zero-heap-escape guarantee.",
        ),
        (
            "memory.perceus_plan",
            "Checked allocation plan descriptor only.",
        ),
        ("memory.layout_soa", "Checked data layout descriptor only."),
        (
            "channel.lock_free_spsc",
            "Checked bounded data queue descriptor only; no concurrent lock-free channel.",
        ),
    ] {
        entries
            .push(S::new(id, E::Pure, H::Always, "DescriptorRecord").with_description(description));
    }
    entries
}

#[cfg(test)]
mod tests {
    #[test]
    fn predicate_binding_schemas_match_actual_record_results_and_optional_input() {
        for (id, source) in [
            (
                "crypto.commit_predicate",
                "= crypto.commit_predicate(\"1 == 0\")",
            ),
            (
                "crypto.verify_commitment",
                "= crypto.verify_commitment(crypto.commit_predicate(\"1 == 0\"))",
            ),
        ] {
            let schema = crate::capability_schema::schema_for(id).unwrap();
            assert_eq!(schema.returns, "Record");
            assert_eq!(schema.effect, crate::capability_schema::EffectClass::Pure);
            assert_eq!(
                crate::effects::binding_effect(id),
                crate::effects::Effect::Pure
            );
            assert_eq!(schema.args.len(), 2);
            assert!(schema.args[0].required);
            assert_eq!(schema.args[1].name, "public_input");
            assert!(!schema.args[1].required);
            let value = crate::eval_cell(
                source,
                &mut crate::LocalHost::default(),
                &mut crate::Env::default(),
            )
            .unwrap();
            let crate::Value::Record(result) = value else {
                panic!("schema promises Record")
            };
            assert_eq!(
                result.get("predicate_truth_verified"),
                Some(&crate::Value::Bool(false))
            );
            assert_eq!(
                result.get("issuer_authenticity_verified"),
                Some(&crate::Value::Bool(false))
            );
            if id == "crypto.verify_commitment" {
                assert_eq!(
                    result.get("integrity_verified"),
                    Some(&crate::Value::Bool(true))
                );
            }
        }
    }
}
