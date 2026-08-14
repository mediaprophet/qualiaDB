use qualia_client_core::api;
use qualia_client_core::local_job_scheduler::{JobQueueSnapshot, LocalJobScheduler};
use std::time::Duration;
use tauri::webview::WebviewWindowBuilder;
use tauri::{command, AppHandle, Manager, State};

use crate::native_surface::NativeSurfaceState;
use crate::runtime::{RuntimeHandle, RuntimeLedgerHealth, RuntimeSnapshotRecord};

// ── Sub-modules ───────────────────────────────────────────────────────────────

pub mod agent_qa;
pub mod binary_registry;
pub mod glb_ingest;
pub mod mesh;
pub mod render_pipeline;
pub use mesh::MeshState;

pub mod browser;
pub mod system;
/// App-wide mindware HID (entity projection, observer, morph) — not browser-only.
pub mod view_api;
pub mod wellfair;
pub use wellfair::{
    sanctuary_basic::wellfair_lock_sanctuary, sync::wellfair_diagnostics,
    sync::wellfair_sync_with_relay,
};
pub mod directory;
pub mod ingest;
pub mod mail;
pub mod personal_directory;
pub mod qapp_host;
pub mod qapp_telemetry;
pub mod semantic;
pub mod social;
pub mod wallet;
pub use qapp_host::HostApiState;
pub mod qapp_export;
pub mod render;
pub use render::{
    render_preview_tick, ActiveAnchor, AnatomyBodyState, PreviewState, RenderLoopState,
};
pub mod browser_10d;
pub mod native_bindings;
pub mod semantic_logic;
pub mod telemetry;
pub mod updater;
pub mod vision_audio;

// ── Shared types & helpers ────────────────────────────────────────────────────

const MAX_CLIENT_DIAGNOSTIC_BYTES: usize = 16 * 1024;

fn bounded_client_text(value: String) -> String {
    if value.len() <= MAX_CLIENT_DIAGNOSTIC_BYTES {
        return value;
    }
    let mut end = MAX_CLIENT_DIAGNOSTIC_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...[truncated]", &value[..end])
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiffusionConfigInput {
    pub width: u32,
    pub height: u32,
    pub diffusion_rate: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalPreviewProbe {
    pub target_url: String,
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QappWasmExport {
    pub url: String,
    pub lan_url: String,
    pub lan_ip: String,
    pub package_dir: String,
    pub note: String,
}

fn resolve_web_pkg_src() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap_or(&manifest_dir)
        .join("webizen-web")
        .join("pkg")
}

fn guess_lan_ipv4() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_ipv4() {
        Some(ip.to_string())
    } else {
        None
    }
}

fn ensure_lan_export_server(export_base: std::path::PathBuf, port: u16) {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(move || {
        tauri::async_runtime::spawn(async move {
            use axum::{routing::get_service, Router};
            use tower_http::services::ServeDir;
            let app = Router::new().fallback_service(get_service(ServeDir::new(export_base)));
            let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", port)).await else {
                eprintln!("LAN export server: failed to bind 0.0.0.0:{port}");
                return;
            };
            println!("LAN export server listening on http://0.0.0.0:{port}");
            if let Err(err) = axum::serve(listener, app).await {
                eprintln!("LAN export server error: {err}");
            }
        });
    });
}

/// Temporal slice state for time-travel navigation
#[derive(Clone)]
pub struct TemporalSlice(pub std::sync::Arc<std::sync::atomic::AtomicU64>);

impl TemporalSlice {
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }

    pub fn set(&self, value: f64) {
        self.0
            .store(value.to_bits(), std::sync::atomic::Ordering::SeqCst);
    }
}

// ── Desktop status / logs / supervisor (kept here — small, core) ──────────────

