use crate::cli::AppAction;
use qualia_client_core::qapp_install::{
    install_package_atomic, load_install_registry, reconcile_registry_with_disk, uninstall_package,
    InstallPolicy,
};
use qualia_client_core::qapp_paths::{qapps_dir, resolve_active_package_dir};
use qualia_client_core::qapp_registry::{QappPackageManifest, QAPP_PACKAGE_MANIFEST};
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_storage_path(custom: Option<PathBuf>) -> PathBuf {
    custom.unwrap_or_else(|| {
        PathBuf::from(qualia_client_core::state::dirs_default_path())
    })
}

pub fn handle(action: &AppAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        AppAction::List { storage } => {
            let storage = resolve_storage_path(storage.clone());
            let registry = load_install_registry(&storage)
                .map_err(|e| format!("Failed to read QApp registry from {}: {e}", storage.display()))?;
            let qdir = qapps_dir(&storage);

            if registry.packages.is_empty() {
                println!("No QApps currently installed in {}", qdir.display());
                return Ok(());
            }

            println!("Installed QApps in {}:", qdir.display());
            println!("{:<32} {:<12} {:<24} {:<8}", "PACKAGE ID", "VERSION", "INSTALLED AT", "REVOKED");
            println!("{:-<78}", "");

            for (id, entry) in &registry.packages {
                let date_str = if entry.installed_at_unix > 0 {
                    chrono::DateTime::from_timestamp(entry.installed_at_unix as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "unknown".into())
                } else {
                    "bundled/legacy".into()
                };

                let revoked_str = if entry.revoked { "YES" } else { "no" };
                println!("{:<32} {:<12} {:<24} {:<8}", id, entry.active_version, date_str, revoked_str);
            }
            Ok(())
        }
        AppAction::Install { source, storage, production } => {
            let storage = resolve_storage_path(storage.clone());
            let policy = if *production {
                InstallPolicy::Production
            } else {
                InstallPolicy::Development
            };

            println!(
                "Installing QApp from {} (policy: {:?})...",
                source.display(),
                policy
            );

            let entry = install_package_atomic(&storage, source, policy, None)
                .map_err(|e| format!("Installation failed: {e}"))?;

            println!(
                "Successfully installed '{}' v{} (content hash: {})",
                entry.package_id, entry.active_version, entry.content_hash
            );
            println!("Active directory: {}", resolve_active_package_dir(&storage, &entry.package_id).display());
            Ok(())
        }
        AppAction::Uninstall { package_id, storage, purge_data } => {
            let storage = resolve_storage_path(storage.clone());
            println!("Uninstalling QApp '{}'...", package_id);

            uninstall_package(&storage, package_id, *purge_data)
                .map_err(|e| format!("Uninstall failed: {e}"))?;

            println!("Successfully uninstalled '{}'.", package_id);
            if *purge_data {
                println!("Application data directory purged.");
            }
            Ok(())
        }
        AppAction::Info { target, storage } => {
            let storage = resolve_storage_path(storage.clone());
            let manifest_path = if Path::new(target).join(QAPP_PACKAGE_MANIFEST).is_file() {
                Path::new(target).join(QAPP_PACKAGE_MANIFEST)
            } else {
                let active = resolve_active_package_dir(&storage, target);
                active.join(QAPP_PACKAGE_MANIFEST)
            };

            if !manifest_path.is_file() {
                return Err(format!(
                    "QApp manifest not found for '{}' (checked: {})",
                    target,
                    manifest_path.display()
                )
                .into());
            }

            let content = fs::read_to_string(&manifest_path)?;
            let manifest: QappPackageManifest = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse {}: {e}", manifest_path.display()))?;

            println!("QApp Information: {}", manifest.name);
            println!("  Version: {}", manifest.version);
            if let Some(port) = manifest.dev_port {
                println!("  Dev Port: {}", port);
            }
            if !manifest.required_shapes.is_empty() {
                println!("  Required Shapes: {}", manifest.required_shapes.join(", "));
            }
            if let Some(ext) = manifest.x_qualia {
                println!("  App ID: {}", ext.app_id);
                println!("  Display Name: {}", ext.display_name);
                println!("  Category: {}", if ext.category.is_empty() { "standard" } else { &ext.category });
                if !ext.launch_modes.is_empty() {
                    println!("  Launch Modes: {}", ext.launch_modes.join(", "));
                }
                if !ext.entrypoints.is_empty() {
                    println!("  Entrypoints:");
                    for (k, v) in ext.entrypoints {
                        println!("    - {}: {}", k, v);
                    }
                }
                if let Some(wasm) = ext.wasm {
                    println!("  WASM Engine: {}", wasm.engine_package);
                }
            }
            Ok(())
        }
        AppAction::Reconcile { storage } => {
            let storage = resolve_storage_path(storage.clone());
            println!("Reconciling registry with disk at {}...", storage.display());
            let registry = reconcile_registry_with_disk(&storage)
                .map_err(|e| format!("Reconciliation failed: {e}"))?;
            println!(
                "Reconciled {} registered package(s).",
                registry.packages.len()
            );
            Ok(())
        }
    }
}
