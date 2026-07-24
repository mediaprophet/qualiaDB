//! Domains & semantic mail/address stack

#![allow(non_snake_case)]

use super::*;



pub fn mail_now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn agent_type_from_token(t: &str) -> crate::domains::AgentType {
    use crate::domains::AgentType::*;
    match t {
        "org" | "organization" => Organization,
        "ai" | "agent" => AiAgent,
        "service" | "humanitarian" => HumanitarianService,
        "content" => ContentProvider,
        "group" => Group,
        _ => NaturalPerson,
    }
}

/// The person's context-domains (personal/work/projects/…), each an agent with a front-door DID.
pub fn list_mail_domains() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::domains::list_domains()).map_err(|e| e.to_string())
}

/// Add a context-domain (single-owner). `agent_type` is a token: person/org/ai/service/content/group.
pub fn add_mail_domain(
    name: String,
    agent_type: String,
    front_door_did: String,
    label: String,
    parent: Option<String>,
) -> Result<serde_json::Value, String> {
    let owner = crate::domains::DomainOwner::Personal { did: front_door_did.clone() };
    let d = crate::domains::make_domain(
        &name,
        agent_type_from_token(&agent_type),
        owner,
        &front_door_did,
        &label,
        parent,
        mail_now_unix(),
    )?;
    crate::domains::upsert_domain(d)?;
    list_mail_domains()
}

/// Built-in purpose-inbox presets (frontdoor/junkmail/mygov/newsletters).
pub fn purpose_inbox_presets() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::domains::purpose_presets()).map_err(|e| e.to_string())
}

/// Addresses (optionally filtered to one domain).
pub fn list_mail_addresses(domain: Option<String>) -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::domains::list_addresses(domain.as_deref()))
        .map_err(|e| e.to_string())
}

/// Mint a purpose inbox (`frontdoor@`, `junkmail@`, …). `rules_json` is a `MailRules` object (or empty).
pub fn mint_purpose_inbox(
    domain: String,
    local: String,
    rules_json: String,
) -> Result<serde_json::Value, String> {
    let rules: crate::domains::MailRules = if rules_json.trim().is_empty() {
        crate::domains::MailRules::default()
    } else {
        serde_json::from_str(&rules_json).map_err(|e| format!("bad rules json: {e}"))?
    };
    let a = crate::domains::make_purpose_address(&domain, &local, rules, mail_now_unix())?;
    crate::domains::upsert_address(a)?;
    list_mail_addresses(Some(domain))
}

/// Onboard mail for a registered domain: mint purpose presets + catchall, then
/// **auto-start the local SMTP receiver** so the product path finishes without a second hunt.
pub fn onboard_mail_domain(domain: String) -> Result<serde_json::Value, String> {
    let minted = crate::domains::onboard_purpose_inboxes(&domain)?;
    let domain_key = domain.trim().to_lowercase();
    let addresses = crate::domains::list_addresses(Some(&domain_key));
    #[cfg(not(target_arch = "wasm32"))]
    let receiver = match crate::mail_inbound::start_receiver("") {
        Ok(v) => v,
        Err(e) => serde_json::json!({
            "started": false,
            "error": e,
            "hint": "Start the receiver from Talk → Mail if bind failed (port in use?).",
        }),
    };
    #[cfg(target_arch = "wasm32")]
    let receiver = serde_json::json!({ "started": false, "hint": "receiver is native-only" });
    let mint_msg = if minted.is_empty() {
        "Mail already onboarded for this domain.".to_string()
    } else {
        format!("Minted {} mailbox(es): {}.", minted.len(), minted.join(", "))
    };
    let recv_msg = if receiver.get("started").and_then(|x| x.as_bool()).unwrap_or(false)
        || receiver.get("already_running").and_then(|x| x.as_bool()).unwrap_or(false)
    {
        " Local SMTP receiver running — inbox can accept mail.".to_string()
    } else if let Some(err) = receiver.get("error").and_then(|e| e.as_str()) {
        format!(" Receiver not started: {err}.")
    } else {
        String::new()
    };
    Ok(serde_json::json!({
        "domain": domain_key,
        "minted": minted,
        "addresses": addresses,
        "receiver": receiver,
        "message": format!("{mint_msg}{recv_msg}"),
    }))
}

