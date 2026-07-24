//! Domains, semantic mail, DNS, magic links, mesh, coop projects

#![allow(non_snake_case)]

use qualia_client_core::api;
use tauri::command;

#[command]
pub fn list_mail_domains() -> Result<serde_json::Value, String> {
    api::list_mail_domains()
}

/// Add a context-domain. `agent_type` token: person/org/ai/service/content/group.
#[command]
pub fn add_mail_domain(
    name: String,
    agent_type: String,
    front_door_did: String,
    label: String,
    parent: Option<String>,
) -> Result<serde_json::Value, String> {
    api::add_mail_domain(name, agent_type, front_door_did, label, parent)
}

/// Built-in purpose-inbox presets (frontdoor/junkmail/mygov/newsletters).
#[command]
pub fn purpose_inbox_presets() -> Result<serde_json::Value, String> {
    api::purpose_inbox_presets()
}

/// Addresses (optionally filtered to one domain).
#[command]
pub fn list_mail_addresses(domain: Option<String>) -> Result<serde_json::Value, String> {
    api::list_mail_addresses(domain)
}

/// Mint a purpose inbox (`frontdoor@`, `junkmail@`, …). `rules_json` is a `MailRules` object (or empty).
#[command]
pub fn mint_purpose_inbox(
    domain: String,
    local: String,
    rules_json: String,
) -> Result<serde_json::Value, String> {
    api::mint_purpose_inbox(domain, local, rules_json)
}

/// Onboard mail: mint purpose presets + catchall; auto-starts local SMTP receiver.
#[command]
pub fn onboard_mail_domain(domain: String) -> Result<serde_json::Value, String> {
    api::onboard_mail_domain(domain)
}

/// Talk first-run readiness (domains, mailboxes, receiver, people).
#[command]
pub fn talk_setup_status() -> Result<serde_json::Value, String> {
    api::talk_setup_status()
}

/// Resolve delivery for a to-address (exact / catchall / reject).
#[command]
pub fn resolve_mail_delivery(to_address: String) -> Result<serde_json::Value, String> {
    api::resolve_mail_delivery(to_address)
}

/// Save SMTP/IMAP transport prefs (local file under app meta).
#[command]
pub fn save_mail_transport_config(
    smtp_json: String,
    imap_json: String,
) -> Result<serde_json::Value, String> {
    api::save_mail_transport_config(smtp_json, imap_json)
}

/// Load SMTP/IMAP transport prefs.
#[command]
pub fn load_mail_transport_config() -> Result<serde_json::Value, String> {
    api::load_mail_transport_config()
}

/// Mint a per-relationship (pairwise) address bound to a relationship DID.
#[command]
pub fn mint_relationship_address(
    domain: String,
    local: String,
    relationship_did: String,
) -> Result<serde_json::Value, String> {
    api::mint_relationship_address(domain, local, relationship_did)
}

/// Enable/disable an address (the surgical per-relationship revoke).
#[command]
pub fn set_mail_address_enabled(address: String, enabled: bool) -> Result<serde_json::Value, String> {
    api::set_mail_address_enabled(address, enabled)
}

/// The QDP front-door forms for a domain — DNS TXT (no-hosting anchor), record name, Turtle, JSON-LD.
#[command]
pub fn front_door_forms(domain: String) -> Result<serde_json::Value, String> {
    api::front_door_forms(domain)
}

/// Verify a Cloudflare API token (easy-install front-door publishing).
#[command]
pub fn cf_verify_token(token: String) -> Result<serde_json::Value, String> {
    api::cf_verify_token(token)
}

/// List the Cloudflare zones (domains) the token can manage.
#[command]
pub fn cf_list_zones(token: String) -> Result<serde_json::Value, String> {
    api::cf_list_zones(token)
}

/// Publish the domain's `_qdp` TXT front-door record to Cloudflare (no hosting needed).
#[command]
pub fn cf_publish_front_door(
    token: String,
    zone_id: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    api::cf_publish_front_door(token, zone_id, domain)
}

