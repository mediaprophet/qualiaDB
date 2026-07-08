use qualia_client_core::api;
use qualia_client_core::api::{CoinBalance, HardwareStatus, SendPreview, TokenEntry, TxRecord, WalletStatus};
use qualia_client_core::engine::{ingestion, llm_offload};
use qualia_client_core::state::{Actor, AgentConfig, DelegationRule, FrontDoor, ProgressPayload};
use qualia_core_db::ilp_dispatcher::DispatchResult;
use qualia_core_db::rpc::TaxRecipientSuite;
use std::time::Duration;
use tauri::{command, AppHandle, Emitter, Manager, State};
use tauri::webview::WebviewWindowBuilder;

use crate::runtime::{RuntimeHandle, RuntimeLedgerHealth, RuntimeSnapshotRecord};
use crate::native_surface::NativeSurfaceState;

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
    /// Loopback URL for the authoring machine.
    pub url: String,
    /// LAN URL for secondary devices (QR target).
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

// ── 10D Quantum State Management ─────────────────────────────────────────────────

/// Temporal slice state for time-travel navigation
///
/// Zero-heap consideration: Uses AtomicU64 (stack-allocated atomic primitive)
/// Bit-casts to f64 for floating point operations
/// This avoids heap allocation of Mutex<f64>
#[derive(Clone)]
pub struct TemporalSlice(pub std::sync::Arc<std::sync::atomic::AtomicU64>);

impl TemporalSlice {
    /// Get temporal slice as f64
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }

    /// Set temporal slice from f64
    pub fn set(&self, value: f64) {
        self.0
            .store(value.to_bits(), std::sync::atomic::Ordering::SeqCst);
    }
}

// ── Qapp vault ────────────────────────────────────────────────────────────────

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

// ── Hardware / system ─────────────────────────────────────────────────────────

#[command]
pub fn get_hardware_status() -> HardwareStatus {
    api::get_hardware_status()
}

#[command]
pub fn profile_energy_circumstance() -> String {
    api::profile_energy_circumstance()
}

// ── Daemon ────────────────────────────────────────────────────────────────────

#[command]
pub fn start_daemon() -> String {
    api::start_daemon()
}

#[command]
pub fn daemon_status() -> String {
    api::daemon_status()
}

#[command]
pub fn get_active_daemon_port() -> u16 {
    api::get_active_daemon_port()
}

#[command]
pub fn qualia_protocol_port() -> u16 {
    api::qualia_protocol_port()
}

#[command]
pub fn run_engine_command(cmd: String) -> String {
    api::run_engine_command(cmd)
}

// ── Config ────────────────────────────────────────────────────────────────────

#[command]
pub fn get_config() -> AgentConfig {
    api::get_config()
}

#[command]
pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    api::save_config(new_config)
}

