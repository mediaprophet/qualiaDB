//! Sync & Backup — surfaces the sync transport (T3.1) and backup/restore (T3.3).
//!
//! - **Sync** with a relay: drains your outbox to the relay and pulls + admits what peers published.
//!   The inbox is fail-closed, so a bad relay can only cause rejections, never bad data.
//! - **Backup / restore**: write a portable archive of your WellFair data (the encrypted Sanctuary
//!   vault stays encrypted inside it), or restore one.

use super::host_client::{
    export_backup, import_backup, pick_file_path, pick_save_path, sync_with_relay,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct SyncBackupUi {
    relay_url: String,
    status: String,
    busy: bool,
}

#[component]
pub fn WellfairSyncBackupPanel() -> Element {
    let mut ui = use_signal(SyncBackupUi::default);

    let do_sync = move |_| {
        let url = ui().relay_url.trim().to_string();
        if url.is_empty() {
            ui.write().status = "Enter a relay URL first (e.g. http://localhost:4242).".into();
            return;
        }
        spawn(async move {
            ui.write().busy = true;
            ui.write().status = "Syncing…".into();
            match sync_with_relay(&url, 0).await {
                Ok(s) => {
                    ui.write().status = format!(
                        "Synced: pushed {}, pulled {} (new {}, duplicate {}, rejected {}).",
                        s.pushed, s.pulled, s.validated, s.duplicate, s.rejected
                    );
                }
                Err(e) => ui.write().status = format!("Sync failed: {e}"),
            }
            ui.write().busy = false;
        });
    };

    let do_export = move |_| {
        spawn(async move {
            match pick_save_path("wellfair-backup.q42backup").await {
                Ok(Some(path)) => {
                    ui.write().busy = true;
                    ui.write().status = "Exporting backup…".into();
                    match export_backup(&path).await {
                        Ok(s) => {
                            ui.write().status =
                                format!("Backup written: {} files, {} bytes.", s.files, s.bytes)
                        }
                        Err(e) => ui.write().status = format!("Backup failed: {e}"),
                    }
                    ui.write().busy = false;
                }
                Ok(None) => {}
                Err(e) => ui.write().status = format!("Couldn't open the save dialog: {e}"),
            }
        });
    };

    let do_import = move |_| {
        spawn(async move {
            match pick_file_path().await {
                Ok(Some(path)) => {
                    ui.write().busy = true;
                    ui.write().status = "Restoring backup…".into();
                    match import_backup(&path).await {
                        Ok(s) => {
                            ui.write().status = format!(
                                "Restored {} files ({} bytes). Reopen the app to see restored data.",
                                s.files, s.bytes
                            )
                        }
                        Err(e) => ui.write().status = format!("Restore failed: {e}"),
                    }
                    ui.write().busy = false;
                }
                Ok(None) => {}
                Err(e) => ui.write().status = format!("Couldn't open the file dialog: {e}"),
            }
        });
    };

    let btn = "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;";

    rsx! {
        section {
            aria_label: "Sync and backup",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Sync & backup" }
            if !ui().status.is_empty() {
                p { style: "margin:0 0 0.6rem;font-size:0.76rem;", "{ui().status}" }
            }

            // Sync.
            h3 { style: "margin:0.4rem 0 0.3rem;font-size:0.88rem;", "Sync with a relay" }
            p {
                style: "margin:0 0 0.4rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Send your queued changes to a relay and receive what others shared. Everything received is checked before anything is accepted."
            }
            div {
                style: "display:flex;gap:0.5rem;align-items:flex-end;flex-wrap:wrap;",
                label {
                    style: "flex:1;min-width:14rem;display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                    "Relay URL"
                    input {
                        value: "{ui().relay_url}",
                        oninput: move |e| ui.write().relay_url = e.value(),
                        placeholder: "http://localhost:4242",
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    disabled: ui().busy,
                    onclick: do_sync,
                    "Sync now"
                }
            }

            // Backup.
            h3 { style: "margin:0.85rem 0 0.3rem;font-size:0.88rem;", "Backup & restore" }
            p {
                style: "margin:0 0 0.4rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Save a portable copy of everything, or restore one. Your Sanctuary vault stays encrypted inside the backup."
            }
            div {
                style: "display:flex;gap:0.5rem;flex-wrap:wrap;",
                button { style: "{btn}", disabled: ui().busy, onclick: do_export, "Export backup…" }
                button { style: "{btn}", disabled: ui().busy, onclick: do_import, "Restore backup…" }
            }
            p {
                style: "margin:0.5rem 0 0;font-size:0.7rem;color:var(--qualia-text-muted,#888);",
                "Restoring overwrites this device's data with the backup's contents."
            }
        }
    }
}