/// Deploy full Cloudflare Node infrastructure (R2 + Worker + Tunnel + DNS).
#[command]
pub fn cf_deploy_infrastructure(
    token: String,
    account_id: String,
    zone_id: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    // 1. Provision R2 Bucket
    let bucket_name = format!("qualia-offline-{}", domain.replace('.', "-"));
    api::cf_provision_r2_bucket(token.clone(), account_id.clone(), bucket_name.clone())?;
    
    // 2. Provision Worker
    let script_name = format!("qualia-relay-{}", domain.replace('.', "-"));
    api::cf_provision_worker(token.clone(), account_id.clone(), script_name.clone())?;
    
    // 3. Provision Tunnel
    let tunnel_name = format!("qualia-tunnel-{}", domain.replace('.', "-"));
    // Generate a secure 32-byte secret (mocked with simple random bytes for demo)
    let secret_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    use base64::{engine::general_purpose, Engine as _};
    let tunnel_secret = general_purpose::STANDARD.encode(&secret_bytes);
    
    let tunnel_res = api::cf_provision_tunnel(token.clone(), account_id.clone(), tunnel_name.clone(), tunnel_secret)?;
    let tunnel_id = tunnel_res.get("tunnel_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    
    // 4. Route Tunnel DNS
    api::cf_route_tunnel_dns(token.clone(), zone_id, domain.clone(), tunnel_id.clone())?;
    
    Ok(serde_json::json!({
        "ok": true,
        "bucket": bucket_name,
        "worker": script_name,
        "tunnel_id": tunnel_id,
        "domain": domain
    }))
}

/// Deploy a static site to GitHub and provision a Cloudflare Pages project.
#[command]
pub fn deploy_static_site_cf_pages(
    github_token: String,
    github_repo: String,
    cf_token: String,
    cf_account: String,
) -> Result<serde_json::Value, String> {
    // 1. Verify GitHub token and create repo
    let full_name = api::github_deploy_static_site(
        github_token.clone(),
        github_repo.clone(),
        std::collections::HashMap::from([
            ("index.html".to_string(), "<h1>Welcome to Qualia Webizen</h1><p>Static site deployed via Cloudflare Pages and GitHub.</p>".to_string()),
        ]),
    )?;
    
    let repo_full_name = full_name.get("full_name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

    // 2. Provision Cloudflare Pages Project
    let project_name = format!("qualia-site-{}", github_repo.replace('.', "-").to_lowercase());
    api::cf_provision_pages_project(cf_token, cf_account, project_name.clone(), repo_full_name.clone())?;

    Ok(serde_json::json!({
        "ok": true,
        "github_repo": repo_full_name,
        "cf_project": project_name
    }))
}

/// Start serving `/.well-known/QDP` for a domain over a local HTTP server (self-host over the mesh).
#[command]
pub fn start_qdp_server(domain: String, bind_addr: String) -> Result<serde_json::Value, String> {
    api::start_qdp_server(domain, bind_addr)
}

/// Parse a magic link (deep link / https / bare `qcx1_…`) into the connection identifier it carries.
#[command]
pub fn parse_magic_link(link: String) -> Result<serde_json::Value, String> {
    api::parse_magic_link(link)
}

/// Send mail via SMTP (`smtp_json` = SmtpConfig, `mail_json` = OutgoingMail).
#[command]
pub fn mail_send(smtp_json: String, mail_json: String) -> Result<serde_json::Value, String> {
    api::mail_send(smtp_json, mail_json)
}

/// Fetch unseen mail via IMAP + apply rules; accepted messages land in the local inbox.
#[command]
pub fn mail_fetch(imap_json: String, mailbox: String) -> Result<serde_json::Value, String> {
    api::mail_fetch(imap_json, mailbox)
}

/// Accept mail into the local product inbox (same path as SMTP DATA).
#[command]
pub fn mail_accept(
    from: String,
    to: String,
    subject: String,
    body: String,
    sender_verified: Option<bool>,
) -> Result<serde_json::Value, String> {
    api::mail_accept(from, to, subject, body, sender_verified.unwrap_or(false))
}

/// List local inbox messages.
#[command]
pub fn mail_list(
    mailbox: Option<String>,
    include_quarantine: Option<bool>,
) -> Result<serde_json::Value, String> {
    api::mail_list(mailbox, include_quarantine)
}

#[command]
pub fn mail_get(id: String) -> Result<serde_json::Value, String> {
    api::mail_get(id)
}

#[command]
pub fn mail_set_read(id: String, read: bool) -> Result<serde_json::Value, String> {
    api::mail_set_read(id, read)
}

#[command]
pub fn mail_delete(id: String) -> Result<serde_json::Value, String> {
    api::mail_delete(id)
}

/// MX/SPF DNS paste block for a domain.
#[command]
pub fn mail_dns_forms(domain: String, mx_host: Option<String>) -> Result<serde_json::Value, String> {
    api::mail_dns_forms(domain, mx_host)
}

#[command]
pub fn mail_receiver_status() -> Result<serde_json::Value, String> {
    api::mail_receiver_status()
}

/// Start local SMTP receiver (default 127.0.0.1:2525).
#[command]
pub fn mail_receiver_start(bind: Option<String>) -> Result<serde_json::Value, String> {
    api::mail_receiver_start(bind)
}

#[command]
pub fn mail_receiver_stop() -> Result<serde_json::Value, String> {
    api::mail_receiver_stop()
}

/// A signed connection identifier for this node (front-door DID + WireGuard peering).
#[command]
pub fn generate_connection_identifier(
    front_door_did: String,
    relation_type: String,
) -> Result<serde_json::Value, String> {
    api::generate_connection_identifier(front_door_did, relation_type)
}

/// A magic link (deep link + https + mailto) carrying this node's connection identifier.
#[command]
pub fn generate_magic_link(
    front_door_did: String,
    relation_type: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    api::generate_magic_link(front_door_did, relation_type, domain)
}

/// Accept a magic link: verify the identifier, then register the sender as a SocialWebNet peer.
#[command]
pub fn accept_connection(link: String) -> Result<serde_json::Value, String> {
    api::accept_connection(link)
}

/// The SocialWebNet peers (accepted connections).
#[command]
pub fn list_social_peers() -> Result<serde_json::Value, String> {
    api::list_social_peers()
}

/// Enable/disable a peer (the socially-defined revoke).
#[command]
pub fn set_social_peer_active(did: String, active: bool) -> Result<serde_json::Value, String> {
    api::set_social_peer_active(did, active)
}

/// Answer a connection challenge — prove this node controls its identity key.
#[command]
pub fn answer_connection_challenge(
    challenge_json: String,
    my_did: String,
) -> Result<serde_json::Value, String> {
    api::answer_connection_challenge(challenge_json, my_did)
}

/// Per-peer SocialWebNet mesh dialability (who can form a tunnel now / on roaming / not at all).
#[command]
pub fn mesh_dialability() -> Result<serde_json::Value, String> {
    api::mesh_dialability()
}

/// Local project collaborator roster (people + agents on a cooperative project).
#[command]
pub fn list_project_collaborators(
    project_id: Option<String>,
) -> Result<serde_json::Value, String> {
    api::list_project_collaborators(project_id)
}

/// Local-first coop projects (no vault required).
#[command]
pub fn list_coop_projects() -> Result<serde_json::Value, String> {
    api::list_coop_projects()
}

#[command]
pub fn create_coop_project(name: String, description: String) -> Result<serde_json::Value, String> {
    api::create_coop_project(name, description)
}

#[command]
pub fn add_project_collaborator(
    project_id: String,
    project_name: String,
    member_did: String,
    display_name: String,
    role: String,
) -> Result<serde_json::Value, String> {
    api::add_project_collaborator(project_id, project_name, member_did, display_name, role)
}

#[command]
pub fn remove_project_collaborator(
    project_id: String,
    member_did: String,
) -> Result<serde_json::Value, String> {
    api::remove_project_collaborator(project_id, member_did)
}

/// Pasteable coop share package (no private keys; embeds connect invite when allowed).
#[command]
pub fn coop_share_package(
    project_id: Option<String>,
    project_name: Option<String>,
) -> Result<serde_json::Value, String> {
    api::coop_share_package(project_id, project_name)
}

/// Accept a full coop share package or bare invite JSON (connect + project scope).
#[command]
pub fn accept_coop_share_package(package_or_invite: String) -> Result<serde_json::Value, String> {
    api::accept_coop_share_package(package_or_invite)
}

/// Group chat from project collaborator roster.
#[command]
pub fn create_project_group_chat(
    project_id: String,
    project_name: String,
    extra_dids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    api::create_project_group_chat(project_id, project_name, extra_dids)
}

/// Set a social peer's dial endpoint (`host:port`) so mesh can dial them.
#[command]
pub fn set_social_peer_endpoint(
    did: String,
    endpoint: Option<String>,
) -> Result<serde_json::Value, String> {
    api::set_social_peer_endpoint(did, endpoint)
}

/// All peer agreements.
#[command]
pub fn list_agreements() -> Result<serde_json::Value, String> {
    api::list_agreements()
}

/// Agreements a DID is party to (fills the directory's agreement slot).
#[command]
pub fn agreements_for(did: String) -> Result<serde_json::Value, String> {
    api::agreements_for(did)
}

/// Create a draft agreement for a relationship (grounded in the values floor).
#[command]
pub fn create_agreement(
    title: String,
    relationship_did: String,
    parties: Vec<String>,
) -> Result<serde_json::Value, String> {
    api::create_agreement(title, relationship_did, parties)
}

/// Persist a full agreement (JSON) — for edits.
#[command]
pub fn save_agreement(agreement_json: String) -> Result<serde_json::Value, String> {
    api::save_agreement(agreement_json)
}

/// Set a party's consent on an agreement (pending / granted / withdrawn).
#[command]
pub fn set_agreement_consent(
    id: String,
    did: String,
    state: String,
) -> Result<serde_json::Value, String> {
    api::set_agreement_consent(id, did, state)
}

// -- QPU Oracle / Advanced Capabilities ----------------------------------------

#[command]
pub fn get_qpu_settings() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    Ok(qualia_client_core::qpu_oracle::get_qpu_settings())
}

#[command]
pub fn save_qpu_settings(
    input: qualia_client_core::qpu_oracle::QpuOracleSettingsInput,
) -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::save_qpu_settings(input)
}

