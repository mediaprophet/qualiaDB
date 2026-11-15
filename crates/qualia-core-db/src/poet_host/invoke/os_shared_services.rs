//! Shared OS-Shell & QualiaDB Services Layer with Identifiers vs. Enumerated Consequence.
//!
//! Provides the core OS-Shell service handlers: Scheduler, Networking, Permissive Commons,
//! Communications & Directory, Identifiers & Enumerated Consequence, and OS Apps Manifest.
//! Adheres to AGENTS.md §0-B (< 360 lines).

use super::args;
use super::ids;
use crate::q_hash;
use vibe::{Diagnostic, Span, Value};

#[inline]
fn save_art(id: &str, domain: &str, title: &str, history_op: &str, props: &[(&str, &str)]) -> (u64, u64, u64) {
    let mut art = super::studio_store::get_artifact(id).unwrap_or_else(|| super::studio_store::StudioArtifact::new(id, domain, title));
    for (k, v) in props { art.properties.insert((*k).into(), (*v).into()); }
    art.history.push(history_op.into());
    let quins = art.compile_to_quins();
    let (did, clock, qcount) = (art.did_q42, art.lamport_clock as u64, quins.len() as u64);
    super::studio_store::upsert_artifact(art);
    (did, clock, qcount)
}

fn scheduler_service(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let job_id = args::rec_str(args_v, "job_id").unwrap_or("job_default");
    let task_type = args::rec_str(args_v, "task_type").unwrap_or("package_download");
    let queue_state = args::rec_str(args_v, "queue_state").or_else(|| args::rec_str(args_v, "status")).unwrap_or("queued");
    let priority = args::rec_u64(args_v, "priority").unwrap_or(1);
    let target_app = args::rec_str(args_v, "target_app").unwrap_or("poet");
    let cron_expr = args::rec_str(args_v, "cron_expr").unwrap_or("*/15 * * * *");
    let (did, clock, qcount) = save_art(job_id, "ServiceScheduler", &format!("Job:{task_type}"), "schedule_task", &[
        ("job_id", job_id), ("task_type", task_type), ("queue_state", queue_state),
        ("priority", &priority.to_string()), ("target_app", target_app), ("cron_expr", cron_expr),
    ]);
    Ok(args::record([
        ("status", Value::String("ok".into())),
        ("op", Value::String("service_scheduler".into())),
        ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
        ("job_id", Value::String(job_id.into())),
        ("task_type", Value::String(task_type.into())),
        ("queue_state", Value::String(queue_state.into())),
        ("priority", Value::U64(priority)),
        ("target_app", Value::String(target_app.into())),
        ("cron_expr", Value::String(cron_expr.into())),
        ("did_q42", Value::U64(did)),
        ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)),
        ("shell", Value::String("poet".into())),
    ]))
}