#[command]
pub fn get_desktop_status(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> serde_json::Value {
    let config = state.config.lock().unwrap().clone();
    let daemon_running = *state.daemon_running.lock().unwrap();
    let jobs = LocalJobScheduler::global()
        .snapshot()
        .unwrap_or(JobQueueSnapshot {
            jobs: vec![],
            queued: 0,
            running: 0,
            completed: 0,
            failed: 0,
        });
    serde_json::json!({
        "settings_port": crate::settings_server::current_settings_port(),
        "graph_daemon_port": qualia_client_core::api::get_active_daemon_port(),
        "graph_daemon_reachable": daemon_running,
        "graph_engine_version": serde_json::Value::Null,
        "qapps_protocol_port": qualia_client_core::qapps_protocol::qualia_protocol_port(),
        "storage_path": config.storage_path,
        "inference_backend": config.inference_backend,
        "daemon_running_flag": daemon_running,
        "job_queue": {
            "queued": jobs.queued, "running": jobs.running,
            "completed": jobs.completed, "failed": jobs.failed,
        }
    })
}

#[command]
pub fn get_desktop_logs() -> serde_json::Value {
    serde_json::json!({
        "log_file": crate::desktop_log::log_path().display().to_string(),
        "debug_enabled": crate::desktop_log::debug_enabled(),
        "entries": crate::desktop_log::recent_entries(),
    })
}

#[command]
pub fn set_desktop_debug_mode(enabled: bool) -> Result<bool, String> {
    crate::desktop_log::set_debug_enabled(enabled)?;
    Ok(crate::desktop_log::debug_enabled())
}

#[command]
pub fn get_supervisor_state(
    supervisor: State<'_, crate::supervisor::DesktopSupervisor>,
) -> serde_json::Value {
    serde_json::json!({
        "services": supervisor.services(),
        "operations": supervisor.operations(),
    })
}

#[command]
pub fn report_client_error(
    kind: String,
    message: String,
    stack: Option<String>,
    url: Option<String>,
) {
    crate::desktop_log::record(
        "error",
        format!(
            "webview {}: {}; url={}; stack={}",
            bounded_client_text(kind),
            bounded_client_text(message),
            url.map(bounded_client_text).unwrap_or_default(),
            stack.map(bounded_client_text).unwrap_or_default(),
        ),
    );
}

// ── QApp vault (kept here — tiny, used by launch flow) ────────────────────────

#[command]
pub fn list_installed_qapps() -> Vec<String> {
    api::list_installed_qapps()
}

#[command]
pub fn generate_qapp_credential(qapp_name: String) -> String {
    api::generate_qapp_credential(qapp_name)
}

#[command]
pub fn verify_and_install_qapp(target_path: String) -> Result<String, String> {
    api::verify_and_install_qapp(target_path)
}

#[command]
pub fn launch_installed_qapp(app: AppHandle, qapp_name: String) -> Result<(), String> {
    let url = api::launch_installed_qapp(qapp_name.clone())?;
    let label: String = format!(
        "qapp-{}",
        qapp_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );

    if let Some(window) = app.get_webview_window(&label) {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let parsed = url
        .parse()
        .map_err(|e| format!("Invalid launch URL '{url}': {e}"))?;
    WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::External(parsed))
        .title(qapp_name)
        .inner_size(1200.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn parse_hex_u64(s: &str) -> Result<u64, String> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(t, 16).map_err(|e| format!("bad hex u64 '{s}': {e}"))
}

pub fn get_invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        // ── Local (mod.rs) ──
        get_desktop_status,
        get_desktop_logs,
        set_desktop_debug_mode,
        get_supervisor_state,
        report_client_error,
        list_installed_qapps,
        generate_qapp_credential,
        verify_and_install_qapp,
        launch_installed_qapp,
        // ── semantic_logic ──
        semantic_logic::execute_sparql_query,
        semantic_logic::fetch_domain_ontology,
        semantic_logic::validate_shacl_shape,
        semantic_logic::evaluate_logic_rules,
        // ── entity-view \(desktop-wide: shell, studio, browser) ──
        view_api::view_session,
        view_api::view_set_observer,
        view_api::view_set_presentation_level,
        view_api::view_project_library,
        view_api::view_project_web_locus,
        view_api::view_morph,
        view_api::view_pick_scene,
        view_api::view_select,
        view_api::view_select_uri,
        view_api::view_clear_selection,
        view_api::view_bifurcate_package,
        view_api::view_capability_report,
        view_api::view_render_memory_spatial,
        view_api::view_remote_controller_info,
        view_api::view_set_circumstance,
        view_api::shell_navigate,
        // ── browser ──
        browser::open_web_url,
        browser::browser_navigate,
        browser::browser_navigate_content,
        browser::browser_content_url,
        browser::browser_focus,
        browser::browser_reload,
        browser::browser_reload_content,
        browser::browser_go_back,
        browser::browser_go_forward,
        browser::browser_status,
        browser::browser_trust_list,
        browser::browser_trust_add_pem,
        browser::browser_trust_add_did,
        browser::browser_trust_set_enabled,
        browser::browser_trust_remove,
        browser::browser_trust_verdict,
        browser::browser_agent_ask,
        browser::browser_cookie_summary,
        browser::browser_cookie_observe,
        browser::browser_clear_site_data,
        browser::browser_cert_escape_hatch,
        browser::browser_agent_tls_status,
        browser::browser_cookies_refresh,
        browser::browser_trust_list_suggested,
        browser::browser_trust_import_suggested,
        browser::browser_cert_override_status,
        browser::browser_cert_override_set_enabled,
        browser::browser_cert_override_attach,
        browser::browser_trust_pin_host,
        browser::browser_engine_status,
        browser::browser_set_engine,
        browser::list_qlinks,
        // ── system ──
        system::get_hardware_status,
        system::profile_energy_circumstance,
        system::start_daemon,
        system::daemon_status,
        system::get_active_daemon_port,
        system::qualia_protocol_port,
        system::run_engine_command,
        system::get_config,
        system::save_config,
        system::get_setup_state,
        system::complete_setup_step,
        system::update_setup_profile,
        system::finish_setup,
        system::get_identity_plane,
        system::list_apparatus_devices,
        system::export_person_public,
        system::export_person_transfer_bundle,
        system::import_person_transfer_bundle,
        system::register_remote_apparatus_device,
        system::resolve_job_device_placement,
        system::schedule_job_on_device,
        system::set_local_control_base_url,
        system::list_remote_job_outbox,
        system::retry_remote_job_outbox,
        system::mint_person_webid_tls_cert,
        system::accept_fleet_job_envelope,
        // ── agent QA / structured diagnostics ──
        agent_qa::agent_qa_snapshot,
        agent_qa::agent_qa_test_active_model,
        // ── wellfair ──
        wellfair::health::wellfair_host_snapshot,
        wellfair::health::wellfair_list_health_records,
        wellfair::health::wellfair_list_receipts,
        wellfair::health::wellfair_export_health_package,
        wellfair::health::wellfair_save_accessibility,
        wellfair::health::wellfair_companion_pairing,
        wellfair::health::wellfair_import_samsung_folder,
        wellfair::health::wellfair_ingest_companion_health,
        wellfair::policy::wellfair_evaluate_policy,
        wellfair::policy::wellfair_grant_consent,
        wellfair::policy::wellfair_revoke_consent,
        wellfair::policy::wellfair_list_consents,
        wellfair::policy::wellfair_add_condition,
        wellfair::policy::wellfair_add_allergy,
        wellfair::policy::wellfair_add_disputed_diagnosis,
        wellfair::policy::wellfair_add_housing_safety,
        wellfair::med_reminder::wellfair_med_reminder_prefs,
        wellfair::med_reminder::wellfair_grant_med_reminder_permission,
        wellfair::med_reminder::wellfair_set_med_reminders_enabled,
        wellfair::med_reminder::wellfair_list_due_med_reminders,
        wellfair::med_reminder::wellfair_query_graph_coverage,
        wellfair::sanctuary_basic::wellfair_sanctuary_prefs,
        wellfair::sanctuary_basic::wellfair_setup_sanctuary,
        wellfair::sanctuary_basic::wellfair_lock_sanctuary,
        wellfair::sanctuary_basic::wellfair_unlock_sanctuary,
        wellfair::life_records::wellfair_add_life_event,
        wellfair::life_records::wellfair_add_welfare_case,
        wellfair::life_records::wellfair_add_case_task,
        wellfair::finance::wellfair_add_ledger_entry,
        wellfair::finance::wellfair_ledger_balance,
        wellfair::projects::wellfair_add_project,
        wellfair::projects::wellfair_add_project_membership,
        wellfair::projects::wellfair_add_contribution,
        wellfair::projects::wellfair_project_obligations,
        wellfair::credentials::wellfair_add_credential,
        wellfair::credentials::wellfair_get_credential,
        wellfair::credentials::wellfair_present_credential,
        wellfair::work_items::wellfair_add_work_item,
        wellfair::work_items::wellfair_add_work_item_status,
        wellfair::work_items::wellfair_work_item_board,
        wellfair::agency::wellfair_list_agency_domains,
        wellfair::agency::wellfair_list_agency_delegations,
        wellfair::agency::wellfair_create_agency_delegation,
        wellfair::agency::wellfair_set_agency_delegation_consent,
        wellfair::agency::wellfair_revoke_agency_delegation,
        wellfair::agency::wellfair_evaluate_agency_access,
        wellfair::sync::wellfair_sync_with_relay,
        wellfair::sync::wellfair_export_backup,
        wellfair::sync::wellfair_import_backup,
        wellfair::sync::wellfair_diagnostics,
        wellfair::assessment_instruments::wellfair_list_assessment_instruments,
        wellfair::assessment::wellfair_record_assessment,
        wellfair::assessment::wellfair_list_assessments,
        wellfair::anatomy::wellfair_compute_anatomy_view,
        wellfair::anatomy::wellfair_eval_comorbidity,
        wellfair::anatomy::wellfair_compute_scorecard,
        wellfair::anatomy::wellfair_get_weight_model,
        wellfair::anatomy::wellfair_set_weight_model,
        wellfair::anatomy::wellfair_reset_weight_model,
        wellfair::anatomy::wellfair_get_physiological_state,
        wellfair::anatomy::wellfair_set_physiological_state,
        wellfair::anatomy::wellfair_reset_physiological_state,
        wellfair::anatomy::wellfair_render_body_snapshot,
        wellfair::anatomy::wellfair_body_assets_status,
        wellfair::anatomy::wellfair_acquire_body_assets,
        wellfair::anatomy::wellfair_load_cached_organ_10d,
        wellfair::anatomy::wellfair_cached_body_organ_percepts,
        wellfair::anatomy::wellfair_clear_body_cache,
        wellfair::ledger::wellfair_ledger_append,
        wellfair::ledger::wellfair_ledger_verify,
        wellfair::ledger::wellfair_ledger_entries,
        wellfair::consent_creds::wellfair_grant_consent_credential,
        wellfair::consent_creds::wellfair_revoke_consent_credential,
        wellfair::consent_creds::wellfair_list_consent_credentials,
        wellfair::consent_creds::wellfair_record_conduct,
        wellfair::consent_creds::wellfair_conduct_audit_trail,
        wellfair::library::wellfair_ingest_document,
        wellfair::library::wellfair_ingest_file_hex,
        wellfair::library::wellfair_search_library,
        wellfair::library::wellfair_search_library_time,
        wellfair::library::wellfair_list_library,
        wellfair::library::wellfair_search_library_text,
        wellfair::library::wellfair_query_library_faceted,
        wellfair::library::wellfair_library_facet_counts,
        wellfair::library::wellfair_seed_studio_qapps,
        wellfair::library::wellfair_seed_perception_library,
        wellfair::library::library_seed_perception_assets,
        wellfair::library::library_list,
        wellfair::library::library_query_faceted,
        wellfair::library::library_stats,
        wellfair::library::library_search,
        wellfair::library::library_search_text,
        wellfair::library::library_search_time,
        wellfair::library::wellfair_ingest_legislation_text,
        wellfair::library::wellfair_ingest_legislation_pdf_hex,
        wellfair::library::wellfair_build_cml_context,
        wellfair::library::wellfair_build_cof_package,
        wellfair::library::wellfair_enrich_library_cml,
        wellfair::library::wellfair_list_qapp_catalog_categories,
        wellfair::library::wellfair_library_stats,
        wellfair::library::wellfair_remove_library_entry,
        wellfair::library::wellfair_export_library_graph,
        wellfair::library::wellfair_set_library_commons,
        wellfair::library::wellfair_library_commons_share_card,
        wellfair::chora::chora_list_worlds,
        wellfair::chora::chora_get_world,
        wellfair::chora::chora_save_world,
        wellfair::chora::chora_delete_world,
        wellfair::chora::chora_seed_demo,
        wellfair::chora::chora_seed_flagships,
        wellfair::chora::chora_navigation,
        wellfair::chora::chora_set_temporal,
        wellfair::chora::chora_set_active_world,
        wellfair::chora::chora_query_region,
        wellfair::chora::chora_publish_asset,
        wellfair::chora::chora_pull_assets,
        wellfair::chora::chora_list_layers,
        wellfair::chora::chora_list_categories,
        wellfair::chora::chora_get_layer,
        wellfair::chora::chora_download_layer,
        wellfair::chora::chora_load_layer_to_gpu,
        wellfair::crypto::wellfair_owner_envelope_public,
        wellfair::crypto::wellfair_seal_and_grant_credential,
        wellfair::crypto::wellfair_open_owner_payload,
        wellfair::safeguard::wellfair_arm_dead_mans_switch,
        wellfair::safeguard::wellfair_dead_mans_alive,
        wellfair::safeguard::wellfair_attest_dead_mans,
        wellfair::safeguard::wellfair_enact_dead_mans,
        wellfair::safeguard::wellfair_list_dead_mans_switches,
        wellfair::safeguard::wellfair_enact_dead_mans_release,
        wellfair::safeguard::wellfair_split_dek_recovery,
        wellfair::safeguard::wellfair_reconstruct_and_release,
        wellfair::safeguard::wellfair_set_peer_envelope_key,
        wellfair::safeguard::wellfair_enact_dead_mans_release_via_peers,
        wellfair::safeguard::wellfair_arm_incapacity_switch,
        wellfair::safeguard::wellfair_activate_incapacity,
        wellfair::safeguard::wellfair_regain_capacity,
        wellfair::safeguard::wellfair_list_incapacity_switches,
        wellfair::disclosure::wellfair_record_transparency_cc,
        wellfair::disclosure::wellfair_record_disclosure,
        wellfair::disclosure::wellfair_disclosure_chain,
        wellfair::disclosure::wellfair_actors_with_access,
        wellfair::disclosure::wellfair_trace_leak,
        wellfair::disclosure::wellfair_list_transparency_ccs,
        wellfair::disclosure::wellfair_assess_duty_of_inquiry,
        wellfair::guardianship::wellfair_propose_proxy_condition,
        wellfair::guardianship::wellfair_list_guardianship_proposals,
        wellfair::guardianship::wellfair_vote_guardianship_proposal,
        wellfair::clinical::wellfair_add_clinical_report,
        wellfair::clinical::wellfair_add_clinical_attachment_from_path,
        wellfair::clinical::wellfair_export_attachment,
        wellfair::clinical::wellfair_pick_file_path,
        wellfair::clinical::wellfair_pick_save_path,
        wellfair::clinical::wellfair_pick_directory,
        wellfair::clinical::wellfair_publish_qapp_pwa,
        wellfair::clinical::wellfair_add_government_letter_attachment_from_path,
        wellfair::welfare_support::wellfair_add_assistance_need,
        wellfair::welfare_support::wellfair_add_welfare_stream,
        wellfair::welfare_support::wellfair_add_government_letter,
        wellfair::welfare_support::wellfair_list_sync_inbox,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_configured,
        wellfair::sanctuary_vault::wellfair_setup_sanctuary_vault,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_add_note,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_list_notes,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_is_keychain_wrapped,
        wellfair::sanctuary_vault::wellfair_setup_sanctuary_vault_wrapped,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_unlock_with_recovery,
        wellfair::sanctuary_vault::wellfair_sanctuary_vault_add_note_in_session,
        wellfair::sanctuary_vault::wellfair_curate_decoy_note,
        wellfair::sanctuary_vault::wellfair_review_decoy_activity,
        wellfair::sanctuary_vault::wellfair_get_decoy_retention_mode,
        wellfair::sanctuary_vault::wellfair_set_decoy_retention_mode,
        wellfair::wellbeing::wellfair_add_wellbeing_observation,
        wellfair::wellbeing::wellfair_add_therapy_note,
        wellfair::wellbeing::wellfair_list_pending_live_shares,
        wellfair::wellbeing::wellfair_decide_live_share,
        wellfair::medication::wellfair_add_medication,
        wellfair::medication::wellfair_record_administration,
        wellfair::medication::wellfair_add_diet_entry,
        wellfair::medication::wellfair_sleep_analytics,
        wellfair::medication::wellfair_add_emergency_contact,
        wellfair::medication::wellfair_list_emergency_contacts,
        // ── wallet ──
        wallet::get_wallet_status,
        wallet::is_first_run,
        wallet::read_identity,
        wallet::save_identity,
        wallet::load_identity,
        wallet::get_coin_balances,
        wallet::get_transaction_history,
        wallet::generate_bip39_seed,
        wallet::derive_wallets_from_seed,
        wallet::import_external_seed,
        wallet::get_tokens,
        wallet::add_token,
        wallet::remove_token,
        wallet::get_tax_suite,
        wallet::save_tax_suite,
        wallet::dispatch_tax_payment,
        wallet::build_send_xec,
        wallet::confirm_send_xec,
        wallet::send_ecash_token,
        wallet::accept_vault_handshake,
        wallet::receive_vault_job,
        // ── ingest ──
        ingest::ingest_pdf,
        ingest::ingest_literature,
        ingest::upsert_cmld_definition,
        ingest::ingest_ontology,
        ingest::export_to_solid,
        ingest::ingest_image,
        ingest::ingest_image_async,
        ingest::verify_graph_equivalence,
        // ── inference (in ingest.rs) ──
        ingest::discover_models,
        ingest::download_and_vectorize,
        ingest::download_model,
        ingest::cancel_download,
        ingest::get_active_model,
        ingest::set_active_model,
        ingest::get_active_downloads,
        ingest::run_agent_inference,
        // ── semantic ──
        semantic::generate_front_door_invite,
        semantic::mint_semantic_token,
        semantic::fetch_wallet_portfolio,
        semantic::toggle_nym_relay,
        semantic::toggle_stark_prover,
        semantic::update_solar_input,
        semantic::fetch_torrent_telemetry,
        semantic::fetch_remote_manifest,
        // ── directory ──
        semantic::load_imported_accounts,
        semantic::save_imported_accounts,
        directory::get_front_doors,
        directory::generate_front_door,
        directory::get_directory_actors,
        directory::add_directory_actor,
        directory::get_delegation_rules,
        directory::add_delegation_rule,
        // ── social ──
        social::generate_connect_invite,
        social::accept_connect_invite,
        social::list_chat_contacts,
        social::get_user_profile,
        social::save_user_profile,
        social::list_chat_sessions,
        social::load_chat_session,
        social::create_group_chat_session,
        social::add_chat_participant,
        social::remove_chat_participant,
        social::get_chat_participants,
        social::append_chat_message,
        social::get_chat_graph,
        social::stream_chat_inference,
        social::cancel_chat_inference,
        social::create_chat_session,
        social::ensure_chat_session,
        social::agent_roster_list,
        social::agent_roster_get,
        social::agent_roster_upsert,
        social::agent_roster_remove,
        social::agent_runtime_status,
        social::agent_roster_add_remote,
        social::provider_credential_store,
        social::provider_credential_remove,
        social::agent_remote_connection_test,
        social::mcp_list_local_tools,
        social::mcp_call_tool_gated,
        social::agent_set_allowed_mcp_tools,
        social::mcp_ensure_safe_tool_allowlist,
        social::ingest_chat_cml,
        social::schedule_agent_job,
        social::list_local_jobs,
        social::cancel_local_job,
        social::retry_local_job,
        social::clear_finished_local_jobs,
        social::schedule_model_download,
        social::schedule_model_activation,
        social::schedule_anatomy_asset_acquire,
        // ── personal_directory ──
        personal_directory::list_directory,
        personal_directory::list_directory_categories,
        personal_directory::create_directory_category,
        personal_directory::set_directory_entry_categories,
        personal_directory::search_directory,
        // ── mail ──
        mail::list_mail_domains,
        mail::add_mail_domain,
        mail::purpose_inbox_presets,
        mail::list_mail_addresses,
        mail::mint_purpose_inbox,
        mail::onboard_mail_domain,
        mail::talk_setup_status,
        mail::resolve_mail_delivery,
        mail::save_mail_transport_config,
        mail::load_mail_transport_config,
        mail::mint_relationship_address,
        mail::set_mail_address_enabled,
        mail::front_door_forms,
        mail::cf_verify_token,
        mail::cf_list_zones,
        mail::cf_publish_front_door,
        mail::cf_deploy_infrastructure,
        mail::deploy_static_site_cf_pages,
        mail::start_qdp_server,
        mail::parse_magic_link,
        mail::mail_send,
        mail::mail_fetch,
        mail::mail_accept,
        mail::mail_list,
        mail::mail_get,
        mail::mail_set_read,
        mail::mail_delete,
        mail::mail_dns_forms,
        mail::mail_receiver_status,
        mail::mail_receiver_start,
        mail::mail_receiver_stop,
        mail::generate_connection_identifier,
        mail::generate_magic_link,
        mail::accept_connection,
        mail::list_social_peers,
        mail::set_social_peer_active,
        mail::answer_connection_challenge,
        mail::mesh_dialability,
        mail::list_project_collaborators,
        mail::list_coop_projects,
        mail::create_coop_project,
        mail::add_project_collaborator,
        mail::remove_project_collaborator,
        mail::coop_share_package,
        mail::accept_coop_share_package,
        mail::create_project_group_chat,
        mail::set_social_peer_endpoint,
        // ── personal_directory: agreements ──
        mail::list_agreements,
        mail::agreements_for,
        mail::create_agreement,
        mail::save_agreement,
        mail::set_agreement_consent,
        // ── qapp_telemetry ──
        qapp_telemetry::wellfair_get_model_lifecycle_status,
        qapp_telemetry::wellfair_force_model_lifecycle_phase,
        qapp_telemetry::wellfair_get_llm_telemetry,
        qapp_host::qapp_analyze,
        qapp_host::certify_forge_physics,
        qapp_host::run_forge_compute_probe,
        qapp_host::get_qualia_compute_profile,
        // ── qapp_host ──
        qapp_host::submit_record,
        qapp_host::get_latest_diffusion_snapshot,
        qapp_host::reconfigure_diffusion,
        qapp_host::get_diffusion_frame_rgba,
        qapp_host::get_diffusion_ledger_health,
        qapp_host::probe_localhost_preview,
        // ── qapp_export ──
        qapp_export::export_qapp_as_wasm_package,
        // ── render ──
        render::update_render_preview,
        render::toggle_render_loop,
        render::navigate_to_node,
        render::select_node_at,
        // ── telemetry ──
        telemetry::collapse_wavefunction,
        telemetry::collapse_wavefunction_legacy,
        telemetry::set_temporal_slice,
        telemetry::register_browser_capabilities,
        // ── native_bindings ──
        native_bindings::calculate_chemistry_properties,
        native_bindings::calculate_framingham_risk,
        native_bindings::calculate_quantum_dft,
        native_bindings::calculate_monte_carlo_var,
        // ── browser_10d ──
        browser_10d::browse_10d_containers,
        browser_10d::browse_vision_10d,
        browser_10d::load_vision_10d,
        browser_10d::scrub_vision_10d_paint,
        browser_10d::inspect_10d_container,
        browser_10d::open_10d_file_picker,
        // ── vision_audio ──
        vision_audio::vision_run_synthetic_demo,
        vision_audio::vision_reject_instance,
        vision_audio::vision_correct_instance,
        vision_audio::vision_generate_image,
        vision_audio::vision_image_to_3d_demo,
        vision_audio::vision_gs_continuum,
        vision_audio::audio_ears_demo,
        vision_audio::audio_cross_modal_demo,
        vision_audio::audio_section18_smoke,
        vision_audio::audio_import_wav,
        vision_audio::audio_reject_instance,
        vision_audio::audio_correct_instance,
        vision_audio::vision_detect_image_file,
        vision_audio::vision_section15_smoke,
        vision_audio::audio_ears_weighted,
        vision_audio::audio_sonify_hear,
        vision_audio::audio_speech_demo,
        vision_audio::audio_capture_policy_demo,
        vision_audio::audio_pick_wav_path,
        vision_audio::audio_mic_start,
        vision_audio::audio_mic_stop,
        vision_audio::audio_mic_status,
        vision_audio::audio_ensure_weights,
        vision_audio::audio_daw_history_demo,
        vision_audio::audio_live_aed,
        vision_audio::audio_speech_disk,
        vision_audio::vision_ensure_weights,
        vision_audio::vision_detect_disk_weights_demo,
        vision_audio::vision_twin_elasticity_demo,
        vision_audio::vision_super_resolve,
        vision_audio::audio_music_demo,
        vision_audio::audio_daw_fx_demo,
        vision_audio::audio_gen_demo,
        vision_audio::audio_shared_clock_demo,
        vision_audio::audio_mixer_default,
        vision_audio::audio_mixer_bounce,
        vision_audio::audio_capabilities,
        // ── updater ──
        updater::updater_check,
        updater::updater_download_and_install,
        updater::updater_restart,
        // ── mesh (existing sub-module) ──
        mesh::mesh_start,
        mesh::mesh_stop,
        mesh::mesh_status,
        // ── Native GPU surface commands (existing, in native_surface) ──
        crate::native_surface::mount_gpu_surface,
        crate::native_surface::set_gpu_scene,
        crate::native_surface::set_gpu_camera,
        crate::native_surface::set_gpu_camera_mode,
        crate::native_surface::upload_gpu_mesh,
        crate::native_surface::upload_gpu_mesh_colored,
        crate::native_surface::upload_gpu_10d_mesh,
        crate::native_surface::load_gpu_10d_asset,
        crate::native_surface::unmount_gpu_surface,
        // ── Binary IPC (existing sub-modules) ──
        telemetry::load_ccf_asset,
        telemetry::list_ccf_assets,
        telemetry::test_ccf_ipc_handshake,
        telemetry::test_larynx_smoke,
        telemetry::test_vasculature_stress,
        // ── QPU (in qapp_telemetry) ──
        mail::get_qpu_settings,
        mail::save_qpu_settings,
        mail::enable_qpu_feature,
        mail::disable_qpu_feature,
        mail::activate_advanced_capabilities,
        mail::get_advanced_activation_status,
        mail::get_commitment_prompt,
        mail::submit_omnibox_query,
        // ── Solid pod (in semantic) ──
        mail::resolve_qdp_did,
        mail::get_ns_records_for_did,
        mail::sync_to_solid_pod,
        mail::fetch_from_solid_pod,
        mail::put_to_solid_pod,
        mail::evaluate_data_request,
        mail::apply_semantic_handshake,
        mail::save_qlink,
        mail::compute_context_hash,
        mail::run_computational_geometry
    ]
}
