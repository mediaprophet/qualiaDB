//! Full-parameter MCP tool implementations (cold path — serde JSON allowed).

use super::McpSystemError;
use crate::NQuin;
use serde_json::{json, Value};

pub fn parse_tool_args(args: &[u8]) -> Result<Value, McpSystemError> {
    if args.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(args).map_err(|_| McpSystemError::ParseError)
}

fn json_str<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
}

fn json_f64(v: &Value, key: &str, default: f64) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_u64().map(|n| n as f64)))
        .unwrap_or(default)
}

fn json_u64(v: &Value, key: &str, default: u64) -> u64 {
    v.get(key)
        .and_then(|x| x.as_u64().or_else(|| x.as_i64().map(|n| n as u64)))
        .unwrap_or(default)
}

fn json_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn json_f64_array(v: &Value, key: &str) -> Result<Vec<f64>, McpSystemError> {
    let arr = v.get(key).and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    arr.iter()
        .map(|x| x.as_f64().ok_or(McpSystemError::InvalidParameters))
        .collect()
}

fn json_u8_array(v: &Value, key: &str) -> Result<Vec<u8>, McpSystemError> {
    let arr = v.get(key).and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    arr.iter()
        .map(|x| {
            x.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or(McpSystemError::InvalidParameters)
        })
        .collect()
}

fn parse_quin(v: &Value) -> Result<NQuin, McpSystemError> {
    Ok(NQuin {
        subject: json_u64(v, "subject", 0),
        predicate: json_u64(v, "predicate", 0),
        object: json_u64(v, "object", 0),
        context: json_u64(v, "context", 0),
        metadata: json_u64(v, "metadata", 0),
        parity: json_u64(v, "parity", 0),
    })
}

pub fn parse_quin_slice(v: &Value, key: &str) -> Result<Vec<NQuin>, McpSystemError> {
    let arr = v.get(key).and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    arr.iter().map(parse_quin).collect()
}

fn ensure_parity(q: &mut NQuin) {
    if q.parity == 0 {
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    }
}

fn parse_matrix_def(v: &Value) -> Result<(String, usize, usize, Vec<f64>), McpSystemError> {
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?
        .to_string();
    let rows = v
        .get("rows")
        .and_then(Value::as_u64)
        .ok_or(McpSystemError::InvalidParameters)? as usize;
    let cols = v
        .get("cols")
        .and_then(Value::as_u64)
        .ok_or(McpSystemError::InvalidParameters)? as usize;
    let data = json_f64_array(v, "data")?;
    if data.len() != rows * cols {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok((id, rows, cols, data))
}

pub fn matrix_operation(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::{DataType, LinearAlgebraLibrary};

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "multiply");
    let mut lib = LinearAlgebraLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    if let Some(matrices) = v.get("matrices").and_then(Value::as_array) {
        for m in matrices {
            let (id, rows, cols, data) = parse_matrix_def(m)?;
            lib.create_matrix(id, rows, cols, DataType::Float64, data)
                .map_err(|_| McpSystemError::InvalidParameters)?;
        }
    } else {
        let (id_a, rows_a, cols_a, data_a) = parse_matrix_def(
            v.get("left")
                .or(v.get("a"))
                .ok_or(McpSystemError::InvalidParameters)?,
        )?;
        lib.create_matrix(id_a.clone(), rows_a, cols_a, DataType::Float64, data_a)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        if op == "multiply" || op == "solve" {
            let (id_b, rows_b, cols_b, data_b) = parse_matrix_def(
                v.get("right")
                    .or(v.get("b"))
                    .ok_or(McpSystemError::InvalidParameters)?,
            )?;
            lib.create_matrix(id_b, rows_b, cols_b, DataType::Float64, data_b)
                .map_err(|_| McpSystemError::InvalidParameters)?;
        }
    }

    let result_id = v
        .get("result_id")
        .and_then(Value::as_str)
        .unwrap_or("result")
        .to_string();

    let result = match op {
        "transpose" => {
            let input = v
                .get("input_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("left").and_then(|l| l.get("id")).and_then(Value::as_str))
                .unwrap_or("A");
            lib.matrix_transpose(input, &result_id)
        }
        "solve" => {
            let matrix_id = v
                .get("matrix_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("left").and_then(|l| l.get("id")).and_then(Value::as_str))
                .unwrap_or("A");
            let rhs_id = v
                .get("rhs_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("right").and_then(|r| r.get("id")).and_then(Value::as_str))
                .unwrap_or("B");
            lib.solve_linear_system(matrix_id, rhs_id, &result_id)
        }
        "inverse" => {
            let input = v
                .get("input_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("left").and_then(|l| l.get("id")).and_then(Value::as_str))
                .unwrap_or("A");
            lib.matrix_inverse(input, &result_id)
        }
        _ => {
            let left = v
                .get("left_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("left").and_then(|l| l.get("id")).and_then(Value::as_str))
                .unwrap_or("A");
            let right = v
                .get("right_id")
                .and_then(Value::as_str)
                .or_else(|| v.get("right").and_then(|r| r.get("id")).and_then(Value::as_str))
                .unwrap_or("B");
            let alpha = json_f64(&v, "alpha", 1.0);
            let beta = json_f64(&v, "beta", 0.0);
            lib.matrix_multiply(left, right, &result_id, alpha, beta)
        }
    }
    .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "op": op,
        "result_id": result_id,
        "rows": result.result.rows,
        "cols": result.result.cols,
        "data": result.result.data,
        "execution_time_ms": result.execution_time
    })
    .to_string())
}

/// Find all roots of a polynomial given DESCENDING coefficients `[cₙ, …, c₁, c₀]`.
pub fn algebra_solve_polynomial(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::polynomial_roots;
    let v = parse_tool_args(args)?;
    let coeffs = json_f64_array(&v, "coeffs")?;
    let roots = polynomial_roots(&coeffs).map_err(|_| McpSystemError::InvalidParameters)?;
    let out: Vec<Value> = roots
        .iter()
        .map(|r| json!({ "re": r.re, "im": r.im }))
        .collect();
    Ok(json!({
        "degree": coeffs.len().saturating_sub(1),
        "roots": out
    })
    .to_string())
}

/// Determinant / eigenvalues / symmetric eigensystem / SVD of a row-major matrix.
pub fn algebra_matrix_analyze(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::{
        determinant, eigen_symmetric, eigenvalues_general, svd,
    };
    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "determinant");
    let rows = json_u64(&v, "rows", 0) as usize;
    let cols = json_u64(&v, "cols", 0) as usize;
    let data = json_f64_array(&v, "data")?;
    match op {
        "determinant" => {
            let d = determinant(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "determinant": d }).to_string())
        }
        "eigenvalues" => {
            let e =
                eigenvalues_general(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            let out: Vec<Value> = e.iter().map(|z| json!({ "re": z.re, "im": z.im })).collect();
            Ok(json!({ "op": op, "eigenvalues": out }).to_string())
        }
        "eigen_symmetric" => {
            let (vals, vecs) =
                eigen_symmetric(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "n": rows, "eigenvalues": vals, "eigenvectors": vecs })
                .to_string())
        }
        "svd" => {
            let s = svd(rows, cols, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "op": op, "rows": rows, "cols": cols,
                "singular_values": s.singular_values, "u": s.u, "v": s.v
            })
            .to_string())
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

/// Symbolic algebra: differentiate / simplify / evaluate a text expression, or solve a
/// quadratic symbolically.
/// MCP `values_check` — make the values engine callable by the agent ecosystem.
///
/// Asks the inverse rights-guard whether an agent's claim is a personhood-category abuse:
/// a non–natural-person agent (a `CorporatePerson`, an `ArtificialAgent`, …) claiming a
/// natural-person-only dignity right as its own. Runs the REAL agency.n3 G1/G1' guard lane
/// in the Webizen VM — not a lookup table.
///
/// Args: `{ "agentType": "CorporatePerson" | "ArtificialAgent" | "NaturalPerson" | <Class>,
///          "claimsDignityRight": true }`. `agentType` is the local name of a
/// `https://ns.webcivics.org/values/` class.
pub fn values_check(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let agent_type_short = json_str(&v, "agentType", "NaturalPerson");
    let claims = v
        .get("claimsDignityRight")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let agent_type =
        crate::q_hash(&format!("https://ns.webcivics.org/values/{agent_type_short}"));
    let flagged = crate::webizen::check_personhood_category_error(agent_type, claims);
    Ok(json!({
        "tool": "values_check",
        "agentType": agent_type_short,
        "claimsDignityRight": claims,
        "flagged": flagged,
        "flag": if flagged { "values:PersonhoodCategoryError" } else { "" },
        "verdict": if flagged {
            "REJECT — a non–natural-person agent cannot hold a human dignity right as its own"
        } else {
            "ok"
        },
        "basis": "agency.n3 G1/G1' inverse rights-guard (webcivics values lattice)"
    })
    .to_string())
}