/// Human-readable readiness for Talk: domains, mailboxes, receiver, people, so first-run can guide.
pub fn talk_setup_status() -> Result<serde_json::Value, String> {
    let domains = crate::domains::list_domains();
    let addresses = crate::domains::list_addresses(None);
    let (mail_total, mail_unread, mail_quarantine) = crate::mail_store::counts();
    let receiver = crate::mail_inbound::receiver_status();
    let peers = crate::social_peers::list_peers();
    let contacts_n = crate::social_connect::list_chat_contacts().len();
    let has_domain = !domains.is_empty();
    let has_mailboxes = !addresses.is_empty();
    let receiver_running = receiver
        .get("running")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let has_people = !peers.is_empty() || contacts_n > 0;
    let collabs = crate::project_collab::list(None);
    let dial = crate::social_mesh::dialability(&peers);
    let dialable = dial.iter().filter(|d| d.dialable_now).count();
    let reachable = dial.iter().filter(|d| d.reachable).count();
    let ready_for_mail = has_domain && has_mailboxes && receiver_running;
    let ready_to_collaborate = has_domain && has_people;
    Ok(serde_json::json!({
        "domains": domains.len(),
        "mailboxes": addresses.len(),
        "mail_inbox": { "total": mail_total, "unread": mail_unread, "quarantine": mail_quarantine },
        "receiver": receiver,
        "peers": peers.len(),
        "contacts": contacts_n,
        "collaborators": collabs.len(),
        "mesh": {
            "reachable_peers": reachable,
            "dialable_now": dialable,
            "reports": dial,
        },
        "has_domain": has_domain,
        "has_mailboxes": has_mailboxes,
        "receiver_running": receiver_running,
        "has_people": has_people,
        "ready_for_mail": ready_for_mail,
        "ready_to_collaborate": ready_to_collaborate,
        "next_step": if !has_domain {
            "reception"
        } else if !has_mailboxes {
            "mail_onboard"
        } else if !receiver_running {
            "mail_receiver"
        } else if !has_people {
            "people"
        } else if collabs.is_empty() {
            "projects"
        } else {
            "chat_or_projects"
        },
    }))
}

/// Ensure the profile allows connect invites (required for a one-paste join package).
fn ensure_connect_invites_enabled() -> Result<(), String> {
    let mut profile = crate::user_profile::load_profile();
    if !profile.sharing.allow_group_chat_invites {
        profile.sharing.allow_group_chat_invites = true;
        crate::user_profile::save_profile(&profile)?;
    }
    Ok(())
}

/// Pure: pull the connect-invite JSON string out of a coop package or bare invite object.
/// Used by accept + unit tests (no APP_STATE).
pub fn extract_invite_json_from_package(v: &serde_json::Value) -> Result<String, String> {
    if v.get("qualia_coop_share").is_some() {
        let invite_payload = v
            .get("invite_json")
            .cloned()
            .filter(|x| !x.is_null())
            .ok_or_else(|| {
                "share package has no invite_json — host must enable invites and rebuild package"
                    .to_string()
            })?;
        if invite_payload.is_string() {
            let s = invite_payload.as_str().unwrap_or("").trim().to_string();
            if s.is_empty() {
                return Err("invite_json string is empty".into());
            }
            return Ok(s);
        }
        return serde_json::to_string(&invite_payload).map_err(|e| e.to_string());
    }
    // Bare ConnectInvitePayload object.
    if v.get("signature_hex").is_some() && v.get("inviter_did").is_some() {
        return serde_json::to_string(v).map_err(|e| e.to_string());
    }
    Err("not a coop share package or connect invite JSON".into())
}