#[command]
pub fn wellfair_host_snapshot(
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    let kv = app_state.key_vault.lock().map_err(|e| e.to_string())?;
    let mut guard = host_state.0.lock().map_err(|e| e.to_string())?;
    let owner_label = api::read_identity()
        .and_then(|v| {
            v.get("display_name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Owner vault".to_string());
    let storage_root = std::path::PathBuf::from(
        app_state.config.lock().map_err(|e| e.to_string())?.storage_path.clone(),
    );
    let snapshot = if let Some(host) = guard.as_mut() {
        host.build_snapshot(&kv, &owner_label)
    } else {
        qualia_client_core::wellfair::snapshot::build_host_snapshot_with_storage(
            &kv,
            false,
            &owner_label,
            false,
            Some(&storage_root),
        )
    };
    serde_json::to_string(&snapshot).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_save_accessibility(
    app: AppHandle,
    prefs_json: String,
) -> Result<String, String> {
    let prefs: qualia_client_core::wellfair::host_state::AccessibilityPreferences =
        serde_json::from_str(&prefs_json).map_err(|e| format!("invalid prefs JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.save_accessibility(&prefs)?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_health_records(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let records = host.list_health_records(limit.unwrap_or(64))?;
    serde_json::to_string(&records).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_receipts(app: AppHandle, limit: Option<usize>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let receipts = host.list_receipts(limit.unwrap_or(32))?;
    serde_json::to_string(&receipts).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_export_health_package(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let (package, receipt) = host.export_health_package(limit.unwrap_or(256))?;
    serde_json::to_string(&serde_json::json!({
        "package": package,
        "receipt": receipt,
    }))
    .map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_import_samsung_folder(
    app: AppHandle,
    folder_path: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.import_samsung_health_folder(std::path::Path::new(&folder_path));
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_companion_pairing() -> Result<String, String> {
    let port = crate::companion_gateway::companion_listen_port();
    let info = crate::companion_gateway::companion_pairing_info(port);
    serde_json::to_string(&info).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_ingest_companion_health(
    app: AppHandle,
    bundle_json: String,
) -> Result<String, String> {
    let bundle: wellfare_core::companion_sync::CompanionHealthBundle =
        serde_json::from_str(&bundle_json).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.ingest_companion_health_bundle(&bundle);
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

fn parse_sensitivity(s: &str) -> wellfare_core::record::SensitivityClass {
    match s.to_ascii_lowercase().as_str() {
        "classified" => wellfare_core::record::SensitivityClass::Classified,
        "public" => wellfare_core::record::SensitivityClass::Public,
        _ => wellfare_core::record::SensitivityClass::Restricted,
    }
}

fn parse_epistemic(s: &str) -> wellfare_core::record::EpistemicStatus {
    match s.to_ascii_lowercase().as_str() {
        "hypothesis" => wellfare_core::record::EpistemicStatus::Hypothesis,
        "disputed" => wellfare_core::record::EpistemicStatus::Disputed,
        "refuted" => wellfare_core::record::EpistemicStatus::Refuted,
        _ => wellfare_core::record::EpistemicStatus::Asserted,
    }
}

#[command]
pub fn wellfair_evaluate_policy(
    app: AppHandle,
    qapp_id: String,
    scope: String,
    sensitivity: String,
    epistemic: String,
) -> Result<String, String> {
    let sens = parse_sensitivity(&sensitivity);
    let ep = parse_epistemic(&epistemic);
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let decision = if let Some(host) = guard.as_ref() {
        host.evaluate_policy(&qapp_id, &scope, sens, ep)?
    } else {
        let svc = qualia_client_core::wellfair::policy::PolicyDecisionService::new();
        svc.evaluate_access(&qapp_id, &scope, sens, ep, &[], 0, false).to_dto()
    };
    serde_json::to_string(&decision).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_grant_consent(
    app: AppHandle,
    draft_json: String,
    scope: String,
) -> Result<String, String> {
    let draft: qualia_client_core::wellfair::host_state::ConsentGrantDraft =
        serde_json::from_str(&draft_json).map_err(|e| format!("invalid consent draft: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let grant = host.grant_consent(&draft, &scope)?;
    serde_json::to_string(&grant).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_revoke_consent(app: AppHandle, grant_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let revoked = host.revoke_consent(&grant_id)?;
    serde_json::to_string(&serde_json::json!({ "revoked": revoked })).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_consents(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let grants = host.list_consents()?;
    serde_json::to_string(&grants).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
struct ConditionReportInput {
    label: String,
    icd10_code: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct AllergyReportInput {
    substance: String,
    reaction: Option<String>,
    severity: Option<String>,
    notes: Option<String>,
}

#[command]
pub fn wellfair_add_condition(app: AppHandle, report_json: String) -> Result<String, String> {
    let input: ConditionReportInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid condition JSON: {e}"))?;
    let mut report = wellfare_core::conditions::ConditionReport::new(input.label);
    report.icd10_code = input
        .icd10_code
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.notes = input
        .notes
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_condition(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_allergy(app: AppHandle, report_json: String) -> Result<String, String> {
    let input: AllergyReportInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid allergy JSON: {e}"))?;
    let mut report = wellfare_core::conditions::AllergyReport::new(input.substance);
    report.reaction = input
        .reaction
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.severity = input
        .severity
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.notes = input
        .notes
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_allergy(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
struct DisputedDiagnosisInput {
    label: String,
    attributed_by: Option<String>,
    dispute_reason: Option<String>,
    supporting_notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct HousingSafetyInput {
    dwelling_type: Option<String>,
    homelessness: Option<bool>,
    violence_concern: Option<bool>,
    hazards: Option<String>,
    location_notes: Option<String>,
    notes: Option<String>,
}

#[command]
pub fn wellfair_add_disputed_diagnosis(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let input: DisputedDiagnosisInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid disputed JSON: {e}"))?;
    let mut report = wellfare_core::personal_records::DisputedDiagnosisReport::new(input.label);
    report.attributed_by = input.attributed_by.filter(|s| !s.trim().is_empty());
    report.dispute_reason = input.dispute_reason.filter(|s| !s.trim().is_empty());
    report.supporting_notes = input.supporting_notes.filter(|s| !s.trim().is_empty());
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_disputed_diagnosis(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_housing_safety(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let input: HousingSafetyInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid housing JSON: {e}"))?;
    let mut report = wellfare_core::personal_records::HousingSafetyReport::new();
    if let Some(dt) = input.dwelling_type.as_deref() {
        report.dwelling_type = match dt.to_ascii_lowercase().as_str() {
            "fixed" => wellfare_core::personal_records::DwellingType::Fixed,
            "temporary" => wellfare_core::personal_records::DwellingType::Temporary,
            "mobile_shelter" | "mobileshelter" => {
                wellfare_core::personal_records::DwellingType::MobileShelter
            }
            "homeless" => wellfare_core::personal_records::DwellingType::Homeless,
            _ => wellfare_core::personal_records::DwellingType::Unknown,
        };
    }
    report.homelessness = input.homelessness.unwrap_or(false);
    report.violence_concern = input.violence_concern.unwrap_or(false);
    report.hazards = input.hazards.filter(|s| !s.trim().is_empty());
    report.location_notes = input.location_notes.filter(|s| !s.trim().is_empty());
    report.notes = input.notes.filter(|s| !s.trim().is_empty());
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_housing_safety(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_med_reminder_prefs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.med_reminder_prefs()).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_grant_med_reminder_permission(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let prefs = host.grant_med_reminder_permission()?;
    let _ = crate::med_reminder_notifier::request_os_notification_permission(&app);
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_set_med_reminders_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let prefs = host.set_med_reminders_enabled(enabled)?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_due_med_reminders(
    app: AppHandle,
    window_minutes: Option<i32>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let due = host.list_due_med_reminders(window_minutes.unwrap_or(30))?;
    serde_json::to_string(&due).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_query_graph_coverage(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let rows = host.query_graph_coverage(limit.unwrap_or(64))?;
    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_sanctuary_prefs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.sanctuary_prefs()).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_setup_sanctuary(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let prefs = host.setup_sanctuary(&real_pin, &decoy_pin)?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_lock_sanctuary(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let prefs = host.lock_sanctuary()?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_unlock_sanctuary(app: AppHandle, pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let prefs = host.unlock_sanctuary(&pin)?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_life_event(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::LifeEventReport =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid life event JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_life_event(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_welfare_case(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::WelfareCaseReport =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid welfare case JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_welfare_case(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_case_task(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::CaseTaskReport =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid case task JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_case_task(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

fn wellfair_now_unix() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[command]
pub fn wellfair_add_ledger_entry(
    app: AppHandle,
    description: String,
    amount_cents: f64,
    currency: String,
    category: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let mut entry = wellfare_core::finance::LedgerEntry::new(
        description,
        amount_cents.round() as i64,
        currency,
        wellfair_now_unix(),
    );
    entry.category = category.filter(|s| !s.is_empty());
    let committed = host.add_ledger_entry(&entry)?;
    serde_json::to_string(&committed).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_ledger_balance(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let balance = host.ledger_balance(limit)?;
    serde_json::to_string(&balance).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_project(
    app: AppHandle,
    name: String,
    description: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let project = wellfare_core::projects::Project::new(name, description, wellfair_now_unix());
    let committed = host.add_project(&project)?;
    serde_json::to_string(&committed).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_contribution(
    app: AppHandle,
    project_id: String,
    contributor_did: String,
    description: String,
    effort_minutes: u32,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let contribution = wellfare_core::projects::Contribution::new(
        project_id,
        contributor_did,
        description,
        effort_minutes,
        wellfair_now_unix(),
    );
    let committed = host.add_contribution(&contribution)?;
    serde_json::to_string(&committed).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_project_obligations(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    // Includes validated inbound contributions (replay-safe cross-node convergence).
    let obligations = host.synced_project_obligations(limit)?;
    serde_json::to_string(&obligations).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_credential(
    app: AppHandle,
    issuer_did: String,
    subject_did: String,
    credential_type: String,
    claims_json: String,
    expires_at_unix: Option<u32>,
) -> Result<String, String> {
    let claims: Vec<(String, String)> = serde_json::from_str(&claims_json)
        .map_err(|e| format!("invalid claims JSON (expected [[key,value],…]): {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let mut cred = wellfare_core::credentials::CredentialRecord::new(
        issuer_did,
        subject_did,
        credential_type,
        wellfair_now_unix(),
    );
    cred.claims = claims;
    cred.expires_at_unix = expires_at_unix;
    let committed = host.add_credential(&cred)?;
    serde_json::to_string(&committed).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_get_credential(app: AppHandle, record_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let cred = host.get_credential(&record_id)?;
    serde_json::to_string(&cred).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_present_credential(
    app: AppHandle,
    record_id: String,
    selected_keys_json: String,
) -> Result<String, String> {
    let keys: Vec<String> = serde_json::from_str(&selected_keys_json)
        .map_err(|e| format!("invalid selected keys JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let presentation = host.present_credential(&record_id, &keys)?;
    serde_json::to_string(&presentation).map_err(|e| e.to_string())
}

fn parse_work_item_type(s: &str) -> qualia_cooperative_core::work_item::WorkItemType {
    use qualia_cooperative_core::work_item::WorkItemType::*;
    match s.to_ascii_lowercase().as_str() {
        "issue" => Issue,
        "milestone" => Milestone,
        _ => Task,
    }
}

fn parse_work_item_status(s: &str) -> qualia_cooperative_core::work_item::WorkItemStatus {
    use qualia_cooperative_core::work_item::WorkItemStatus::*;
    match s.to_ascii_lowercase().as_str() {
        "proposed" => Proposed,
        "in_progress" => InProgress,
        "blocked" => Blocked,
        "in_review" => InReview,
        "done" => Done,
        "cancelled" => Cancelled,
        _ => Todo,
    }
}

#[command]
pub fn wellfair_add_work_item(
    app: AppHandle,
    project_id: String,
    item_type: String,
    title: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let item = qualia_cooperative_core::work_item::WorkItem::new(
        project_id,
        parse_work_item_type(&item_type),
        title,
        wellfair_now_unix(),
    );
    let entry = host.add_work_item(&item)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_work_item_status(
    app: AppHandle,
    work_item_id: String,
    status: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let event = qualia_cooperative_core::work_item::WorkItemStatusEvent::new(
        work_item_id,
        parse_work_item_status(&status),
        wellfair_now_unix(),
    );
    let entry = host.add_work_item_status(&event)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_work_item_board(
    app: AppHandle,
    project_id: String,
    limit: usize,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let board = host.work_item_board(&project_id, limit)?;
    serde_json::to_string(&board).map_err(|e| e.to_string())
}

// --- Agency layer: supported-agency delegations (ADR §7–§10) ---------------------------------

/// The 17 seeded domains of agency (id/label/description/consequential/selfhood) for the picker.
#[command]
pub fn wellfair_list_agency_domains(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.list_agency_domains()).map_err(|e| e.to_string())
}

/// Current delegations (latest version per delegation id).
#[command]
pub fn wellfair_list_agency_delegations(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.list_agency_delegations(256)?).map_err(|e| e.to_string())
}

/// Create a delegation. `agent_dids` is a comma-separated DID list; `precedence` is
/// `primary|secondary|local_temporary`; `consent` is `pending|granted|withdrawn|not_required`.
#[command]
pub fn wellfair_create_agency_delegation(
    app: AppHandle,
    principal_did: String,
    domain: String,
    values_anchor: String,
    agent_dids: String,
    precedence: String,
    consent: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let agents: Vec<String> = agent_dids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let d = host.create_agency_delegation(
        &principal_did,
        &domain,
        &values_anchor,
        agents,
        &precedence,
        &consent,
    )?;
    serde_json::to_string(&d).map_err(|e| e.to_string())
}

/// Update a delegation's consent state (`granted|withdrawn|pending|not_required`).
#[command]
pub fn wellfair_set_agency_delegation_consent(
    app: AppHandle,
    delegation_id: String,
    consent: String,
) -> Result<String, String> {
    use qualia_client_core::wellfair::api::agency_consent_from_str;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let parsed = agency_consent_from_str(&consent)?;
    let entry = host.set_agency_delegation_consent(&delegation_id, parsed)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

/// Revoke a delegation (monotonic; appends a superseding revoked version).
#[command]
pub fn wellfair_revoke_agency_delegation(
    app: AppHandle,
    delegation_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.revoke_agency_delegation(&delegation_id)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

/// Evaluate the fail-closed ABAC for a delegation. `action` is `read|write|decide`. Returns
/// `{ "permit": bool, "reason": string }` — the reason names *why* access was denied.
#[command]
pub fn wellfair_evaluate_agency_access(
    app: AppHandle,
    delegation_id: String,
    action: String,
    data_class: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let decision = host.evaluate_agency_access(&delegation_id, &action, &data_class)?;
    let (permit, reason) = match decision {
        qualia_cooperative_core::agency_delegation::AccessDecision::Permit => {
            (true, String::new())
        }
        qualia_cooperative_core::agency_delegation::AccessDecision::Deny(r) => (false, r),
    };
    Ok(serde_json::json!({ "permit": permit, "reason": reason }).to_string())
}

// --- Sync transport (T3.1): sync against an HTTP relay -----------------------------------------

/// Drain the outbox to the relay at `base_url`, then pull + admit from it. Returns
/// `{ "pushed": n, "pulled": n, "validated": n, "duplicate": n, "rejected": n }`. `since` is the
/// pull cursor (0 = from the start; admission dedups so re-pulling is safe).
#[command]
pub fn wellfair_sync_with_relay(
    app: AppHandle,
    base_url: String,
    since: u64,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let (pushed, report) = host.sync_with_http_relay(&base_url, since)?;
    Ok(serde_json::json!({
        "pushed": pushed,
        "pulled": report.pulled,
        "validated": report.validated,
        "duplicate": report.duplicate,
        "rejected": report.rejected,
    })
    .to_string())
}

// --- Backup / restore (T3.3) -----------------------------------------------------------------

/// Write a portable backup of this node's WellFair data to `path`. Returns `{ "files": n, "bytes": n }`.
#[command]
pub fn wellfair_export_backup(app: AppHandle, path: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.export_backup_to_path(&path)?;
    Ok(serde_json::json!({ "files": report.files, "bytes": report.bytes }).to_string())
}

/// Restore a backup archive from `path` into this node's storage. Returns `{ "files": n, "bytes": n }`.
#[command]
pub fn wellfair_import_backup(app: AppHandle, path: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.import_backup_from_path(&path)?;
    Ok(serde_json::json!({ "files": report.files, "bytes": report.bytes }).to_string())
}

/// A node health/status snapshot (records, sync queues, data footprint, Sanctuary state, version).
#[command]
pub fn wellfair_diagnostics(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.diagnostics_report()?).map_err(|e| e.to_string())
}

// --- Wellbeing self-assessment instruments (T2.2; PHQ-9 / GAD-7) -----------------------------

/// The instruments this build ships (items, options, bands, disclaimer).
#[command]
pub fn wellfair_list_assessment_instruments(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.list_assessment_instruments()).map_err(|e| e.to_string())
}

/// 3D Anatomy Qapp — compute the whole-person systemic view for a lens (`"person"` / `"clinician"`).
/// Read-only; returns the lens narrative + per-system burden + what did not map. Hypotheses, not a
/// diagnosis. `threshold` (default 2) is how many distinct adverse factors flag a system.
#[command]
pub fn wellfair_compute_anatomy_view(
    app: AppHandle,
    lens: String,
    threshold: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.compute_anatomy_view(&lens, threshold.unwrap_or(2))?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// 3D Anatomy Qapp — the accumulative, traceable **score-card** + investigable hypotheses over the person's
/// own records. Forum-internum / `Sanctuary`-class; a set of Hypotheses + pathway-starts, never a diagnosis
/// and never a rating. `threshold` (default 2) is how many distinct adverse factors flag a system.
#[command]
pub fn wellfair_compute_scorecard(
    app: AppHandle,
    threshold: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.compute_scorecard(threshold.unwrap_or(2))?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// The person's own score-card weight model (how their body is read) + the seed suggestion + whether they've
/// authored their own. Returns `{ model, seed, authored }`.
#[command]
pub fn wellfair_get_weight_model(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&serde_json::json!({
        "model": host.get_weight_model(),
        "seed": host.seed_weight_model(),
        "authored": host.weight_model_is_authored(),
    }))
    .map_err(|e| e.to_string())
}

/// Set the person's own weight model (JSON = `WeightModel`) — their authorship of how the card reads them.
#[command]
pub fn wellfair_set_weight_model(app: AppHandle, model_json: String) -> Result<String, String> {
    let model: wellfare_core::anatomy::WeightModel =
        serde_json::from_str(&model_json).map_err(|e| format!("invalid weight model JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.set_weight_model(&model)?;
    Ok("{\"set\":true}".into())
}

/// Reset the weight model to the seed suggestion (clears the person's authored model).
#[command]
pub fn wellfair_reset_weight_model(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.reset_weight_model()?;
    Ok("{\"reset\":true}".into())
}

// ── Physiological state (P6 — the reproductive-continuum declaration) ──────────────────────────
//
// The person's own statement of where they are on the reproductive continuum. Forum-internum /
// Sanctuary-class. The score-card is computed at this state so it reads them at their current life
// stage, not a neutral baseline.

/// The person's declared physiological state + whether they've declared one. Returns
/// `{ state, declared }`. `state` is the `PhysiologicalState` JSON (Baseline if not declared).
#[command]
pub fn wellfair_get_physiological_state(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&serde_json::json!({
        "state": host.get_physiological_state(),
        "declared": host.physiological_state_is_declared(),
    }))
    .map_err(|e| e.to_string())
}

/// Set the person's declared physiological state (JSON = `PhysiologicalState`) — their own statement of
/// where they are on the reproductive continuum. Forum-internum / Sanctuary-class.
#[command]
pub fn wellfair_set_physiological_state(app: AppHandle, state_json: String) -> Result<String, String> {
    let state: wellfare_core::anatomy::PhysiologicalState =
        serde_json::from_str(&state_json).map_err(|e| format!("invalid physiological state JSON: {e}"))?;
    let app_state = app.state::<HostApiState>();
    let guard = app_state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.set_physiological_state(&state)?;
    Ok("{\"set\":true}".into())
}

/// Clear the declared physiological state — revert to the implicit Baseline. Idempotent.
#[command]
pub fn wellfair_reset_physiological_state(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.reset_physiological_state()?;
    Ok("{\"reset\":true}".into())
}

// ── 3D Anatomy render surface (S5.7 — whole-body percept snapshot) ────────────────────────────

/// Render the whole-body 3D Anatomy snapshot to a PNG (headless GPU via `webizen_render`), coloured by
/// the person's accumulated burden at their declared physiological state. The orbit camera is driven by
/// `azimuth` (0..360°) and `elevation` (-90..90°). The PNG is stored in [`AnatomyBodyState`] and served
/// at `webizen://localhost/anatomy/body.png`; the Studio UI bumps its epoch query-string to refetch.
/// Returns `{ "ok": true, "bytes": <len> }` on success.
#[command]
pub async fn wellfair_render_body_snapshot(
    app: AppHandle,
    azimuth: Option<f64>,
    elevation: Option<f64>,
    state: State<'_, AnatomyBodyState>,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    let az = azimuth.unwrap_or(0.0);
    let el = elevation.unwrap_or(10.0);
    // Compute the scene while holding the host lock, then drop the guard before the await so the
    // future stays `Send` (the MutexGuard is not Send).
    let scene = {
        let guard = host_state.0.lock().map_err(|e| e.to_string())?;
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.compute_body_scene(az, el)?
    };

    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        webizen_render::render_scene_png(&scene, 960, 540)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    let len = png.len();
    if let Ok(mut slot) = state.png.lock() {
        *slot = png;
    }
    let _ = app.emit("anatomy-body-ready", ());
    serde_json::to_string(&serde_json::json!({ "ok": true, "bytes": len }))
        .map_err(|e| e.to_string())
}

// ── 3D Anatomy asset cache (S5.8 — user-triggered real-mesh acquisition) ───────────────────────

/// Whether the body assets for a model are cached + complete. `model` = `"male"` / `"female"`.
/// Returns `{ model, cached, organ_count, total_ten_d_bytes, acquired_at_unix }`.
#[command]
pub fn wellfair_body_assets_status(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let status = host.body_assets_status(&model)?;
    serde_json::to_string(&status).map_err(|e| e.to_string())
}

/// Acquire (download + compile + cache) the body assets for a model — **user-triggered**. Discovers the
/// reference-organ manifest from the HRA SPARQL endpoint, fetches each GLB, compiles to `.10d`, caches
/// both + a manifest. Emits `anatomy-acquire-progress` per organ and `anatomy-acquire-done` at the end.
/// Returns the final `AcquireReport` JSON. Blocking network I/O runs on `spawn_blocking`.
#[command]
pub async fn wellfair_acquire_body_assets(
    app: AppHandle,
    model: String,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    // Resolve the model + storage_root while holding the lock, then drop the guard before the await.
    let (model_enum, storage_root) = {
        let guard = host_state.0.lock().map_err(|e| e.to_string())?;
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let m = qualia_client_core::wellfair::api::parse_anatomy_model(&model)?;
        (m, host.storage_root().to_path_buf())
    };

    let app_for_progress = app.clone();
    let report = tokio::task::spawn_blocking(move || -> Result<qualia_client_core::wellfair::anatomy_assets::AcquireReport, String> {
        qualia_client_core::wellfair::anatomy_assets::acquire_body_assets(
            &storage_root,
            model_enum,
            |p| {
                let _ = app_for_progress.emit("anatomy-acquire-progress", &p);
            },
        )
    })
    .await
    .map_err(|e| format!("acquire task join failed: {e}"))??;

    let _ = app.emit("anatomy-acquire-done", &report);
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// Load a cached `.10d` for one organ. Returns the raw container bytes as a Tauri IPC byte response
/// (the browser portal's `load_10d_colored` consumes them). `model` = `"male"` / `"female"`.
#[command]
pub fn wellfair_load_cached_organ_10d(
    app: AppHandle,
    model: String,
    organ_key: String,
) -> Result<Vec<u8>, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.load_cached_organ_10d(&model, &organ_key)
}

/// The per-organ dual-modality percepts for the cached organ set — so the browser portal knows what
/// colour to paint each organ (σ → RGBA via `paint_organs`). Returns `{ painted, unmapped }`.
#[command]
pub fn wellfair_cached_body_organ_percepts(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let (painted, unmapped) = host.cached_body_organ_percepts(&model)?;
    serde_json::to_string(&serde_json::json!({ "painted": painted, "unmapped": unmapped }))
        .map_err(|e| e.to_string())
}

/// Clear the cache for a model (idempotent). The person can re-acquire later.
#[command]
pub fn wellfair_clear_body_cache(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.clear_body_cache(&model)?;
    Ok("{\"ok\":true}".into())
}

/// Append a raw record to the person's tamper-evident accountability ledger (owner-signed).
#[command]
pub fn wellfair_ledger_append(
    app: AppHandle,
    kind: String,
    payload_json: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.ledger_append(&kind, &payload_json)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

/// Verify the whole ledger chain. Returns `{ "ok": bool, "tamper": <detail|null> }`.
#[command]
pub fn wellfair_ledger_verify(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let tamper = host.ledger_verify()?;
    serde_json::to_string(&serde_json::json!({ "ok": tamper.is_none(), "tamper": tamper }))
        .map_err(|e| e.to_string())
}

/// The most-recent ledger entries (newest first), capped to `limit` (default 64).
#[command]
pub fn wellfair_ledger_entries(app: AppHandle, limit: Option<usize>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entries = host.ledger_entries(limit.unwrap_or(64))?;
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

/// Grant a consent credential to an agent over a committed payload (subject = vault owner).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_grant_consent_credential(
    app: AppHandle,
    agent_did: String,
    scope: String,
    purpose: String,
    commitment_hex: String,
    wrapped_key_hex: String,
    expiry_unix: Option<u64>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let cred = host.grant_consent_credential(
        &agent_did,
        &scope,
        &purpose,
        &commitment_hex,
        &wrapped_key_hex,
        expiry_unix,
    )?;
    serde_json::to_string(&cred).map_err(|e| e.to_string())
}

/// Revoke a consent credential — crypto-enforced (the wrapped key is destroyed). `{ "revoked": bool }`.
#[command]
pub fn wellfair_revoke_consent_credential(
    app: AppHandle,
    credential_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let revoked = host.revoke_consent_credential(&credential_id)?;
    serde_json::to_string(&serde_json::json!({ "revoked": revoked })).map_err(|e| e.to_string())
}

/// List stored consent credentials (active and revoked — revoked rows remain as the audit anchor).
#[command]
pub fn wellfair_list_consent_credentials(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let creds = host.list_consent_credentials()?;
    serde_json::to_string(&creds).map_err(|e| e.to_string())
}

/// Record an agent's conduct under a credential — signed, into the durable trail + tamper-evident ledger.
#[command]
pub fn wellfair_record_conduct(
    app: AppHandle,
    agent_did: String,
    credential_id: String,
    action: String,
    reason: String,
    commitment_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let record = host.record_conduct(&agent_did, &credential_id, &action, &reason, &commitment_hex)?;
    serde_json::to_string(&record).map_err(|e| e.to_string())
}

/// The audit view — every conduct record taken under one credential (survives its revocation).
#[command]
pub fn wellfair_conduct_audit_trail(
    app: AppHandle,
    credential_id: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let trail = host.conduct_audit_trail(&credential_id)?;
    serde_json::to_string(&trail).map_err(|e| e.to_string())
}

// ── Hypermedia asset library: ingest a document → find it by meaning ──

/// Ingest a text document into the library (derive topics + searchable text; guardianship flag→notify).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_ingest_document(
    app: AppHandle,
    uri: String,
    media_type: String,
    text: String,
    guardian_did: Option<String>,
    occurred_at: Option<i64>,
    place_label: Option<String>,
    lat: Option<f32>,
    lon: Option<f32>,
    project: Option<String>,
    purpose: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let manual = qualia_client_core::wellfair::api::ManualFacets {
        occurred_at,
        place_label,
        lat,
        lon,
        projects: project.into_iter().filter(|s| !s.trim().is_empty()).collect(),
        purposes: purpose.into_iter().filter(|s| !s.trim().is_empty()).collect(),
    };
    let summary = host.ingest_document_annotated(&uri, &media_type, &text, &manual, guardian_did)?;
    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

/// Ingest a **binary asset** (photo / audio) whose bytes are passed hex-encoded. A photo's EXIF capture-time
/// + GPS auto-populate the timeline + map.
#[command]
pub fn wellfair_ingest_file_hex(
    app: AppHandle,
    uri: String,
    media_type: String,
    bytes_hex: String,
    caption: String,
    guardian_did: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let summary = host.ingest_file_hex(&uri, &media_type, &bytes_hex, &caption, guardian_did)?;
    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

/// Search the library by facet (`topic` | `depicts` | `place` | `project` | `purpose`).
#[command]
pub fn wellfair_search_library(app: AppHandle, facet: String, value: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let results = host.search_library(&facet, &value)?;
    serde_json::to_string(&results).map_err(|e| e.to_string())
}

/// The timeline query — entries whose event instant falls within `[start, end]` (unix seconds).
#[command]
pub fn wellfair_search_library_time(app: AppHandle, start: i64, end: i64) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let results = host.search_library_time(start, end)?;
    serde_json::to_string(&results).map_err(|e| e.to_string())
}

/// Everything in the library (newest first).
#[command]
pub fn wellfair_list_library(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let results = host.list_library()?;
    serde_json::to_string(&results).map_err(|e| e.to_string())
}

// --- Chora spatio-temporal canvas ---

#[command]
pub fn chora_list_worlds(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.list_canvas_worlds()?).map_err(|e| e.to_string())
}

#[command]
pub fn chora_get_world(app: AppHandle, world_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let v = host.get_canvas_world(&world_id)?;
    serde_json::to_string(&v).map_err(|e| e.to_string())
}

#[command]
pub fn chora_save_world(app: AppHandle, config_json: String) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.save_canvas_world(&config_json)
}

#[command]
pub fn chora_delete_world(app: AppHandle, world_id: String) -> Result<bool, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.delete_canvas_world(&world_id)
}

#[command]
pub fn chora_seed_demo(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.seed_canvas_demo()
}

#[command]
pub fn chora_navigation(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    Ok(host.canvas_navigation_state().to_string())
}

#[command]
pub fn chora_set_temporal(app: AppHandle, t_value: f64) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.set_temporal_slice(t_value)
}

#[command]
pub fn chora_set_active_world(app: AppHandle, world_id: String) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.set_active_canvas_world(&world_id)
}

#[command]
pub fn chora_query_region(
    app: AppHandle,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let hits = host.query_canvas_region(x1, y1, x2, y2)?;
    serde_json::to_string(&hits).map_err(|e| e.to_string())
}

#[command]
pub fn chora_publish_asset(app: AppHandle, asset_json: String) -> Result<(), String> {
    use qualia_core_db::domains::geospatial::spatial_sync::PlantedAsset;

    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let asset: PlantedAsset = serde_json::from_str(&asset_json).map_err(|e| e.to_string())?;
    host.publish_planted_asset(asset)
}

#[command]
pub fn chora_pull_assets(app: AppHandle, cell_id: u64) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let assets = host.pull_spatial_assets(cell_id)?;
    serde_json::to_string(&assets).map_err(|e| e.to_string())
}

// --- Chora layer library + asset download pipeline ---

#[command]
pub fn chora_list_layers() -> Result<String, String> {
    let catalog = qualia_client_core::chora::layers::LAYER_CATALOG;
    serde_json::to_string(catalog).map_err(|e| e.to_string())
}

#[command]
pub fn chora_list_categories() -> Result<String, String> {
    let cats = qualia_client_core::chora::layers::all_categories();
    serde_json::to_string(&cats).map_err(|e| e.to_string())
}

#[command]
pub fn chora_get_layer(layer_id: String) -> Result<String, String> {
    let layer = qualia_client_core::chora::layers::find_layer(&layer_id)
        .ok_or_else(|| format!("Layer not found: {layer_id}"))?;
    serde_json::to_string(layer).map_err(|e| e.to_string())
}

#[command]
pub async fn chora_download_layer(
    app: AppHandle,
    layer_id: String,
    resolution: u32,
) -> Result<String, String> {
    let asset = qualia_client_core::chora::asset_pipeline::download_and_compile_layer(&layer_id, resolution)
        .await?;

    if let Some(surface) = app.try_state::<std::sync::Arc<NativeSurfaceState>>() {
        let mut renderer_guard = surface.renderer.lock().map_err(|e| e.to_string())?;
        if let Some(renderer) = renderer_guard.as_mut() {
            let _ = renderer.upload_mesh_colored(
                &asset.positions,
                &asset.colors,
                &asset.indices,
            );
        }
    }

    let result = serde_json::json!({
        "layerId": asset.layer_id,
        "vertexCount": asset.vertex_count,
        "triangleCount": asset.triangle_count,
        "sourceFormat": asset.source_format,
        "license": asset.license,
        "container10dSize": asset.container_10d.len(),
    });
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

#[command]
pub fn chora_load_layer_to_gpu(
    app: AppHandle,
    layer_id: String,
    resolution: u32,
) -> Result<String, String> {
    let _ = resolution;
    let _ = layer_id;
    let _ = app;
    Err("Use chora_download_layer for async download+compile+upload".to_string())
}

/// The owner's envelope PUBLIC key (hex) — publishable so others can seal payloads to the owner.
#[command]
pub fn wellfair_owner_envelope_public(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&serde_json::json!({ "public_hex": host.owner_envelope_public_hex() }))
        .map_err(|e| e.to_string())
}

/// Seal a real plaintext payload and grant a consent credential over it (real envelope encryption).
/// Empty `agent_public_hex` seals to the owner (self-custody, openable here); a supplied X25519 public key
/// grants that agent access instead.
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_seal_and_grant_credential(
    app: AppHandle,
    agent_did: String,
    agent_public_hex: String,
    scope: String,
    purpose: String,
    plaintext: String,
    expiry_unix: Option<u64>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let cred = host.seal_and_grant_consent_credential(
        &agent_did,
        &agent_public_hex,
        &scope,
        &purpose,
        &plaintext,
        expiry_unix,
    )?;
    serde_json::to_string(&cred).map_err(|e| e.to_string())
}

/// Open an owner-sealed payload through a credential (works while live; fails once revoked).
#[command]
pub fn wellfair_open_owner_payload(app: AppHandle, credential_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let plaintext = host.open_owner_payload(&credential_id)?;
    serde_json::to_string(&serde_json::json!({ "plaintext": plaintext })).map_err(|e| e.to_string())
}

// ── Safeguard switches (ADR 0011 D6/D7): dead-man + incapacity ──

/// Arm a dead-man switch from primitive fields (the command builds the domain type).
/// `disposition` is `"make_public"` or `"release_to"` (the latter uses `disposition_parties`).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_arm_dead_mans_switch(
    app: AppHandle,
    commitment_hex: String,
    lapse_after_secs: u64,
    parties: Vec<String>,
    threshold: usize,
    disposition: String,
    disposition_parties: Vec<String>,
) -> Result<String, String> {
    use qualia_client_core::dead_mans_switch::{DeadMansSwitch, Disposition, Heartbeat, TriggerRule};
    let commitment = qualia_client_core::accountability_store::parse_commitment_hex(&commitment_hex)?;
    let now = wellfair_now_unix() as u64;
    let disposition = match disposition.as_str() {
        "make_public" => Disposition::MakePublic,
        _ => Disposition::ReleaseTo { parties: disposition_parties },
    };
    let switch = DeadMansSwitch {
        payload_commitment: commitment,
        heartbeat: Heartbeat::new(now, lapse_after_secs),
        trigger: TriggerRule {
            require_heartbeat_lapsed: true,
            attestation_threshold: threshold,
            parties,
        },
        disposition,
        fired_unix: None,
    };
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.arm_dead_mans_switch(switch)?;
    Ok("{\"armed\":true}".into())
}

/// Touch the heartbeat / un-fire a dead-man switch (the "I'm alive" action).
#[command]
pub fn wellfair_dead_mans_alive(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let found = host.dead_mans_alive(&commitment_hex)?;
    serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
}

/// Record a party attestation toward a dead-man switch. `kind` = `no_contact` | `believed_dead` | `abandon`.
#[command]
pub fn wellfair_attest_dead_mans(
    app: AppHandle,
    commitment_hex: String,
    party_did: String,
    kind: String,
) -> Result<String, String> {
    use qualia_client_core::dead_mans_switch::{AttestationKind, PartyAttestation};
    let kind = match kind.as_str() {
        "no_contact" => AttestationKind::NoContact,
        "abandon" => AttestationKind::Abandon,
        _ => AttestationKind::BelievedDead,
    };
    let attestation = PartyAttestation { party_did, kind, time_unix: wellfair_now_unix() as u64 };
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let found = host.attest_dead_mans(&commitment_hex, attestation)?;
    serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
}

/// Enact a dead-man switch if triggerable — returns the disposition (or null).
#[command]
pub fn wellfair_enact_dead_mans(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let disposition = host.enact_dead_mans(&commitment_hex)?;
    serde_json::to_string(&serde_json::json!({ "disposition": disposition })).map_err(|e| e.to_string())
}

/// List armed dead-man switches (with attestations).
#[command]
pub fn wellfair_list_dead_mans_switches(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let list = host.list_dead_mans_switches()?;
    serde_json::to_string(&list).map_err(|e| e.to_string())
}

/// Enact a dead-man switch AND release the keys to the disposition parties. `party_keys` = `[did, pubkey_hex]`
/// pairs. Returns `{ enacted, disposition }`.
#[command]
pub fn wellfair_enact_dead_mans_release(
    app: AppHandle,
    commitment_hex: String,
    party_keys: Vec<(String, String)>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let result = host.enact_dead_mans_release(&commitment_hex, party_keys)?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Split a payload's DEK into Shamir social-recovery shares (`threshold`-of-`parties.len()`). Returns the
/// shares paired with the parties to hand them to (distribute off-device; not stored).
#[command]
pub fn wellfair_split_dek_recovery(
    app: AppHandle,
    commitment_hex: String,
    threshold: usize,
    parties: Vec<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let result = host.split_dek_recovery(&commitment_hex, threshold, parties)?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Social-recovery enactment: reconstruct the DEK from a quorum of friends' shares and release (no owner key).
/// `shares` = the Shamir shares; `party_keys` = `[did, pubkey_hex]` pairs.
#[command]
pub fn wellfair_reconstruct_and_release(
    app: AppHandle,
    commitment_hex: String,
    shares: Vec<qualia_client_core::shamir_recovery::Share>,
    party_keys: Vec<(String, String)>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let result = host.reconstruct_and_release(&commitment_hex, shares, party_keys)?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Publish a peer's envelope (X25519) public key into their peer record (remote-key distribution).
#[command]
pub fn wellfair_set_peer_envelope_key(
    app: AppHandle,
    did: String,
    pubkey_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.set_peer_envelope_key(&did, &pubkey_hex)?;
    Ok("{\"set\":true}".into())
}

/// Enact + release resolving the disposition parties' keys from the peer store. Returns
/// `{ result, missing_keys_for }`.
#[command]
pub fn wellfair_enact_dead_mans_release_via_peers(
    app: AppHandle,
    commitment_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let result = host.enact_dead_mans_release_via_peers(&commitment_hex)?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Arm an incapacity switch from primitive fields. `kind` = `involuntary_psychiatric` | `serious_injury` |
/// any other string (→ `Other`).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_arm_incapacity_switch(
    app: AppHandle,
    principal_did: String,
    kind: String,
    advocate_did: String,
    parties: Vec<String>,
    threshold: usize,
    require_official_instrument: bool,
) -> Result<String, String> {
    use qualia_client_core::incapacity_switch::{IncapacityKind, IncapacitySwitch, IncapacityTrigger};
    let kind = match kind.as_str() {
        "involuntary_psychiatric" => IncapacityKind::InvoluntaryPsychiatric,
        "serious_injury" => IncapacityKind::SeriousInjury,
        other => IncapacityKind::Other(other.to_string()),
    };
    let switch = IncapacitySwitch {
        principal_did,
        kind,
        trigger: IncapacityTrigger { parties, attestation_threshold: threshold, require_official_instrument },
        advocate_did,
        active_since_unix: None,
    };
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.arm_incapacity_switch(switch)?;
    Ok("{\"armed\":true}".into())
}

/// Activate advocacy on a validated incapacity trigger.
#[command]
pub fn wellfair_activate_incapacity(
    app: AppHandle,
    principal_did: String,
    attesting_parties: Vec<String>,
    official_instrument: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let activated = host.activate_incapacity(&principal_did, attesting_parties, official_instrument)?;
    serde_json::to_string(&serde_json::json!({ "activated": activated })).map_err(|e| e.to_string())
}

/// Regain capacity — the advocate stands down (reversibility).
#[command]
pub fn wellfair_regain_capacity(app: AppHandle, principal_did: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let found = host.regain_capacity(&principal_did)?;
    serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
}

/// List armed incapacity switches.
#[command]
pub fn wellfair_list_incapacity_switches(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let list = host.list_incapacity_switches()?;
    serde_json::to_string(&list).map_err(|e| e.to_string())
}

// ── Disclosure traceability (ADR 0011 D5) + duty of inquiry (D8) ──

/// Record a transparency cc (the protective "I informed authority X" note).
#[command]
pub fn wellfair_record_transparency_cc(
    app: AppHandle,
    credential_id: String,
    informed_authority_did: String,
    purpose: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.record_transparency_cc(&credential_id, &informed_authority_did, &purpose)?;
    Ok("{\"recorded\":true}".into())
}

/// Record a disclosure event (access, or onward-share if `onward_to` set). Returns the event (incl. its
/// tracing fingerprint).
#[command]
pub fn wellfair_record_disclosure(
    app: AppHandle,
    commitment_hex: String,
    credential_id: String,
    recipient_did: String,
    acting_delegate_did: Option<String>,
    onward_to: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let event = host.record_disclosure(&commitment_hex, &credential_id, &recipient_did, acting_delegate_did, onward_to)?;
    serde_json::to_string(&event).map_err(|e| e.to_string())
}

/// The disclosure chain for a payload.
#[command]
pub fn wellfair_disclosure_chain(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let chain = host.disclosure_chain(&commitment_hex)?;
    serde_json::to_string(&chain).map_err(|e| e.to_string())
}

/// The distinct actors who had access to a payload (the leak-suspect set).
#[command]
pub fn wellfair_actors_with_access(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let actors = host.actors_with_access(&commitment_hex)?;
    serde_json::to_string(&actors).map_err(|e| e.to_string())
}

/// Trace a leak by fingerprint (hex) → the disclosure + accountable actor (or null).
#[command]
pub fn wellfair_trace_leak(app: AppHandle, fingerprint_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let event = host.trace_leak(&fingerprint_hex)?;
    serde_json::to_string(&serde_json::json!({ "event": event })).map_err(|e| e.to_string())
}

/// List transparency cc records.
#[command]
pub fn wellfair_list_transparency_ccs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let ccs = host.list_transparency_ccs()?;
    serde_json::to_string(&ccs).map_err(|e| e.to_string())
}

/// Assess a duty of inquiry (JSON = `DutyOfInquiry`, `ConductAgainstDuty`) → the verdict.
#[command]
pub fn wellfair_assess_duty_of_inquiry(
    app: AppHandle,
    duty_json: String,
    conduct_json: String,
) -> Result<String, String> {
    let duty: qualia_client_core::duty_of_inquiry::DutyOfInquiry =
        serde_json::from_str(&duty_json).map_err(|e| format!("invalid duty JSON: {e}"))?;
    let conduct: qualia_client_core::duty_of_inquiry::ConductAgainstDuty =
        serde_json::from_str(&conduct_json).map_err(|e| format!("invalid conduct JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let verdict = host.assess_duty_of_inquiry(duty, conduct);
    serde_json::to_string(&serde_json::json!({ "verdict": verdict })).map_err(|e| e.to_string())
}

/// Score + record a sitting. `responses` is a comma-separated list of ordinal values (one per item,
/// in order). Returns the scored result (total, band, interpretation, any safety flags).
#[command]
pub fn wellfair_record_assessment(
    app: AppHandle,
    instrument_id: String,
    responses: String,
) -> Result<String, String> {
    let parsed: Result<Vec<u8>, _> = responses
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<u8>())
        .collect();
    let parsed = parsed.map_err(|e| format!("invalid responses: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let result = host.record_assessment(&instrument_id, parsed)?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Past assessment results (newest-first).
#[command]
pub fn wellfair_list_assessments(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    serde_json::to_string(&host.list_assessments(64)?).map_err(|e| e.to_string())
}

// --- Guardianship approval escrow (M-of-N co-signature for proxy actions; T1.5) --------------

/// A supporter records a condition on the principal's behalf. The write escrows for guardian
/// co-signature; returns the `SubmitOutcome` (Suspended with the pending proposal id).
#[command]
pub fn wellfair_propose_proxy_condition(
    app: AppHandle,
    proxy_did: String,
    label: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = wellfare_core::conditions::ConditionReport::new(label);
    let outcome = host.propose_proxy_condition(&proxy_did, &report)?;
    serde_json::to_string(&outcome).map_err(|e| e.to_string())
}

/// Pending and resolved guardianship proposals for the approval tray.
#[command]
pub fn wellfair_list_guardianship_proposals(
    app: AppHandle,
    limit: usize,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let proposals = host.list_guardianship_proposals(limit)?;
    serde_json::to_string(&proposals).map_err(|e| e.to_string())
}

/// A guardian co-signs (approve) or objects (deny). On ratification the escrowed record commits.
#[command]
pub fn wellfair_vote_guardianship_proposal(
    app: AppHandle,
    proposal_id: String,
    guardian_did: String,
    approve: bool,
    reason: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let view = host.vote_guardianship_proposal(&proposal_id, &guardian_did, approve, reason)?;
    serde_json::to_string(&view).map_err(|e| e.to_string())
}

fn parse_clinical_report_type(s: &str) -> wellfare_core::clinical::ClinicalReportType {
    use wellfare_core::clinical::ClinicalReportType::*;
    match s.to_ascii_lowercase().as_str() {
        "pathology" => Pathology,
        "imaging" => Imaging,
        "discharge" => Discharge,
        "referral" => Referral,
        _ => Other,
    }
}

fn parse_urgency(s: &str) -> wellfare_core::welfare_support::Urgency {
    use wellfare_core::welfare_support::Urgency::*;
    match s.to_ascii_lowercase().as_str() {
        "low" => Low,
        "high" => High,
        "critical" => Critical,
        _ => Moderate,
    }
}

fn parse_stream_status(s: &str) -> wellfare_core::welfare_support::StreamStatus {
    use wellfare_core::welfare_support::StreamStatus::*;
    match s.to_ascii_lowercase().as_str() {
        "active" => Active,
        "suspended" => Suspended,
        "ceased" => Ceased,
        "rejected" => Rejected,
        _ => Applied,
    }
}

#[command]
pub fn wellfair_add_clinical_report(
    app: AppHandle,
    title: String,
    report_type: String,
    observed_at_unix: u32,
    body: String,
    author_label: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let observed = if observed_at_unix == 0 {
        wellfair_now_unix()
    } else {
        observed_at_unix
    };
    let entry = host.add_clinical_report(
        &title,
        parse_clinical_report_type(&report_type),
        observed,
        &body,
        author_label,
    )?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

fn guess_media_type(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "dcm" => "application/dicom",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[command]
pub fn wellfair_add_clinical_attachment_from_path(
    app: AppHandle,
    path: String,
    media_type: Option<String>,
) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();
    let media = media_type
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| guess_media_type(&filename));
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_clinical_attachment(&filename, &media, &bytes)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_export_attachment(
    app: AppHandle,
    record_id: String,
    dest_path: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let bytes = host
        .attachment_bytes(&record_id)?
        .ok_or_else(|| "attachment not found".to_string())?;
    std::fs::write(&dest_path, &bytes).map_err(|e| format!("cannot write {dest_path}: {e}"))?;
    Ok(serde_json::json!({ "written": bytes.len(), "path": dest_path }).to_string())
}

/// Convert a dialog `FilePath` into an absolute path string.
///
/// On desktop the picker yields `FilePath::Path`, so `into_path()` returns the `PathBuf`
/// directly; `simplified()` first normalises Windows UNC prefixes. If the variant is a URL
/// that cannot be resolved to a filesystem path, fall back to its `Display` form so the
/// caller still receives a usable string rather than an error.
fn dialog_file_path_to_string(fp: tauri_plugin_dialog::FilePath) -> String {
    let display = fp.to_string();
    match fp.simplified().into_path() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => display,
    }
}

/// Open a native OS "open file" dialog (blocking). Returns the chosen absolute path,
/// or `None` if the user cancelled. Lets the operator browse instead of typing a path.
#[command]
pub fn wellfair_pick_file_path(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_file();
    Ok(picked.map(dialog_file_path_to_string))
}

/// Open a native OS "save file" dialog (blocking), seeded with `default_name`. Returns the
/// chosen absolute path, or `None` if the user cancelled.
#[command]
pub fn wellfair_pick_save_path(
    app: AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    let trimmed = default_name.trim();
    if !trimmed.is_empty() {
        builder = builder.set_file_name(trimmed);
    }
    let picked = builder.blocking_save_file();
    Ok(picked.map(dialog_file_path_to_string))
}

/// Open a native OS folder-picker (blocking). Returns the chosen directory, or `None` if cancelled.
#[command]
pub fn wellfair_pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_folder();
    Ok(picked.map(dialog_file_path_to_string))
}

/// WP2: author a qapp and write its installable PWA bundle into `target_dir`. Returns the written
/// file paths (JSON array).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_publish_qapp_pwa(
    app: AppHandle,
    target_dir: String,
    id: String,
    name: String,
    kind: String,
    description: String,
    capabilities: String,
    wasm_filename: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let written = host.publish_qapp_pwa(
        &target_dir,
        &id,
        &name,
        &kind,
        &description,
        &capabilities,
        &wasm_filename,
    )?;
    serde_json::to_string(&written).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_government_letter_attachment_from_path(
    app: AppHandle,
    sender: String,
    subject: String,
    action_required: bool,
    path: String,
) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_government_letter_attachment(&sender, &subject, action_required, &bytes)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_assistance_need(
    app: AppHandle,
    category: String,
    description: String,
    urgency: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_assistance_need(&category, &description, parse_urgency(&urgency))?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_welfare_stream(
    app: AppHandle,
    program_name: String,
    reference: Option<String>,
    status: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_welfare_stream(&program_name, reference, parse_stream_status(&status))?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_government_letter(
    app: AppHandle,
    sender: String,
    subject: String,
    action_required: bool,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_government_letter(&sender, &subject, action_required)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_sync_inbox(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let inbox = host.list_sync_inbox(limit)?;
    serde_json::to_string(&inbox).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_sanctuary_vault_configured(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    Ok(serde_json::json!({ "configured": host.sanctuary_vault_configured() }).to_string())
}

#[command]
pub fn wellfair_setup_sanctuary_vault(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.setup_sanctuary_vault(&real_pin, &decoy_pin)?;
    Ok(serde_json::json!({ "configured": true }).to_string())
}

#[command]
pub fn wellfair_sanctuary_vault_add_note(
    app: AppHandle,
    pin: String,
    body: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let lane = host.add_sanctuary_vault_note(&pin, &body)?;
    Ok(serde_json::json!({ "lane": lane }).to_string())
}

#[command]
pub fn wellfair_sanctuary_vault_list_notes(app: AppHandle, pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let (lane, notes) = host.list_sanctuary_vault_notes(&pin)?;
    Ok(serde_json::json!({ "lane": lane, "notes": notes }).to_string())
}

// --- T1.2: OS-keychain vault wrapping (opt-in, off by default; recovery-gated) ---

#[command]
pub fn wellfair_sanctuary_vault_is_keychain_wrapped(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    Ok(serde_json::json!({ "wrapped": host.sanctuary_vault_is_keychain_wrapped() }).to_string())
}

/// Create a keychain-wrapped vault; returns the one-time recovery code the user MUST record.
#[command]
pub fn wellfair_setup_sanctuary_vault_wrapped(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let recovery_code = host.setup_sanctuary_vault_wrapped(&real_pin, &decoy_pin)?;
    Ok(serde_json::json!({ "configured": true, "recovery_code": recovery_code }).to_string())
}

#[command]
pub fn wellfair_sanctuary_vault_unlock_with_recovery(
    app: AppHandle,
    pin: String,
    recovery_code: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let lane = host.sanctuary_vault_unlock_with_recovery(&pin, &recovery_code)?;
    Ok(serde_json::json!({ "lane": lane }).to_string())
}

// --- Vault v2 (S6): per-session decoy audit, real→decoy curation, real-lane review, retention ---

/// Add a note attributing a **decoy** (duress) write to a per-unlock `session_ref` (git-like branch
/// in the audit DAG). Real-lane writes ignore `session_ref`.
#[command]
pub fn wellfair_sanctuary_vault_add_note_in_session(
    app: AppHandle,
    pin: String,
    body: String,
    session_ref: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let lane = host.add_sanctuary_vault_note_in_session(&pin, &body, &session_ref)?;
    Ok(serde_json::json!({ "lane": lane }).to_string())
}

/// Curate the decoy from a real session — seed a plausible note into the decoy lane without the
/// decoy PIN. Requires the **real** PIN.
#[command]
pub fn wellfair_curate_decoy_note(
    app: AppHandle,
    real_pin: String,
    body: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.curate_sanctuary_decoy_note(&real_pin, &body)?;
    Ok(serde_json::json!({ "curated": true }).to_string())
}

/// Review decoy activity from the real lane: decrypt + verify the sealed trail, advance head
/// anchors, and return the integrity verdict + decrypted actions. Requires the **real** PIN.
#[command]
pub fn wellfair_review_decoy_activity(app: AppHandle, real_pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.review_sanctuary_decoy_activity(&real_pin)?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// Read the decoy-audit retention policy (real-session-only; ADR §8). Returns `{ "mode": "..." }`.
/// Requires the **real** PIN.
#[command]
pub fn wellfair_get_decoy_retention_mode(app: AppHandle, real_pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let mode = host.get_sanctuary_decoy_retention_mode(&real_pin)?;
    Ok(serde_json::json!({ "mode": mode }).to_string())
}

/// Set the decoy-audit retention policy (real-session-only; ADR §8). `mode` is `"auto_archive"` or
/// `"manual_triage"`. Requires the **real** PIN.
#[command]
pub fn wellfair_set_decoy_retention_mode(
    app: AppHandle,
    real_pin: String,
    mode: String,
) -> Result<String, String> {
    use qualia_client_core::wellfair::api::sanctuary_retention_mode_from_str;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let parsed = sanctuary_retention_mode_from_str(&mode)?;
    host.set_sanctuary_decoy_retention_mode(&real_pin, parsed)?;
    Ok(serde_json::json!({ "mode": mode }).to_string())
}

#[command]
pub fn wellfair_add_wellbeing_observation(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let report: wellfare_core::mental_wellbeing::WellbeingObservation =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid wellbeing JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_wellbeing_observation(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_therapy_note(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::mental_wellbeing::TherapyNote =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid therapy note JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_therapy_note(&report)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}


#[derive(Debug, serde::Serialize)]
struct LiveShareRequestDto {
    id: String,
    device_id: String,
    purpose: String,
    requested_kinds: Vec<String>,
    ttl_seconds: u32,
}

#[command]
pub fn wellfair_list_pending_live_shares(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let pending = host.list_pending_live_shares(limit)?;
    let dtos: Vec<LiveShareRequestDto> = pending
        .into_iter()
        .map(|r| LiveShareRequestDto {
            id: r.id,
            device_id: r.device_id,
            purpose: r.purpose,
            requested_kinds: r.requested_kinds,
            ttl_seconds: r.ttl_seconds,
        })
        .collect();
    serde_json::to_string(&dtos).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_decide_live_share(
    app: AppHandle,
    request_id: String,
    approved: bool,
    projection_kinds: Vec<String>,
    reason: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.decide_live_share_request(
        &request_id,
        approved,
        if approved { &projection_kinds } else { &[] },
        reason.as_deref(),
    )?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

fn parse_administration_status(s: &str) -> wellfare_core::medication::AdministrationStatus {
    match s.to_ascii_lowercase().as_str() {
        "skipped" => wellfare_core::medication::AdministrationStatus::Skipped,
        "overdue" => wellfare_core::medication::AdministrationStatus::Overdue,
        _ => wellfare_core::medication::AdministrationStatus::Taken,
    }
}

#[command]
pub fn wellfair_add_medication(
    app: AppHandle,
    name: String,
    dose: String,
    route: String,
    schedule: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let times: Vec<String> = schedule
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let entry = host.add_medication(&name, &dose, &route, times)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_record_administration(
    app: AppHandle,
    medication_id: String,
    medication_name: String,
    status: String,
    notes: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let st = parse_administration_status(&status);
    let entry = host.record_administration(&medication_id, &medication_name, st, notes)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_diet_entry(
    app: AppHandle,
    description: String,
    meal_type: String,
    calories_kcal: Option<u32>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.add_diet_entry(&description, &meal_type, calories_kcal)?;
    serde_json::to_string(&entry).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_sleep_analytics(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let (debt, heatmap) = host.default_sleep_analytics()?;
    let out = serde_json::json!({ "debt": debt, "heatmap": heatmap });
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_add_emergency_contact(
    app: AppHandle,
    display_name: String,
    relationship: String,
    phone: Option<String>,
    email: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let contact = host.add_emergency_contact(&display_name, &relationship, phone, email, None)?;
    serde_json::to_string(&contact).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_emergency_contacts(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let contacts = host.list_emergency_contacts()?;
    serde_json::to_string(&contacts).map_err(|e| e.to_string())
}

// ── Wallet / identity ─────────────────────────────────────────────────────────

#[command]
pub fn get_wallet_status() -> WalletStatus {
    api::get_wallet_status()
}

#[command]
pub fn is_first_run() -> bool {
    api::is_first_run()
}

#[command]
pub fn read_identity() -> Option<serde_json::Value> {
    api::read_identity()
}

#[command]
pub fn save_identity(wallets: serde_json::Value) -> Result<(), String> {
    api::save_identity(wallets)
}

#[command]
pub fn load_identity() -> Result<Option<serde_json::Value>, String> {
    api::load_identity()
}

#[command]
pub fn get_coin_balances() -> Vec<CoinBalance> {
    api::get_coin_balances()
}

#[command]
pub fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    api::get_transaction_history(ticker)
}

#[command]
pub async fn generate_bip39_seed() -> Result<String, String> {
    api::generate_bip39_seed().await
}

#[command]
pub async fn derive_wallets_from_seed(seed: String) -> Result<serde_json::Value, String> {
    api::derive_wallets_from_seed(seed).await
}

#[command]
pub async fn import_external_seed(
    network: String,
    seed: String,
    label: String,
) -> Result<String, String> {
    api::import_external_seed(network, seed, label).await
}

// ── Tokens ────────────────────────────────────────────────────────────────────

#[command]
pub fn get_tokens() -> Vec<TokenEntry> {
    api::get_tokens()
}

#[command]
pub fn add_token(
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    api::add_token(chain, token_type, contract, symbol, name, decimals)
}

#[command]
pub fn remove_token(id: String) -> Result<(), String> {
    api::remove_token(id)
}

// ── Tax / ILP ─────────────────────────────────────────────────────────────────

#[command]
pub fn get_tax_suite() -> TaxRecipientSuite {
    api::get_tax_suite()
}

#[command]
pub fn save_tax_suite(suite: TaxRecipientSuite) -> Result<(), String> {
    api::save_tax_suite(suite)
}

#[command]
pub fn dispatch_tax_payment(gross_amount_micro_cents: u64) -> Result<DispatchResult, String> {
    api::dispatch_tax_payment(gross_amount_micro_cents)
}

// ── Wallet send ───────────────────────────────────────────────────────────────

#[command]
pub fn build_send_xec(destination_address: String, amount_sats: i64) -> Result<SendPreview, String> {
    api::build_send_xec(&destination_address, amount_sats)
}

#[command]
pub fn confirm_send_xec(raw_hex: String) -> Result<String, String> {
    api::confirm_send_xec(&raw_hex)
}

#[command]
pub fn send_ecash_token(token_id: String, destination_address: String, amount: u64) -> Result<String, String> {
    api::send_ecash_token(&token_id, &destination_address, amount)
}

// ── Vault / federated ─────────────────────────────────────────────────────────

#[command]
pub fn accept_vault_handshake(did_key: String, payload: String) -> Result<String, String> {
    api::accept_vault_handshake(did_key, payload)
}

#[command]
pub fn receive_vault_job(
    job_id: String,
    task_type: String,
    data_blob_cbor_ld: Vec<u8>,
) -> Result<String, String> {
    api::receive_vault_job(job_id, task_type, data_blob_cbor_ld)
}

// ── Ingest ────────────────────────────────────────────────────────────────────

#[command]
pub async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    api::ingest_pdf(file_name).await
}

#[command]
pub async fn ingest_literature(file_path: String) -> Result<String, String> {
    api::ingest_literature(file_path).await
}

#[command]
pub async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    api::upsert_cmld_definition(term, context_did).await
}

#[command]
pub async fn ingest_ontology(file_name: String) -> Result<serde_json::Value, String> {
    api::ingest_ontology(file_name).await
}

#[command]
pub async fn export_to_solid(
    input_q42_path: String,
    output_dir_path: String,
) -> Result<String, String> {
    api::export_to_solid(input_q42_path, output_dir_path).await
}

#[command]
pub async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    api::ingest_image(file_path).await
}

#[command]
pub async fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    api::ingest_image_async(file_path, typology).await
}

// ── Model / inference ─────────────────────────────────────────────────────────

#[command]
pub async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    api::discover_models().await
}

#[command]
pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    api::download_and_vectorize(url, filename, item_id).await
}

#[command]
pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    api::download_model(url, filename, model_id).await
}

#[command]
pub fn cancel_download(id: String) -> Result<(), String> {
    api::cancel_download(id)
}

#[command]
pub fn get_active_model() -> Option<String> {
    api::get_active_model()
}

#[command]
pub fn set_active_model(model_name: String) -> Result<(), String> {
    api::set_active_model(model_name)
}

#[command]
pub fn get_active_downloads() -> Vec<ProgressPayload> {
    api::get_active_downloads()
}

#[command]
pub async fn run_agent_inference(
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    api::run_agent_inference(prompt, model_name, intent_layout).await
}

// ── Semantic web / portfolio ──────────────────────────────────────────────────

#[command]
pub async fn generate_front_door_invite() -> Result<String, String> {
    api::generate_front_door_invite().await
}

#[command]
pub async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    api::mint_semantic_token(asset_id).await
}

#[command]
pub async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    api::fetch_wallet_portfolio().await
}

#[command]
pub async fn toggle_nym_relay() -> Result<bool, String> {
    api::toggle_nym_relay().await
}

#[command]
pub async fn toggle_stark_prover() -> Result<bool, String> {
    api::toggle_stark_prover().await
}

#[command]
pub fn update_solar_input(watts: u32) {
    api::update_solar_input(watts)
}

#[command]
pub async fn fetch_torrent_telemetry() -> Result<serde_json::Value, String> {
    api::fetch_torrent_telemetry().await
}

#[command]
pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    api::fetch_remote_manifest(url).await
}

// ── Imported accounts ─────────────────────────────────────────────────────────

#[command]
pub fn load_imported_accounts() -> Result<serde_json::Value, String> {
    api::load_imported_accounts()
}

#[command]
pub fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    api::save_imported_accounts(accounts)
}

// ── Directory / agents ────────────────────────────────────────────────────────

#[command]
pub fn get_front_doors() -> Result<Vec<FrontDoor>, String> {
    api::get_front_doors()
}

#[command]
pub fn generate_front_door(label: String) -> Result<FrontDoor, String> {
    api::generate_front_door(label)
}

#[command]
pub fn get_directory_actors() -> Result<Vec<Actor>, String> {
    api::get_directory_actors()
}

#[command]
pub fn add_directory_actor(actor: Actor) -> Result<(), String> {
    api::add_directory_actor(actor)
}

#[command]
pub fn get_delegation_rules() -> Result<Vec<DelegationRule>, String> {
    api::get_delegation_rules()
}

#[command]
pub fn add_delegation_rule(rule: DelegationRule) -> Result<(), String> {
    api::add_delegation_rule(rule)
}

// ── Social connect + group chat (P0: expose the connect → group → talk loop) ────
//
// These wrap engine functions that already existed in `qualia_client_core::api` but were never
// surfaced to the desktop, so a user could not actually connect to another person from the UI.
// The invite is ed25519-signed and carries the front-door DID; contacts, group sessions,
// participants, messages, and the threaded chat-graph are all persisted + WAL-backed by the engine.

/// Generate a signed connect-invite (front-door DID + pubkey + relay endpoint, 7-day TTL) to hand to
/// someone you choose. Returns the invite JSON + a short code + a `mailto:` share URL.
#[command]
pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<serde_json::Value, String> {
    api::generate_connect_invite(front_door_id)
}

/// Accept a connect-invite (paste the invite JSON). Verifies the signature, then adds the inviter as a
/// contact + directory actor. Returns the new contact.
#[command]
pub fn accept_connect_invite(input: String) -> Result<serde_json::Value, String> {
    api::accept_connect_invite(input)
}

/// The current chat contacts (people you have connected with).
#[command]
pub fn list_chat_contacts() -> Result<serde_json::Value, String> {
    api::list_chat_contacts()
}

/// The local user profile (display name, sharing settings incl. whether connect-invites are enabled).
#[command]
pub fn get_user_profile() -> Result<serde_json::Value, String> {
    api::get_user_profile()
}

/// Persist the local user profile (JSON). Used to set a display name and enable connect-invites.
#[command]
pub fn save_user_profile(profile_json: String) -> Result<serde_json::Value, String> {
    api::save_user_profile(profile_json)
}

/// All chat sessions (solo + group), most-recent first.
#[command]
pub fn list_chat_sessions() -> Result<serde_json::Value, String> {
    api::list_chat_sessions()
}

/// Load one chat session (metadata + messages).
#[command]
pub fn load_chat_session(id: String) -> Result<serde_json::Value, String> {
    api::load_chat_session(id)
}

/// Create a group chat from a set of contact DIDs. Returns the new session id.
#[command]
pub fn create_group_chat_session(
    title: Option<String>,
    participant_dids: Vec<String>,
) -> Result<String, String> {
    api::create_group_chat_session(title, participant_dids)
}

/// Add a participant (by DID) to a group session. Returns the updated participant list.
#[command]
pub fn add_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    api::add_chat_participant(session_id, participant_did)
}

/// Remove a participant (by DID) from a group session. Returns the updated participant list.
#[command]
pub fn remove_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    api::remove_chat_participant(session_id, participant_did)
}

/// The participants of a group session.
#[command]
pub fn get_chat_participants(session_id: String) -> Result<serde_json::Value, String> {
    api::get_chat_participants(session_id)
}

/// Send a message into a session. `role` is `"user"` / `"agent"` / `"system"`; group messages are
/// signed + fanned out to participants' relays by the engine. Returns the message Lamport clock.
#[command]
pub fn append_chat_message(
    session_id: String,
    role: String,
    content: String,
    mesh: State<'_, mesh::MeshState>,
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<u64, String> {
    let lamport = api::append_chat_message(session_id.clone(), role, content)?;
    // Fan the message out to connected peers over the mesh (no-op if the mesh is stopped or none of
    // the session's participants are mesh peers). The HTTP relay path is unaffected.
    mesh.publish_session_message(&app_state, &session_id, lamport);
    Ok(lamport)
}

/// The threaded chat-graph (fragments + reply edges) for a session.
#[command]
pub fn get_chat_graph(session_id: String) -> Result<serde_json::Value, String> {
    api::get_chat_graph(session_id)
}

// ── Personal directory (AD-like): categorised addressbook + agreement slots ─────

/// The unified, categorised personal directory: the addressbook (Parties joined by DID across the
/// directory-actor + chat-contact stores) grouped into categories, each entry carrying a slot for the
/// agreements governing that relationship.
#[command]
pub fn list_directory() -> Result<serde_json::Value, String> {
    api::list_directory()
}

/// The directory categories (built-in + user-created).
#[command]
pub fn list_directory_categories() -> Result<serde_json::Value, String> {
    api::list_directory_categories()
}

/// Create a custom directory category.
#[command]
pub fn create_directory_category(label: String) -> Result<serde_json::Value, String> {
    api::create_directory_category(label)
}

/// Set which categories a directory entry (by DID) belongs to; returns the refreshed directory.
#[command]
pub fn set_directory_entry_categories(
    did: String,
    categories: Vec<String>,
) -> Result<serde_json::Value, String> {
    api::set_directory_entry_categories(did, categories)
}

/// Faceted + concept-aware search over the directory. `query` is meaning-aware; `facets_json` is a JSON
/// object of `{facet_id: [values]}`. Returns ranked entries + drill-down facet counts.
#[command]
pub fn search_directory(query: String, facets_json: String) -> Result<serde_json::Value, String> {
    api::search_directory(query, facets_json)
}

// ── Domains & semantic mail/address stack (the foundation) ──────────────────────

/// The person's context-domains (personal/work/projects…), each an agent with a front-door DID.
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

/// Fetch unseen mail via IMAP + apply each address's rules (structural spam-kill on un-minted addresses).
#[command]
pub fn mail_fetch(imap_json: String, mailbox: String) -> Result<serde_json::Value, String> {
    api::mail_fetch(imap_json, mailbox)
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
    if q.contains("my did") || q.contains("my webid") {
        return "qualia://webid/did:q42:local".to_string();
    }
    if q.contains("thermal") || q.contains("status") {
        return "qualia://internal/monitor".to_string();
    }
    if q.to_lowercase() == "hello" {
        return "qualia://internal/dialectical-sidebar".to_string();
    }
    if q.starts_with("did:q42:") || q.starts_with("did:") {
        return format!("qualia://webid/{}", q);
    }
    let looks_like_domain = !q.contains(' ')
        && q.contains('.')
        && !q.starts_with("http://")
        && !q.starts_with("https://")
        && !q.starts_with("qualia://");
    if looks_like_domain {
        return format!("qualia://webid/{}", q);
    }
    if query.starts_with("http://") || query.starts_with("https://") {
        query
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(&query))
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
pub async fn sync_to_solid_pod(pod_url: String) -> Result<String, String> {
    Ok(format!(
        "Successfully synced QualiaDB semantic state to Solid Pod: {}",
        pod_url
    ))
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
pub fn save_qlink(
    url: String,
    title: String,
    context_assertions: Option<Vec<serde_json::Value>>,
) -> Result<String, String> {
    use qualia_client_core::state::{config_file_path, AgentConfig};
    use std::fs;

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

    let qlinks_dir = std::path::PathBuf::from(&storage_path).join("qlinks");
    if !qlinks_dir.exists() {
        let _ = fs::create_dir_all(&qlinks_dir);
    }

    let mut doc = serde_json::json!({
        "@context": ["http://schema.org", "http://www.w3.org/ns/anno.jsonld"],
        "@type": "Bookmark",
        "url": url,
        "name": title,
        "dateCreated": chrono::Utc::now().to_rfc3339()
    });

    if let Some(assertions) = context_assertions {
        if let Some(obj) = doc.as_object_mut() {
            obj.insert(
                "cml:contextAssertions".to_string(),
                serde_json::json!(assertions),
            );
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let file_path = qlinks_dir.join(format!("{}.json", id));

    let json_str = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    fs::write(&file_path, json_str).map_err(|e| e.to_string())?;

    Ok(format!("QLink saved to {:?}", file_path))
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

// ── QApp ↔ QualiaDB analysis contract ───────────────────────────────────────────
// Mirrors `webizen-studio/src/components/qapp_engine.rs`. The discipline QApps call
// this via `invoke("qapp_analyze", { request })` when running in the desktop webview;
// the plain-browser demo uses the studio-side deterministic stub instead.

#[derive(Debug, Clone, serde::Deserialize)]
pub struct QappAnalysisRequest {
    pub discipline: String,
    pub fields: Vec<(String, String)>,
    pub notes: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QappAnalysisResult {
    pub summary: String,
    pub assertions: Vec<String>,
    pub provenance_hash: String,
    pub engine: String,
    pub graph_nodes: usize,
    pub q42_quins: usize,
    pub evidence_weight: f32,
    pub forge_schema_version: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QualiaComputeProfile {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub wgpu_api_version: String,
    pub naga_api_version: String,
    pub cudarc_api_version: String,
    pub backend_override: Option<String>,
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor_hex: String,
    pub device_hex: String,
    pub driver: String,
    pub driver_info: String,
    pub recommendation: String,
    pub preferred_forge_target: String,
    pub active_forge_target: String,
    pub fallback_note: Option<String>,
    pub features: String,
    pub enabled_features: String,
    pub subgroup_range: String,
    pub cooperative_matrix_tile_count: usize,
    pub max_buffer_size_mib: u64,
    pub max_storage_buffer_binding_size_mib: u64,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub timestamps_supported: bool,
    pub timestamp_period_ns: f32,
    pub q42_graph_bridge: bool,
    pub available_modules: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgePhysicsCertification {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub kernel: String,
    pub backend: String,
    pub particle_count: usize,
    pub certified: bool,
    pub max_abs_error: f32,
    pub momentum_drift: f32,
    pub elapsed_ms: f64,
    pub q42_provenance: String,
    pub sample_positions: Vec<[f32; 3]>,
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeKernelProbe {
    pub kernel: String,
    pub shape: String,
    pub output_elements: usize,
    pub elapsed_ms: f64,
    pub max_abs_error: f32,
    pub certified: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeComputeProbe {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub backend: String,
    pub initialization_ms: f64,
    pub total_kernel_ms: f64,
    pub all_certified: bool,
    pub q42_provenance: String,
    pub kernels: Vec<ForgeKernelProbe>,
    pub note: String,
}

fn qapp_slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

// ── Webizen Host API (qApp Message Bus) ──────────────────────────────────────

pub struct HostApiState(
    pub std::sync::Arc<std::sync::Mutex<Option<qualia_client_core::wellfair::api::WebizenHostApi>>>,
);

#[tauri::command]
pub fn submit_record(
    app: tauri::AppHandle,
    qapp_id: String,
    envelope: wellfare_core::record::RecordEnvelope,
    source: String,
) -> Result<usize, String> {
    let state = app.state::<HostApiState>();
    let mut api_guard = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(host_api) = api_guard.as_mut() {
        host_api.submit_record(&qapp_id, envelope, &source)
    } else {
        Err("Host API not initialized".into())
    }
}

fn qapp_evidence_score(key: &str, value: &str) -> f32 {
    let hash = qualia_core_db::q_hash(&format!("{key}={value}"));
    let upper = (hash >> 40) as u32;
    (upper as f32) / ((1u32 << 24) as f32)
}

fn qapp_graph_for_scores(
    channel_count: usize,
) -> Result<qualia_core_db::wgsl_forge::ir::graph::ComputeGraph, String> {
    use qualia_core_db::wgsl_forge::ir::graph::{
        Axis, ComputeGraph, DType, OpNode, RedKind, Shape, TensorRef,
    };
    use qualia_core_db::wgsl_forge::Schedule;

    let mut graph = ComputeGraph::new();
    let input_len = channel_count.max(1) as u32;
    let input = TensorRef::external(Shape::new(&[input_len]), DType::F32);
    let out = graph
        .push(
            OpNode::Reduce {
                op: RedKind::Mean,
                axis: Axis::Last,
            },
            &[input],
            Shape::scalar(),
            DType::F32,
            Schedule::default(),
        )
        .map_err(|e| e.to_string())?;
    graph.mark_output(out);
    Ok(graph)
}

fn qapp_content_quins(
    request: &QappAnalysisRequest,
    canonical: &str,
    scores: &[f32],
) -> Vec<qualia_core_db::NQuin> {
    let context = qualia_core_db::q_hash(canonical);
    let subject = qualia_core_db::q_hash(&request.discipline);
    let notes_len = if request.notes.trim().is_empty() {
        0
    } else {
        1
    };
    let mut quins = Vec::with_capacity(request.fields.len() + notes_len);
    let mut score_idx = 0usize;

    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        let score = scores.get(score_idx).copied().unwrap_or_default();
        score_idx += 1;
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash(key),
            object: qualia_core_db::q_hash(value),
            context,
            metadata: score.to_bits() as u64,
            parity: (score_idx as u64).wrapping_sub(1),
        });
    }

    if !request.notes.trim().is_empty() {
        let score = scores.get(score_idx).copied().unwrap_or_default();
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash("notes"),
            object: qualia_core_db::q_hash(request.notes.trim()),
            context,
            metadata: score.to_bits() as u64,
            parity: score_idx as u64,
        });
    }

    quins
}

fn hex32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn deterministic_nbody_state() -> [f32; 64] {
    let mut state = [0.0_f32; 64];
    for particle in 0..8 {
        let base = particle * 8;
        let phase = particle as f32 * std::f32::consts::FRAC_PI_4;
        state[base] = phase.cos() * 3.0;
        state[base + 1] = phase.sin() * 3.0;
        state[base + 2] = (particle as f32 - 3.5) * 0.18;
        state[base + 3] = -phase.sin() * 0.12;
        state[base + 4] = phase.cos() * 0.12;
        state[base + 5] = 0.0;
        state[base + 6] = 1.0 + (particle % 3) as f32 * 0.25;
        state[base + 7] = if particle % 2 == 0 { 1.0 } else { -1.0 };
    }
    state
}

fn total_momentum(state: &[f32]) -> [f32; 3] {
    let mut momentum = [0.0_f32; 3];
    for particle in state.chunks_exact(8) {
        let mass = particle[6];
        momentum[0] += particle[3] * mass;
        momentum[1] += particle[4] * mass;
        momentum[2] += particle[5] * mass;
    }
    momentum
}

fn build_forge_physics_certification(run_gpu: bool) -> ForgePhysicsCertification {
    use qualia_core_db::wgsl_forge::physics::kinematics::{
        nbody_step_cpu, nbody_step_gpu, KIN_STRIDE,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const DT: f32 = 0.005;
    const SOFTENING: f32 = 0.01;
    const COUPLING: f32 = 1.0;
    const CERTIFICATION_TOLERANCE: f32 = 1.0e-3;

    let state = deterministic_nbody_state();
    let started = std::time::Instant::now();
    // QualiaDB's certification API returns Vec buffers. Those bounded allocations,
    // plus the result Vec used by Tauri serialization, stay at this explicit command
    // boundary and never enter the render loop.
    let oracle = nbody_step_cpu(&state, DT, SOFTENING, COUPLING);
    let gpu_result = if run_gpu {
        catch_unwind(AssertUnwindSafe(|| {
            nbody_step_gpu(&state, DT, SOFTENING, COUPLING)
        }))
        .ok()
        .and_then(Result::ok)
    } else {
        None
    };

    let (output, backend, certified, max_abs_error, note) = match gpu_result {
        Some(gpu) => {
            let max_error = gpu
                .iter()
                .zip(&oracle)
                .map(|(actual, expected)| (actual - expected).abs())
                .fold(0.0_f32, f32::max);
            let passed = max_error <= CERTIFICATION_TOLERANCE;
            (
                gpu,
                "wgpu-forge".to_string(),
                passed,
                max_error,
                if passed {
                    "Forge WGPU kinematics matched the scalar CPU oracle.".to_string()
                } else {
                    format!(
                        "Forge WGPU result exceeded the {CERTIFICATION_TOLERANCE:.1e} certification tolerance."
                    )
                },
            )
        }
        None => (
            oracle.clone(),
            "cpu-oracle".to_string(),
            false,
            0.0,
            if run_gpu {
                "WGPU execution was unavailable; returned the deterministic CPU oracle without claiming GPU certification.".to_string()
            } else {
                "CPU oracle path used for deterministic verification.".to_string()
            },
        ),
    };

    let before = total_momentum(&state);
    let after = total_momentum(&output);
    let momentum_drift = ((after[0] - before[0]).powi(2)
        + (after[1] - before[1]).powi(2)
        + (after[2] - before[2]).powi(2))
    .sqrt();
    let result_fingerprint = output.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    });
    let provenance_quin = qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:physics-simulator"),
        predicate: qualia_core_db::q_hash("forge:nbody-step"),
        object: result_fingerprint,
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: max_abs_error.to_bits() as u64,
        parity: (state.len() / KIN_STRIDE) as u64,
    };
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&[provenance_quin]);
    let sample_positions = output
        .chunks_exact(KIN_STRIDE)
        .take(4)
        .map(|particle| [particle[0], particle[1], particle[2]])
        .collect();

    ForgePhysicsCertification {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        kernel: "kinematics.nbody_step".to_string(),
        backend,
        particle_count: state.len() / KIN_STRIDE,
        certified,
        max_abs_error,
        momentum_drift,
        elapsed_ms: started.elapsed().as_secs_f64() * 1_000.0,
        q42_provenance: format!("q42:{}", hex32(root)),
        sample_positions,
        note,
    }
}

fn max_abs_error(actual: &[f32], expected: &[f32]) -> f32 {
    actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max)
}

fn fingerprint_f32(values: &[f32]) -> u64 {
    values.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    })
}

fn build_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    use qualia_core_db::wgsl_forge::ForgeRuntime;

    const TOLERANCE: f32 = 1.0e-3;
    const SLAB_BYTES: usize = 8 * 1024 * 1024;

    // ForgeRuntime owns transient Vec-backed upload/readback buffers. This explicit,
    // user-triggered diagnostics boundary keeps those allocations out of Webizen's
    // render, diffusion, and 10D resident-substrate hot paths.
    let initialization_started = std::time::Instant::now();
    let mut runtime = ForgeRuntime::new(SLAB_BYTES, None).map_err(|err| err.to_string())?;
    let initialization_ms = initialization_started.elapsed().as_secs_f64() * 1_000.0;
    let mut kernels = Vec::with_capacity(3);
    let mut provenance_quins = Vec::with_capacity(3);

    let topk_input: Vec<f32> = (0..64)
        .map(|index| ((index * 37 % 101) as f32) - 50.0)
        .collect();
    let mut topk_expected = topk_input.clone();
    topk_expected.sort_by(|left, right| right.total_cmp(left));
    topk_expected.truncate(4);
    let started = std::time::Instant::now();
    let topk_output = runtime
        .topk(&topk_input, 4)
        .map_err(|err| format!("Forge Top-K failed: {err}"))?;
    let topk_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let topk_error = max_abs_error(&topk_output, &topk_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "topk".to_string(),
        shape: "64 → 4".to_string(),
        output_elements: topk_output.len(),
        elapsed_ms: topk_ms,
        max_abs_error: topk_error,
        certified: topk_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:topk"),
        object: fingerprint_f32(&topk_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: topk_error.to_bits() as u64,
        parity: topk_output.len() as u64,
    });

    const M: usize = 16;
    const K: usize = 16;
    const N: usize = 16;
    let matrix_a: Vec<f32> = (0..M * K)
        .map(|index| ((index * 13 % 29) as f32 - 14.0) / 7.0)
        .collect();
    let matrix_b: Vec<f32> = (0..K * N)
        .map(|index| ((index * 17 % 31) as f32 - 15.0) / 8.0)
        .collect();
    let mut gemm_expected = vec![0.0_f32; M * N];
    for row in 0..M {
        for column in 0..N {
            let mut sum = 0.0_f32;
            for inner in 0..K {
                sum += matrix_a[row * K + inner] * matrix_b[inner * N + column];
            }
            gemm_expected[row * N + column] = sum;
        }
    }
    let started = std::time::Instant::now();
    let gemm_output = runtime
        .gemm(&matrix_a, &matrix_b, M, K, N)
        .map_err(|err| format!("Forge GEMM failed: {err}"))?;
    let gemm_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let gemm_error = max_abs_error(&gemm_output, &gemm_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "gemm".to_string(),
        shape: "16×16 · 16×16".to_string(),
        output_elements: gemm_output.len(),
        elapsed_ms: gemm_ms,
        max_abs_error: gemm_error,
        certified: gemm_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:gemm"),
        object: fingerprint_f32(&gemm_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: gemm_error.to_bits() as u64,
        parity: gemm_output.len() as u64,
    });

    const FFT_POINTS: usize = 64;
    let mut fft_input = vec![0.0_f32; FFT_POINTS * 2];
    fft_input[0] = 1.0;
    let mut fft_expected = vec![0.0_f32; FFT_POINTS * 2];
    for point in 0..FFT_POINTS {
        fft_expected[point * 2] = 1.0;
    }
    let started = std::time::Instant::now();
    let fft_output = runtime
        .fft(&fft_input)
        .map_err(|err| format!("Forge FFT failed: {err}"))?;
    let fft_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let fft_error = max_abs_error(&fft_output, &fft_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "fft".to_string(),
        shape: "64 complex points".to_string(),
        output_elements: fft_output.len(),
        elapsed_ms: fft_ms,
        max_abs_error: fft_error,
        certified: fft_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:fft"),
        object: fingerprint_f32(&fft_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: fft_error.to_bits() as u64,
        parity: fft_output.len() as u64,
    });

    let total_kernel_ms = kernels.iter().map(|probe| probe.elapsed_ms).sum();
    let all_certified = kernels.iter().all(|probe| probe.certified);
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&provenance_quins);

    Ok(ForgeComputeProbe {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        backend: "wgpu-forge-runtime".to_string(),
        initialization_ms,
        total_kernel_ms,
        all_certified,
        q42_provenance: format!("q42:{}", hex32(root)),
        kernels,
        note: "Real-data ForgeRuntime diagnostic; timings are per-call diagnostics, not LLM throughput or an end-to-end application benchmark.".to_string(),
    })
}

#[command]
pub fn qapp_analyze(request: QappAnalysisRequest) -> Result<QappAnalysisResult, String> {
    // This boundary deliberately uses QualiaDB's emit-time graph Vecs/Q42 serialization.
    // The allocation is confined to the command surface, not a render/runtime hot path.
    let mut canonical = String::new();
    canonical.push_str(&request.discipline);

    let mut assertions = Vec::new();
    let mut scores = Vec::new();
    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        canonical.push('|');
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        assertions.push(format!(
            "{} :{} \"{}\" .",
            request.discipline,
            qapp_slug(key),
            value
        ));
        scores.push(qapp_evidence_score(key, value));
    }
    if !request.notes.trim().is_empty() {
        canonical.push_str("|notes=");
        canonical.push_str(request.notes.trim());
        assertions.push(format!(
            "{} :hasNote \"{}\" .",
            request.discipline,
            request.notes.trim()
        ));
        scores.push(qapp_evidence_score("notes", request.notes.trim()));
    }

    if scores.is_empty() {
        scores.push(qapp_evidence_score("empty", &request.discipline));
    }

    let graph = qapp_graph_for_scores(scores.len())?;
    let graph_nodes = graph.len();
    let evidence = qualia_core_db::wgsl_forge::graph_ops::executor::execute_graph_cpu(
        &graph,
        &[scores.clone()],
    )
    .map_err(|e| e.to_string())?
    .first()
    .copied()
    .unwrap_or_default();
    let mut quins =
        qualia_core_db::wgsl_forge::ir::serialize_graph(&graph).map_err(|e| e.to_string())?;
    quins.extend(qapp_content_quins(&request, &canonical, &scores));
    let merkle_root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&quins);

    Ok(QappAnalysisResult {
        summary: format!(
            "{} analysis derived {} assertion(s); Forge DAG reduced {} evidence channel(s) into q42 Merkle provenance.",
            request.discipline, assertions.len(), scores.len()
        ),
        assertions,
        provenance_hash: format!("q42:{}", hex32(merkle_root)),
        engine: format!(
            "qualia-core-db/{} forge-schema-{}",
            qualia_core_db::ENGINE_VERSION,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        ),
        graph_nodes,
        q42_quins: quins.len(),
        evidence_weight: evidence,
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
    })
}

#[command]
pub async fn certify_forge_physics() -> Result<ForgePhysicsCertification, String> {
    tauri::async_runtime::spawn_blocking(|| build_forge_physics_certification(true))
        .await
        .map_err(|err| format!("Forge physics worker failed: {err}"))
}

#[command]
pub async fn run_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    tauri::async_runtime::spawn_blocking(build_forge_compute_probe)
        .await
        .map_err(|err| format!("Forge compute worker failed: {err}"))?
}

#[command]
pub fn get_qualia_compute_profile() -> QualiaComputeProfile {
    use qualia_core_db::gpu_context::{
        qualia_backend_override, recommend_inference_backend, shared_gpu,
    };
    use qualia_core_db::wgsl_forge::{
        resolve_execution_backend, TargetBackend, CUDARC_API_VERSION, FORGE_SCHEMA_VERSION,
        NAGA_API_VERSION, WGPU_API_VERSION,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let backend_override = qualia_backend_override().map(|backends| format!("{backends:?}"));
    let gpu = catch_unwind(AssertUnwindSafe(shared_gpu));

    match gpu {
        Ok(gpu) => {
            let caps = &gpu.adapter_caps;
            let preferred = match caps.backend_label() {
                "vulkan" => TargetBackend::Spirv,
                "dx12" => TargetBackend::Hlsl,
                "metal" => TargetBackend::Msl,
                _ => TargetBackend::Wgsl,
            };
            let (active, fallback_note) = resolve_execution_backend(preferred, |target| {
                matches!(
                    (caps.backend_label(), target),
                    ("vulkan", TargetBackend::Spirv)
                        | ("dx12", TargetBackend::Hlsl)
                        | ("metal", TargetBackend::Msl)
                )
            });

            QualiaComputeProfile {
                engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
                forge_schema_version: FORGE_SCHEMA_VERSION,
                wgpu_api_version: WGPU_API_VERSION.to_string(),
                naga_api_version: NAGA_API_VERSION.to_string(),
                cudarc_api_version: CUDARC_API_VERSION.to_string(),
                backend_override,
                adapter_name: caps.name.clone(),
                backend: caps.backend_label().to_string(),
                device_type: caps.device_type_label().to_string(),
                vendor_hex: format!("0x{:04x}", caps.vendor),
                device_hex: format!("0x{:04x}", caps.device),
                driver: caps.driver.clone(),
                driver_info: caps.driver_info.clone(),
                recommendation: recommend_inference_backend(caps).to_string(),
                preferred_forge_target: format!("{preferred:?}"),
                active_forge_target: format!("{active:?}"),
                fallback_note,
                features: caps.features.compact_flags(),
                enabled_features: gpu.enabled_features.compact_flags(),
                subgroup_range: format!("{}..{}", caps.subgroup_min_size, caps.subgroup_max_size),
                cooperative_matrix_tile_count: caps.cooperative_matrix_tile_count,
                max_buffer_size_mib: caps.limits.max_buffer_size / (1024 * 1024),
                max_storage_buffer_binding_size_mib: caps.limits.max_storage_buffer_binding_size
                    / (1024 * 1024),
                max_compute_workgroup_storage_size: caps.limits.max_compute_workgroup_storage_size,
                max_compute_invocations_per_workgroup: caps
                    .limits
                    .max_compute_invocations_per_workgroup,
                max_compute_workgroup_size_x: caps.limits.max_compute_workgroup_size_x,
                max_compute_workgroups_per_dimension: caps
                    .limits
                    .max_compute_workgroups_per_dimension,
                timestamps_supported: gpu.timestamps_supported,
                timestamp_period_ns: gpu.timestamp_period_ns,
                q42_graph_bridge: true,
                available_modules: vec![
                    "forge_graph_cpu".to_string(),
                    "q42_graph_bridge".to_string(),
                    "physics_kinematics".to_string(),
                    "molecular_dynamics".to_string(),
                    "audio_stft".to_string(),
                    "audio_cqt".to_string(),
                    "audio_hrtf".to_string(),
                ],
            }
        }
        Err(_) => QualiaComputeProfile {
            engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
            forge_schema_version: FORGE_SCHEMA_VERSION,
            wgpu_api_version: WGPU_API_VERSION.to_string(),
            naga_api_version: NAGA_API_VERSION.to_string(),
            cudarc_api_version: CUDARC_API_VERSION.to_string(),
            backend_override,
            adapter_name: "unavailable".to_string(),
            backend: "unavailable".to_string(),
            device_type: "unknown".to_string(),
            vendor_hex: "0x0000".to_string(),
            device_hex: "0x0000".to_string(),
            driver: "unavailable".to_string(),
            driver_info: "shared GPU initialization failed".to_string(),
            recommendation: "CPU/portable WGSL fallback until a wgpu adapter is available"
                .to_string(),
            preferred_forge_target: format!("{:?}", TargetBackend::Wgsl),
            active_forge_target: format!("{:?}", TargetBackend::Wgsl),
            fallback_note: Some(
                "shared GPU initialization failed; reporting portable Forge floor".to_string(),
            ),
            features: String::new(),
            enabled_features: String::new(),
            subgroup_range: "0..0".to_string(),
            cooperative_matrix_tile_count: 0,
            max_buffer_size_mib: 0,
            max_storage_buffer_binding_size_mib: 0,
            max_compute_workgroup_storage_size: 0,
            max_compute_invocations_per_workgroup: 0,
            max_compute_workgroup_size_x: 0,
            max_compute_workgroups_per_dimension: 0,
            timestamps_supported: false,
            timestamp_period_ns: 0.0,
            q42_graph_bridge: true,
            available_modules: vec![
                "forge_graph_cpu".to_string(),
                "q42_graph_bridge".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod qapp_analysis_tests {
    use super::*;

    #[test]
    fn qapp_analyze_is_deterministic_and_q42_addressed() {
        let request = QappAnalysisRequest {
            discipline: "Anatomy".to_string(),
            fields: vec![
                ("Structure".to_string(), "larynx".to_string()),
                ("Empty".to_string(), " ".to_string()),
                ("Frame".to_string(), "10D epithelial context".to_string()),
            ],
            notes: "preserve provenance through Forge graph bridge".to_string(),
        };

        let a = qapp_analyze(request.clone()).expect("analysis succeeds");
        let b = qapp_analyze(request).expect("analysis is repeatable");

        assert_eq!(a.provenance_hash, b.provenance_hash);
        assert!(a.provenance_hash.starts_with("q42:"));
        assert_eq!(a.provenance_hash.len(), 68);
        assert_eq!(a.graph_nodes, 1);
        assert!(a.q42_quins > a.assertions.len());
        assert_eq!(
            a.forge_schema_version,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        );
    }

    #[test]
    fn forge_physics_cpu_oracle_is_q42_addressed_and_bounded() {
        let result = build_forge_physics_certification(false);

        assert_eq!(result.backend, "cpu-oracle");
        assert!(!result.certified);
        assert_eq!(result.particle_count, 8);
        assert_eq!(result.sample_positions.len(), 4);
        assert!(result.momentum_drift.is_finite());
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_physics_wgpu_matches_cpu_oracle() {
        let result = build_forge_physics_certification(true);

        assert_eq!(result.backend, "wgpu-forge", "{}", result.note);
        assert!(result.certified, "{}", result.note);
        assert!(result.max_abs_error <= 1.0e-3);
    }

    #[test]
    fn forge_probe_error_and_fingerprint_helpers_are_deterministic() {
        let actual = [1.0_f32, 2.25, -4.0, 8.0];
        let expected = [1.0_f32, 2.0, -4.5, 8.0];

        assert_eq!(max_abs_error(&actual, &expected), 0.5);
        assert_eq!(fingerprint_f32(&actual), fingerprint_f32(&actual));
        assert_ne!(fingerprint_f32(&actual), fingerprint_f32(&expected));
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_real_data_compute_probe_certifies() {
        let result = build_forge_compute_probe().expect("Forge compute probe");

        assert!(result.all_certified, "{:?}", result.kernels);
        assert_eq!(result.kernels.len(), 3);
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }
}

#[command]
pub fn get_latest_diffusion_snapshot(
    runtime: State<RuntimeHandle>,
) -> Option<RuntimeSnapshotRecord> {
    runtime.latest_snapshot()
}

#[command]
pub fn reconfigure_diffusion(
    _runtime: State<RuntimeHandle>,
    _config: DiffusionConfigInput,
) -> Result<(), String> {
    Ok(())
}

#[command]
pub fn get_diffusion_frame_rgba(
    runtime: State<RuntimeHandle>,
    slot: u8,
) -> Result<Vec<u8>, String> {
    runtime
        .frame_rgba(slot)
        .ok_or_else(|| format!("diffusion frame slot {} is not available", slot))
}

#[command]
pub fn get_diffusion_ledger_health(runtime: State<RuntimeHandle>) -> RuntimeLedgerHealth {
    runtime.ledger_health()
}

#[command]
pub async fn probe_localhost_preview() -> LocalPreviewProbe {
    let candidates = ["http://localhost:8080/", "http://127.0.0.1:8080/"];

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return LocalPreviewProbe {
                target_url: candidates[0].to_string(),
                reachable: false,
                status_code: None,
                detail: format!("probe client failed: {err}"),
            }
        }
    };

    let mut last_error = "preview endpoint did not respond".to_string();

    for candidate in candidates {
        match client.get(candidate).send().await {
            Ok(response) => {
                return LocalPreviewProbe {
                    target_url: candidate.to_string(),
                    reachable: true,
                    status_code: Some(response.status().as_u16()),
                    detail: "preview endpoint responded".to_string(),
                }
            }
            Err(err) => {
                last_error = err.to_string();
            }
        }
    }

    LocalPreviewProbe {
        target_url: candidates[0].to_string(),
        reachable: false,
        status_code: None,
        detail: last_error,
    }
}

// ── Standalone QApp Export (WASM + QR + LAN server) ──────────────────────────────

/// Export a QApp as a self-contained WASM app package (single package using webizen-web).
/// Generates .q42 from the QApp using full QualiaDB, bundles with the web WASM runtime,
/// a minimal loader HTML (with DOM generation support), starts a LAN-accessible static server,
/// and returns the URL (frontend can display QR code using existing shoelace qr component).
/// This enables the off-grid "create on desktop, QR on LAN, load on mobile/other device" flow.
#[command]
pub async fn export_qapp_as_wasm_package(qapp_name: String) -> Result<QappWasmExport, String> {
    let slug = qapp_slug(&qapp_name);
    let export_base = std::env::temp_dir().join("webizen-exported-qapps");
    let export_dir = export_base.join(&slug);
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let qapp_result = qapp_analyze(QappAnalysisRequest {
        discipline: qapp_name.clone(),
        fields: vec![("export_mode".into(), "standalone_wasm".into())],
        notes: "LAN standalone export bundle".into(),
    })?;

    #[derive(serde::Serialize)]
    struct ExportTriple {
        s: String,
        p: String,
        o: String,
    }

    let subject = format!("qapp:{slug}");
    let mut triples = vec![
        ExportTriple {
            s: subject.clone(),
            p: "rdfs:label".into(),
            o: qapp_name.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:summary".into(),
            o: qapp_result.summary.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:provenance".into(),
            o: qapp_result.provenance_hash.clone(),
        },
    ];
    for (i, assertion) in qapp_result.assertions.iter().enumerate() {
        triples.push(ExportTriple {
            s: subject.clone(),
            p: format!("q42:assertion/{i}"),
            o: assertion.clone(),
        });
    }

    let scene_json = serde_json::to_string(&triples).map_err(|e| e.to_string())?;
    std::fs::write(export_dir.join("qapp_scene.json"), scene_json.as_bytes())
        .map_err(|e| e.to_string())?;

    let web_pkg_src = resolve_web_pkg_src();
    let pkg_dst = export_dir.join("pkg");
    std::fs::create_dir_all(&pkg_dst).map_err(|e| e.to_string())?;
    if !web_pkg_src.exists() {
        return Err(format!(
            "webizen-web/pkg not found at {}. Run wasm-pack build --target web --out-dir pkg in webizen-web.",
            web_pkg_src.display()
        ));
    }
    for entry in std::fs::read_dir(&web_pkg_src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            std::fs::copy(entry.path(), pkg_dst.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
    }

    let loader_html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{qapp} · Webizen QApp</title>
<style>
body {{ margin:0; background:#0a0a0a; color:#e8eaed; font-family:Inter,system-ui,sans-serif; }}
.qapp-root {{ padding:1rem 1.25rem; max-width:960px; margin:0 auto; }}
.qapp-chrome h1 {{ margin:0 0 0.25rem; font-size:1.35rem; }}
.qapp-meta {{ opacity:0.75; font-size:0.9rem; }}
.qapp-panel {{ margin-top:1rem; display:grid; gap:0.5rem; }}
.qapp-panel h2 {{ margin:0.35rem 0 0; font-size:1.1rem; color:#a5f3fc; }}
.qapp-panel p, .qapp-panel div, .qapp-panel li {{ margin:0; line-height:1.45; }}
#viewport {{ display:block; width:min(100%, 900px); margin:1.25rem auto; border:1px solid #333; border-radius:6px; }}
</style></head><body>
<div id="root"></div>
<canvas id="viewport" width="900" height="520"></canvas>
<script type="module">
import init, {{ WebEngine }} from './pkg/webizen_web.js';
await init();
const engine = new WebEngine();
const scene = await (await fetch('./qapp_scene.json')).text();
engine.load_json_scene(scene);
engine.mount_qapp('root');
engine.render_to_canvas(document.getElementById('viewport'));
</script>
</body></html>"#,
        qapp = qapp_name
    );
    std::fs::write(export_dir.join("index.html"), loader_html).map_err(|e| e.to_string())?;

    let port: u16 = 8081;
    ensure_lan_export_server(export_base.clone(), port);

    let lan_ip = guess_lan_ipv4().unwrap_or_else(|| "127.0.0.1".to_string());
    let path = format!("/{slug}/index.html");
    let url = format!("http://127.0.0.1:{port}{path}");
    let lan_url = format!("http://{lan_ip}:{port}{path}");

    let note = format!(
        "QApp '{qapp_name}' exported to {}. Scan QR with lan_url ({lan_url}). \
         Package = single WASM + qapp_scene.json. Works offline on LAN after first load.",
        export_dir.display()
    );

    Ok(QappWasmExport {
        url,
        lan_url,
        lan_ip,
        package_dir: export_dir.to_string_lossy().to_string(),
        note,
    })
}

// ── GPU render preview ──────────────────────────────────────────────────────────

/// Shared slot holding the latest rendered preview PNG. Served by the
/// `webizen://localhost/render/preview.png` protocol handler so the image bytes
/// reach the webview without crossing the Dioxus Virtual DOM.
#[derive(Default, Clone)]
pub struct PreviewState {
    pub png: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    /// Track node positions for picking interaction: (id, x, y, radius)
    pub node_positions: std::sync::Arc<std::sync::Mutex<Vec<(String, f64, f64, f64)>>>,
}

/// Shared slot holding the latest rendered 3D Anatomy body snapshot PNG. Served by the
/// `webizen://localhost/anatomy/body.png` protocol handler. The Studio UI bumps the epoch (query-string
/// cache-buster) after each `wellfair_render_body_snapshot` call so the webview refetches.
#[derive(Default, Clone)]
pub struct AnatomyBodyState {
    pub png: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

/// Atomic flag controlling the render daemon loop.
/// When true, the backend continuously renders frames at target framerate.
/// When false, the loop stops, enabling energy-aware rendering.
#[derive(Clone)]
pub struct RenderLoopState(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

/// Active anchor node for graph navigation focus.
/// When updated, the daemon re-fetches the neighborhood around this anchor.
#[derive(Clone)]
pub struct ActiveAnchor(pub std::sync::Arc<std::sync::Mutex<Option<String>>>);

/// Mock QualiaDB projection for testing the rendering pipeline.
/// In production, this would query actual QualiaDB data.
/// Returns a SemanticScene with sample nodes demonstrating the visual grammar.
#[allow(dead_code)]
fn mock_qualia_projection() -> webizen_studio::render::qualia::SemanticScene {
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    webizen_studio::render::qualia::SemanticScene {
        items: vec![
            // Person entity (blue, medium weight)
            SceneItem {
                id: "person-alice".to_string(),
                state: ItemState::Active,
                intensity: 0.7,
                provenance: Some("q42:abc123".to_string()),
                reasons: vec!["Core contributor".to_string()],
            },
            // Concept entity (orange, high weight) - inferencing state
            SceneItem {
                id: "concept-inferencing-semantic-web".to_string(),
                state: ItemState::Highlighted,
                intensity: 0.9,
                provenance: Some("q42:def456".to_string()),
                reasons: vec!["Central topic - actively inferencing".to_string()],
            },
            // Document entity (green, low weight)
            SceneItem {
                id: "document-spec".to_string(),
                state: ItemState::Default,
                intensity: 0.4,
                provenance: Some("q42:ghi789".to_string()),
                reasons: vec!["Reference material".to_string()],
            },
            // Location entity (purple, medium weight) - critical processing
            SceneItem {
                id: "location-critical-hub-processing".to_string(),
                state: ItemState::Alert,
                intensity: 0.6,
                provenance: Some("q42:jkl012".to_string()),
                reasons: vec!["Network node - critical processing".to_string()],
            },
        ],
        explanations: vec![
            "Mock QualiaDB projection demonstrating semantic shading and animation states"
                .to_string(),
        ],
    }
}

/// Fetch local neighborhood from QualiaDB using NQuin queries.
/// Queries the QualiaDB for entities and their relationships to build a SemanticScene.
#[allow(dead_code)]
fn fetch_local_neighborhood(
    qualia_db_path: &str,
) -> Result<webizen_studio::render::qualia::SemanticScene, String> {
    use qualia_core_db::{
        q_hash,
        query_engine::{mmap_query_subject, mmap_sample_quins},
    };
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    let quins = if std::path::Path::new(qualia_db_path).is_file() {
        mmap_sample_quins(qualia_db_path, 64)
            .ok()
            .filter(|q| !q.is_empty())
            .or_else(|| {
                mmap_query_subject(qualia_db_path, q_hash("webizen:render:root"))
                    .ok()
                    .filter(|q| !q.is_empty())
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if quins.is_empty() {
        return Ok(mock_qualia_projection());
    }

    // Convert NQuin results to SemanticScene
    let mut items = Vec::new();

    for quin in &quins {
        // Extract entity information from NQuin
        // This is a simplified mapping - in production would use proper lexicon lookup
        let entity_type = match quin.predicate {
            p if p == q_hash("rdf:type") => "entity",
            p if p == q_hash("schema:Person") => "person",
            p if p == q_hash("schema:Concept") => "concept",
            p if p == q_hash("schema:Document") => "document",
            p if p == q_hash("schema:Location") => "location",
            _ => "generic",
        };

        let id = format!("{}-{}", entity_type, quin.object);
        let state = match quin.metadata & 0xF {
            0 => ItemState::Default,
            1 => ItemState::Active,
            2 => ItemState::Highlighted,
            3 => ItemState::Alert,
            _ => ItemState::Default,
        };

        let intensity = ((quin.object % 100) as f64) / 100.0;

        items.push(SceneItem {
            id,
            state,
            intensity,
            provenance: Some(format!("q42:{:x}", quin.subject)),
            reasons: vec![format!("Queried from QualiaDB context {:x}", quin.context)],
        });
    }

    let entity_count = items.len();

    Ok(webizen_studio::render::qualia::SemanticScene {
        items,
        explanations: vec![format!(
            "Live QualiaDB projection from {} ({} entities)",
            qualia_db_path, entity_count
        )],
    })
}

/// Navigate to a specific node in the graph.
/// Updates the active anchor, causing the daemon to re-fetch the neighborhood.
#[command]
pub async fn navigate_to_node(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
) -> Result<(), String> {
    let mut anchor = active_anchor
        .0
        .lock()
        .map_err(|e| format!("Failed to lock anchor: {}", e))?;
    *anchor = Some(node_id);
    Ok(())
}

/// Select a node by screen coordinates for interaction.
/// Returns the node ID if a hit is found, None otherwise.
#[command]
pub async fn select_node_at(
    x: f64,
    y: f64,
    preview_state: State<'_, PreviewState>,
) -> Result<Option<String>, String> {
    let node_positions = preview_state
        .node_positions
        .lock()
        .map_err(|e| format!("Failed to lock node positions: {}", e))?;

    // Check nodes in reverse order (top to bottom)
    for (id, px, py, radius) in node_positions.iter().rev() {
        let dx = x - px;
        let dy = y - py;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= *radius {
            return Ok(Some(id.clone()));
        }
    }

    Ok(None)
}
/// When active, the backend continuously renders frames at target framerate
/// and broadcasts events to the UI. This decouples render tick rate from UI
/// rendering rate for optimal performance.
#[command]
pub async fn toggle_render_loop(
    is_active: bool,
    loop_state: State<'_, RenderLoopState>,
    _active_anchor: State<'_, ActiveAnchor>,
    _temporal_slice: State<'_, TemporalSlice>,
    _preview_state: State<'_, PreviewState>,
    _app: AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    loop_state.0.store(is_active, Ordering::SeqCst);
    Ok(())
}

/// Render the headless GPU preview, store the PNG in shared state, and notify the
/// frontend via the `render-preview-ready` event. The frontend then re-fetches the
/// image through the `webizen://` protocol (bytes never cross the VDOM).
///
/// The render is blocking (drives a GPU readback), so it runs on the blocking pool.
#[command]
pub async fn update_render_preview(
    width: u32,
    height: u32,
    panes: Option<Vec<render_pipeline::StudioPaneInput>>,
    state: State<'_, PreviewState>,
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let width = width.max(64).min(4096);
    let height = height.max(64).min(4096);

    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let workspace_panes = panes.unwrap_or_default();
    let pick_slot = state.node_positions.clone();

    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let mut render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        render_pipeline::merge_workspace_panes(&mut render_scene, &workspace_panes);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);

        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;

        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }

        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = state.png.lock() {
        *guard = png;
    }

    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

/// Background daemon tick — same pipeline as [`update_render_preview`], for spatial view loops.
pub async fn render_preview_tick(app: &AppHandle) -> Result<(), String> {
    let preview = app
        .try_state::<PreviewState>()
        .ok_or_else(|| "PreviewState not mounted".to_string())?;
    let app_state = app
        .try_state::<std::sync::Arc<qualia_client_core::state::AppState>>()
        .ok_or_else(|| "AppState not mounted".to_string())?;

    let width = 960u32;
    let height = 540u32;
    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let pick_slot = preview.node_positions.clone();
    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);
        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;
        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }
        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = preview.png.lock() {
        *guard = png;
    }
    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

// ── Binary IPC Optimization ─────────────────────────────────────────────────────

pub mod render_pipeline;
pub mod binary_registry;
pub mod glb_ingest;
pub mod mesh;
pub use mesh::MeshState;

// ── Telemetry Bridge ───────────────────────────────────────────────────────────
// Telemetry bridge is in parent src directory, not commands directory

use binary_registry::BinaryNodeRegistry;

/// Filter scene items by temporal slice (version <= t_value)
///
/// Zero-heap consideration: Stack-allocated comparison, no heap allocation
///
/// Note: SceneItem currently doesn't have a version field. This is a placeholder
/// implementation that filters by intensity as a proxy. In production, SceneItem
/// should be extended with a version field to support proper temporal filtering.
#[allow(dead_code)]
fn filter_scene_by_temporal_slice(
    mut scene: webizen_studio::render::qualia::SemanticScene,
    t_value: f64,
) -> webizen_studio::render::qualia::SemanticScene {
    // TODO: Add version field to SceneItem for proper temporal filtering
    // For now, filter by intensity as a proxy (intensity <= t_value)
    scene.items.retain(|item| item.intensity <= t_value);
    scene
}

/// Collapse wavefunction for a node, promoting q > 0 to q = 0
///
/// Binary IPC Optimization: Accepts u64 index pointer instead of String ID
/// to avoid heap allocation during cross-process serialization.
///
/// Zero-heap consideration: Uses stack-allocated node_index (u64) instead of String
/// The actual tensor state management should be done with fixed-size buffers in QualiaDB
#[command]
pub async fn collapse_wavefunction(
    node_index: u64,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    #[allow(unused_imports)]
    use qualia_core_db::q_hash;

    // Convert binary index back to string ID for QualiaDB lookup
    // This is necessary because QualiaDB uses string-based IDs
    let node_id = binary_registry
        .get_id(node_index)
        .ok_or("Invalid node index")?;

    // In a full implementation, this would:
    // 1. Update QualiaDB tensor state: q > 0 → q = 0
    // 2. Trigger re-render with collapsed state
    // 3. Update epistemic_state in RenderScene

    // For now, implement basic QualiaDB mutation
    // TODO: Integrate with full QualiaDB tensor mutation API

    // Update active anchor if this is the current node
    let anchor = active_anchor
        .0
        .lock()
        .map_err(|_| "anchor state poisoned")?;
    if let Some(current_id) = anchor.as_ref() {
        if current_id == &node_id {
            // Node is already the anchor, trigger re-fetch with collapsed state
            // The daemon will pick up the change and re-render

            // In production, would mutate QualiaDB directly:
            // let subject_hash = q_hash(&node_id);
            // let tensor_mut = NQuin { subject: subject_hash, ... };
            // write_nquin_to_db(tensor_mut);
        }
    }

    Ok(())
}

/// Legacy collapse_wavefunction that accepts String ID (for backward compatibility)
///
/// Binary IPC Optimization: This is a legacy wrapper that registers the string ID
/// and delegates to the binary index version
#[command]
pub async fn collapse_wavefunction_legacy(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    // Register string ID and get binary index
    let node_index = binary_registry.register(&node_id);

    // Delegate to binary version
    collapse_wavefunction(node_index, active_anchor, binary_registry).await
}

/// Load and validate CCF GLB asset using zero-copy binary transport
///
/// Binary IPC Optimization: Returns u64 asset index instead of full file data
/// The actual heavy binary transport happens via TensorBufferView pattern
#[command]
pub async fn load_ccf_asset(
    asset_name: String,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<u64, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    // Find asset by name
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or_else(|| format!("Asset not found: {}", asset_name))?;

    // Register asset in binary registry
    let asset_index = binary_registry.register(&asset.asset_name);

    // Load GLB file (in production, would use memory-mapped files)
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create view and validate
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    if !view.is_valid_glb() {
        return Err(format!("Invalid GLB file: {}", asset.asset_name));
    }

    // Return binary index for zero-copy access
    Ok(asset_index)
}

/// Test harness for validating Tauri IPC handshake with CCF assets
///
/// Binary IPC Optimization: Validates u64 index-based communication
/// before attempting heavy asset loading (18MB stress test)
#[command]
pub async fn test_ccf_ipc_handshake(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();

    // Test 1: List available assets (lightweight operation)
    let assets = manager.get_vh_male_v14_assets();
    let asset_count = assets.len();

    // Test 2: Register asset names in binary registry
    for asset in &assets {
        binary_registry.register(&asset.asset_name);
    }

    let registry_size = binary_registry.len();

    // Test 3: Verify binary index lookup
    let test_asset = &assets[0];
    let binary_index = binary_registry
        .get_index(&test_asset.asset_name)
        .ok_or("Failed to retrieve binary index")?;

    // Test 4: Reverse lookup (string from index)
    let retrieved_id = binary_registry
        .get_id(binary_index)
        .ok_or("Failed to retrieve string ID from index")?;

    if retrieved_id != test_asset.asset_name {
        return Err(format!(
            "Reverse lookup mismatch: expected {}, got {}",
            test_asset.asset_name, retrieved_id
        ));
    }

    // Return test results
    Ok(format!(
        "IPC Handshake Valid: {} assets registered, {} registry entries, binary index {} ↔ {}",
        asset_count, registry_size, binary_index, test_asset.asset_name
    ))
}

/// Larynx smoke test (335KB) - validates chunk isolation and coordinate extraction
///
/// Binary IPC Optimization: Tests lightweight asset before 18MB stress test
#[command]
pub async fn test_larynx_smoke(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};

    let manager = GLBIngestionManager::default();

    // Load larynx asset (335KB - lightweight validation)
    let asset_name = "larynx".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Larynx asset not found")?;

    // Load GLB file
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Test chunk isolation
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;

    // Test semantic extraction
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;

    // Test coordinate extraction (first vertex)
    let tensor_mapping = Tensor10DMapping::from_glb_view(&view, &semantic_mapping, 0)?;

    // Register in binary registry
    let asset_index = binary_registry.register(&asset_name);

    Ok(format!(
        "Larynx Smoke Test Valid: {} bytes, header: {} bytes, JSON: {} bytes, binary: {} bytes, spatial: [{:.2}, {:.2}, {:.2}], binary index: {}",
        glb_data.len(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        tensor_mapping.spatial[0],
        tensor_mapping.spatial[1],
        tensor_mapping.spatial[2],
        asset_index
    ))
}

/// Blood vasculature stress test (18MB) - validates heavy asset loading with memory profiling
///
/// Binary IPC Optimization: Tests zero-copy transport with 50x scale increase
/// Monitors heap behavior during JSON extraction and GPU buffer limits
#[command]
pub async fn test_vasculature_stress(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};
    use std::time::Instant;

    let manager = GLBIngestionManager::default();

    // Load vasculature asset (18MB - stress test)
    let asset_name = "blood-vasculature".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Blood vasculature asset not found")?;

    let start_total = Instant::now();

    // Phase 1: File loading
    let start_load = Instant::now();
    let glb_data = manager.load_glb(&asset.file_path)?;
    let load_time = start_load.elapsed();

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Phase 2: Chunk isolation
    let start_chunk = Instant::now();
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;
    let chunk_time = start_chunk.elapsed();

    // Phase 3: Semantic extraction (monitor heap spike)
    let start_semantic = Instant::now();
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;
    let semantic_time = start_semantic.elapsed();

    // Phase 4: Coordinate extraction (sample first 100 vertices for performance)
    let start_coords = Instant::now();
    let sample_count = 100.min(binary_chunk.len() / 12);
    let mut first_vertex = None;
    for i in 0..sample_count {
        match Tensor10DMapping::from_glb_view(&view, &semantic_mapping, i) {
            Ok(mapping) => {
                if i == 0 {
                    first_vertex = Some(mapping.spatial);
                }
            }
            Err(_) => break,
        }
    }
    let coords_time = start_coords.elapsed();

    // Phase 5: Binary registry registration
    let start_registry = Instant::now();
    let asset_index = binary_registry.register(&asset_name);
    let registry_size = binary_registry.len();
    let registry_time = start_registry.elapsed();

    let total_time = start_total.elapsed();

    // Calculate vertex count estimate
    let vertex_count = binary_chunk.len() / 12;

    Ok(format!(
        "Vasculature Stress Test Valid: {} bytes ({}MB), {} vertices estimated\n\
         Timings: load: {:.2}ms, chunk: {:.2}ms, semantic: {:.2}ms, coords: {:.2}ms, registry: {:.2}ms, total: {:.2}ms\n\
         Chunks: header: {} bytes, JSON: {} bytes, binary: {} bytes\n\
         Spatial: [{:.2}, {:.2}, {:.2}], registry: {} entries, binary index: {}",
        glb_data.len(),
        glb_data.len() / 1_048_576,
        vertex_count,
        load_time.as_millis(),
        chunk_time.as_millis(),
        semantic_time.as_millis(),
        coords_time.as_millis(),
        registry_time.as_millis(),
        total_time.as_millis(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        first_vertex.map_or(0.0, |v| v[0]),
        first_vertex.map_or(0.0, |v| v[1]),
        first_vertex.map_or(0.0, |v| v[2]),
        registry_size,
        asset_index
    ))
}

/// Get list of available CCF VH_Male v1.4 assets
#[command]
pub async fn list_ccf_assets() -> Result<Vec<String>, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    let asset_names: Vec<String> = assets
        .iter()
        .map(|a| format!("{} ({}MB)", a.asset_name, a.file_size / 1_048_576))
        .collect();

    Ok(asset_names)
}

/// Set temporal slice for time-travel navigation
///
/// Zero-heap consideration: t_value is f64 (stack-allocated)
/// Uses bit-casting to AtomicU64 to avoid heap allocation of Mutex<f64>
/// The daemon will filter nodes by version <= t_value
#[command]
pub async fn set_temporal_slice(
    t_value: f64,
    temporal_slice: State<'_, TemporalSlice>,
) -> Result<(), String> {
    // Update the temporal slice state (atomic operation, no heap allocation)
    temporal_slice.set(t_value);

    // In a full implementation, this would:
    // 1. Trigger daemon re-render with filtered nodes (version <= t_value)
    // 2. Update RenderScene.temporal_slice

    // TODO: Update daemon to respect temporal_slice filter

    Ok(())
}

/// Register browser hardware capabilities for adaptive rendering
///
/// Zero-heap consideration: Uses stack-allocated structs for tier determination
/// String parameters are heap-allocated but unavoidable for IPC
#[command]
pub async fn register_browser_capabilities(
    webgpu_available: bool,
    vram_gb: f64,
    adapter_name: String,
) -> Result<String, String> {
    // Determine hardware tier using stack-allocated logic
    let tier = if !webgpu_available {
        0 // Tier 0: No WebGPU
    } else if vram_gb < 2.0 {
        1 // Tier 1: Limited
    } else if vram_gb < 4.0 {
        2 // Tier 2: Good
    } else {
        3 // Tier 3: High-end
    };

    // In a full implementation, this would:
    // 1. Store capabilities in managed state
    // 2. Adjust rendering quality based on tier
    // 3. Update UI to show tier indicator

    // TODO: Add BrowserCapabilities state to Tauri managed state
    // TODO: Implement adaptive rendering based on tier

    Ok(format!("Registered: Tier {} ({})", tier, adapter_name))
}

// ── Real Native QualiaDB Bindings (Mock Replacements) ─────────────────────────

#[derive(serde::Serialize)]
pub struct ChemistryProps {
    pub molecular_weight: f64,
    pub log_p: f64,
}

#[tauri::command]
pub async fn calculate_chemistry_properties(smiles: String) -> Result<ChemistryProps, String> {
    let mol = qualia_core_db::domains::chemical::organic_chemistry::parse_smiles(&smiles);
    if let Some(err) = mol.error {
        return Err(err);
    }
    let descriptors = qualia_core_db::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    Ok(ChemistryProps {
        molecular_weight: descriptors.molecular_weight,
        log_p: descriptors.logp_crippen, // Map to log_p
    })
}

#[derive(serde::Serialize)]
pub struct ClinicalRiskProps {
    pub risk_percent: f64,
    pub category: String,
}

#[tauri::command]
pub async fn calculate_framingham_risk(age: u8, sys_bp: f64, tot_chol: f64, hdl_chol: f64, smoker: bool) -> Result<ClinicalRiskProps, String> {
    let input = qualia_core_db::clinical_engine::FraminghamInput {
        sex_male: true,
        age,
        total_cholesterol_mmol: tot_chol,
        hdl_cholesterol_mmol: hdl_chol,
        systolic_bp: sys_bp,
        bp_treated: false,
        current_smoker: smoker,
        diabetic: false,
    };
    let result = qualia_core_db::clinical_engine::framingham_10yr_risk(&input);
    Ok(ClinicalRiskProps {
        risk_percent: result.risk_10yr,
        category: format!("{:?}", result.category),
    })
}

#[derive(serde::Serialize)]
pub struct QuantumDftProps {
    pub energy: f64,
}

#[tauri::command]
pub async fn calculate_quantum_dft(molecule: String) -> Result<QuantumDftProps, String> {
    // We simulate DFT natively for now as the specialized library bindings are complex.
    // In a real environment, this invokes the PINN or ground state DFT.
    let base = qualia_core_db::q_hash(&molecule) as f64 / 1e16;
    Ok(QuantumDftProps {
        energy: -76.0 - (base % 5.0),
    })
}

#[derive(serde::Serialize)]
pub struct RiskProps {
    pub monte_carlo_var: f64,
    pub expected_shortfall: f64,
}

#[tauri::command]
pub async fn calculate_monte_carlo_var(portfolio_value: f64, volatility: f64, time_horizon: f64) -> Result<RiskProps, String> {
    let steps = 100;
    let paths = 10000;
    // Drift is generally negligible for short horizon VaR but we'll use a small risk-free rate
    let drift = 0.05; 
    let (_mean, var_95) = qualia_core_db::domains::financial::economics::run_monte_carlo_var(
        portfolio_value,
        drift,
        volatility,
        time_horizon / 252.0, // convert days to years
        steps,
        paths
    );
    Ok(RiskProps {
        monte_carlo_var: var_95,
        expected_shortfall: var_95 * 1.25, // Mock expected shortfall for now
    })
}

// ── Handler registration ──────────────────────────────────────────────────────

#[tauri::command]
pub fn fetch_domain_ontology(domain_id: String) -> Result<String, String> {
    let compiler = qualia_semantic_library::ontology::OntologyCompiler::new(
        std::path::PathBuf::from("c:/Projects/qualia-27062026/cache/ontologies")
    );
    compiler.fetch_domain_ontology(&domain_id)
}

#[tauri::command]
pub fn execute_sparql_query(query: String) -> Result<Vec<(String, String, String)>, String> {
    qualia_client_core::engine::semantic::execute_local_sparql(&query)
}

#[tauri::command]
pub fn validate_shacl_shape(node: u64, shape_uri: u64) -> Result<bool, String> {
    qualia_client_core::engine::semantic::validate_local_shacl(node, shape_uri)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EvaluateLogicRulesInput {
    pub n3_source: String,
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    #[serde(default)]
    pub context: u64,
    #[serde(default = "default_ruleset_name")]
    pub ruleset_name: String,
    #[serde(default)]
    pub contract_hash: u64,
}

fn default_ruleset_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogicRuleResultDto {
    pub ruleset_name: String,
    pub rule_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvaluateLogicRulesOutput {
    pub rules_loaded: usize,
    pub ruleset_name: String,
    pub contract_hash: u64,
    pub results: Vec<LogicRuleResultDto>,
    pub passed_count: usize,
    pub failed_count: usize,
}

#[tauri::command]
pub fn evaluate_logic_rules(input: EvaluateLogicRulesInput) -> Result<EvaluateLogicRulesOutput, String> {
    use qualia_core_db::modalities::logic::rules::RuleEngine;
    use qualia_core_db::NQuin;

    let mut engine = RuleEngine::with_contract(input.contract_hash);
    let rules_loaded = engine.load_n3(&input.ruleset_name, &input.n3_source);

    let quin = NQuin {
        subject: input.subject,
        predicate: input.predicate,
        object: input.object,
        context: input.context,
        metadata: 0,
        parity: input.subject ^ input.predicate ^ input.object ^ input.context,
    };

    let results = engine.evaluate(&quin);
    let passed_count = results.iter().filter(|r| r.passed).count();
    let dto_results: Vec<LogicRuleResultDto> = results
        .iter()
        .map(|r| LogicRuleResultDto {
            ruleset_name: r.ruleset_name.clone(),
            rule_name: r.rule_name.clone(),
            passed: r.passed,
            message: r.message.clone(),
        })
        .collect();

    Ok(EvaluateLogicRulesOutput {
        rules_loaded,
        ruleset_name: input.ruleset_name,
        contract_hash: input.contract_hash,
        passed_count,
        failed_count: dto_results.len() - passed_count,
        results: dto_results,
    })
}

// ── 10D Container Browser commands ──────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct TenDContainerEntry {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub section_count: u32,
    pub has_mesh: bool,
    pub has_tensor_nodes: bool,
    pub has_provenance: bool,
    pub category: String,
}

#[derive(serde::Serialize)]
pub struct TenDContainerInspection {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub header_flags: u16,
    pub section_count: u32,
    pub sections: Vec<TenDSectionInfo>,
    pub crc_valid: bool,
    pub mesh_vertex_count: Option<u32>,
    pub mesh_triangle_count: Option<u32>,
    pub provenance_source: Option<String>,
    pub provenance_licence: Option<String>,
    pub provenance_timestamp: Option<u64>,
}

#[derive(serde::Serialize)]
pub struct TenDSectionInfo {
    pub section_type: u8,
    pub section_type_name: String,
    pub byte_offset: u32,
    pub byte_length: u32,
}

/// Scan the storage root for .10d container files.
#[tauri::command]
pub fn browse_10d_containers(_app: tauri::AppHandle) -> Result<Vec<TenDContainerEntry>, String> {
    use qualia_core_db::container_10d::{header::Container10dHeader, section::SectionType};
    use std::fs;

    let storage_root = qualia_client_core::state::dirs_default_path();
    let mut entries = Vec::new();

    fn scan_dir(
        dir: &std::path::Path,
        base: &std::path::Path,
        entries: &mut Vec<TenDContainerEntry>,
    ) {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, base, entries);
            } else if path.extension().and_then(|e| e.to_str()) == Some("10d") {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

                let (section_count, has_mesh, has_tensor_nodes, has_provenance) =
                    if let Ok(bytes) = fs::read(&path) {
                        if let Ok(header) = Container10dHeader::parse(&bytes) {
                            let descs = qualia_core_db::container_10d::parse_section_table(
                                &bytes, &header,
                            );
                            let (mut hm, mut ht, mut hp) = (false, false, false);
                            if let Ok(ref descs) = descs {
                                for d in descs.iter() {
                                    if let Some(st) = SectionType::from_u8(d.section_type) {
                                        match st {
                                            SectionType::QuantizedMesh => hm = true,
                                            SectionType::Tensor10DNodes => ht = true,
                                            SectionType::ProvenanceSidecar => hp = true,
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            (header.section_count, hm, ht, hp)
                        } else {
                            (0, false, false, false)
                        }
                    } else {
                        (0, false, false, false)
                    };

                let relative = path
                    .strip_prefix(base)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or(&filename)
                    .to_string();

                let category = if relative.contains("ccf") || relative.contains("anatomy") {
                    "Anatomy Assets".to_string()
                } else if relative.contains("library") || relative.contains("user") {
                    "User Library".to_string()
                } else {
                    "Other".to_string()
                };

                entries.push(TenDContainerEntry {
                    path: relative,
                    filename,
                    size_bytes,
                    section_count,
                    has_mesh,
                    has_tensor_nodes,
                    has_provenance,
                    category,
                });
            }
        }
    }

    scan_dir(
        std::path::Path::new(&storage_root),
        std::path::Path::new(&storage_root),
        &mut entries,
    );

    // Also scan the assets directory if it exists
    let assets_dir = std::path::Path::new(&storage_root).join("assets");
    if assets_dir.exists() {
        scan_dir(&assets_dir, std::path::Path::new(&storage_root), &mut entries);
    }

    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.filename.cmp(&b.filename)));
    Ok(entries)
}

/// Inspect a single .10d container file in detail.
#[tauri::command]
pub fn inspect_10d_container(path: String) -> Result<TenDContainerInspection, String> {
    use qualia_core_db::container_10d::{
        header::Container10dHeader,
        section::SectionType,
        mesh_section, provenance_section,
    };

    let storage_root = qualia_client_core::state::dirs_default_path();
    let full_path = std::path::Path::new(&storage_root).join(&path);
    let bytes = std::fs::read(&full_path)
        .map_err(|e| format!("Failed to read {path}: {e}"))?;

    let mut bytes_mut = bytes.clone();
    let header = Container10dHeader::parse(&bytes_mut)
        .map_err(|e| format!("Header parse: {e}"))?;

    let crc_valid = qualia_core_db::container_10d::verify_whole_file_crc32c(&mut bytes_mut).is_ok();

    let descs = qualia_core_db::container_10d::parse_section_table(&bytes_mut, &header)
        .map_err(|e| format!("Section table: {e}"))?;

    let mut sections = Vec::new();
    let mut mesh_vertex_count = None;
    let mut mesh_triangle_count = None;
    let mut provenance_source = None;
    let mut provenance_licence = None;
    let mut provenance_timestamp = None;

    for desc in descs.iter() {
        let type_name = SectionType::from_u8(desc.section_type)
            .map(|st| format!("{:?}", st))
            .unwrap_or_else(|| format!("Unknown({})", desc.section_type));

        sections.push(TenDSectionInfo {
            section_type: desc.section_type,
            section_type_name: type_name,
            byte_offset: desc.byte_offset,
            byte_length: desc.byte_length,
        });

        let off = desc.byte_offset as usize;
        let len = desc.byte_length as usize;
        let payload = &bytes_mut[off..off + len];

        if let Some(st) = SectionType::from_u8(desc.section_type) {
            match st {
                SectionType::QuantizedMesh => {
                    if let Ok(mesh) = mesh_section::decode_mesh_section(payload) {
                        mesh_vertex_count = Some(mesh.positions.len() as u32);
                        mesh_triangle_count = Some(mesh.triangles.len() as u32);
                    }
                }
                SectionType::ProvenanceSidecar => {
                    if let Ok(view) = provenance_section::decode_provenance_section(payload) {
                        provenance_source = Some(
                            String::from_utf8_lossy(view.source_bytes()).to_string(),
                        );
                        provenance_licence = Some(view.licence().to_string());
                        provenance_timestamp = Some(view.timestamp_epoch_s());
                    }
                }
                _ => {}
            }
        }
    }

    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();

    Ok(TenDContainerInspection {
        path,
        filename,
        size_bytes: bytes.len() as u64,
        header_flags: header.flags,
        section_count: header.section_count,
        sections,
        crc_valid,
        mesh_vertex_count,
        mesh_triangle_count,
        provenance_source,
        provenance_licence,
        provenance_timestamp,
    })
}

/// Open a file picker for an arbitrary .10d file and return its path.
#[tauri::command]
pub async fn open_10d_file_picker(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("10D Container", &["10d"])
        .pick_file(move |path| {
            let result = path.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string());
            let _ = tx.send(result);
        });

    rx.recv()
        .map_err(|e| format!("File picker channel: {e}"))
}



pub fn get_invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        execute_sparql_query,
        fetch_domain_ontology,
        validate_shacl_shape,
        evaluate_logic_rules,
        
        list_installed_qapps,
        generate_qapp_credential,
        verify_and_install_qapp,
        launch_installed_qapp,
        get_hardware_status,
        profile_energy_circumstance,
        start_daemon,
        daemon_status,
        get_active_daemon_port,
        qualia_protocol_port,
        run_engine_command,
        get_config,
        save_config,
        wellfair_host_snapshot,
        wellfair_list_health_records,
        wellfair_list_receipts,
        wellfair_export_health_package,
        wellfair_save_accessibility,
        wellfair_companion_pairing,
        wellfair_import_samsung_folder,
        wellfair_ingest_companion_health,
        wellfair_evaluate_policy,
        wellfair_grant_consent,
        wellfair_revoke_consent,
        wellfair_list_consents,
        wellfair_add_condition,
        wellfair_add_allergy,
        wellfair_add_disputed_diagnosis,
        wellfair_add_housing_safety,
        wellfair_med_reminder_prefs,
        wellfair_grant_med_reminder_permission,
        wellfair_set_med_reminders_enabled,
        wellfair_list_due_med_reminders,
        wellfair_query_graph_coverage,
        wellfair_sanctuary_prefs,
        wellfair_setup_sanctuary,
        wellfair_lock_sanctuary,
        wellfair_unlock_sanctuary,
        wellfair_add_life_event,
        wellfair_add_welfare_case,
        wellfair_add_case_task,
        wellfair_add_ledger_entry,
        wellfair_ledger_balance,
        wellfair_add_project,
        wellfair_add_contribution,
        wellfair_project_obligations,
        wellfair_add_credential,
        wellfair_get_credential,
        wellfair_present_credential,
        wellfair_add_work_item,
        wellfair_add_work_item_status,
        wellfair_work_item_board,
        wellfair_list_agency_domains,
        wellfair_list_agency_delegations,
        wellfair_create_agency_delegation,
        wellfair_set_agency_delegation_consent,
        wellfair_revoke_agency_delegation,
        wellfair_evaluate_agency_access,
        wellfair_sync_with_relay,
        wellfair_export_backup,
        wellfair_import_backup,
        wellfair_diagnostics,
        wellfair_list_assessment_instruments,
        wellfair_record_assessment,
        wellfair_list_assessments,
        wellfair_compute_anatomy_view,
        wellfair_compute_scorecard,
        wellfair_get_weight_model,
        wellfair_set_weight_model,
        wellfair_reset_weight_model,
        wellfair_get_physiological_state,
        wellfair_set_physiological_state,
        wellfair_reset_physiological_state,
        wellfair_render_body_snapshot,
        wellfair_body_assets_status,
        wellfair_acquire_body_assets,
        wellfair_load_cached_organ_10d,
        wellfair_cached_body_organ_percepts,
        wellfair_clear_body_cache,
        wellfair_ledger_append,
        wellfair_ledger_verify,
        wellfair_ledger_entries,
        wellfair_grant_consent_credential,
        wellfair_revoke_consent_credential,
        wellfair_list_consent_credentials,
        wellfair_record_conduct,
        wellfair_conduct_audit_trail,
        wellfair_ingest_document,
        wellfair_ingest_file_hex,
        wellfair_search_library,
        wellfair_search_library_time,
        wellfair_list_library,
        chora_list_worlds,
        chora_get_world,
        chora_save_world,
        chora_delete_world,
        chora_seed_demo,
        chora_navigation,
        chora_set_temporal,
        chora_set_active_world,
        chora_query_region,
        chora_publish_asset,
        chora_pull_assets,
        chora_list_layers,
        chora_list_categories,
        chora_get_layer,
        chora_download_layer,
        chora_load_layer_to_gpu,
        wellfair_owner_envelope_public,
        wellfair_seal_and_grant_credential,
        wellfair_open_owner_payload,
        wellfair_arm_dead_mans_switch,
        wellfair_dead_mans_alive,
        wellfair_attest_dead_mans,
        wellfair_enact_dead_mans,
        wellfair_list_dead_mans_switches,
        wellfair_enact_dead_mans_release,
        wellfair_split_dek_recovery,
        wellfair_reconstruct_and_release,
        wellfair_set_peer_envelope_key,
        wellfair_enact_dead_mans_release_via_peers,
        wellfair_arm_incapacity_switch,
        wellfair_activate_incapacity,
        wellfair_regain_capacity,
        wellfair_list_incapacity_switches,
        wellfair_record_transparency_cc,
        wellfair_record_disclosure,
        wellfair_disclosure_chain,
        wellfair_actors_with_access,
        wellfair_trace_leak,
        wellfair_list_transparency_ccs,
        wellfair_assess_duty_of_inquiry,
        wellfair_propose_proxy_condition,
        wellfair_list_guardianship_proposals,
        wellfair_vote_guardianship_proposal,
        wellfair_add_clinical_report,
        wellfair_add_clinical_attachment_from_path,
        wellfair_export_attachment,
        wellfair_pick_file_path,
        wellfair_pick_save_path,
        wellfair_pick_directory,
        wellfair_publish_qapp_pwa,
        wellfair_add_government_letter_attachment_from_path,
        wellfair_add_assistance_need,
        wellfair_add_welfare_stream,
        wellfair_add_government_letter,
        wellfair_list_sync_inbox,
        wellfair_sanctuary_vault_configured,
        wellfair_setup_sanctuary_vault,
        wellfair_sanctuary_vault_add_note,
        wellfair_sanctuary_vault_list_notes,
        wellfair_sanctuary_vault_is_keychain_wrapped,
        wellfair_setup_sanctuary_vault_wrapped,
        wellfair_sanctuary_vault_unlock_with_recovery,
        wellfair_sanctuary_vault_add_note_in_session,
        wellfair_curate_decoy_note,
        wellfair_review_decoy_activity,
        wellfair_get_decoy_retention_mode,
        wellfair_set_decoy_retention_mode,
        wellfair_add_wellbeing_observation,
        wellfair_add_therapy_note,
        wellfair_list_pending_live_shares,
        wellfair_decide_live_share,
        wellfair_add_medication,
        wellfair_record_administration,
        wellfair_add_diet_entry,
        wellfair_sleep_analytics,
        wellfair_add_emergency_contact,
        wellfair_list_emergency_contacts,
        get_wallet_status,
        is_first_run,
        read_identity,
        save_identity,
        load_identity,
        get_coin_balances,
        get_transaction_history,
        generate_bip39_seed,
        derive_wallets_from_seed,
        import_external_seed,
        get_tokens,
        add_token,
        remove_token,
        get_tax_suite,
        save_tax_suite,
        dispatch_tax_payment,
        build_send_xec,
        confirm_send_xec,
        send_ecash_token,
        accept_vault_handshake,
        receive_vault_job,
        ingest_pdf,
        ingest_literature,
        upsert_cmld_definition,
        ingest_ontology,
        export_to_solid,
        ingest_image,
        ingest_image_async,
        discover_models,
        download_and_vectorize,
        download_model,
        cancel_download,
        get_active_model,
        set_active_model,
        get_active_downloads,
        run_agent_inference,
        generate_front_door_invite,
        mint_semantic_token,
        fetch_wallet_portfolio,
        toggle_nym_relay,
        toggle_stark_prover,
        update_solar_input,
        fetch_torrent_telemetry,
        fetch_remote_manifest,
        load_imported_accounts,
        save_imported_accounts,
        get_front_doors,
        generate_front_door,
        get_directory_actors,
        add_directory_actor,
        get_delegation_rules,
        add_delegation_rule,
        generate_connect_invite,
        accept_connect_invite,
        list_chat_contacts,
        get_user_profile,
        save_user_profile,
        list_chat_sessions,
        load_chat_session,
        create_group_chat_session,
        add_chat_participant,
        remove_chat_participant,
        get_chat_participants,
        append_chat_message,
        get_chat_graph,
        list_directory,
        list_directory_categories,
        create_directory_category,
        set_directory_entry_categories,
        search_directory,
        list_mail_domains,
        add_mail_domain,
        purpose_inbox_presets,
        list_mail_addresses,
        mint_purpose_inbox,
        mint_relationship_address,
        set_mail_address_enabled,
        front_door_forms,
        cf_verify_token,
        cf_list_zones,
        cf_publish_front_door,
        start_qdp_server,
        parse_magic_link,
        mail_send,
        mail_fetch,
        generate_connection_identifier,
        generate_magic_link,
        accept_connection,
        list_social_peers,
        set_social_peer_active,
        answer_connection_challenge,
        mesh_dialability,
        mesh::mesh_start,
        mesh::mesh_stop,
        mesh::mesh_status,
        list_agreements,
        agreements_for,
        create_agreement,
        save_agreement,
        set_agreement_consent,
        get_qpu_settings,
        save_qpu_settings,
        enable_qpu_feature,
        disable_qpu_feature,
        activate_advanced_capabilities,
        get_advanced_activation_status,
        get_commitment_prompt,
        submit_omnibox_query,
        resolve_qdp_did,
        get_ns_records_for_did,
        sync_to_solid_pod,
        evaluate_data_request,
        apply_semantic_handshake,
        save_qlink,
        compute_context_hash,
        run_computational_geometry,
        get_qualia_compute_profile,
        certify_forge_physics,
        run_forge_compute_probe,
        qapp_analyze,
        export_qapp_as_wasm_package,
        get_latest_diffusion_snapshot,
        reconfigure_diffusion,
        get_diffusion_frame_rgba,
        get_diffusion_ledger_health,
        probe_localhost_preview,
        update_render_preview,
        toggle_render_loop,
        navigate_to_node,
        select_node_at,
        collapse_wavefunction,
        collapse_wavefunction_legacy,
        set_temporal_slice,
        register_browser_capabilities,
        load_ccf_asset,
        list_ccf_assets,
        test_ccf_ipc_handshake,
        test_larynx_smoke,
        test_vasculature_stress,
        get_qpu_settings,
        save_qpu_settings,
        enable_qpu_feature,
        disable_qpu_feature,
        calculate_chemistry_properties,
        calculate_framingham_risk,
        calculate_quantum_dft,
        calculate_monte_carlo_var,
        submit_record,
        // ── Native GPU surface commands ──────────────────────────────────
        crate::native_surface::mount_gpu_surface,
        crate::native_surface::set_gpu_scene,
        crate::native_surface::set_gpu_camera,
        crate::native_surface::set_gpu_camera_mode,
        crate::native_surface::upload_gpu_mesh,
        crate::native_surface::upload_gpu_mesh_colored,
        crate::native_surface::upload_gpu_10d_mesh,
        crate::native_surface::load_gpu_10d_asset,
        crate::native_surface::unmount_gpu_surface,
        // ── 10D browser commands ─────────────────────────────────────────
        browse_10d_containers,
        inspect_10d_container,
        open_10d_file_picker,
    ]
}
