//! MCP tools for SHACL Validation, Enumerated Identity, and Credential Gates.
//!
//! Migrated from legacy `mcp_stub_impls.rs` to dedicated library module.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_quin_slice, parse_tool_args, McpSystemError};
use qualia_core_db::modalities::logic::shacl_extensions::{
    credential_gates_target, degrade_violations, loci_for_shape, shapes_for_locus,
    validate_enumerated_identity, CredentialGate, CryptoScheme, IdentifierBinding,
    IdentityValidation, OperationMode, ShaclSeverity, ShapeRoute, ShapeViolation,
    MAX_IDENTITY_BINDINGS, MAX_SHAPE_ROUTES,
};
use qualia_core_db::shacl_compiler::{validate_shacl_property, ShaclConstraint, ShaclDatatype};
use qualia_core_db::verifiable_credential::Credential;
use qualia_core_db::NQuin;
use serde_json::{json, Value};

fn parse_crypto_scheme(s: &str) -> CryptoScheme {
    match s.to_ascii_lowercase().as_str() {
        "ed25519" => CryptoScheme::Ed25519,
        "mldsa65" | "ml-dsa-65" | "mldsa" => CryptoScheme::MlDsa65,
        "blake3" | "blake3commitment" => CryptoScheme::Blake3Commitment,
        "x25519" => CryptoScheme::X25519,
        _ => CryptoScheme::Unknown,
    }
}

fn parse_shacl_severity(s: &str) -> ShaclSeverity {
    match s.to_ascii_lowercase().as_str() {
        "critical" => ShaclSeverity::Critical,
        "violation" => ShaclSeverity::Violation,
        "warning" => ShaclSeverity::Warning,
        _ => ShaclSeverity::Info,
    }
}