/// Build a pasteable **coop share package** — one blob the joiner pastes under People → Accept.
/// Always embeds a signed connect invite (enables invites if needed). Never includes private keys.
pub fn coop_share_package(
    project_id: Option<String>,
    project_name: Option<String>,
) -> Result<serde_json::Value, String> {
    ensure_connect_invites_enabled()?;

    let domains = crate::domains::list_domains();
    let domain = domains.first().map(|d| d.name.clone()).unwrap_or_default();
    let front_door = domains
        .first()
        .map(|d| d.front_door_did.clone())
        .unwrap_or_default();
    let pid = project_id.unwrap_or_default();
    let pname = project_name.unwrap_or_default();
    let profile = crate::user_profile::load_profile();

    // Required: invite must succeed for one-paste join (fail closed, not half-package).
    let invite = crate::social_connect::generate_connect_invite(None).map_err(|e| {
        format!(
            "could not embed connect invite: {e}. Set a display name under Talk → People and try again."
        )
    })?;
    let invite_json: serde_json::Value = serde_json::from_str(&invite.invite_json)
        .unwrap_or_else(|_| serde_json::Value::String(invite.invite_json.clone()));

    #[cfg(not(target_arch = "wasm32"))]
    let magic = {
        let fd = if front_door.is_empty() {
            String::new()
        } else {
            front_door.clone()
        };
        // Mesh peer material for the joiner (optional second path).
        generate_magic_link(fd, "collaborator".into(), domain.clone()).ok()
    };
    #[cfg(target_arch = "wasm32")]
    let magic: Option<serde_json::Value> = None;

    // Host also records themselves as steward on the project roster when a project is named.
    if !pid.is_empty() {
        let self_did = profile.public_did.clone();
        if !self_did.is_empty() {
            let _ = crate::project_collab::add(
                &pid,
                &pname,
                &self_did,
                &profile.display_name,
                "steward",
            );
        }
    }

    let package = serde_json::json!({
        "qualia_coop_share": "1",
        "from_display_name": profile.display_name,
        "domain": domain,
        "front_door_did": front_door,
        "project_id": pid,
        "project_name": pname,
        "members": crate::project_collab::list(if pid.is_empty() { None } else { Some(pid.as_str()) }),
        "invite_code": invite.code,
        "invite_json": invite_json,
        "invite_expires_at": invite.expires_at,
        "magic_link": magic,
        "how": [
            "1. Open Webizen (0.0.25+).",
            "2. Talk → People → paste this entire JSON under Accept package / invite.",
            "3. You are connected and the project is scoped on your device.",
            "4. Talk → People → Start mesh for live peer chat.",
            "5. Chat with #project:Name_With_Underscores for scoped work.",
        ],
        "note": "No private keys. Package is self-contained — one paste joins.",
    });
    Ok(package)
}