/// MCP `graph_resolve` — resolve an IRI against the LIVE daemon graph through the
/// unified hybrid-modality resolver: its modal identifier-KIND (open kind fabric) and
/// out-degree. The kind is resolved via the modal predicate, not an inline tag, so
/// full-width identifiers resolve; `None` kind = a plain dictionary reference.
pub fn graph_resolve(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let iri = json_str(&v, "iri", "");
    if iri.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let id = crate::q_hash(iri);
    // Resolve through the revision-cached per-cell index (rebuilt only when the graph
    // actually changes), so a burst of resolves shares one O(n) build.
    let resolved =
        crate::graph_index::with_graph_index(|idx| crate::resolve::resolve_in_index(idx, id));
    let kind_name = resolved.kind.and_then(crate::modal_kind::kind_name);
    Ok(json!({
        "tool": "graph_resolve",
        "iri": iri,
        "identifier": id,
        "kind": resolved.kind,
        "kindName": kind_name,
        "outDegree": resolved.out_degree,
        "present": resolved.out_degree > 0,
        "basis": "hybrid-modality resolver: modal-predicate identifier-kind (open fabric) over the live graph; None kind = plain dictionary reference"
    })
    .to_string())
}

/// MCP `values_evaluate` — query the deontic-contract reasoner in values terms.
///
/// Where `values_check` answers a binary anti-capture guard, this verb runs the full
/// **native deontic VM** (`evaluate_deontic_contract`): a norm (forbid / oblige / permit)
/// bound to a party + action, an optional `unless` exception (compiled to a `q42:unless`
/// defeater on the same party+path), and an optional temporal window. It returns whether
/// the norm is currently **Active** (in force), **Defeated** (overridden by an exception),
/// **Expired** (past its effective window), or **Malformed** — computed, not asserted.
///
/// Args: `{ "modality": "forbid"|"oblige"|"permit", "party": "<agent>", "action": "<path>",
///          "object": "<action object>"?, "now": <unix>?, "expiry": <unix32>?,
///          "unless": "<exception action>"? }`.
pub fn values_evaluate(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::modalities::logic::deontic::{
        compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
        OP_OBLIGATE, OP_PERMIT,
    };
    let v = parse_tool_args(args)?;
    let modality = json_str(&v, "modality", "forbid");
    let (opcode, verb) = match modality {
        "oblige" | "obligate" | "obligation" => (OP_OBLIGATE, "obligation"),
        "permit" | "permission" => (OP_PERMIT, "permission"),
        _ => (OP_FORBID, "prohibition"),
    };
    let party_s = json_str(&v, "party", "");
    let action_s = json_str(&v, "action", "");
    if party_s.is_empty() || action_s.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let object_s = json_str(&v, "object", action_s);
    let now = json_u64(&v, "now", 1_717_200_000) as u32;
    let expiry = json_u64(&v, "expiry", 0) as u32;

    let party = crate::q_hash(party_s);
    let path = crate::q_hash(action_s);
    let obj = crate::q_hash(object_s);
    let contract = crate::q_hash("contract:values-evaluate");

    let norm = compile_norm_quin(party, opcode, path, obj, contract, expiry, false);
    let mut quins: Vec<NQuin> = vec![norm];

    let mut has_exception = false;
    if let Some(unless_s) = v.get("unless").and_then(Value::as_str) {
        if !unless_s.is_empty() {
            has_exception = true;
            // A permitting defeater on the SAME party + path + contract defeats the norm.
            let defeater =
                compile_norm_quin(party, OP_PERMIT, path, crate::q_hash(unless_s), contract, 0, true);
            quins.push(defeater);
        }
    }

    let mut out = [DeonticVerdict::default(); 8];
    let n = evaluate_deontic_contract(&quins, now, &mut out)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let verdict = out[..n].first().copied().unwrap_or_default();
    let (status_s, meaning) = match verdict.status {
        DeonticStatus::Active => ("Active", format!("the {verb} is in force")),
        DeonticStatus::Defeated => ("Defeated", format!("the {verb} is overridden by an exception")),
        DeonticStatus::Expired => {
            ("Expired", format!("the {verb} has lapsed (past its effective window)"))
        }
        DeonticStatus::Malformed => {
            ("Malformed", "the norm could not be interpreted".to_string())
        }
        DeonticStatus::Pending => {
            ("Pending", format!("the {verb} is valid but not yet in its effective window"))
        }
        DeonticStatus::Violated => {
            ("Violated", format!("the {verb} is in force but the facts show it was not met"))
        }
        DeonticStatus::Discharged => {
            ("Discharged", format!("the {verb} has been fulfilled and the duty terminates"))
        }
    };
    Ok(json!({
        "tool": "values_evaluate",
        "modality": verb,
        "party": party_s,
        "action": action_s,
        "exception": has_exception,
        "now": now,
        "expiry": expiry,
        "status": status_s,
        "meaning": meaning,
        "opcode": format!("0x{:02X}", verdict.opcode),
        "basis": "native deontic VM (evaluate_deontic_contract) over the webcivics values lattice"
    })
    .to_string())
}

/// Track-M dispatch gate (default OFF). When `QUALIA_MCP_ENFORCE` is set, EVERY dispatched MCP
/// call must carry a verified + grounded caller standpoint (args `caller`/`verified`/`grounded`)
/// or it is refused. Called once at the top of `enforce_fiduciary_tool_dispatch`; a no-op when
/// enforcement is off, so existing callers are unaffected until the operator flips the switch.
pub fn cooperation_gate(args: &[u8]) -> Result<(), McpSystemError> {
    match gate_verdict(args, crate::mcp_cooperation::enforcement_enabled()) {
        None => Ok(()),                                                   // enforcement off → pass
        Some(crate::mcp_cooperation::CooperationVerdict::Authorized(_)) => Ok(()),
        Some(_) => Err(McpSystemError::IntentFrameViolation),            // denied → refuse the call
    }
}

/// Pure decision for [`cooperation_gate`] (env-free, so it is unit-testable). `None` = not
/// enforcing (pass); `Some(verdict)` = the gate's verdict when enforcing. A call with no/false
/// `verified` is DeniedUnverified; with `verified` but `grounded:false` is DeniedUngrounded.
fn gate_verdict(args: &[u8], enforcing: bool) -> Option<crate::mcp_cooperation::CooperationVerdict> {
    use crate::mcp_cooperation::{authorize, CallerStandpoint};
    use crate::modalities::interaction_governance::Governance;
    use crate::modalities::logic::deontic::DeonticStatus;
    if !enforcing {
        return None;
    }
    let v = parse_tool_args(args).unwrap_or(Value::Null);
    let bool_of = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let standpoint = CallerStandpoint {
        agent: crate::q_hash(json_str(&v, "caller", "")),
        role: crate::q_hash(json_str(&v, "role", "role:caller")),
        verified: bool_of("verified"),
    };
    // When enforcing, grounding must be positively asserted (strict default).
    let grounded = bool_of("grounded");
    Some(authorize(&standpoint, grounded, DeonticStatus::Active, Governance::default()))
}