#[command]
pub fn enable_qpu_feature() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::enable_qpu_feature()
}

#[command]
pub fn disable_qpu_feature() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::disable_qpu_feature()
}

/// Activate the QPU Oracle and advanced capabilities by affirming the
/// Universal Human Rights commitment.
///
/// `commitment` must be "I Affirm My Commitment to Universal Human Rights"
/// or the base64 form `SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz`.
#[command]
pub fn activate_advanced_capabilities(
    commitment: String,
) -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::activate_with_commitment(&commitment)
}

/// Check whether the advanced capabilities commitment has been affirmed.
#[command]
pub fn get_advanced_activation_status() -> bool {
    qualia_client_core::qpu_oracle::is_qpu_feature_unlocked()
}

/// Return the commitment text that must be affirmed to activate.
#[command]
pub fn get_commitment_prompt() -> serde_json::Value {
    serde_json::json!({
        "text": "I Affirm My Commitment to Universal Human Rights",
        "key": "SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz",
        "description": "By affirming this commitment you agree that the advanced computational \
                        capabilities of QualiaDB — including quantum computing offload, \
                        physics-informed neural networks, and advanced scientific solvers — \
                        will be used in accordance with the Universal Declaration of Human Rights \
                        and in ways that benefit humanity.",
        "udhr_url": "https://www.un.org/en/about-us/universal-declaration-of-human-rights"
    })
}
#[command]
pub fn submit_omnibox_query(query: String) -> String {
    let q = query.trim();
    let ql = q.to_ascii_lowercase();
    // Home / Chora universe (browser default content).
    if ql.is_empty()
        || ql == "home"
        || ql == "chora"
        || ql == "universe"
        || ql == "about:home"
        || crate::browser::is_chora_universe_url(q)
    {
        return crate::browser::DEFAULT_HOME.to_string();
    }
    if q.contains("my did") || q.contains("my webid") {
        return "qualia://webid/did:q42:local".to_string();
    }
    if q.contains("thermal") || q.contains("status") {
        return "qualia://internal/monitor".to_string();
    }
    if ql == "hello" {
        return "qualia://internal/dialectical-sidebar".to_string();
    }
    if q.starts_with("did:q42:") || q.starts_with("did:") {
        return format!("qualia://webid/{}", q);
    }
    let looks_like_domain = !q.contains(' ')
        && q.contains('.')
        && !q.starts_with("http://")
        && !q.starts_with("https://")
        && !q.starts_with("qualia://")
        && !q.starts_with("webizen://");
    if looks_like_domain {
        return format!("https://{}", q);
    }
    if q.starts_with("http://") || q.starts_with("https://") || q.starts_with("qualia://") || q.starts_with("webizen://") {
        q.to_string()
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(q))
    }
}