/// Accept a **coop share package** or a bare connect-invite JSON (one paste).
/// Connects to the host, scopes the project, admits host+self on the local roster,
/// registers mesh peer when magic_link is present, and opens a project group chat when possible.
pub fn accept_coop_share_package(package_or_invite: String) -> Result<serde_json::Value, String> {
    let raw = package_or_invite.trim();
    if raw.is_empty() {
        return Err("paste a coop share package or invite JSON".into());
    }
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("not JSON: {e}"))?;

    let invite_str = extract_invite_json_from_package(&v).or_else(|e| {
        // Bare invite string body already parsed as object without qualia_coop_share.
        if v.get("inviter_did").is_some() {
            serde_json::to_string(&v).map_err(|e2| e2.to_string())
        } else {
            Err(e)
        }
    })?;
    let contact = crate::social_connect::accept_connect_invite(&invite_str)?;

    let project_id = v
        .get("project_id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let project_name = v
        .get("project_name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let profile = crate::user_profile::load_profile();
    let mut admitted = Vec::new();
    if !project_id.is_empty() {
        let host_did = contact.did.clone();
        let host_name = if contact.display_name.is_empty() {
            contact.did.clone()
        } else {
            contact.display_name.clone()
        };
        if !host_did.is_empty() {
            if let Ok(row) = crate::project_collab::add(
                &project_id,
                &project_name,
                &host_did,
                &host_name,
                "steward",
            ) {
                admitted.push(row);
            }
        }
        let self_did = profile.public_did.clone();
        if self_did.is_empty() {
            // Ensure we have a resolvable self DID for the roster.
            let resolved = crate::user_profile::resolve_public_did(&profile);
            if !resolved.is_empty() {
                if let Ok(row) = crate::project_collab::add(
                    &project_id,
                    &project_name,
                    &resolved,
                    &profile.display_name,
                    "contributor",
                ) {
                    admitted.push(row);
                }
            }
        } else if let Ok(row) = crate::project_collab::add(
            &project_id,
            &project_name,
            &self_did,
            &profile.display_name,
            "contributor",
        ) {
            admitted.push(row);
        }
    }

    // Magic link → SocialWebNet peer (mesh), when present.
    let mut peer_registered = false;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(magic) = v.get("magic_link") {
        let link = magic
            .get("deep_link")
            .or_else(|| magic.get("https_link"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if !link.is_empty() {
            if accept_connection(link.to_string()).is_ok() {
                peer_registered = true;
            }
        }
    }

    // Project group chat from roster (host + self + any prior members).
    let mut group_chat: Option<serde_json::Value> = None;
    if !project_id.is_empty() {
        match create_project_group_chat(
            project_id.clone(),
            if project_name.is_empty() {
                project_id.clone()
            } else {
                project_name.clone()
            },
            Some(vec![contact.did.clone()]),
        ) {
            Ok(g) => group_chat = Some(g),
            Err(_) => {
                // Still ok — roster may be thin; user can create group later.
            }
        }
    }

    Ok(serde_json::json!({
        "connected": true,
        "contact": contact,
        "project_id": project_id,
        "project_name": project_name,
        "admitted": admitted,
        "peer_registered": peer_registered,
        "group_chat": group_chat,
        "message": "Joined. Contact saved; project scoped; roster updated; group chat created when possible. Start mesh under People for live peer traffic.",
    }))
}

#[cfg(test)]
mod coop_share_tests {
    use super::*;

    #[test]
    fn extract_invite_from_full_package() {
        let package = serde_json::json!({
            "qualia_coop_share": "1",
            "project_id": "board-1",
            "project_name": "QualiaDB Development Cooperative",
            "invite_json": {
                "version": 1,
                "code": "QUALIA-TEST-TEST",
                "inviter_name": "Host",
                "inviter_did": "did:host",
                "inviter_pubkey_hex": "",
                "relay_endpoint": "",
                "front_door_did": "did:host",
                "profile_card": {},
                "created_at": 1,
                "expires_at": 9_999_999_999u64,
                "signature_hex": ""
            }
        });
        let s = extract_invite_json_from_package(&package).expect("extract");
        assert!(s.contains("did:host"));
        assert!(s.contains("QUALIA-TEST-TEST"));
    }

    #[test]
    fn extract_invite_from_string_field() {
        let package = serde_json::json!({
            "qualia_coop_share": "1",
            "invite_json": "{\"version\":1,\"code\":\"X\",\"inviter_did\":\"did:x\",\"signature_hex\":\"\"}"
        });
        let s = extract_invite_json_from_package(&package).expect("extract string");
        assert!(s.contains("did:x"));
    }

    #[test]
    fn extract_bare_invite_object() {
        let invite = serde_json::json!({
            "version": 1,
            "inviter_did": "did:bare",
            "signature_hex": "aa",
            "code": "C"
        });
        let s = extract_invite_json_from_package(&invite).expect("bare");
        assert!(s.contains("did:bare"));
    }

    #[test]
    fn extract_rejects_empty_package() {
        let package = serde_json::json!({ "qualia_coop_share": "1" });
        assert!(extract_invite_json_from_package(&package).is_err());
    }
}

/// Create a group chat for a project from its collaborator roster (+ optional extra DIDs).
/// Returns `{ session_id, title, participants }`.
pub fn create_project_group_chat(
    project_id: String,
    project_name: String,
    extra_dids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let mut dids: Vec<String> = crate::project_collab::list(Some(&project_id))
        .into_iter()
        .map(|c| c.member_did)
        .collect();
    if let Some(extra) = extra_dids {
        for d in extra {
            let d = d.trim().to_string();
            if !d.is_empty() && !dids.iter().any(|x| x == &d) {
                dids.push(d);
            }
        }
    }
    // Also fold in all chat contacts if roster empty — last resort so a group can start.
    if dids.is_empty() {
        for c in crate::social_connect::list_chat_contacts() {
            if !c.did.is_empty() {
                dids.push(c.did);
            }
        }
    }
    if dids.is_empty() {
        return Err(
            "No participants — admit collaborators on the project or accept invites under People first."
                .into(),
        );
    }
    let title = if project_name.trim().is_empty() {
        format!("Project {project_id}")
    } else {
        format!("Project: {project_name}")
    };
    let session_id = create_group_chat_session(Some(title.clone()), dids.clone())?;
    Ok(serde_json::json!({
        "session_id": session_id,
        "title": title,
        "participants": dids,
        "project_id": project_id,
        "message": "Group chat created — open it under Talk → Chat → Conversations.",
    }))
}

/// Resolve how mail to `to_address` would be handled (exact / catchall / reject) — pure, for UI/debug.
pub fn resolve_mail_delivery(to_address: String) -> Result<serde_json::Value, String> {
    let addresses = crate::domains::list_addresses(None);
    match crate::domains::resolve_delivery(&addresses, &to_address) {
        crate::domains::DeliveryResolution::Deliver { address, via } => Ok(serde_json::json!({
            "deliver": true,
            "via": via,
            "address": address,
        })),
        crate::domains::DeliveryResolution::Reject { reason } => Ok(serde_json::json!({
            "deliver": false,
            "rejected": reason,
        })),
    }
}

/// Persist SMTP/IMAP prefs (app_meta_dir). Passwords stored locally — same trust as desktop secrets.
pub fn save_mail_transport_config(smtp_json: String, imap_json: String) -> Result<serde_json::Value, String> {
    let path = crate::state::app_meta_dir().join("mail_transport.json");
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let smtp: Option<crate::mail_transport::SmtpConfig> = if smtp_json.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str(&smtp_json).map_err(|e| format!("bad smtp: {e}"))?)
    };
    let imap: Option<crate::mail_transport::ImapConfig> = if imap_json.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str(&imap_json).map_err(|e| format!("bad imap: {e}"))?)
    };
    let doc = serde_json::json!({ "smtp": smtp, "imap": imap });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "saved": true, "path": path.display().to_string() }))
}