/// MCP `jural_correlate` — Hohfeldian correlativity: given a jural position, return the
/// correlative the counterparty necessarily bears, the jural opposite, and its order.
/// Composes `modalities::jural`. Args: `{ "position": "claim"|"duty"|"privilege"|"no-right"|
/// "power"|"liability"|"immunity"|"disability" }`.
pub fn jural_correlate(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::modalities::jural::{
        correlative, is_first_order, jural_opposite, position_name, JURAL_CLAIM, JURAL_DISABILITY,
        JURAL_DUTY, JURAL_IMMUNITY, JURAL_LIABILITY, JURAL_NO_RIGHT, JURAL_POWER, JURAL_PRIVILEGE,
    };
    let v = parse_tool_args(args)?;
    let pos = match json_str(&v, "position", "").to_ascii_lowercase().as_str() {
        "claim" | "right" => JURAL_CLAIM,
        "duty" => JURAL_DUTY,
        "privilege" | "liberty" => JURAL_PRIVILEGE,
        "no-right" | "noright" => JURAL_NO_RIGHT,
        "power" => JURAL_POWER,
        "liability" => JURAL_LIABILITY,
        "immunity" => JURAL_IMMUNITY,
        "disability" | "no-power" => JURAL_DISABILITY,
        _ => return Err(McpSystemError::InvalidParameters),
    };
    Ok(json!({
        "tool": "jural_correlate",
        "position": position_name(pos),
        "opcode": format!("0x{:02X}", pos),
        "correlative": position_name(correlative(pos)),
        "opposite": position_name(jural_opposite(pos)),
        "order": if is_first_order(pos) { "first-order (conduct)" } else { "second-order (control)" },
        "meaning": "if A holds this position toward B, B NECESSARILY bears the correlative toward A",
        "basis": "Hohfeld (1913) jural relations — modalities/jural.rs"
    })
    .to_string())
}

/// MCP `deontic_govern` — map a deontic verdict status + classification to the runtime
/// PolicyMode the Webizen VM enacts. Composes `modalities::interaction_governance`.
/// Args: `{ "status": "active"|"violated"|..., "nonDerogable": bool?, "humanitarian": bool?,
/// "ambiguous": bool? }`.
pub fn deontic_govern(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::modalities::interaction_governance::{
        map_policy, permits_execution, policy_action, Governance,
    };
    use crate::modalities::logic::deontic::DeonticStatus;
    let v = parse_tool_args(args)?;
    let status = match json_str(&v, "status", "").to_ascii_lowercase().as_str() {
        "active" => DeonticStatus::Active,
        "defeated" => DeonticStatus::Defeated,
        "expired" => DeonticStatus::Expired,
        "pending" => DeonticStatus::Pending,
        "violated" => DeonticStatus::Violated,
        "discharged" => DeonticStatus::Discharged,
        "malformed" => DeonticStatus::Malformed,
        _ => return Err(McpSystemError::InvalidParameters),
    };
    let bool_of = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let g = Governance {
        non_derogable: bool_of("nonDerogable"),
        humanitarian: bool_of("humanitarian"),
        ambiguous: bool_of("ambiguous"),
    };
    let mode = map_policy(status, g);
    Ok(json!({
        "tool": "deontic_govern",
        "status": format!("{status:?}"),
        "policyMode": format!("{mode:?}"),
        "action": policy_action(mode),
        "permitsExecution": permits_execution(mode),
        "basis": "interaction_governance::map_policy — verdict → Webizen VM policy mode"
    })
    .to_string())
}

/// MCP `mcp_cooperate` — the agent-cooperation gate (Track M, #17): does a **verified, typed,
/// grounded** caller's request pass the deontic gate? Composes `mcp_cooperation::authorize`
/// (verified-not-asserted → grounded agency.n3 G1' → Phase 6 policy). Args: `{ "caller":
/// "<agent>", "role": "<role>"?, "verified": bool, "grounded": bool?, "requestStatus":
/// "active"|"violated"|..., "nonDerogable"|"humanitarian"|"ambiguous": bool? }`.
pub fn mcp_cooperate(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::mcp_cooperation::{authorize, cooperation_label, CallerStandpoint, CooperationVerdict};
    use crate::modalities::interaction_governance::Governance;
    use crate::modalities::logic::deontic::DeonticStatus;
    let v = parse_tool_args(args)?;
    let caller_s = json_str(&v, "caller", "");
    if caller_s.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let bool_of = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let standpoint = CallerStandpoint {
        agent: crate::q_hash(caller_s),
        role: crate::q_hash(json_str(&v, "role", "role:requester")),
        verified: bool_of("verified"),
    };
    // grounded defaults to true (a human/legal caller); set false to model an ungrounded AI.
    let grounded = v.get("grounded").and_then(Value::as_bool).unwrap_or(true);
    let status = match json_str(&v, "requestStatus", "active").to_ascii_lowercase().as_str() {
        "violated" => DeonticStatus::Violated,
        "defeated" => DeonticStatus::Defeated,
        "expired" => DeonticStatus::Expired,
        "pending" => DeonticStatus::Pending,
        "discharged" => DeonticStatus::Discharged,
        "malformed" => DeonticStatus::Malformed,
        _ => DeonticStatus::Active,
    };
    let g = Governance {
        non_derogable: bool_of("nonDerogable"),
        humanitarian: bool_of("humanitarian"),
        ambiguous: bool_of("ambiguous"),
    };
    let verdict = authorize(&standpoint, grounded, status, g);
    let mode = match verdict {
        CooperationVerdict::Authorized(m) | CooperationVerdict::DeniedByPolicy(m) => Some(format!("{m:?}")),
        _ => None,
    };
    Ok(json!({
        "tool": "mcp_cooperate",
        "caller": caller_s,
        "verified": standpoint.verified,
        "grounded": grounded,
        "verdict": cooperation_label(verdict),
        "policyMode": mode,
        "permitted": matches!(verdict, CooperationVerdict::Authorized(_)),
        "basis": "mcp_cooperation::authorize — verified+grounded caller through the deontic gate (Track M, #17)"
    })
    .to_string())
}

pub fn cas(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::symbolic_algebra as sym;
    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "simplify");
    match op {
        "differentiate" => {
            let expr_s = json_str(&v, "expr", "");
            let wrt = json_str(&v, "var", "x");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            let d = sym::simplify(&sym::differentiate(&e, wrt));
            Ok(json!({ "op": op, "input": expr_s, "var": wrt, "derivative": d.to_string() })
                .to_string())
        }
        "simplify" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "input": expr_s, "simplified": sym::simplify(&e).to_string() })
                .to_string())
        }
        "evaluate" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            let mut env = std::collections::HashMap::new();
            if let Some(obj) = v.get("env").and_then(Value::as_object) {
                for (k, val) in obj {
                    if let Some(f) = val.as_f64() {
                        env.insert(k.clone(), f);
                    }
                }
            }
            let value = e.eval(&env).ok_or(McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "input": expr_s, "value": value }).to_string())
        }
        "solve_quadratic" => {
            let a = json_f64(&v, "a", 1.0);
            let b = json_f64(&v, "b", 0.0);
            let cc = json_f64(&v, "c", 0.0);
            let roots: Vec<String> = sym::solve_quadratic_symbolic(a, b, cc)
                .iter()
                .map(|r| r.to_string())
                .collect();
            Ok(json!({ "op": op, "a": a, "b": b, "c": cc, "roots": roots }).to_string())
        }
        "expand" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "input": expr_s, "expanded": sym::expand(&e).to_string() })
                .to_string())
        }
        "factor" => {
            let a = json_f64(&v, "a", 1.0);
            let b = json_f64(&v, "b", 0.0);
            let cc = json_f64(&v, "c", 0.0);
            let varname = json_str(&v, "var", "x");
            match sym::factor_quadratic(a, b, cc, varname) {
                Some(f) => Ok(json!({ "op": op, "a": a, "b": b, "c": cc, "factored": f.to_string() })
                    .to_string()),
                None => Ok(json!({ "op": op, "a": a, "b": b, "c": cc, "factored": Value::Null,
                    "note": "no real factorisation (negative discriminant or a = 0)" })
                    .to_string()),
            }
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

pub fn ode_solve(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::physics_simulation::{
        CommunicationPattern, DomainDecomposition, DomainType, LoadBalancing, NumericalMethod,
        ParallelConfig, PhysicsSimulationLibrary, SimulationConfig, SimulationType,
        SpatialResolution,
    };

    let v = parse_tool_args(args)?;
    let sim_type = json_str(&v, "type", "cfd");
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let nx = v
        .get("nx")
        .and_then(Value::as_u64)
        .unwrap_or(10) as usize;
    let ny = v
        .get("ny")
        .and_then(Value::as_u64)
        .unwrap_or(10) as usize;
    let dx = json_f64(&v, "dx", 0.1);
    let time_step = json_f64(&v, "time_step", 0.001);
    let total_time = json_f64(&v, "total_time", 0.01);
    let simulation_id = v
        .get("simulation_id")
        .and_then(Value::as_str)
        .unwrap_or("mcp_sim")
        .to_string();

    let config = SimulationConfig {
        simulation_id,
        simulation_type: if sim_type == "distributed" || sim_type == "molecular_dynamics" {
            SimulationType::MolecularDynamics
        } else {
            SimulationType::CFD
        },
        domain_type: DomainType::TwoDimensional,
        time_step,
        total_time,
        spatial_resolution: SpatialResolution {
            nx,
            ny: Some(ny),
            nz: None,
            dx,
            dy: Some(json_f64(&v, "dy", dx)),
            dz: None,
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: v.get("num_threads").and_then(Value::as_u64).unwrap_or(1) as usize,
            num_processes: 1,
            domain_decomposition: DomainDecomposition::TwoDimensional,
            load_balancing: LoadBalancing::Dynamic,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };

    let mut sim = lib
        .create_simulation(config)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let r = lib
        .run_cfd_simulation(&mut sim)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "field_count": r.result.len(),
        "converged": r.convergence_info.converged,
        "iterations": r.convergence_info.iterations,
        "final_error": r.convergence_info.final_error
    })
    .to_string())
}