fn networking_service(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let peer_id = args::rec_str(args_v, "peer_id").unwrap_or("peer_topo_01");
    let transport = args::rec_str(args_v, "transport").unwrap_or("quic_p2p");
    let topo_route = args::rec_str(args_v, "topological_route").unwrap_or("did:q42:peer:topo_alpha");
    let sync_mode = args::rec_str(args_v, "sync_mode").unwrap_or("lww_crdt");
    let routing_lane = args::rec_str(args_v, "routing_lane").unwrap_or("commons");
    let bytes_trans = args::rec_u64(args_v, "bytes_transferred").unwrap_or(65536);
    let net_status = args::rec_str(args_v, "status").unwrap_or("connected");

    let sub_op = args::rec_str(args_v, "sub_op").or_else(|| args::rec_str(args_v, "action")).or_else(|| args::rec_str(args_v, "op")).unwrap_or("status");
    let nym_action = args::rec_str(args_v, "nym_action").unwrap_or("enable");
    let nym_network = args::rec_str(args_v, "nym_network").unwrap_or("sandbox");
    let nym_live = nym_action != "disable";
    let nym_locator = format!("nym:{}:{:016x}@gateway.nymtech.net", nym_network, q_hash(nym_network));

    let domain_name = args::rec_str(args_v, "domain_name").unwrap_or("personal.webizen.example");
    let agent_type = args::rec_str(args_v, "agent_type").unwrap_or("NaturalPerson");
    let front_door_did = args::rec_str(args_v, "front_door_did").unwrap_or("did:q42:frontdoor");
    let dns_mx = format!("{domain_name}. 3600 IN MX 10 mail.{domain_name}.");
    let dns_spf = format!("{domain_name}. 3600 IN TXT \"v=spf1 mx -all\"");
    let well_known_qdp_url = format!("https://{domain_name}/.well-known/QDP");
    let purpose_inboxes = "frontdoor,junkmail,mygov,newsletters";

    let nym_pairs = super::qdnf_bridge::evaluate_qdnf_nym_bearer(nym_network, peer_id);
    let (dns_pairs, quin_opt) = super::qdnf_bridge::evaluate_qdnf_dns_front_door(domain_name, front_door_did, agent_type, &nym_locator);

    let (did, clock, qcount) = if sub_op == "nym_setup" || args::rec_str(args_v, "nym_network").is_some() || args::rec_str(args_v, "nym_action").is_some() {
        save_art(&format!("nym:{nym_network}"), "NymMixnet", "Nym Mixnet Gateway", "nym_setup", &[
            ("nym_network", nym_network), ("nym_action", nym_action), ("nym_locator", &nym_locator),
            ("key_role", "mixnet"), ("seed_holder", "keyvault"), ("client", "nym-sdk@1.21.6"),
        ])
    } else {
        save_art(domain_name, "DomainDns", &format!("DNS:{domain_name}"), "dns_provision", &[
            ("domain_name", domain_name), ("agent_type", agent_type), ("front_door_did", front_door_did),
            ("dns_mx", &dns_mx), ("dns_spf", &dns_spf), ("well_known_qdp_url", &well_known_qdp_url),
            ("purpose_inboxes", purpose_inboxes), ("peer_id", peer_id), ("net_status", net_status),
        ])
    };

    let mut out = std::collections::BTreeMap::new();
    for (k, v) in [
        ("status", Value::String("ok".into())), ("op", Value::String("service_networking".into())),
        ("sub_op", Value::String(sub_op.into())), ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
        ("peer_id", Value::String(peer_id.into())), ("transport", Value::String(transport.into())),
        ("topological_route", Value::String(topo_route.into())), ("sync_mode", Value::String(sync_mode.into())),
        ("routing_lane", Value::String(routing_lane.into())), ("bytes_transferred", Value::U64(bytes_trans)),
        ("network_status", Value::String(net_status.into())), ("nym_live", Value::Bool(nym_live)),
        ("nym_network", Value::String(nym_network.into())), ("nym_locator", Value::String(nym_locator)),
        ("key_role", Value::String("mixnet".into())), ("seed_holder", Value::String("keyvault".into())),
        ("domain_name", Value::String(domain_name.into())), ("dns_mx", Value::String(dns_mx)),
        ("dns_spf", Value::String(dns_spf)), ("well_known_qdp_url", Value::String(well_known_qdp_url)),
        ("purpose_inboxes", Value::String(purpose_inboxes.into())), ("did_q42", Value::U64(did)),
        ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)), ("shell", Value::String("poet".into())),
    ] { out.insert(k.to_string(), v); }
    for (k, v) in nym_pairs { out.insert(k.to_string(), v); }
    for (k, v) in dns_pairs { out.insert(k.to_string(), v); }
    if let Some(q) = quin_opt {
        out.insert("front_door_quin_parity".to_string(), Value::Bool(q.verify_ecc_parity()));
    }
    Ok(Value::Record(out))
}

