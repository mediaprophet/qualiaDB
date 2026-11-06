//! Identifier Fabric static plane-collapse guard (`IDF-04`).
//!
//! Detects the six plane-collapse anti-patterns (`F5-A` through `F5-F`) defined in
//! `docs/work-in-progress/IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` and Playbook v4 §4.1 (`IDF-04`),
//! emitting structured `suggested_fix` diagnostics without widening EBNF keywords or `Host` opcodes.

use crate::ast::*;
use crate::check::MAX_DIAGNOSTICS;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use std::collections::HashMap;

/// Structured Identifier Fabric diagnostic payload serialized into `Diagnostic::suggested_fix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FabricSuggestedFix {
    pub rule: &'static str,
    pub from: &'static str,
    pub to: &'static str,
    pub plane: &'static str,
    pub framing: &'static str,
    pub instrument_kind: Option<&'static str>,
    pub suggested_form: &'static str,
}

impl FabricSuggestedFix {
    /// Serialize into canonical compact JSON for `Diagnostic::suggested_fix`.
    pub fn to_json(&self) -> String {
        let inst = match self.instrument_kind {
            Some(k) => format!("\"{}\"", k),
            None => "null".to_string(),
        };
        format!(
            "{{\"rule\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"plane\":\"{}\",\"framing\":\"{}\",\"instrumentKind\":{},\"suggested_form\":\"{}\"}}",
            self.rule, self.from, self.to, self.plane, self.framing, inst, self.suggested_form
        )
    }
}

const FIX_F5_A: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-A",
    from: "did_as_who",
    to: "natural_agent_with_instrument_binding",
    plane: "NaturalAgent",
    framing: "living-SHACL",
    instrument_kind: Some("Did"),
    suggested_form: "Bind the NaturalAgent (who) via a time-bounded InstrumentBinding to the DID/QRC instrument instead of collapsing did: into who.",
};

const FIX_F5_B: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-B",
    from: "vc_verified_as_truth",
    to: "claim_opinion_evidence",
    plane: "ClaimOpinion",
    framing: "artifact-OWL",
    instrument_kind: Some("VerifiableCredential"),
    suggested_form: "Route VC signature verification into ClaimOpinion evidential support (mu, lambda) rather than objective truth.",
};

const FIX_F5_C: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-C",
    from: "qrc_or_observer_as_person_join_key",
    to: "spatiotemporal_handle_join",
    plane: "SpatiotemporalHandle",
    framing: "artifact-OWL",
    instrument_kind: Some("QrcCoordinate"),
    suggested_form: "Join on SpatiotemporalHandle / QRC topological coordinates or explicit InstrumentBinding, never on did:q42: as a person primary key.",
};

const FIX_F5_D: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-D",
    from: "biometric_instance_as_who",
    to: "biometric_family_with_consent",
    plane: "Instrument",
    framing: "living-SHACL",
    instrument_kind: Some("BiometricInstance"),
    suggested_form: "Attach BiometricInstance to a BiometricFamily under an explicit revocable consent scope; never equate a biometric sample with NaturalAgent identity.",
};

const FIX_F5_E: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-E",
    from: "natural_agent_as_owl_thing",
    to: "living_shacl_natural_agent",
    plane: "NaturalAgent",
    framing: "living-SHACL",
    instrument_kind: None,
    suggested_form: "Keep NaturalAgent and living flora/fauna under living-SHACL shapes; never type a living being as owl:Thing or inert artifact.",
};

const FIX_F5_F: FabricSuggestedFix = FabricSuggestedFix {
    rule: "F5-F",
    from: "ai_agent_or_same_as_person_collapse",
    to: "instrument_with_sponsor_natural_agent",
    plane: "Instrument",
    framing: "artifact-OWL",
    instrument_kind: Some("MachineDeviceId"),
    suggested_form: "Model AI/device systems on the Instrument plane with a sponsoring NaturalAgent; never collapse AiAgent to person or apply owl:sameAs to NaturalAgent.",
};