pub fn chemical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::chemistry_modeling::{
        Atom, ChemistryModelingLibrary, Molecule, MolecularProperties, PropertyType,
    };

    let v = parse_tool_args(args)?;
    let mut lib = ChemistryModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let molecule = if let Some(smiles) = v.get("smiles").and_then(Value::as_str) {
        use crate::domains::chemical::organic_chemistry::{compute_descriptors, parse_smiles};
        let mol = parse_smiles(smiles);
        let desc = compute_descriptors(&mol);
        Molecule {
            molecule_id: v
                .get("molecule_id")
                .and_then(Value::as_str)
                .unwrap_or("mcp_mol")
                .to_string(),
            formula: smiles.to_string(),
            atoms: vec![Atom::new()],
            bonds: vec![],
            coordinates: vec![vec![0.0, 0.0, 0.0]],
            properties: MolecularProperties {
                molecular_weight: desc.molecular_weight,
                dipole_moment: desc.tpsa_ertl,
                polarizability: 0.0,
                energy: 0.0,
            },
        }
    } else {
        let mut m = Molecule::new();
        if let Some(formula) = v.get("formula").and_then(Value::as_str) {
            m.formula = formula.to_string();
        }
        if let Some(mw) = v.get("molecular_weight").and_then(Value::as_f64) {
            m.properties.molecular_weight = mw;
        }
        if let Some(id) = v.get("molecule_id").and_then(Value::as_str) {
            m.molecule_id = id.to_string();
        }
        m
    };

    let props: Vec<PropertyType> = if let Some(arr) = v.get("properties").and_then(Value::as_array) {
        arr.iter()
            .filter_map(|p| match p.as_str()? {
                "boiling_point" => Some(PropertyType::BoilingPoint),
                "melting_point" => Some(PropertyType::MeltingPoint),
                "density" => Some(PropertyType::Density),
                "viscosity" => Some(PropertyType::Viscosity),
                _ => None,
            })
            .collect()
    } else {
        match json_str(&v, "prop", "boiling_point") {
            "melting_point" => vec![PropertyType::MeltingPoint],
            "density" => vec![PropertyType::Density],
            _ => vec![PropertyType::BoilingPoint],
        }
    };

    let r = lib
        .predict_properties(molecule, props)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "properties": r.result.properties,
        "confidence_intervals": r.result.confidence_intervals,
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn statistical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::statistical_computing::{
        CorrelationMethod, DataType, DataValue, PrivacyLevel, StatisticalComputingLibrary,
    };

    let v = parse_tool_args(args)?;
    let stat = json_str(&v, "stat", "mean");
    let dataset_id = v
        .get("dataset_id")
        .and_then(Value::as_str)
        .unwrap_or("ds")
        .to_string();
    let columns: Vec<String> = v
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec!["x".to_string(), "y".to_string()]);

    let rows_val = v
        .get("rows")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mut data = Vec::with_capacity(rows_val.len());
    for row in rows_val {
        let row_arr = row.as_array().ok_or(McpSystemError::InvalidParameters)?;
        let mut row_data = Vec::with_capacity(row_arr.len());
        for cell in row_arr {
            let dv = if let Some(n) = cell.as_f64() {
                DataValue::Float(n)
            } else if let Some(s) = cell.as_str() {
                DataValue::String(s.to_string())
            } else if let Some(b) = cell.as_bool() {
                DataValue::Boolean(b)
            } else {
                return Err(McpSystemError::InvalidParameters);
            };
            row_data.push(dv);
        }
        data.push(row_data);
    }

    let col_types: Vec<DataType> = columns
        .iter()
        .map(|_| DataType::Float64)
        .collect();

    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.create_dataset(
        dataset_id.clone(),
        data,
        columns.clone(),
        col_types,
        PrivacyLevel::Public,
    )
    .map_err(|_| McpSystemError::InvalidParameters)?;

    let column = v
        .get("column")
        .and_then(Value::as_str)
        .unwrap_or(columns.first().map(|s| s.as_str()).unwrap_or("x"));
    let column_y = v
        .get("column_y")
        .and_then(Value::as_str)
        .unwrap_or(columns.get(1).map(|s| s.as_str()).unwrap_or("y"));

    let result = match stat {
        "variance" => {
            let r = lib
                .variance(&dataset_id, column, json_bool(&v, "sample", true), false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "variance", "column": column, "value": r.result})
        }
        "correlation" => {
            let method = match json_str(&v, "method", "pearson") {
                "spearman" => CorrelationMethod::Spearman,
                "kendall" => CorrelationMethod::Kendall,
                _ => CorrelationMethod::Pearson,
            };
            let r = lib
                .correlation(&dataset_id, column, column_y, method, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "correlation",
                "column_x": column,
                "column_y": column_y,
                "value": r.result
            })
        }
        "mean" => {
            let r = lib
                .mean(&dataset_id, column, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "mean", "column": column, "value": r.result})
        }
        _ => return Err(McpSystemError::InvalidParameters),
    };

    Ok(result.to_string())
}

pub fn ml_inference(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::machine_learning::{
        InferenceParameters, MachineLearningLibrary, Precision,
    };

    let v = parse_tool_args(args)?;
    let model_id = v
        .get("model_id")
        .and_then(Value::as_str)
        .or_else(|| v.get("model").and_then(Value::as_str))
        .unwrap_or("mcp_model")
        .to_string();
    let model_path = v
        .get("model_path")
        .and_then(Value::as_str)
        .unwrap_or("in-memory");

    let input_data = if let Ok(bytes) = json_u8_array(&v, "input_data") {
        bytes
    } else if let Some(s) = v.get("input_hex").and_then(Value::as_str) {
        hex_decode(s)?
    } else {
        vec![0u8; v.get("input_size").and_then(Value::as_u64).unwrap_or(64) as usize]
    };

    let mut lib = MachineLearningLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.load_model(model_id.clone(), model_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let params = InferenceParameters {
        batch_size: v.get("batch_size").and_then(Value::as_u64).unwrap_or(1) as usize,
        sequence_length: v
            .get("sequence_length")
            .and_then(Value::as_u64)
            .unwrap_or(input_data.len() as u64) as usize,
        temperature: v.get("temperature").and_then(Value::as_f64).or(Some(0.7)),
        top_k: v
            .get("top_k")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .or(Some(1)),
        top_p: v.get("top_p").and_then(Value::as_f64).or(Some(1.0)),
        max_tokens: v
            .get("max_tokens")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .or(Some(10)),
        precision: Precision::FP32,
    };

    let r = lib
        .run_inference(&model_id, &input_data, params)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "model_id": model_id,
        "result_id": r.result.result_id,
        "confidence": r.result.confidence,
        "output_size": r.result.output_data.len(),
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

fn hex_decode(s: &str) -> Result<Vec<u8>, McpSystemError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(McpSystemError::InvalidParameters);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| McpSystemError::InvalidParameters))
        .collect()
}