pub fn load_mail_transport_config() -> Result<serde_json::Value, String> {
    let path = crate::state::app_meta_dir().join("mail_transport.json");
    match std::fs::read_to_string(path) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| e.to_string()),
        Err(_) => Ok(serde_json::json!({ "smtp": null, "imap": null })),
    }
}

/// Mint a per-relationship (pairwise) address bound to a relationship DID.
pub fn mint_relationship_address(
    domain: String,
    local: String,
    relationship_did: String,
) -> Result<serde_json::Value, String> {
    let a = crate::domains::make_relationship_address(&domain, &local, &relationship_did, mail_now_unix())?;
    crate::domains::upsert_address(a)?;
    list_mail_addresses(Some(domain))
}

/// Enable/disable an address (the surgical per-relationship revoke). Returns the refreshed list.
pub fn set_mail_address_enabled(
    address: String,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    crate::domains::set_address_enabled(&address, enabled)?;
    list_mail_addresses(None)
}

/// The QDP front-door forms for a domain — the **DNS TXT** (no-hosting anchor), the DNS record name, and
/// the rich profile in **Turtle + JSON-LD** (served by the local HTTP server over the mesh when hosting).
pub fn front_door_forms(domain: String) -> Result<serde_json::Value, String> {
    let d = crate::domains::list_domains()
        .into_iter()
        .find(|d| d.name == domain)
        .ok_or_else(|| format!("unknown domain '{domain}'"))?;
    // Any eCash address minted as a service could be attached here later; start from the domain identity.
    let rec = crate::front_door::FrontDoorRecord {
        domain: d.name.clone(),
        agent_type: d.agent_type.clone(),
        front_door_did: d.front_door_did.clone(),
        name: if d.label.is_empty() { None } else { Some(d.label.clone()) },
        webid: None,
        services: vec![],
        identity_pubkey_hex: None,
        wireguard_pubkey_hex: None,
        overlay_addr: None,
        profile_url: None,
    };
    let mail_dns = crate::mail_inbound::mail_dns_forms(&d.name, None);
    Ok(serde_json::json!({
        "dns_name": crate::front_door::dns_record_name(&d.name),
        "dns_txt": rec.to_dns_txt(),
        "turtle": rec.to_turtle(),
        "jsonld": rec.to_json_ld(),
        "mail_dns": mail_dns,
    }))
}