#[command]
pub async fn resolve_qdp_did(domain: String) -> Result<String, String> {
    qualia_client_core::dns_resolver::resolve_qdp_did(&domain).await
}

#[command]
pub fn get_ns_records_for_did(did: String) -> Result<Vec<String>, String> {
    qualia_client_core::dns_resolver::ns_records_for_did(&did)
        .map(|(ns1, ns2)| vec![ns1, ns2])
        .ok_or_else(|| {
            format!(
                "Cannot encode '{}' as NS records — only did:q42: is supported",
                did
            )
        })
}

#[command]
pub async fn sync_to_solid_pod(
    pod_url: String,
    body_or_path: Option<String>,
    bearer_token: Option<String>,
) -> Result<String, String> {
    api::sync_to_solid_pod(pod_url, body_or_path, bearer_token).await
}

#[command]
pub async fn fetch_from_solid_pod(
    url: String,
    bearer_token: Option<String>,
) -> Result<serde_json::Value, String> {
    api::fetch_from_solid_pod(url, bearer_token).await
}

#[command]
pub async fn put_to_solid_pod(
    url: String,
    body: Vec<u8>,
    content_type: Option<String>,
    bearer_token: Option<String>,
) -> Result<serde_json::Value, String> {
    api::put_to_solid_pod(url, body, content_type, bearer_token).await
}