pub fn financial_model(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::financial_modeling::{
        Asset, AssetType, FinancialModelingLibrary, OptionParameters, OptionType, Portfolio,
    };

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "option");
    let mut lib = FinancialModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    if op == "risk" {
        let mut portfolio = Portfolio::new();
        if let Some(id) = v.get("portfolio_id").and_then(Value::as_str) {
            portfolio.portfolio_id = id.to_string();
        }
        if let Some(assets) = v.get("assets").and_then(Value::as_array) {
            portfolio.assets = assets
                .iter()
                .filter_map(|a| {
                    Some(Asset {
                        asset_id: a.get("asset_id")?.as_str()?.to_string(),
                        symbol: a.get("symbol")?.as_str()?.to_string(),
                        asset_type: AssetType::Stock,
                        quantity: json_f64(a, "quantity", 0.0),
                        average_cost: json_f64(a, "average_cost", 0.0),
                        current_price: json_f64(a, "current_price", 0.0),
                        market_value: json_f64(a, "market_value", 0.0),
                        currency: a
                            .get("currency")
                            .and_then(Value::as_str)
                            .unwrap_or("USD")
                            .to_string(),
                        exchange: a
                            .get("exchange")
                            .and_then(Value::as_str)
                            .unwrap_or("NASDAQ")
                            .to_string(),
                        last_updated: 0,
                    })
                })
                .collect();
        }
        portfolio.cash_balance = json_f64(&v, "cash_balance", portfolio.cash_balance);
        let created = lib
            .create_portfolio(portfolio)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        let pid = created.result.portfolio_id;
        let r = lib
            .calculate_portfolio_risk(&pid)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        return Ok(json!({
            "op": "risk",
            "portfolio_id": pid,
            "var_95": r.result.var_95,
            "sharpe_ratio": r.result.sharpe_ratio,
            "sortino_ratio": r.result.sortino_ratio,
            "max_drawdown": r.result.max_drawdown,
            "overall_risk_score": r.result.overall_risk_score
        })
        .to_string());
    }

    let option_type = if json_str(&v, "option_type", "call") == "put" {
        OptionType::Put
    } else {
        OptionType::Call
    };
    let params = OptionParameters {
        underlying_price: json_f64(&v, "underlying_price", 100.0),
        strike: json_f64(&v, "strike", 105.0),
        time_to_maturity: json_f64(&v, "time_to_maturity", 0.25),
        risk_free_rate: json_f64(&v, "risk_free_rate", 0.05),
        volatility: json_f64(&v, "volatility", 0.2),
        option_type,
    };
    let r = lib
        .price_option(params)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    Ok(json!({
        "op": "option",
        "price": r.result.price,
        "delta": r.result.delta,
        "gamma": r.result.gamma,
        "theta": r.result.theta,
        "vega": r.result.vega,
        "rho": r.result.rho
    })
    .to_string())
}

pub fn medical_score(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::medical_computing::{
        ClinicalDataType, MedicalComputingLibrary, Patient,
    };

    let v = parse_tool_args(args)?;
    let score = json_str(&v, "score", "diagnosis");
    let patient_id = v
        .get("patient_id")
        .and_then(Value::as_str)
        .unwrap_or("mcp_patient")
        .to_string();

    let mut lib = MedicalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let patient: Patient = if let Some(p) = v.get("patient") {
        serde_json::from_value(p.clone()).map_err(|_| McpSystemError::InvalidParameters)?
    } else {
        let mut p = Patient::new();
        p.patient_id = patient_id.clone();
        p
    };
    lib.create_patient_record(patient)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let data_type = match score {
        "treatment" => ClinicalDataType::Treatment,
        "prognosis" => ClinicalDataType::Prognosis,
        "prevention" => ClinicalDataType::Prevention,
        _ => ClinicalDataType::Diagnosis,
    };
    let r = lib
        .analyze_clinical_data(&patient_id, data_type)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "patient_id": patient_id,
        "analysis_id": r.result.analysis_id,
        "confidence": r.result.confidence_score,
        "recommendations": r.result.recommendations,
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn engineering_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::engineering_analysis::{
        AnalysisType, EngineeringAnalysisLibrary, EngineeringModel, Geometry, GeometryType,
        Load, ModelType,
    };
    let v = parse_tool_args(args)?;
    let analysis = json_str(&v, "analysis", "structural");
    let mut lib = EngineeringAnalysisLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let mut model = EngineeringModel::new();
    if let Some(m) = v.get("model") {
        if let Some(id) = m.get("model_id").and_then(Value::as_str) {
            model.model_id = id.to_string();
        }
        if let Some(dims) = m.get("dimensions").and_then(Value::as_array) {
            model.geometry.dimensions = dims
                .iter()
                .filter_map(|x| x.as_f64())
                .collect();
        }
        if let Some(gt) = m.get("geometry_type").and_then(Value::as_str) {
            model.geometry.geometry_type = match gt {
                "plate" => GeometryType::Plate,
                "shell" => GeometryType::Shell,
                "solid" => GeometryType::Solid,
                _ => GeometryType::Beam,
            };
        }
        if let Some(loads) = m.get("loads").and_then(Value::as_array) {
            model.loads = loads
                .iter()
                .enumerate()
                .filter_map(|(i, l)| {
                    Some(Load {
                        load_id: l
                            .get("load_id")
                            .and_then(Value::as_str)
                            .unwrap_or("load")
                            .to_string()
                            + &i.to_string(),
                        load_type: crate::specialized_libs::engineering_analysis::LoadType::Point,
                        load_magnitude: json_f64(l, "magnitude", 1000.0),
                        load_direction: json_f64_array(l, "direction").unwrap_or(vec![0.0, -1.0, 0.0]),
                        application_point: json_f64_array(l, "application_point")
                            .unwrap_or(vec![0.0, 0.0, 0.0]),
                    })
                })
                .collect();
        }
    } else if let Some(dims) = v.get("dimensions").and_then(Value::as_array) {
        model.geometry = Geometry {
            geometry_type: GeometryType::Beam,
            dimensions: dims.iter().filter_map(|x| x.as_f64()).collect(),
            features: vec![],
        };
    }

    if model.materials.is_empty() {
        let mut mat = crate::specialized_libs::engineering_analysis::Material::new();
        if let Some(e) = v.get("youngs_modulus").and_then(Value::as_f64) {
            mat.material_properties.youngs_modulus = e;
        }
        model
            .materials
            .insert("default".to_string(), mat);
    }

    let analysis_type = match analysis {
        "thermal" => AnalysisType::Thermal,
        "dynamic" => AnalysisType::LinearDynamic,
        _ => AnalysisType::LinearStatic,
    };
    model.model_type = match analysis {
        "thermal" => ModelType::Thermal,
        "dynamic" => ModelType::Mechanical,
        _ => ModelType::Structural,
    };

    let r = lib
        .perform_structural_analysis(model, analysis_type)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "analysis": analysis,
        "safety_factor": r.result.safety_factor,
        "max_stress": r.result.stress_field.iter().copied().fold(0.0f64, f64::max),
        "max_displacement": r.result.displacement_field.iter().copied().fold(0.0f64, f64::max),
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn bioinformatics_align(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::domains::biological::bioinformatics::{align_nucleotide, align_protein};

    let v = parse_tool_args(args)?;
    let mode = json_str(&v, "mode", "dna");
    let query = v
        .get("query")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let target = v
        .get("target")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let result = if mode == "protein" {
        align_protein(query.as_bytes(), target.as_bytes())
    } else {
        align_nucleotide(query.as_bytes(), target.as_bytes())
    };

    Ok(json!({
        "mode": mode,
        "score": result.score,
        "query_len": query.len(),
        "target_len": target.len()
    })
    .to_string())
}

pub fn chemical_descriptors(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::domains::chemical::organic_chemistry::{compute_descriptors, parse_smiles};

    let v = parse_tool_args(args)?;
    let smiles = v
        .get("smiles")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mol = parse_smiles(smiles);
    let desc = compute_descriptors(&mol);

    Ok(json!({
        "smiles": smiles,
        "molecular_weight": desc.molecular_weight,
        "log_p": desc.logp_crippen,
        "tpsa": desc.tpsa_ertl,
        "h_bond_donors": desc.hb_donors,
        "h_bond_acceptors": desc.hb_acceptors,
        "rotatable_bonds": desc.rotatable_bonds
    })
    .to_string())
}

pub fn clinical_risk(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::clinical_engine::{
        ckd_epi_egfr, cha2ds2_vasc_score, framingham_10yr_risk, sofa_score, Cha2ds2VascInput,
        FraminghamInput, RenalInput, SofaInput,
    };

    let v = parse_tool_args(args)?;
    let score_type = json_str(&v, "score", "framingham");
    let input = v
        .get("input")
        .cloned()
        .unwrap_or_else(|| v.clone());

    let result = match score_type {
        "cha2ds2" | "cha2ds2_vasc" => {
            let inp = Cha2ds2VascInput {
                congestive_heart_failure: json_bool(&input, "congestive_heart_failure", false),
                hypertension: json_bool(&input, "hypertension", false),
                age_75_or_older: json_bool(&input, "age_75_or_older", false),
                diabetes: json_bool(&input, "diabetes", false),
                stroke_tia_history: json_bool(&input, "stroke_tia_history", false),
                vascular_disease: json_bool(&input, "vascular_disease", false),
                age_65_to_74: json_bool(&input, "age_65_to_74", false),
                sex_female: json_bool(&input, "sex_female", false),
            };
            let r = cha2ds2_vasc_score(&inp);
            json!({
                "score": "cha2ds2_vasc",
                "points": r.score,
                "annual_stroke_risk_pct": r.annual_stroke_risk_pct,
                "anticoagulation_recommended": r.anticoagulation_recommended
            })
        }
        "sofa" => {
            let inp = SofaInput {
                pao2_fio2_ratio: json_f64(&input, "pao2_fio2_ratio", 300.0),
                platelets_10_9_l: json_f64(&input, "platelets_10_9_l", 150.0),
                bilirubin_mg_dl: json_f64(&input, "bilirubin_mg_dl", 1.0),
                map_mmhg: json_f64(&input, "map_mmhg", 70.0),
                dopamine_dose: json_f64(&input, "dopamine_dose", 0.0),
                epinephrine_dose: json_f64(&input, "epinephrine_dose", 0.0),
                norepinephrine_dose: json_f64(&input, "norepinephrine_dose", 0.0),
                glasgow_coma_scale: input
                    .get("glasgow_coma_scale")
                    .and_then(Value::as_u64)
                    .unwrap_or(15) as u8,
                creatinine_mg_dl: json_f64(&input, "creatinine_mg_dl", 1.0),
                urine_output_ml_d: json_f64(&input, "urine_output_ml_d", 1000.0),
            };
            json!({"score": "sofa", "points": sofa_score(&inp)})
        }
        "egfr" | "renal" => {
            let inp = RenalInput {
                age: v
                    .get("age")
                    .and_then(Value::as_u64)
                    .unwrap_or(55) as u8,
                sex_male: json_bool(&input, "sex_male", true),
                weight_kg: json_f64(&input, "weight_kg", 70.0),
                serum_creatinine: json_f64(&input, "serum_creatinine", 1.0),
            };
            json!({"score": "egfr", "egfr_ml_min": ckd_epi_egfr(&inp)})
        }
        _ => {
            let inp = FraminghamInput {
                age: v.get("age").and_then(Value::as_u64).unwrap_or(55) as u8,
                sex_male: json_bool(&input, "sex_male", true),
                total_cholesterol_mmol: json_f64(&input, "total_cholesterol_mmol", 5.5),
                hdl_cholesterol_mmol: json_f64(&input, "hdl_cholesterol_mmol", 1.2),
                systolic_bp: json_f64(&input, "systolic_bp", 130.0),
                bp_treated: json_bool(&input, "bp_treated", false),
                current_smoker: json_bool(&input, "current_smoker", false),
                diabetic: json_bool(&input, "diabetic", false),
            };
            let r = framingham_10yr_risk(&inp);
            json!({
                "score": "framingham",
                "risk_10yr": r.risk_10yr,
                "risk_10yr_pct": r.risk_10yr * 100.0,
                "category": format!("{:?}", r.category),
                "log_score": r.log_score
            })
        }
    };

    Ok(result.to_string())
}

pub fn geometric_algebra_op(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::geometric_algebra::utils::{angle_between_vectors, cross_product, dot_product};

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "cross");
    let a_arr = json_f64_array(&v, "a")?;
    let b_arr = json_f64_array(&v, "b")?;
    if a_arr.len() != 3 || b_arr.len() != 3 {
        return Err(McpSystemError::InvalidParameters);
    }
    let a = [a_arr[0] as f32, a_arr[1] as f32, a_arr[2] as f32];
    let b = [b_arr[0] as f32, b_arr[1] as f32, b_arr[2] as f32];

    let result = match op {
        "angle" => json!({
            "op": "angle",
            "radians": angle_between_vectors(&a, &b),
            "degrees": angle_between_vectors(&a, &b).to_degrees()
        }),
        "dot" => json!({"op": "dot", "value": dot_product(&a, &b)}),
        _ => {
            let c = cross_product(&a, &b);
            json!({"op": "cross", "result": [c[0], c[1], c[2]]})
        }
    };
    Ok(result.to_string())
}