fn permissive_commons_service(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let lane_arg = args::rec_str(args_v, "routing_lane").unwrap_or("commons");
    let (lane_bits, lane_name) = match lane_arg.to_ascii_lowercase().as_str() {
        "00" | "passthrough" => (0b00u64, "passthrough"),
        "10" | "bilateral" => (0b10, "bilateral"),
        "11" | "spatial" => (0b11, "spatial"),
        _ => (0b01, "commons"),
    };
    let quota_bytes = args::rec_u64(args_v, "quota_bytes").unwrap_or(26_843_545_600);
    let consumed_bytes = args::rec_u64(args_v, "consumed_bytes").unwrap_or(1_073_741_824);
    let remaining_bytes = quota_bytes.saturating_sub(consumed_bytes);
    let power_wh = args::rec_f64(args_v, "power_watt_hours").unwrap_or(42.5);
    let cost_cents = args::rec_f64(args_v, "true_cost_cents").unwrap_or(3.2);
    let clear_status = args::rec_str(args_v, "clearing_status").unwrap_or("settled");
    let comm_id = format!("comm_{:016x}", q_hash(&format!("{lane_name}:{quota_bytes}")));
    let (did, clock, qcount) = save_art(&comm_id, "ServicePermissiveCommons", "Permissive Commons & Metering", "meter_commons", &[
        ("routing_lane", lane_name), ("quota_bytes", &quota_bytes.to_string()),
        ("consumed_bytes", &consumed_bytes.to_string()), ("clearing_status", clear_status),
    ]);
    Ok(args::record([
        ("status", Value::String("ok".into())),
        ("op", Value::String("service_permissive_commons".into())),
        ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
        ("routing_lane_bits", Value::U64(lane_bits)),
        ("routing_lane_name", Value::String(lane_name.into())),
        ("commons_quota_bytes", Value::U64(quota_bytes)),
        ("consumed_bytes", Value::U64(consumed_bytes)),
        ("quota_remaining_bytes", Value::U64(remaining_bytes)),
        ("power_watt_hours", Value::F64(power_wh)),
        ("true_cost_cents", Value::F64(cost_cents)),
        ("clearing_status", Value::String(clear_status.into())),
        ("did_q42", Value::U64(did)),
        ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)),
        ("shell", Value::String("poet".into())),
    ]))
}

fn communications_service(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mailbox = args::rec_str(args_v, "mailbox").unwrap_or("inbox");
    let sender = args::rec_str(args_v, "sender_did").unwrap_or("did:q42:agent_alpha");
    let recipient = args::rec_str(args_v, "recipient_did").unwrap_or("did:q42:agent_beta");
    let subject = args::rec_str(args_v, "subject").unwrap_or("Consent Request for Shared Manifold");
    let msg_type = args::rec_str(args_v, "message_type").unwrap_or("consent_request");
    let consent = args::rec_str(args_v, "consent_status").unwrap_or("granted");
    let msg_id = format!("msg_{:016x}", q_hash(&format!("{sender}->{recipient}:{subject}")));

    let conn_act = args::rec_str(args_v, "connection_action").or_else(|| args::rec_str(args_v, "action")).unwrap_or("send_request");
    let rel_type = args::rec_str(args_v, "relation_type").unwrap_or("spc:Collaboration");
    let domain = args::rec_str(args_v, "domain").unwrap_or("personal.webizen.example");
    let peer_did = args::rec_str(args_v, "peer_did").unwrap_or("did:q42:peer_collab");
    let token_hex = format!("{:016x}", q_hash(&format!("{peer_did}:{rel_type}:{domain}")));
    let deep_link = format!("webizen://connect?did={peer_did}&rel={rel_type}&domain={domain}&token={token_hex}");
    let https_link = format!("https://{domain}/.well-known/QDP/connect?did={peer_did}&rel={rel_type}&token={token_hex}");
    let mailto_link = format!("mailto:frontdoor@{domain}?subject=Webizen%20Connection%20Request&body={token_hex}");
    let conn_state = if conn_act == "accept_connect" { "connected" } else { "pending_handshake" };

    let qdnf_contact_pairs = super::qdnf_bridge::evaluate_qdnf_contact_gate(peer_did, rel_type, conn_act);

    let conn_id = format!("conn_req:{token_hex}");
    let dir_count = super::studio_store::list_artifacts().len().max(8);
    let (did, clock, qcount) = save_art(&conn_id, "ConnectionRequest", &format!("ConnReq:{peer_did}"), conn_act, &[
        ("connection_action", conn_act), ("relation_type", rel_type), ("domain", domain),
        ("peer_did", peer_did), ("token_hex", &token_hex), ("deep_link", &deep_link),
        ("https_link", &https_link), ("mailto_link", &mailto_link), ("connection_state", conn_state),
        ("mailbox", mailbox), ("sender_did", sender), ("recipient_did", recipient),
        ("message_type", msg_type), ("consent_status", consent),
    ]);

    let mut out = std::collections::BTreeMap::new();
    for (k, v) in [
        ("status", Value::String("ok".into())), ("op", Value::String("service_communications".into())),
        ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())), ("message_id", Value::String(msg_id)),
        ("mailbox", Value::String(mailbox.into())), ("sender_did", Value::String(sender.into())),
        ("recipient_did", Value::String(recipient.into())), ("subject", Value::String(subject.into())),
        ("message_type", Value::String(msg_type.into())), ("consent_status", Value::String(consent.into())),
        ("directory_entry_count", Value::U64(dir_count as u64)), ("connection_action", Value::String(conn_act.into())),
        ("relation_type", Value::String(rel_type.into())), ("peer_did", Value::String(peer_did.into())),
        ("domain", Value::String(domain.into())), ("token_hex", Value::String(token_hex)),
        ("deep_link", Value::String(deep_link)), ("https_link", Value::String(https_link)),
        ("mailto_link", Value::String(mailto_link)), ("connection_state", Value::String(conn_state.into())),
        ("did_q42", Value::U64(did)), ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)), ("shell", Value::String("poet".into())),
    ] { out.insert(k.to_string(), v); }
    for (k, v) in qdnf_contact_pairs { out.insert(k.to_string(), v); }
    Ok(Value::Record(out))
}