#[command]
pub async fn evaluate_data_request(
    requester_did: String,
    _requested_subgraph: String,
) -> Result<String, String> {
    if requester_did.contains("professional") {
        Ok("Permit".to_string())
    } else if requester_did.contains("suspended") || requester_did.contains("handshake") {
        Ok("Suspended".to_string())
    } else {
        Ok("Forbid".to_string())
    }
}

#[command]
pub async fn apply_semantic_handshake(
    requester_did: String,
    decision: String,
) -> Result<String, String> {
    if decision == "Accept" {
        Ok(format!("Semantic Handshake Accepted for {}", requester_did))
    } else {
        Ok(format!("Semantic Handshake Rejected for {}", requester_did))
    }
}

#[command]
pub async fn save_qlink(
    app: tauri::AppHandle,
    url: String,
    title: String,
    context_assertions: Option<Vec<serde_json::Value>>,
) -> Result<String, String> {
    use qualia_client_core::state::{config_file_path, AgentConfig};
    use qualia_client_core::wellfair::bookmarks;
    use scraper::{Html, Selector};
    use std::fs;
    use tauri::Manager;

    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("empty URL".into());
    }

    let mut final_title = title.clone();
    let mut description = String::new();
    let mut extracted_content = String::new();

    if let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(html_text) = response.text().await {
                let document = Html::parse_document(&html_text);
                if let Ok(title_sel) = Selector::parse("title") {
                    if let Some(t_el) = document.select(&title_sel).next() {
                        let t = t_el.text().collect::<Vec<_>>().join("");
                        if !t.trim().is_empty() {
                            final_title = t.trim().to_string();
                        }
                    }
                }
                if let Ok(og_desc) =
                    Selector::parse("meta[property='og:description'], meta[name='description']")
                {
                    if let Some(desc_el) = document.select(&og_desc).next() {
                        if let Some(content) = desc_el.value().attr("content") {
                            description = content.to_string();
                        }
                    }
                }
                if let Ok(json_ld) = Selector::parse("script[type='application/ld+json']") {
                    for script in document.select(&json_ld) {
                        let ld_text = script.text().collect::<Vec<_>>().join("");
                        extracted_content
                            .push_str(&format!("\n```json\n{}\n```\n", ld_text));
                    }
                }
            }
        }
    }
    if final_title.trim().is_empty() {
        final_title = url.clone();
    }

    let combined_text = format!(
        "Bookmark: {}\nURL: {}\nDescription: {}\nStructured Data:\n{}",
        final_title, url, description, extracted_content
    );

    let state = app.state::<crate::HostApiState>();
    let url_clone = url.clone();
    let combined_text_clone = combined_text.clone();
    let ingested_result = {
        state.0.execute_sync(move |guard| {
            if let Some(host) = guard.as_ref() {
                let manual = qualia_client_core::wellfair::api::ManualFacets {
                    occurred_at: Some(chrono::Utc::now().timestamp()),
                    place_label: None,
                    lat: None,
                    lon: None,
                    projects: vec!["browser".into()],
                    purposes: vec!["bookmark".to_string()],
                    sensitivity: Some("public".into()),
                    section: Some("personal".into()),
                    commons_visibility: Some("none".into()),
                };
                host.ingest_document_annotated(
                    &url_clone,
                    "text/html",
                    &combined_text_clone,
                    &manual,
                    None,
                )
                .map_err(|e| e.to_string())
            } else {
                Err("Host API not initialized (vault/host locked or not started)".to_string())
            }
        })
    };

    let config_path = config_file_path();
    let storage_path = if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<AgentConfig>(&config_str) {
            config.storage_path
        } else {
            qualia_client_core::state::dirs_default_path()
        }
    } else {
        qualia_client_core::state::dirs_default_path()
    };

    let ingested_ok = ingested_result.is_ok();
    let (_id, file_path) = bookmarks::write_qlink_json(
        std::path::Path::new(&storage_path),
        &url,
        &final_title,
        &description,
        ingested_ok,
        context_assertions,
    )?;

    match ingested_result {
        Ok(_) => Ok(format!(
            "Bookmark saved to Library (purpose=bookmark) and {:?}",
            file_path
        )),
        Err(e) => Ok(format!(
            "Bookmark saved offline to {:?} — library ingest skipped: {}",
            file_path, e
        )),
    }
}

#[command]
pub fn compute_context_hash(url: String) -> serde_json::Value {
    let context_hash = qualia_core_db::q_hash(&url);
    serde_json::json!({
        "url": url,
        "context_hash": context_hash,
        "context_hash_hex": format!("{:016x}", context_hash),
    })
}

/// Native computational-geometry host route for qapps.
///
/// This shares the exact JSON contract used by the MCP tool, so a qapp can use
/// `invoke("run_computational_geometry", { request })` in the desktop shell
/// and the same operation through MCP in agent/development workflows.
#[command]
pub fn run_computational_geometry(
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let output =
        qualia_core_db::specialized_libs::computational_geometry::execute_geometry_tool_json(
            &request.to_string(),
        )
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&output).map_err(|error| error.to_string())
}