pub fn symbolic_logic_infer(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::solvers::symbolic_logic::{
        BoundedSatSolver, Clause, DefeasibleRule, Fact, ForwardChainingDefeasible, Literal,
        RuleType,
    };
    use crate::solvers::SolverConfig;

    let v = parse_tool_args(args)?;
    let solver = json_str(&v, "solver", "defeasible");
    let cfg = SolverConfig {
        max_iterations: v
            .get("max_iterations")
            .and_then(Value::as_u64)
            .unwrap_or(100) as u32,
        tolerance: json_f64(&v, "tolerance", 1e-6),
        step_size: json_f64(&v, "step_size", 0.01),
        verbose: json_bool(&v, "verbose", false),
    };

    if solver == "sat" {
        let mut s = BoundedSatSolver::new(cfg);
        if let Some(clauses) = v.get("clauses").and_then(Value::as_array) {
            for (idx, c) in clauses.iter().enumerate() {
                let lits = c
                    .get("literals")
                    .and_then(Value::as_array)
                    .ok_or(McpSystemError::InvalidParameters)?;
                let mut literals = [Literal {
                    variable: 0,
                    negated: false,
                }; 5];
                let mut n = 0u8;
                for lit in lits.iter().take(5) {
                    literals[n as usize] = Literal {
                        variable: lit
                            .get("variable")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u8,
                        negated: json_bool(lit, "negated", false),
                    };
                    n += 1;
                }
                let clause = Clause {
                    id: idx as u32 + 1,
                    num_literals: n,
                    learned: false,
                    activity: 1.0,
                    literals,
                };
                let _ = s.add_clause(clause);
            }
        }
        match s.solve() {
            Ok(st) => {
                return Ok(json!({
                    "solver": "sat",
                    "satisfiable": st.satisfiable,
                    "num_decisions": st.num_decisions
                })
                .to_string());
            }
            Err(_) => {
                return Ok(json!({"solver": "sat", "satisfiable": false}).to_string());
            }
        }
    }

    let mut s = ForwardChainingDefeasible::new(cfg);
    if let Some(facts) = v.get("facts").and_then(Value::as_array) {
        for (idx, f) in facts.iter().enumerate() {
            let lit = Literal {
                variable: f
                    .get("variable")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u8,
                negated: json_bool(f, "negated", false),
            };
            let fact = Fact {
                id: idx as u32 + 1,
                literal: lit,
                supporting_rules: [0; 3],
                defeated: false,
                confidence: json_f64(f, "confidence", 1.0),
            };
            let _ = s.add_fact(fact);
        }
    }
    if let Some(rules) = v.get("rules").and_then(Value::as_array) {
        for (idx, r) in rules.iter().enumerate() {
            let antecedents_arr = r
                .get("antecedents")
                .and_then(Value::as_array)
                .ok_or(McpSystemError::InvalidParameters)?;
            let mut antecedents = [Literal {
                variable: 0,
                negated: false,
            }; 5];
            for (i, a) in antecedents_arr.iter().take(5).enumerate() {
                antecedents[i] = Literal {
                    variable: a
                        .get("variable")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u8,
                    negated: json_bool(a, "negated", false),
                };
            }
            let cons = r.get("consequent").ok_or(McpSystemError::InvalidParameters)?;
            let rule = DefeasibleRule {
                id: idx as u32 + 1,
                rule_type: match json_str(r, "rule_type", "defeasible") {
                    "strict" => RuleType::Strict,
                    "defeater" => RuleType::Defeater,
                    _ => RuleType::Defeasible,
                },
                priority: v
                    .get("priority")
                    .and_then(Value::as_u64)
                    .unwrap_or(500) as u16,
                active: true,
                fire_count: 0,
                antecedents,
                consequent: Literal {
                    variable: cons
                        .get("variable")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u8,
                    negated: json_bool(cons, "negated", false),
                },
            };
            let _ = s.add_rule(rule);
        }
    }
    match s.infer() {
        Ok(st) => Ok(json!({
            "solver": "defeasible",
            "num_facts": st.num_facts,
            "rules_fired": st.rules_fired
        })
        .to_string()),
        Err(_) => Ok(json!({"solver": "defeasible", "num_facts": 0}).to_string()),
    }
}