/// Run the Identifier Fabric plane-collapse guard across a parsed [`Program`].
pub fn check_program_fabric(program: &Program, errors: &mut Vec<Diagnostic>) {
    let mut env_tags = HashMap::new();
    for item in &program.items {
        if errors.len() >= MAX_DIAGNOSTICS {
            return;
        }
        check_item_fabric(item, &mut env_tags, errors);
    }
}

/// Run the Identifier Fabric plane-collapse guard on a standalone cell [`Expr`].
pub fn check_expr_fabric(expr: &Expr) -> Result<(), Diagnostic> {
    let mut env_tags = HashMap::new();
    let mut errors = Vec::new();
    walk_expr_fabric(expr, &mut env_tags, &mut errors);
    errors.into_iter().next().map_or(Ok(()), Err)
}

fn check_item_fabric(
    item: &Item,
    env_tags: &mut HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    match item {
        Item::Const(c) => {
            check_binding_pair(&c.name, &c.value, c.span, env_tags, errors);
            walk_expr_fabric(&c.value, env_tags, errors);
        }
        Item::Function(f) => {
            let mut fn_tags = env_tags.clone();
            for p in &f.params {
                fn_tags.insert(p.name.to_ascii_lowercase(), p.name.to_ascii_lowercase());
            }
            for s in &f.body.stmts {
                check_stmt_fabric(s, &mut fn_tags, errors);
            }
        }
        Item::Hook(h) => {
            let mut hook_tags = env_tags.clone();
            for p in &h.params {
                hook_tags.insert(p.name.to_ascii_lowercase(), p.name.to_ascii_lowercase());
            }
            for s in &h.body.stmts {
                check_stmt_fabric(s, &mut hook_tags, errors);
            }
        }
        Item::Statement(s) => check_stmt_fabric(s, env_tags, errors),
        Item::Cell(c) => {
            if let Some(w) = &c.when {
                walk_expr_fabric(w, env_tags, errors);
            }
            walk_expr_fabric(&c.expr, env_tags, errors);
        }
        Item::Law(l) => {
            walk_expr_fabric(&l.condition, env_tags, errors);
            walk_expr_fabric(&l.consequence, env_tags, errors);
        }
        Item::Bind(b) => {
            walk_expr_fabric(&b.left, env_tags, errors);
            walk_expr_fabric(&b.right, env_tags, errors);
        }
        Item::Material(m) => {
            for p in &m.properties {
                check_binding_pair(&p.name, &p.value, p.span, env_tags, errors);
                walk_expr_fabric(&p.value, env_tags, errors);
            }
        }
        Item::Present(p) => {
            for prop in &p.properties {
                check_binding_pair(&prop.name, &prop.value, prop.span, env_tags, errors);
                walk_expr_fabric(&prop.value, env_tags, errors);
            }
        }
        Item::Enum(_) | Item::Field(_) => {}
    }
}

fn check_stmt_fabric(
    stmt: &Stmt,
    env_tags: &mut HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    if errors.len() >= MAX_DIAGNOSTICS {
        return;
    }
    match stmt {
        Stmt::Let {
            span,
            name,
            value: Some(val),
            ..
        } => {
            check_binding_pair(name, val, *span, env_tags, errors);
            walk_expr_fabric(val, env_tags, errors);
        }
        Stmt::Let { value: None, .. } | Stmt::Return { value: None, .. } | Stmt::Yield { value: None, .. } => {}
        Stmt::LetPat { value, .. } => walk_expr_fabric(value, env_tags, errors),
        Stmt::Assign {
            span,
            target,
            value,
        } => {
            if let ExprKind::Ident(name) = &target.kind {
                check_binding_pair(name, value, *span, env_tags, errors);
            }
            walk_expr_fabric(value, env_tags, errors);
        }
        Stmt::Return { value: Some(e), .. }
        | Stmt::Yield { value: Some(e), .. }
        | Stmt::Effect { expr: e, .. }
        | Stmt::Expr { expr: e, .. } => walk_expr_fabric(e, env_tags, errors),
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr_fabric(cond, env_tags, errors);
            for s in &then_block.stmts {
                check_stmt_fabric(s, env_tags, errors);
            }
            if let Some(eb) = else_block {
                check_stmt_fabric(eb, env_tags, errors);
            }
        }
        Stmt::For { iter, body, .. } => {
            walk_expr_fabric(iter, env_tags, errors);
            for s in &body.stmts {
                check_stmt_fabric(s, env_tags, errors);
            }
        }
        Stmt::While { cond, body, .. } => {
            walk_expr_fabric(cond, env_tags, errors);
            for s in &body.stmts {
                check_stmt_fabric(s, env_tags, errors);
            }
        }
        Stmt::Match {
            scrutinee, arms, ..
        } => {
            walk_expr_fabric(scrutinee, env_tags, errors);
            for arm in arms {
                match &arm.body {
                    ArmBody::Block(b) => {
                        for s in &b.stmts {
                            check_stmt_fabric(s, env_tags, errors);
                        }
                    }
                    ArmBody::Expr(e) => walk_expr_fabric(e, env_tags, errors),
                }
            }
        }
        Stmt::Transaction { args, body, .. } => {
            for na in args {
                check_binding_pair(&na.name, &na.value, na.span, env_tags, errors);
                walk_expr_fabric(&na.value, env_tags, errors);
            }
            for s in &body.stmts {
                check_stmt_fabric(s, env_tags, errors);
            }
        }
        Stmt::Block(b) => {
            for s in &b.stmts {
                check_stmt_fabric(s, env_tags, errors);
            }
        }
    }
}