/// Build a QDP front-door record from a stored domain's identity (shared by publish/serve).
fn build_front_door_record(domain: &str) -> Result<crate::front_door::FrontDoorRecord, String> {
    let d = crate::domains::list_domains()
        .into_iter()
        .find(|d| d.name == domain)
        .ok_or_else(|| format!("unknown domain '{domain}'"))?;
    Ok(crate::front_door::FrontDoorRecord {
        domain: d.name.clone(),
        agent_type: d.agent_type.clone(),
        front_door_did: d.front_door_did.clone(),
        name: if d.label.is_empty() { None } else { Some(d.label.clone()) },
        webid: None,
        services: vec![],
        identity_pubkey_hex: None,
        wireguard_pubkey_hex: None,
        overlay_addr: None,
        profile_url: None,
    })
}

/// Verify a Cloudflare API token (the easy-install front-door publishing path).
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_verify_token(token: String) -> Result<serde_json::Value, String> {
    crate::cloudflare::verify_token(&token)?;
    Ok(serde_json::json!({ "ok": true }))
}

/// List the Cloudflare zones (domains) the token can manage.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_list_zones(token: String) -> Result<serde_json::Value, String> {
    let zones = crate::cloudflare::list_zones(&token)?;
    Ok(serde_json::json!(zones
        .into_iter()
        .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
        .collect::<Vec<_>>()))
}

/// Publish the domain's `_qdp` TXT front-door record to Cloudflare (no hosting needed).
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_publish_front_door(
    token: String,
    zone_id: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    let rec = build_front_door_record(&domain)?;
    let cfg = crate::cloudflare::CfConfig { api_token: token, zone_id };
    let id = crate::cloudflare::publish_front_door(&cfg, &rec)?;
    Ok(serde_json::json!({
        "record_id": id,
        "dns_name": crate::front_door::dns_record_name(&domain),
        "dns_txt": rec.to_dns_txt(),
    }))
}

/// Provision a Cloudflare R2 bucket.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_provision_r2_bucket(
    token: String,
    account_id: String,
    bucket_name: String,
) -> Result<serde_json::Value, String> {
    crate::cloudflare::provision_r2_bucket(&token, &account_id, &bucket_name)?;
    Ok(serde_json::json!({ "ok": true, "bucket": bucket_name }))
}

/// Provision a Cloudflare Worker using the local vendor JS file.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_provision_worker(
    token: String,
    account_id: String,
    script_name: String,
) -> Result<serde_json::Value, String> {
    let script_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/cloudflare-worker/worker.js");
    let script_content = std::fs::read_to_string(&script_path)
        .map_err(|e| format!("Failed to read worker.js: {e}"))?;
        
    crate::cloudflare::provision_worker(&token, &account_id, &script_name, &script_content)?;
    Ok(serde_json::json!({ "ok": true, "script": script_name }))
}

/// Provision a Cloudflare Tunnel.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_provision_tunnel(
    token: String,
    account_id: String,
    tunnel_name: String,
    tunnel_secret: String,
) -> Result<serde_json::Value, String> {
    let tunnel_id = crate::cloudflare::provision_tunnel(&token, &account_id, &tunnel_name, &tunnel_secret)?;
    Ok(serde_json::json!({ "ok": true, "tunnel_id": tunnel_id }))
}

/// Route a Cloudflare Tunnel in DNS.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_route_tunnel_dns(
    token: String,
    zone_id: String,
    record_name: String,
    tunnel_id: String,
) -> Result<serde_json::Value, String> {
    let record_id = crate::cloudflare::route_tunnel_dns(&token, &zone_id, &record_name, &tunnel_id)?;
    Ok(serde_json::json!({ "ok": true, "record_id": record_id }))
}