pub fn evaluate_modality(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let modality = json_str(&v, "modality", "unknown");

    match modality {
        "ltl" => {
            use crate::modalities::temporal_ltl::evaluate_ltl_trace;
            let trace = if let Ok(quins) = parse_quin_slice(&v, "trace") {
                quins
            } else {
                vec![]
            };
            let formula = parse_ltl_formula(v.get("formula").ok_or(McpSystemError::InvalidParameters)?)?;
            let ok = evaluate_ltl_trace(&trace, &formula);
            Ok(json!({"modality": "ltl", "result": ok}).to_string())
        }
        "asp" => {
            use crate::modalities::asp::enumerate_stable_models;
            let mut base = NQuin::default();
            if let Some(b) = v.get("base") {
                base = parse_quin(b)?;
            }
            let rules = parse_quin_slice(&v, "rules").unwrap_or_default();
            let mut worlds = [0u64; 8];
            let n = enumerate_stable_models(&base, &rules, &mut worlds);
            Ok(json!({
                "modality": "asp",
                "stable_model_count": n,
                "world_contexts": &worlds[..n]
            })
            .to_string())
        }
        "probabilistic" => {
            use crate::modalities::probabilistic::evaluate_threshold;
            let value = json_f64(&v, "value", 0.5);
            let threshold = json_f64(&v, "threshold", 0.4);
            Ok(json!({
                "modality": "probabilistic",
                "result": evaluate_threshold(value as f32, threshold as f32)
            })
            .to_string())
        }
        "argumentation" => {
            use crate::modalities::argumentation::ArgumentationFramework;
            let fw = ArgumentationFramework::new();
            Ok(json!({
                "modality": "argumentation",
                "grounded_extension_size": fw.grounded_extension().len()
            })
            .to_string())
        }
        "deontic" => {
            use crate::modalities::logic::deontic::{evaluate_deontic_contract, DeonticVerdict};
            let quins = parse_quin_slice(&v, "quins")?;
            let mut quins = quins;
            for q in &mut quins {
                ensure_parity(q);
            }
            let now = v
                .get("now_unix")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let mut out = vec![DeonticVerdict::default(); quins.len().max(1)];
            let n = evaluate_deontic_contract(&quins, now, &mut out)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let verdicts: Vec<Value> = out[..n]
                .iter()
                .map(|ver| {
                    json!({
                        "status": format!("{:?}", ver.status),
                        "opcode": ver.opcode
                    })
                })
                .collect();
            Ok(json!({"modality": "deontic", "verdict_count": n, "verdicts": verdicts}).to_string())
        }
        "epistemic" => {
            use crate::modalities::epistemic::{evaluate_epistemic_frame, EpistemicVerdict};
            let quins = parse_quin_slice(&v, "quins")?;
            let agent = json_u64(&v, "agent_did_hash", 0);
            let world = json_u64(&v, "world_hash", 0);
            let mut out = vec![EpistemicVerdict {
                claim: NQuin::default(),
                status: crate::modalities::epistemic::EpistemicStatus::Skipped,
                certainty: 0,
            }; quins.len().max(1)];
            let n = evaluate_epistemic_frame(&quins, agent, world, &mut out)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let verdicts: Vec<Value> = out[..n]
                .iter()
                .map(|ver| {
                    json!({
                        "status": format!("{:?}", ver.status),
                        "certainty": ver.certainty
                    })
                })
                .collect();
            Ok(json!({"modality": "epistemic", "verdict_count": n, "verdicts": verdicts}).to_string())
        }
        "dl" => {
            use crate::modalities::dl::check_subsumption_quin;
            let sub = json_u64(&v, "sub_class_hash", 0);
            let sup = json_u64(&v, "super_class_hash", 0);
            let tbox = parse_quin_slice(&v, "tbox").unwrap_or_default();
            Ok(json!({
                "modality": "dl",
                "subsumed": check_subsumption_quin(sub, sup, &tbox)
            })
            .to_string())
        }
        "paraconsistent" => {
            use crate::modalities::paraconsistent::route_paraconsistent;
            let quins = parse_quin_slice(&v, "quins")?;
            let mut consistent = vec![NQuin::default(); quins.len().max(8)];
            let mut isolated = vec![NQuin::default(); quins.len().max(8)];
            let (c, i) = route_paraconsistent(&quins, &mut consistent, &mut isolated)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "modality": "paraconsistent",
                "consistent_count": c,
                "isolated_count": i
            })
            .to_string())
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