fn walk_expr_fabric(
    expr: &Expr,
    env_tags: &mut HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    if errors.len() >= MAX_DIAGNOSTICS {
        return;
    }
    match &expr.kind {
        ExprKind::Record(fields) => {
            for na in fields {
                check_binding_pair(&na.name, &na.value, na.span, env_tags, errors);
                walk_expr_fabric(&na.value, env_tags, errors);
            }
        }
        ExprKind::Call { callee, args } => {
            walk_expr_fabric(callee, env_tags, errors);
            for arg in args {
                match arg {
                    Arg::Named(na) => {
                        check_binding_pair(&na.name, &na.value, na.span, env_tags, errors);
                        walk_expr_fabric(&na.value, env_tags, errors);
                    }
                    Arg::Pos(e) => walk_expr_fabric(e, env_tags, errors),
                }
            }
            check_call_fabric(args, expr.span, env_tags, errors);
        }
        ExprKind::Binary { left, right, .. } | ExprKind::Pipe { left, right } => {
            walk_expr_fabric(left, env_tags, errors);
            walk_expr_fabric(right, env_tags, errors);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Member { recv: inner, .. }
        | ExprKind::Await(inner)
        | ExprKind::Try(inner) => walk_expr_fabric(inner, env_tags, errors),
        ExprKind::List(items) | ExprKind::Interpolate(items) => {
            for item in items {
                walk_expr_fabric(item, env_tags, errors);
            }
        }
        ExprKind::IfElse {
            cond,
            then_value,
            else_value,
        } => {
            walk_expr_fabric(cond, env_tags, errors);
            walk_expr_fabric(then_value, env_tags, errors);
            walk_expr_fabric(else_value, env_tags, errors);
        }
        _ => {}
    }
}

fn expr_token_summary(expr: &Expr, env_tags: &HashMap<String, String>) -> String {
    match &expr.kind {
        ExprKind::Literal(Literal::String(s)) => s.to_ascii_lowercase(),
        ExprKind::Iri(iri) => iri.to_ascii_lowercase(),
        ExprKind::Prefixed(prefix, local) => {
            format!("{}:{}", prefix.to_ascii_lowercase(), local.to_ascii_lowercase())
        }
        ExprKind::Ident(id) => {
            let low = id.to_ascii_lowercase();
            env_tags.get(&low).cloned().unwrap_or(low)
        }
        ExprKind::Member { recv, name } => {
            format!(
                "{}.{}",
                expr_token_summary(recv, env_tags),
                name.to_ascii_lowercase()
            )
        }
        _ => String::new(),
    }
}

fn check_binding_pair(
    raw_name: &str,
    value: &Expr,
    span: Span,
    env_tags: &mut HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    let name = raw_name.to_ascii_lowercase();
    let rhs = expr_token_summary(value, env_tags);
    if !rhs.is_empty() {
        env_tags.insert(name.clone(), rhs.clone());
    }

    let is_join_key = matches!(
        name.as_str(),
        "person_join_key" | "who_join_key" | "join_person_by_qrc" | "observer_as_person_key" | "person_primary_key"
    );
    if is_join_key && (rhs.starts_with("did:q42:") || rhs.contains("observer_did") || rhs.contains("qrc")) {
        return push_fabric_diag(errors, span, "F5-C plane collapse: `did:q42:` QRC coordinate or observer handle used as a person join key", FIX_F5_C);
    }

    let is_who = matches!(
        name.as_str(),
        "who" | "natural_agent" | "natural_person" | "person_identity" | "human_identity"
    );
    if is_who && rhs.starts_with("did:") {
        return push_fabric_diag(errors, span, "F5-A plane collapse: DID / QRC instrument bound directly to `NaturalAgent` (`who`) identity", FIX_F5_A);
    }

    let is_truth = matches!(
        name.as_str(),
        "claim_true" | "objective_truth" | "proven_true" | "fact_true" | "truth_verified"
    );
    if is_truth
        && (rhs.contains("vc_verified")
            || rhs.contains("credential_verified")
            || rhs.contains("vc.verified")
            || rhs.contains("credential.verified"))
    {
        return push_fabric_diag(errors, span, "F5-B plane collapse: Verifiable Credential verification boolean treated as objective claim truth", FIX_F5_B);
    }

    if is_who
        && (rhs.contains("biometric_instance")
            || rhs.contains("fingerprint_sample")
            || rhs.contains("iris_scan_instance")
            || rhs.contains("face_capture_instance"))
        && !rhs.contains("biometric_family")
    {
        return push_fabric_diag(errors, span, "F5-D plane collapse: single `BiometricInstance` used as timeless `NaturalAgent` (`who`) identity without `BiometricFamily` and consent scope", FIX_F5_D);
    }

    let is_living_type = matches!(
        name.as_str(),
        "natural_agent_class" | "natural_agent_type" | "natural_agent_superclass" | "person_ontology_class" | "living_being_type"
    );
    if is_living_type && (rhs == "owl:thing" || rhs == "thing" || rhs.ends_with("#thing") || rhs == "artifact:thing") {
        return push_fabric_diag(errors, span, "F5-E plane collapse: `NaturalAgent` or living being typed as `owl:Thing` / inert artifact", FIX_F5_E);
    }

    if is_who
        && (rhs.contains("ai_agent")
            || rhs.contains("llm_agent")
            || rhs.contains("machine_device")
            || rhs.contains("autonomous_bot"))
    {
        push_fabric_diag(errors, span, "F5-F plane collapse: `AiAgent` or `MachineDevice` collapsed into `NaturalAgent` (`person` / `who`)", FIX_F5_F);
    }
}

fn check_call_fabric(
    args: &[Arg],
    span: Span,
    env_tags: &HashMap<String, String>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut pred = String::new();
    let mut obj = String::new();
    for arg in args {
        if let Arg::Named(na) = arg {
            match na.name.as_str() {
                "predicate" => pred = expr_token_summary(&na.value, env_tags),
                "object" => obj = expr_token_summary(&na.value, env_tags),
                _ => {}
            }
        }
    }
    if pred == "owl:sameas" || pred.ends_with("#sameas") {
        push_fabric_diag(errors, span, "F5-F plane collapse: `owl:sameAs` forbidden as a person-merge operator", FIX_F5_F);
    } else if (pred == "rdf:type" || pred.ends_with("#type")) && (obj == "owl:thing" || obj.ends_with("#thing")) {
        push_fabric_diag(errors, span, "F5-E plane collapse: `owl:Thing` forbidden as a superclass of `NaturalAgent`", FIX_F5_E);
    }
}

fn push_fabric_diag(
    errors: &mut Vec<Diagnostic>,
    span: Span,
    message: &str,
    fix: FabricSuggestedFix,
) {
    if errors.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let diag = Diagnostic::new(DiagCode::E100, span, message.to_string())
        .with_fix(fix.to_json())
        .with_evidential(1.0, 0.0);
    if !errors.iter().any(|x| x.span == diag.span && x.message == diag.message) {
        errors.push(diag);
    }
}
