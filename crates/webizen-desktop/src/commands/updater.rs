//! Updater commands

#![allow(non_snake_case)]

#[derive(Clone, serde::Serialize)]
struct UpdaterProgressPayload {
    downloaded: u64,
    total: u64,
}

#[tauri::command]
pub async fn updater_check(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("Failed to get updater: {}", e))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(update.version.clone())),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Update check error: {}", e)),
    }
}

#[tauri::command]
pub async fn updater_download_and_install(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("Failed to get updater: {}", e))?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    let app_clone = app.clone();
    let mut downloaded = 0;
    update
        .download_and_install(
            move |chunk_length, content_length| {
                downloaded += chunk_length as u64;
                let _ = app_clone.emit(
                    "updater-progress",
                    UpdaterProgressPayload {
                        downloaded,
                        total: content_length.unwrap_or(0),
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn updater_restart(app: tauri::AppHandle) -> Result<(), String> {
    app.restart();
}