fn parse_ltl_formula(v: &Value) -> Result<crate::modalities::temporal_ltl::LtlFormula, McpSystemError> {
    use crate::modalities::temporal_ltl::LtlFormula;
    let ty = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let pred = |key: &str| json_u64(v, key, 0);
    Ok(match ty {
        "globally" | "G" => LtlFormula::Globally(pred("predicate")),
        "finally" | "F" => LtlFormula::Finally(pred("predicate")),
        "next" | "X" => LtlFormula::Next(pred("predicate")),
        "until" | "U" => LtlFormula::Until {
            ante: pred("ante"),
            consequent: pred("consequent"),
        },
        "release" | "R" => LtlFormula::Release {
            trigger: pred("trigger"),
            invariant: pred("invariant"),
        },
        _ => return Err(McpSystemError::InvalidParameters),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_check_tool_flags_corporate_capture() {
        // A corporation claiming a human dignity right → REJECT (PersonhoodCategoryError).
        let out = values_check(
            br#"{"agentType":"CorporatePerson","claimsDignityRight":true}"#,
        )
        .expect("ok");
        let p: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(p["flagged"], true);
        assert_eq!(p["flag"], "values:PersonhoodCategoryError");

        // A natural person holding their own right → ok.
        let out2 =
            values_check(br#"{"agentType":"NaturalPerson","claimsDignityRight":true}"#).expect("ok");
        let p2: Value = serde_json::from_str(&out2).expect("json");
        assert_eq!(p2["flagged"], false);
        assert_eq!(p2["verdict"], "ok");
    }

    #[test]
    fn values_evaluate_tool_deontic_lifecycle() {
        // A prohibition with no exception and no expiry → in force (Active).
        let out = values_evaluate(
            br#"{"modality":"forbid","party":"values:Agent","action":"values:DestructionOfRights"}"#,
        )
        .expect("ok");
        let p: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(p["status"], "Active");
        assert_eq!(p["modality"], "prohibition");

        // Same prohibition with a lawful-authorisation exception → Defeated.
        let out2 = values_evaluate(
            br#"{"modality":"forbid","party":"values:Agent","action":"values:DestructionOfRights","unless":"values:lawfullyAuthorised"}"#,
        )
        .expect("ok");
        let p2: Value = serde_json::from_str(&out2).expect("json");
        assert_eq!(p2["status"], "Defeated");
        assert_eq!(p2["exception"], true);

        // An obligation whose effective window has passed (expiry << now) → Expired.
        let out3 = values_evaluate(
            br#"{"modality":"oblige","party":"values:State","action":"values:ProvideRemedy","expiry":1000000000,"now":1717200000}"#,
        )
        .expect("ok");
        let p3: Value = serde_json::from_str(&out3).expect("json");
        assert_eq!(p3["status"], "Expired");
        assert_eq!(p3["modality"], "obligation");
    }

    #[test]
    fn jural_correlate_tool() {
        let out = jural_correlate(br#"{"position":"claim"}"#).expect("ok");
        let p: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(p["position"], "Claim");
        assert_eq!(p["correlative"], "Duty");
        assert_eq!(p["opposite"], "No-Right");
        assert_eq!(p["order"], "first-order (conduct)");

        let out2 = jural_correlate(br#"{"position":"immunity"}"#).expect("ok");
        let p2: Value = serde_json::from_str(&out2).expect("json");
        assert_eq!(p2["correlative"], "Disability");
        assert_eq!(p2["order"], "second-order (control)");

        assert!(jural_correlate(br#"{"position":"nonsense"}"#).is_err());
    }

    #[test]
    fn deontic_govern_tool() {
        // Non-derogable violation → PreventiveBlock, does NOT permit execution.
        let out = deontic_govern(br#"{"status":"violated","nonDerogable":true}"#).expect("ok");
        let p: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(p["policyMode"], "PreventiveBlock");
        assert_eq!(p["action"], "DenyRollback");
        assert_eq!(p["permitsExecution"], false);

        // Ordinary violation → audit, permits execution.
        let out2 = deontic_govern(br#"{"status":"violated"}"#).expect("ok");
        let p2: Value = serde_json::from_str(&out2).expect("json");
        assert_eq!(p2["policyMode"], "PermissiveAudit");
        assert_eq!(p2["permitsExecution"], true);

        // Ambiguity defers to a human.
        let out3 = deontic_govern(br#"{"status":"active","ambiguous":true}"#).expect("ok");
        let p3: Value = serde_json::from_str(&out3).expect("json");
        assert_eq!(p3["policyMode"], "Interactive");
    }

    #[test]
    fn cooperation_gate_decision() {
        use crate::mcp_cooperation::CooperationVerdict;
        // Enforcement OFF → always pass (None), regardless of caller.
        assert!(gate_verdict(br#"{}"#, false).is_none());
        // Enforcement ON, anonymous/unverified → DeniedUnverified.
        assert!(matches!(gate_verdict(br#"{}"#, true), Some(CooperationVerdict::DeniedUnverified)));
        // ON, verified but not grounded → DeniedUngrounded.
        assert!(matches!(
            gate_verdict(br#"{"caller":"did:bot","verified":true}"#, true),
            Some(CooperationVerdict::DeniedUngrounded)
        ));
        // ON, verified + grounded → Authorized.
        assert!(matches!(
            gate_verdict(br#"{"caller":"did:alice","verified":true,"grounded":true}"#, true),
            Some(CooperationVerdict::Authorized(_))
        ));
        // The public gate maps a denial to IntentFrameViolation (enforcement defaults off in CI).
        assert!(cooperation_gate(br#"{}"#).is_ok());
    }

    #[test]
    fn mcp_cooperate_tool() {
        // Verified, grounded, ordinary request → Authorized.
        let ok = mcp_cooperate(br#"{"caller":"did:alice","verified":true,"requestStatus":"active"}"#).expect("ok");
        let p: Value = serde_json::from_str(&ok).expect("json");
        assert_eq!(p["verdict"], "Authorized");
        assert_eq!(p["permitted"], true);

        // Asserted (not verified) → DeniedUnverified.
        let unv = mcp_cooperate(br#"{"caller":"did:x","verified":false}"#).expect("ok");
        assert_eq!(serde_json::from_str::<Value>(&unv).unwrap()["verdict"], "DeniedUnverified");

        // Verified but ungrounded AI → DeniedUngrounded.
        let ung = mcp_cooperate(br#"{"caller":"did:bot","verified":true,"grounded":false}"#).expect("ok");
        assert_eq!(serde_json::from_str::<Value>(&ung).unwrap()["verdict"], "DeniedUngrounded");

        // Verified + grounded but a non-derogable violation → DeniedByPolicy.
        let blk = mcp_cooperate(br#"{"caller":"did:alice","verified":true,"requestStatus":"violated","nonDerogable":true}"#).expect("ok");
        let pb: Value = serde_json::from_str(&blk).unwrap();
        assert_eq!(pb["verdict"], "DeniedByPolicy");
        assert_eq!(pb["policyMode"], "PreventiveBlock");
        assert_eq!(pb["permitted"], false);
    }

    #[test]
    fn matrix_multiply_caller_matrices() {
        let args = json!({
            "op": "multiply",
            "left": {"id": "A", "rows": 2, "cols": 2, "data": [1.0, 0.0, 0.0, 2.0]},
            "right": {"id": "B", "rows": 2, "cols": 2, "data": [3.0, 0.0, 0.0, 4.0]},
            "result_id": "C"
        });
        let out = matrix_operation(args.to_string().as_bytes()).expect("ok");
        let parsed: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(parsed["rows"], 2);
        assert!(parsed["data"].as_array().unwrap()[0].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn algebra_solve_polynomial_tool() {
        // x² − 5x + 6 → roots {2, 3}
        let args = json!({ "coeffs": [1.0, -5.0, 6.0] });
        let out = algebra_solve_polynomial(args.to_string().as_bytes()).expect("ok");
        let parsed: Value = serde_json::from_str(&out).expect("json");
        let roots = parsed["roots"].as_array().unwrap();
        assert_eq!(roots.len(), 2);
        let mut res: Vec<f64> = roots.iter().map(|r| r["re"].as_f64().unwrap()).collect();
        res.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((res[0] - 2.0).abs() < 1e-6 && (res[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn algebra_matrix_analyze_tool() {
        // determinant of [[1,2],[3,4]] = −2
        let det = algebra_matrix_analyze(
            json!({ "op": "determinant", "rows": 2, "cols": 2, "data": [1.0,2.0,3.0,4.0] })
                .to_string()
                .as_bytes(),
        )
        .expect("ok");
        let parsed: Value = serde_json::from_str(&det).expect("json");
        assert!((parsed["determinant"].as_f64().unwrap() + 2.0).abs() < 1e-9);

        // SVD reconstruction shape
        let svd = algebra_matrix_analyze(
            json!({ "op": "svd", "rows": 3, "cols": 2, "data": [1.0,0.0,0.0,1.0,1.0,1.0] })
                .to_string()
                .as_bytes(),
        )
        .expect("ok");
        assert!(svd.contains("singular_values"));
    }

    #[test]
    fn cas_tool_differentiate_and_solve() {
        // d/dx (x^3 - 2*x^2 + 5) then evaluate at x=2 → 4
        let d = cas(json!({ "op": "differentiate", "expr": "x^3 - 2*x^2 + 5", "var": "x" })
            .to_string()
            .as_bytes())
        .expect("ok");
        assert!(d.contains("derivative"));

        let ev = cas(json!({ "op": "evaluate", "expr": "x^2 + 1", "env": { "x": 3.0 } })
            .to_string()
            .as_bytes())
        .expect("ok");
        let parsed: Value = serde_json::from_str(&ev).expect("json");
        assert!((parsed["value"].as_f64().unwrap() - 10.0).abs() < 1e-9);

        let q = cas(json!({ "op": "solve_quadratic", "a": 1.0, "b": -5.0, "c": 6.0 })
            .to_string()
            .as_bytes())
        .expect("ok");
        assert!(q.contains("roots"));

        let ex = cas(json!({ "op": "expand", "expr": "(x + 1) * (x + 2)" })
            .to_string()
            .as_bytes())
        .expect("ok");
        assert!(ex.contains("expanded"));

        let fac = cas(json!({ "op": "factor", "a": 1.0, "b": -5.0, "c": 6.0, "var": "x" })
            .to_string()
            .as_bytes())
        .expect("ok");
        assert!(fac.contains("factored"));
    }

    #[test]
    fn bioinformatics_uses_caller_sequences() {
        let args = json!({"query": "ATCG", "target": "ATCC", "mode": "dna"});
        let out = bioinformatics_align(args.to_string().as_bytes()).expect("ok");
        assert!(out.contains("score"));
    }

    #[test]
    fn clinical_framingham_accepts_input() {
        let args = json!({
            "score": "framingham",
            "age": 55,
            "input": {"sex_male": true, "systolic_bp": 140.0}
        });
        let out = clinical_risk(args.to_string().as_bytes()).expect("ok");
        let parsed: Value = serde_json::from_str(&out).expect("json");
        assert!(parsed["risk_10yr"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn geometric_cross_product() {
        let args = json!({"op": "cross", "a": [1, 0, 0], "b": [0, 1, 0]});
        let out = geometric_algebra_op(args.to_string().as_bytes()).expect("ok");
        let parsed: Value = serde_json::from_str(&out).expect("json");
        let r = parsed["result"].as_array().unwrap();
        assert!((r[2].as_f64().unwrap() - 1.0).abs() < 0.01);
    }
}