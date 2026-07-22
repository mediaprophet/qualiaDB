//! Qualia protocol + desktop updater

#![allow(non_snake_case)]


use std::path::Path;


pub fn seed_bundled_qapps() -> Result<Vec<String>, String> {
    crate::bundled_qapps::seed_bundled_qapps()
}

pub fn seed_bundled_ontologies() -> Result<Vec<String>, String> {
    crate::bundled_ontologies::seed_bundled_ontologies()
}

pub fn installed_qapp_version(qapp_name: &str) -> Result<Option<String>, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    Ok(crate::bundled_qapps::installed_qapp_version(
        Path::new(&storage),
        qapp_name,
    ))
}

pub fn check_qapp_update(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    let status = crate::bundled_qapps::check_bundled_qapp_update(&qapp_name, Path::new(&storage));
    serde_json::to_string(&status).map_err(|e| e.to_string())
}

pub fn check_qapp_update_from_path(
    qapp_name: String,
    source_path: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    let status = crate::bundled_qapps::check_qapp_update_from_source(
        &qapp_name,
        Path::new(&storage),
        Path::new(&source_path),
    )?;
    serde_json::to_string(&status).map_err(|e| e.to_string())
}

pub fn list_qapp_update_offers() -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    let offers = crate::bundled_qapps::list_bundled_qapp_updates(Path::new(&storage));
    serde_json::to_string(&offers).map_err(|e| e.to_string())
}

pub fn apply_qapp_update(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    crate::bundled_qapps::apply_bundled_qapp_update(Path::new(&storage), &qapp_name)
}

pub fn apply_qapp_update_from_path(
    qapp_name: String,
    source_path: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .storage_path
        .clone();
    crate::bundled_qapps::upgrade_qapp_from_source(
        Path::new(&storage),
        &qapp_name,
        Path::new(&source_path),
    )
}

pub fn start_qualia_protocol() -> Result<u16, String> {
    crate::qapps_protocol::start_qualia_protocol()
}

pub fn qualia_protocol_port() -> u16 {
    crate::qapps_protocol::qualia_protocol_port()
}

pub async fn download_and_install_update(url: String) -> Result<(), String> {
    crate::update_installer::download_and_install_update(url).await
}

pub fn register_qualia_uri_handler(exe_path: String) -> Result<(), String> {
    crate::qapps_protocol::register_qualia_uri_handler(&exe_path)
}

