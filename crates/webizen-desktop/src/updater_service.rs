use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::{sleep, Duration};
use crate::supervisor::DesktopSupervisor;

pub async fn start_updater_service(app: AppHandle) {
    let supervisor = match app.try_state::<DesktopSupervisor>() {
        Some(s) => s.inner().clone(),
        None => return,
    };

    supervisor.service_starting("updater", "Waiting for initial startup before checking updates...");
    
    // Delay check after first paint
    sleep(Duration::from_secs(10)).await;
    
    supervisor.service_ready("updater", "Checking for updates");

    if let Err(e) = check_for_updates(&app, &supervisor).await {
        supervisor.service_failed("updater", format!("Update check failed: {}", e));
    }
}

async fn check_for_updates(app: &AppHandle, supervisor: &DesktopSupervisor) -> Result<(), String> {
    let updater = app.updater().map_err(|e| format!("Failed to get updater: {}", e))?;
    
    let update_op = supervisor.start_operation("updater", "Checking for updates");
    update_op.stage("checking");
    
    match updater.check().await {
        Ok(Some(update)) => {
            update_op.complete();
            supervisor.service_ready("updater", format!("Update available: {}. Go to Settings to install.", update.version));
            Ok(())
        }
        Ok(None) => {
            update_op.complete();
            supervisor.service_ready("updater", "Up to date");
            Ok(())
        }
        Err(e) => {
            update_op.fail(format!("Update check error: {}", e));
            Err(e.to_string())
        }
    }
}