/// Verify a GitHub PAT.
#[cfg(not(target_arch = "wasm32"))]
pub fn github_verify_token(token: String) -> Result<serde_json::Value, String> {
    let login = crate::github::verify_github_token(&token).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "login": login }))
}

/// Create a GitHub repository and push static site files.
#[cfg(not(target_arch = "wasm32"))]
pub fn github_deploy_static_site(
    token: String,
    repo_name: String,
    files: std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    let full_name = crate::github::create_repository(&token, &repo_name).map_err(|e| e.to_string())?;
    crate::github::push_static_site(&token, &full_name, files).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "full_name": full_name }))
}

/// Provision a Cloudflare Pages project linked to a GitHub repo.
#[cfg(not(target_arch = "wasm32"))]
pub fn cf_provision_pages_project(
    token: String,
    account_id: String,
    project_name: String,
    github_repo: String,
) -> Result<serde_json::Value, String> {
    crate::cloudflare::provision_pages_project(&token, &account_id, &project_name, &github_repo)?;
    Ok(serde_json::json!({ "ok": true, "project": project_name }))
}

/// Start serving `/.well-known/QDP` for a domain from a local HTTP server (self-host over the mesh).
#[cfg(not(target_arch = "wasm32"))]
pub fn start_qdp_server(domain: String, bind_addr: String) -> Result<serde_json::Value, String> {
    let rec = build_front_door_record(&domain)?;
    let addr = bind_addr.clone();
    std::thread::spawn(move || {
        let _ = crate::qdp_http::serve_blocking(rec, &addr);
    });
    Ok(serde_json::json!({ "serving": bind_addr, "path": crate::qdp_server::WELL_KNOWN_QDP_PATH }))
}

/// Parse a magic link (deep link / https / bare `qcx1_…`) into the connection identifier it carries.
pub fn parse_magic_link(link: String) -> Result<serde_json::Value, String> {
    let id = crate::magic_link::from_link(&link)?;
    serde_json::to_value(id).map_err(|e| e.to_string())
}

/// Send mail via SMTP. `smtp_json` = `SmtpConfig`, `mail_json` = `OutgoingMail`.
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_send(smtp_json: String, mail_json: String) -> Result<serde_json::Value, String> {
    let cfg: crate::mail_transport::SmtpConfig =
        serde_json::from_str(&smtp_json).map_err(|e| format!("bad smtp config: {e}"))?;
    let mail: crate::mail_transport::OutgoingMail =
        serde_json::from_str(&mail_json).map_err(|e| format!("bad mail: {e}"))?;
    crate::mail_transport::send(&cfg, &mail)?;
    Ok(serde_json::json!({ "sent": true }))
}

/// Fetch unseen mail via IMAP, apply semantic delivery + rules, and **store accepted mail**
/// in the local inbox (same store as the SMTP receiver).
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_fetch(imap_json: String, mailbox: String) -> Result<serde_json::Value, String> {
    let cfg: crate::mail_transport::ImapConfig =
        serde_json::from_str(&imap_json).map_err(|e| format!("bad imap config: {e}"))?;
    let msgs = crate::mail_transport::fetch_unseen(&cfg, &mailbox)?;
    let mut stored_ids = Vec::new();
    let evaluated: Vec<serde_json::Value> = msgs
        .into_iter()
        .map(|m| {
            let to = if m.to_address.contains('@') {
                m.to_address.clone()
            } else {
                cfg.username.clone()
            };
            // Reuse the product accept path so IMAP import and SMTP land in one inbox.
            let r = crate::mail_inbound::accept_message(
                &m.from_address,
                &to,
                &m.subject,
                &format!("(imported via IMAP; size {} bytes)", m.size_bytes),
                m.sender_verified,
                m.sender_did.clone(),
            );
            if let Some(ref s) = r.stored {
                stored_ids.push(s.id.clone());
            }
            serde_json::json!({
                "message": m,
                "accepted": r.accepted,
                "rejected": r.rejected,
                "stored": r.stored,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "messages": evaluated,
        "stored_ids": stored_ids,
        "stored_count": stored_ids.len(),
    }))
}

