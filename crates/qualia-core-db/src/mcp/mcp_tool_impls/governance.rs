use super::*;


/// Return the canonical capability catalogue used by chat, CLI, and MCP.
///
/// Internal/library-only operations remain visible with no MCP route, while
/// unavailable families are labelled `fail-closed` rather than advertised as
/// operational.
pub fn list_capabilities(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let domain = v.get("domain").and_then(Value::as_str);
    let maturity = v.get("maturity").and_then(Value::as_str);
    let surface = v.get("surface").and_then(Value::as_str);

    let capabilities: Vec<Value> = crate::CAPABILITY_DESCRIPTORS
        .iter()
        .filter(|capability| {
            domain.map_or(true, |wanted| {
                capability.domain.eq_ignore_ascii_case(wanted)
            }) && maturity.map_or(true, |wanted| {
                capability.maturity.eq_ignore_ascii_case(wanted)
            }) && surface.map_or(true, |wanted| {
                capability
                    .surfaces
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(wanted))
            })
        })
        .map(|capability| {
            json!({
                "name": capability.name,
                "domain": capability.domain,
                "maturity": capability.maturity,
                "surfaces": capability.surfaces,
                "operations": capability.operations,
                "mcp_tools": capability.mcp_tools,
                "directly_callable": !capability.mcp_tools.is_empty(),
                "operational": capability.maturity != "fail-closed",
            })
        })
        .collect();

    let operation_count: usize = capabilities
        .iter()
        .filter_map(|capability| capability["operations"].as_array())
        .map(Vec::len)
        .sum();
    Ok(json!({
        "engine_version": crate::ENGINE_VERSION,
        "capability_count": capabilities.len(),
        "operation_group_count": operation_count,
        "capabilities": capabilities,
    })
    .to_string())
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
/// `https://ns.webcivics.net/values/` class.
pub fn values_check(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let agent_type_short = json_str(&v, "agentType", "NaturalPerson");
    let claims = v
        .get("claimsDignityRight")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let agent_type = crate::q_hash(&format!(
        "https://ns.webcivics.net/values/{agent_type_short}"
    ));
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
            let defeater = compile_norm_quin(
                party,
                OP_PERMIT,
                path,
                crate::q_hash(unless_s),
                contract,
                0,
                true,
            );
            quins.push(defeater);
        }
    }

    let mut out = [DeonticVerdict::default(); 8];
    let n = evaluate_deontic_contract(&quins, now, &mut out)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let verdict = out[..n].first().copied().unwrap_or_default();
    let (status_s, meaning) = match verdict.status {
        DeonticStatus::Active => ("Active", format!("the {verb} is in force")),
        DeonticStatus::Defeated => (
            "Defeated",
            format!("the {verb} is overridden by an exception"),
        ),
        DeonticStatus::Expired => (
            "Expired",
            format!("the {verb} has lapsed (past its effective window)"),
        ),
        DeonticStatus::Malformed => ("Malformed", "the norm could not be interpreted".to_string()),
        DeonticStatus::Pending => (
            "Pending",
            format!("the {verb} is valid but not yet in its effective window"),
        ),
        DeonticStatus::Violated => (
            "Violated",
            format!("the {verb} is in force but the facts show it was not met"),
        ),
        DeonticStatus::Discharged => (
            "Discharged",
            format!("the {verb} has been fulfilled and the duty terminates"),
        ),
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
        None => Ok(()), // enforcement off → pass
        Some(crate::mcp_cooperation::CooperationVerdict::Authorized(_)) => Ok(()),
        Some(_) => Err(McpSystemError::IntentFrameViolation), // denied → refuse the call
    }
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
    use crate::mcp_cooperation::{
        authorize, cooperation_label, CallerStandpoint, CooperationVerdict,
    };
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
    let status = match json_str(&v, "requestStatus", "active")
        .to_ascii_lowercase()
        .as_str()
    {
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
        CooperationVerdict::Authorized(m) | CooperationVerdict::DeniedByPolicy(m) => {
            Some(format!("{m:?}"))
        }
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