fn identifiers_service(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let id_did = args::rec_str(args_v, "identifier_did").or_else(|| args::rec_str(args_v, "did")).unwrap_or("did:q42:entity_01");
    let posture = args::rec_str(args_v, "key_posture").unwrap_or("ed25519_hardware_bound");
    let multi_keys = "ed25519:taproot:secp256k1:sr25519";

    let artifacts = super::studio_store::list_artifacts();
    let mut actions_tally = 0u64;
    for art in &artifacts {
        if art.id.contains(id_did) || art.properties.values().any(|v| v.contains(id_did)) {
            actions_tally += (art.history.len() as u64).max(1);
        }
    }
    let extra_actions = args::rec_u64(args_v, "actions_observed").unwrap_or(3);
    let total_actions = actions_tally.max(extra_actions);
    let consequence_score = ((total_actions as f64) * 12.5).min(100.0);
    let consequence_tier = if consequence_score >= 60.0 { "proven_contributor" } else if consequence_score >= 20.0 { "active_participant" } else { "nascent_observer" };
    let prov_ref = format!("did:q42:provenance:{:016x}", q_hash(id_did) & 0x0FFF_FFFF_FFFF_FFFF);

    let qdnf_id_pairs = super::qdnf_bridge::evaluate_qdnf_identifier_fabric(id_did);

    let (did, clock, qcount) = save_art(id_did, "ServiceIdentifiers", "Cryptographic Identifier & Consequence Profile", "enumerate_consequence", &[
        ("identifier_did", id_did), ("key_posture", posture), ("multi_chain_keys", multi_keys),
        ("consequence_tier", consequence_tier), ("provenance_graph_ref", &prov_ref),
    ]);

    let mut out = std::collections::BTreeMap::new();
    for (k, v) in [
        ("status", Value::String("ok".into())), ("op", Value::String("service_identifiers".into())),
        ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())), ("identifier_did", Value::String(id_did.into())),
        ("key_posture", Value::String(posture.into())), ("multi_chain_keys", Value::String(multi_keys.into())),
        ("enumerated_actions", Value::U64(total_actions)), ("enumerated_consequence_score", Value::F64(consequence_score)),
        ("consequence_tier", Value::String(consequence_tier.into())), ("provenance_graph_ref", Value::String(prov_ref)),
        ("did_q42", Value::U64(did)), ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)), ("shell", Value::String("poet".into())),
    ] { out.insert(k.to_string(), v); }
    for (k, v) in qdnf_id_pairs { out.insert(k.to_string(), v); }
    Ok(Value::Record(out))
}