fn parse_shacl_constraints(v: &Value) -> Result<Vec<ShaclConstraint>, McpSystemError> {
    let arr = v
        .get("constraints")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mut out = Vec::new();
    for item in arr {
        let kind = item
            .get("kind")
            .or_else(|| item.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match kind {
            "datatype" | "Datatype" => {
                let dt = item
                    .get("datatype")
                    .and_then(Value::as_str)
                    .unwrap_or("xsd:string");
                let iri = format!("xsd:{dt}");
                if let Some(dt) = ShaclDatatype::from_iri_hash(qualia_core_db::q_hash(&iri)) {
                    out.push(ShaclConstraint::Datatype(dt));
                }
            }
            "minCount" | "MinCount" => {
                let n = item.get("value").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
                out.push(ShaclConstraint::MinCount(n));
            }
            "maxCount" | "MaxCount" => {
                let n = item.get("value").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
                out.push(ShaclConstraint::MaxCount(n));
            }
            "deonticObligate" => out.push(ShaclConstraint::DeonticObligate),
            "deonticPermit" => out.push(ShaclConstraint::DeonticPermit),
            "deonticForbid" => out.push(ShaclConstraint::DeonticForbid),
            "epistemicKnowledge" => {
                let min = item
                    .get("min_certainty")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(128) as u8;
                out.push(ShaclConstraint::EpistemicKnowledge { min_certainty: min });
            }
            "commonKnowledge" => out.push(ShaclConstraint::CommonKnowledge),
            _ => {}
        }
    }
    Ok(out)
}

/// `validate_shacl` tool — executes property shape validation against target nodes and property paths.
pub fn validate_shacl(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let quins = parse_quin_slice(&v, "quins")?;
    let target_subject = v.get("target_subject").and_then(|x| x.as_u64()).unwrap_or(0);
    let target_property = v.get("target_property").and_then(|x| x.as_u64()).unwrap_or(0);
    if target_subject == 0 || target_property == 0 {
        return Err(McpSystemError::InvalidParameters);
    }
    let constraints = parse_shacl_constraints(&v)?;
    let valid = validate_shacl_property(&quins, target_subject, target_property, &constraints);
    let payload = json!({
        "valid": valid,
        "targetSubject": target_subject,
        "targetProperty": target_property,
        "constraintCount": constraints.len(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `validate_enumerated_identity` tool — checks multi-identifier bindings across decentralized cryptographic schemes.
pub fn validate_enumerated_identity_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let arr = v
        .get("bindings")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let n = arr.len().min(MAX_IDENTITY_BINDINGS);
    let mut buf = [IdentifierBinding {
        identifier: 0,
        scheme: CryptoScheme::Unknown,
        attested: false,
        confidence: 0.0,
    }; MAX_IDENTITY_BINDINGS];
    for (i, b) in arr.iter().take(n).enumerate() {
        buf[i] = IdentifierBinding {
            identifier: b.get("identifier").and_then(|x| x.as_u64()).unwrap_or(0),
            scheme: parse_crypto_scheme(b.get("scheme").and_then(Value::as_str).unwrap_or("unknown")),
            attested: b.get("attested").and_then(Value::as_bool).unwrap_or(false),
            confidence: b.get("confidence").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        };
    }
    let min_distinct = v.get("min_distinct").and_then(|x| x.as_u64()).unwrap_or(2) as u16;
    let min_attested = v.get("min_attested").and_then(|x| x.as_u64()).unwrap_or(1) as u16;
    let payload = match validate_enumerated_identity(&buf[..n], min_distinct, min_attested) {
        IdentityValidation::Valid {
            distinct,
            attested,
            aggregate_confidence,
        } => json!({
            "verdict": "valid", "distinct": distinct, "attested": attested,
            "aggregateConfidence": aggregate_confidence,
        }),
        IdentityValidation::Underdetermined { distinct, attested } => json!({
            "verdict": "underdetermined", "distinct": distinct, "attested": attested,
        }),
        IdentityValidation::DefinitiveCollapse => json!({
            "verdict": "definitive_collapse",
            "reason": "a binding asserts certainty (confidence>=1.0); identity must stay a confidence-relation",
        }),
    };
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `shacl_credential_gate` tool — verifies if a verifiable credential (VC) unlocks a SHACL shape target.
pub fn shacl_credential_gate(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let focus_node = v.get("focus_node").and_then(|x| x.as_u64()).unwrap_or(0);
    let g = v.get("gate").ok_or(McpSystemError::InvalidParameters)?;
    let gate = CredentialGate {
        shape: g.get("shape").and_then(|x| x.as_u64()).unwrap_or(0),
        required_claim_predicate: g.get("required_claim_predicate").and_then(|x| x.as_u64()).unwrap_or(0),
        required_claim_object: g.get("required_claim_object").and_then(|x| x.as_u64()).unwrap_or(0),
        accepted_issuer: g.get("accepted_issuer").and_then(|x| x.as_u64()).unwrap_or(0),
    };
    let c = v.get("credential").ok_or(McpSystemError::InvalidParameters)?;
    let mut claims = Vec::new();
    if let Some(items) = c.get("claims").and_then(Value::as_array) {
        for q in items {
            claims.push(NQuin {
                subject: q.get("s").and_then(|x| x.as_u64()).unwrap_or(0),
                predicate: q.get("p").and_then(|x| x.as_u64()).unwrap_or(0),
                object: q.get("o").and_then(|x| x.as_u64()).unwrap_or(0),
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
    }
    let vc = Credential {
        issuer: c.get("issuer").and_then(|x| x.as_u64()).unwrap_or(0),
        subject: c.get("subject").and_then(|x| x.as_u64()).unwrap_or(0),
        issued_at: c.get("issued_at").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        valid_until: c.get("valid_until").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        claims,
    };
    let applies = credential_gates_target(&gate, focus_node, &vc);
    let payload = json!({ "applies": applies, "focusNode": focus_node, "shape": gate.shape });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `shacl_degrade_violations` tool — gracefully degrades SHACL constraint violations during off-grid operation.
pub fn shacl_degrade_violations(args: &[u8]) -> Result<String, McpSystemError> {
    const MAX_V: usize = 256;
    let v = parse_tool_args(args)?;
    let arr = v
        .get("violations")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let n = arr.len().min(MAX_V);
    let mut buf = [ShapeViolation {
        shape: 0,
        focus_node: 0,
        severity: ShaclSeverity::Info,
    }; MAX_V];
    for (i, x) in arr.iter().take(n).enumerate() {
        buf[i] = ShapeViolation {
            shape: x.get("shape").and_then(|k| k.as_u64()).unwrap_or(0),
            focus_node: x.get("focus_node").and_then(|k| k.as_u64()).unwrap_or(0),
            severity: parse_shacl_severity(x.get("severity").and_then(Value::as_str).unwrap_or("info")),
        };
    }
    let mode = if v.get("mode").and_then(Value::as_str).unwrap_or("online").eq_ignore_ascii_case("offgrid") {
        OperationMode::OffGrid
    } else {
        OperationMode::Online
    };
    let mut out = buf;
    let outcome = degrade_violations(&buf[..n], mode, &mut out[..n]);
    let payload = json!({
        "blocking": outcome.blocking,
        "degraded": outcome.degraded,
        "subgraphUsable": outcome.subgraph_usable,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `shacl_route` tool — decentralized topological routing of shapes to loci and vice versa.
pub fn shacl_route(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let arr = v
        .get("routes")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let n = arr.len().min(MAX_SHAPE_ROUTES);
    let mut routes = [ShapeRoute { shape: 0, locus: 0 }; MAX_SHAPE_ROUTES];
    for (i, r) in arr.iter().take(n).enumerate() {
        routes[i] = ShapeRoute {
            shape: r.get("shape").and_then(|k| k.as_u64()).unwrap_or(0),
            locus: r.get("locus").and_then(|k| k.as_u64()).unwrap_or(0),
        };
    }
    let mut out = [0u64; MAX_SHAPE_ROUTES];
    let payload = if let Some(locus) = v.get("query_locus").and_then(Value::as_u64) {
        let k = shapes_for_locus(&routes[..n], locus, &mut out);
        json!({ "queryLocus": locus, "shapes": out[..k].to_vec() })
    } else if let Some(shape) = v.get("query_shape").and_then(Value::as_u64) {
        let k = loci_for_shape(&routes[..n], shape, &mut out);
        json!({ "queryShape": shape, "loci": out[..k].to_vec() })
    } else {
        return Err(McpSystemError::InvalidParameters);
    };
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}