const CANONICAL_QAPS: &[(&str, &str, &str, &str, &str, &str, &str, &str, &str)] = &[
    ("poet", "POET Core HyperCanvas & Spatial Studio", "1.0.0", "1.0.1", "https://github.com/webizen/qap-poet", "did:git:webizen:qap-poet", "packaged_core", "installed", "scheduler,networking,permissive_commons,identifiers"),
    ("wallet-crypto", "Multi-Chain Sovereign Wallet & Economic Settlement", "1.0.0", "1.0.1", "https://github.com/webizen/qap-wallet", "did:git:webizen:qap-wallet", "packaged_core", "installed", "identifiers,permissive_commons,networking"),
    ("cooperative-projects", "Cooperative Projects & Labor Accounting Commons", "1.0.0", "1.0.1", "https://github.com/webizen/qap-coop", "did:git:webizen:qap-coop", "packaged_core", "installed", "scheduler,communications,permissive_commons"),
    ("hypermedia-library", "Hypermedia Asset & Aspect Graph Library", "1.0.0", "1.0.1", "https://github.com/webizen/qap-hypermedia", "did:git:webizen:qap-hypermedia", "packaged_core", "installed", "networking,scheduler,permissive_commons"),
    ("llm-agents", "Local LLM Swarm & Agentic Choreography", "1.0.0", "1.1.0", "https://github.com/webizen/qap-llm-agents", "did:git:webizen:qap-llm-agents", "separate_repo_downloadable", "available_to_install", "scheduler,permissive_commons,communications"),
    ("communications-directory", "Consent-Gated Purpose Messaging & DID Directory", "1.0.0", "1.0.1", "https://github.com/webizen/qap-comms", "did:git:webizen:qap-comms", "packaged_core", "installed", "communications,identifiers,networking"),
    ("web-browser", "Webizen Spatial Browser & Solid LDP Client", "1.0.0", "1.1.0", "https://github.com/webizen/qap-browser", "did:git:webizen:qap-browser", "separate_repo_downloadable", "available_to_install", "networking,permissive_commons,identifiers"),
];

fn os_apps_manifest(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let action = args::rec_str(args_v, "action").unwrap_or("list");

    if matches!(action, "install" | "update" | "rollback" | "uninstall" | "verify_repo") {
        let target_qap = args::rec_str(args_v, "qap_id").or_else(|| args::rec_str(args_v, "id")).unwrap_or("poet");
        let target_ver = args::rec_str(args_v, "target_version").or_else(|| args::rec_str(args_v, "version")).unwrap_or("1.0.1");
        let default_tuple = CANONICAL_QAPS.iter().find(|q| q.0 == target_qap);
        let target_repo = args::rec_str(args_v, "repo_url").unwrap_or_else(|| default_tuple.map(|q| q.4).unwrap_or("https://github.com/webizen/qap-pkg"));
        let did_git = default_tuple.map(|q| q.5).unwrap_or("did:git:webizen:qap-pkg");
        let signer = "did:q42:release-signer";
        let new_state = match action {
            "install" | "update" => "installed",
            "rollback" => "rolled_back",
            "verify_repo" => "verified",
            "uninstall" => "available_to_install",
            _ => "installed",
        };
        let (did, clock, qcount) = save_art(target_qap, "QapPackage", &format!("Qap:{target_qap}"), action, &[
            ("qap_id", target_qap),
            ("version", target_ver),
            ("repo_url", target_repo),
            ("did_git", did_git),
            ("action", action),
            ("install_state", new_state),
            ("signer_identifier_did", signer),
        ]);
        return Ok(args::record([
            ("status", Value::String("ok".into())),
            ("op", Value::String("os_apps_manifest".into())),
            ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
            ("action", Value::String(action.into())),
            ("qap_id", Value::String(target_qap.into())),
            ("installed_version", Value::String(target_ver.into())),
            ("repo_url", Value::String(target_repo.into())),
            ("did_git", Value::String(did_git.into())),
            ("install_state", Value::String(new_state.into())),
            ("signer_identifier_did", Value::String(signer.into())),
            ("did_q42", Value::U64(did)),
            ("lamport_clock", Value::U64(clock)),
            ("quin_count", Value::U64(qcount)),
            ("shell", Value::String("poet".into())),
        ]));
    }

    let mut app_records = Vec::new();
    for &(qap_id, title, ver, off_ver, repo, did_git, dist, default_state, services) in CANONICAL_QAPS {
        let (curr_ver, curr_repo, curr_state) = if let Some(art) = super::studio_store::get_artifact(qap_id) {
            let v = art.properties.get("version").cloned().unwrap_or_else(|| ver.to_string());
            let r = art.properties.get("repo_url").cloned().unwrap_or_else(|| repo.to_string());
            let st = art.properties.get("install_state").cloned().unwrap_or_else(|| default_state.to_string());
            (v, r, st)
        } else {
            (ver.to_string(), repo.to_string(), default_state.to_string())
        };
        app_records.push(args::record([
            ("qap_id", Value::String(qap_id.into())),
            ("title", Value::String(title.into())),
            ("version", Value::String(curr_ver)),
            ("offered_version", Value::String(off_ver.into())),
            ("repo_url", Value::String(curr_repo)),
            ("did_git", Value::String(did_git.into())),
            ("distribution", Value::String(dist.into())),
            ("install_state", Value::String(curr_state)),
            ("signer_identifier_did", Value::String("did:q42:release-signer".into())),
            ("services", Value::String(services.into())),
        ]));
    }

    let shared_services = Value::List(vec![
        Value::String("service_scheduler".into()),
        Value::String("service_networking".into()),
        Value::String("service_permissive_commons".into()),
        Value::String("service_communications".into()),
        Value::String("service_identifiers".into()),
    ]);
    let (did, clock, qcount) = save_art("os:apps_manifest", "OSAppsManifest", "Webizen OS-Shell Launchable Apps", "query_manifest", &[
        ("app_count", "7"),
        ("action", "list"),
    ]);
    Ok(args::record([
        ("status", Value::String("ok".into())),
        ("op", Value::String("os_apps_manifest".into())),
        ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
        ("action", Value::String("list".into())),
        ("app_count", Value::U64(7)),
        ("apps", Value::List(app_records)),
        ("bound_shared_services", shared_services),
        ("did_q42", Value::U64(did)),
        ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Dispatches an OS-Shell shared service operation by op name.
pub fn handle_shared_service(op: &str, args_v: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match op {
        "service_scheduler" => Some(scheduler_service(args_v, span)),
        "service_networking" | "networking" | "nym_setup" | "domain_dns_setup"
        | "qdnf_inspect" | "qdnf_vertical_slice" | "qdnf_dns_front_door" => Some(networking_service(args_v, span)),
        "service_permissive_commons" => Some(permissive_commons_service(args_v, span)),
        "service_communications" | "communications" | "connection_request"
        | "qdnf_contact_transition" => Some(communications_service(args_v, span)),
        "service_identifiers" => Some(identifiers_service(args_v, span)),
        "os_apps_manifest" | "qap_package_lifecycle" => Some(os_apps_manifest(args_v, span)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = std::collections::BTreeMap::new();
        for (k, v) in pairs { m.insert((*k).into(), v.clone()); }
        Value::Record(m)
    }

    #[test]
    fn test_all_shared_services() {
        let span = Span { start: 0, end: 0 };
        // 1. Scheduler
        let sched = handle_shared_service("service_scheduler", &rec(&[("job_id", Value::String("job_test".into())), ("task_type", Value::String("model_download".into()))]), span).unwrap().unwrap();
        let Value::Record(r1) = sched else { panic!("not rec"); };
        assert_eq!(r1.get("op"), Some(&Value::String("service_scheduler".into())));

        // 2. Networking (verified with QDNF & DNS SDN integration)
        let net = handle_shared_service("service_networking", &rec(&[("peer_id", Value::String("peer_99".into()))]), span).unwrap().unwrap();
        let Value::Record(r2) = net else { panic!("not rec"); };
        assert_eq!(r2.get("op"), Some(&Value::String("service_networking".into())));
        assert!(r2.get("nym_locator").is_some());
        assert_eq!(r2.get("key_role"), Some(&Value::String("mixnet".into())));
        assert!(r2.get("dns_qdp_txt").is_some());
        assert_eq!(r2.get("qdnf_profile"), Some(&Value::String("NymMixnetTransitionV1".into())));
        assert_eq!(r2.get("nym_magic"), Some(&Value::String("QNYM".into())));
        assert_eq!(r2.get("front_door_parity_valid"), Some(&Value::Bool(true)));

        // 3. Permissive Commons
        let pc = handle_shared_service("service_permissive_commons", &rec(&[("routing_lane", Value::String("commons".into()))]), span).unwrap().unwrap();
        let Value::Record(r3) = pc else { panic!("not rec"); };
        assert_eq!(r3.get("op"), Some(&Value::String("service_permissive_commons".into())));
        assert_eq!(r3.get("routing_lane_bits"), Some(&Value::U64(0b01)));

        // 4. Communications (verified with QDNF contact gate)
        let comms = handle_shared_service("service_communications", &rec(&[("mailbox", Value::String("live_share".into())), ("connection_action", Value::String("accept_connect".into()))]), span).unwrap().unwrap();
        let Value::Record(r4) = comms else { panic!("not rec"); };
        assert_eq!(r4.get("op"), Some(&Value::String("service_communications".into())));
        assert!(r4.get("deep_link").is_some());
        assert_eq!(r4.get("connection_state"), Some(&Value::String("connected".into())));
        assert_eq!(r4.get("qdnf_contact_state"), Some(&Value::String("Active".into())));
        assert_eq!(r4.get("qdnf_contact_admitted"), Some(&Value::Bool(true)));

        // 5. Identifiers (verified with QDNF identifier fabric separation)
        let ids_svc = handle_shared_service("service_identifiers", &rec(&[("identifier_did", Value::String("did:q42:test_subject".into()))]), span).unwrap().unwrap();
        let Value::Record(r5) = ids_svc else { panic!("not rec"); };
        assert_eq!(r5.get("op"), Some(&Value::String("service_identifiers".into())));
        assert_eq!(r5.get("handle_is_person"), Some(&Value::Bool(false)));
        assert_eq!(r5.get("did_wallet_join_refused"), Some(&Value::Bool(true)));

        // 6. Manifest
        let man = handle_shared_service("os_apps_manifest", &rec(&[]), span).unwrap().unwrap();
        let Value::Record(r6) = man else { panic!("not rec"); };
        assert_eq!(r6.get("op"), Some(&Value::String("os_apps_manifest".into())));
        assert_eq!(r6.get("app_count"), Some(&Value::U64(7)));
    }

    #[test]
    fn test_qap_package_lifecycle() {
        let span = Span { start: 0, end: 0 };
        let inst = handle_shared_service("os_apps_manifest", &rec(&[("action", Value::String("install".into())), ("qap_id", Value::String("llm-agents".into())), ("version", Value::String("1.1.0".into()))]), span).unwrap().unwrap();
        let Value::Record(r1) = inst else { panic!("not rec"); };
        assert_eq!(r1.get("install_state"), Some(&Value::String("installed".into())));
        assert_eq!(r1.get("installed_version"), Some(&Value::String("1.1.0".into())));

        let upd = handle_shared_service("qap_package_lifecycle", &rec(&[("action", Value::String("update".into())), ("qap_id", Value::String("poet".into())), ("version", Value::String("1.0.2".into()))]), span).unwrap().unwrap();
        let Value::Record(r2) = upd else { panic!("not rec"); };
        assert_eq!(r2.get("installed_version"), Some(&Value::String("1.0.2".into())));

        let rb = handle_shared_service("os_apps_manifest", &rec(&[("action", Value::String("rollback".into())), ("qap_id", Value::String("poet".into())), ("version", Value::String("1.0.0".into()))]), span).unwrap().unwrap();
        let Value::Record(r3) = rb else { panic!("not rec"); };
        assert_eq!(r3.get("install_state"), Some(&Value::String("rolled_back".into())));

        let vf = handle_shared_service("os_apps_manifest", &rec(&[("action", Value::String("verify_repo".into())), ("qap_id", Value::String("web-browser".into()))]), span).unwrap().unwrap();
        let Value::Record(r4) = vf else { panic!("not rec"); };
        assert_eq!(r4.get("install_state"), Some(&Value::String("verified".into())));

        let list_res = handle_shared_service("os_apps_manifest", &rec(&[]), span).unwrap().unwrap();
        let Value::Record(r5) = list_res else { panic!("not rec"); };
        assert_eq!(r5.get("action"), Some(&Value::String("list".into())));
        assert_eq!(r5.get("app_count"), Some(&Value::U64(7)));
    }
}
