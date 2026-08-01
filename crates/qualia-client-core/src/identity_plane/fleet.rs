//! Local fleet registry: one person, many apparatus installs.

use super::device::{DeviceRecord, DeviceRecordPublic};
use super::person::{PersonPrincipal, PersonPublic, PersonTransferBundle};
use crate::node_identity::NodeIdentity;
use crate::setup::DeviceContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const FLEET_FORMAT: &str = "qualia.device_fleet.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceFleetFile {
    format: String,
    person_id: String,
    local_device_id: String,
    devices: Vec<DeviceRecord>,
}

/// UI/API snapshot: person public half + devices (no secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPlaneSnapshot {
    pub person: PersonPublic,
    pub local_device_id: String,
    pub devices: Vec<DeviceRecordPublic>,
    /// Explicit: OS login is not used as a Qualia principal.
    pub os_account_is_not_principal: bool,
    pub notes: Vec<String>,
}

/// Where a job should run relative to this process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "placement")]
pub enum JobPlacement {
    /// Run on this process / local job queue.
    Local {
        device_id: String,
    },
    /// Target is a registered peer device — dispatch not yet live (fail-closed).
    RemoteRegistered {
        device_id: String,
        label: String,
    },
    /// Unknown device id.
    Unknown {
        device_id: String,
    },
}

fn fleet_path() -> PathBuf {
    crate::state::app_meta_dir().join("device_fleet.json")
}

fn load_fleet() -> Result<Option<DeviceFleetFile>, String> {
    let path = fleet_path();
    if !path.exists() {
        return Ok(None);
    }
    let bytes =
        std::fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let fleet: DeviceFleetFile = serde_json::from_slice(&bytes)
        .map_err(|e| format!("failed to parse device fleet: {e}"))?;
    Ok(Some(fleet))
}

fn save_fleet(fleet: &DeviceFleetFile) -> Result<(), String> {
    let path = fleet_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(fleet)
        .map_err(|e| format!("failed to encode device fleet: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(())
}

/// Ensure person + local apparatus + fleet entry exist.
///
/// Safe to call at startup and after setup. Does not use OS username as identity.
pub fn ensure_local_apparatus(device_context: Option<DeviceContext>) -> Result<IdentityPlaneSnapshot, String> {
    let mut person = PersonPrincipal::load_or_create(None)?;
    if let Ok(setup) = crate::setup::get_setup_state() {
        if !setup.profile.preferred_name.trim().is_empty()
            && person.display_hint.trim().is_empty()
        {
            person.display_hint = setup.profile.preferred_name.clone();
            person.persist()?;
        }
    }

    let node = NodeIdentity::load_or_create()?;
    let pubkey = node.identity_pubkey_hex();
    let ctx = device_context.unwrap_or_else(|| {
        crate::setup::get_setup_state()
            .map(|s| s.profile.device_context)
            .unwrap_or_default()
    });

    let label = if !ctx.notes.trim().is_empty() {
        ctx.notes.chars().take(48).collect::<String>()
    } else {
        let host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "this machine".into());
        format!("Webizen on {host}")
    };

    let mut local = DeviceRecord::new_local(&person.person_id, pubkey, ctx, label);
    // Prefer explicit LAN URL already stored; else stamp loopback control port when known.
    if local.control_base_url.is_empty() {
        if let Some(port) = crate::state::APP_STATE
            .get()
            .and_then(|s| s.config.lock().ok())
            .map(|c| c.settings_port)
        {
            if port > 0 {
                local.control_base_url = format!("http://127.0.0.1:{port}");
            }
        }
    }
    local.touch();

    let fleet = match load_fleet()? {
        Some(mut f) => {
            f.person_id = person.person_id.clone();
            f.local_device_id = local.device_id.clone();
            // Upsert local record
            if let Some(slot) = f.devices.iter_mut().find(|d| d.device_id == local.device_id) {
                slot.device_context = local.device_context.clone();
                slot.label = local.label.clone();
                slot.hostname = local.hostname.clone();
                slot.is_local = true;
                slot.person_id = person.person_id.clone();
                slot.identity_pubkey_hex = local.identity_pubkey_hex.clone();
                slot.capabilities = local.capabilities.clone();
                if !local.control_base_url.is_empty() {
                    // Preserve a user-set non-loopback URL if present.
                    let existing = slot.control_base_url.clone();
                    if existing.is_empty()
                        || existing.contains("127.0.0.1")
                        || existing.contains("localhost")
                    {
                        slot.control_base_url = local.control_base_url.clone();
                    }
                }
                slot.touch();
            } else {
                // Mark any previous local flags false (reinstall edge case)
                for d in f.devices.iter_mut() {
                    d.is_local = false;
                }
                f.devices.push(local.clone());
            }
            // Keep person_id consistent on all devices we own in this fleet file
            for d in f.devices.iter_mut() {
                if d.is_local {
                    d.person_id = person.person_id.clone();
                }
            }
            f.format = FLEET_FORMAT.to_string();
            f
        }
        None => DeviceFleetFile {
            format: FLEET_FORMAT.to_string(),
            person_id: person.person_id.clone(),
            local_device_id: local.device_id.clone(),
            devices: vec![local],
        },
    };

    save_fleet(&fleet)?;
    Ok(snapshot_from(person.to_public(), &fleet))
}

fn snapshot_from(person: PersonPublic, fleet: &DeviceFleetFile) -> IdentityPlaneSnapshot {
    IdentityPlaneSnapshot {
        person,
        local_device_id: fleet.local_device_id.clone(),
        devices: fleet.devices.clone(),
        os_account_is_not_principal: true,
        notes: vec![
            "The person principal is not the machine and not the OS login.".into(),
            "Each Qualia install is a separate apparatus (device) under the person.".into(),
            "Import a person transfer bundle on another machine to share the same person principal.".into(),
            "Jobs may name a target device_id; only the local apparatus runs work in this build.".into(),
        ],
    }
}

pub fn get_identity_plane() -> Result<IdentityPlaneSnapshot, String> {
    ensure_local_apparatus(None)
}

pub fn list_devices() -> Result<Vec<DeviceRecordPublic>, String> {
    Ok(get_identity_plane()?.devices)
}

pub fn sync_local_device_context(device_context: &DeviceContext) -> Result<IdentityPlaneSnapshot, String> {
    ensure_local_apparatus(Some(device_context.clone()))
}

pub fn export_person_public() -> Result<PersonPublic, String> {
    Ok(PersonPrincipal::load_or_create(None)?.to_public())
}

/// Full person secret for multi-machine install. Caller must treat as recovery material.
pub fn export_person_transfer_bundle() -> Result<PersonTransferBundle, String> {
    Ok(PersonPrincipal::load_or_create(None)?.transfer_bundle())
}

/// Import person principal onto this machine, then re-bind local apparatus under them.
pub fn import_person_transfer_bundle(bundle: PersonTransferBundle) -> Result<IdentityPlaneSnapshot, String> {
    PersonPrincipal::from_transfer_bundle(bundle)?;
    // Rebuild fleet local entry under the imported person.
    ensure_local_apparatus(None)
}

/// Register another apparatus (peer / second PC) for fleet awareness and job targeting.
pub fn register_remote_device(mut device: DeviceRecordPublic) -> Result<IdentityPlaneSnapshot, String> {
    if !device.device_id.starts_with("did:q42:device:") {
        return Err("device_id must be did:q42:device:…".into());
    }
    let plane = ensure_local_apparatus(None)?;
    if device.device_id == plane.local_device_id {
        return Err("cannot register the local apparatus as a remote device".into());
    }
    device.is_local = false;
    if device.person_id.trim().is_empty() {
        device.person_id = plane.person.person_id.clone();
    }
    device.touch();

    let mut fleet = load_fleet()?.ok_or("device fleet missing after ensure")?;
    if let Some(slot) = fleet
        .devices
        .iter_mut()
        .find(|d| d.device_id == device.device_id)
    {
        *slot = device;
    } else {
        fleet.devices.push(device);
    }
    save_fleet(&fleet)?;
    Ok(snapshot_from(plane.person, &fleet))
}

/// Update this apparatus' advertised control URL (LAN/VPN base for fleet jobs).
pub fn set_local_control_base_url(url: impl Into<String>) -> Result<IdentityPlaneSnapshot, String> {
    let mut plane = ensure_local_apparatus(None)?;
    let url = url.into().trim().trim_end_matches('/').to_string();
    let mut fleet = load_fleet()?.ok_or("device fleet missing")?;
    if let Some(slot) = fleet
        .devices
        .iter_mut()
        .find(|d| d.device_id == fleet.local_device_id)
    {
        slot.control_base_url = url;
        slot.touch();
    }
    save_fleet(&fleet)?;
    plane = snapshot_from(plane.person, &fleet);
    Ok(plane)
}

pub fn resolve_job_placement(target_device_id: Option<&str>) -> Result<JobPlacement, String> {
    let plane = ensure_local_apparatus(None)?;
    let Some(target) = target_device_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(JobPlacement::Local {
            device_id: plane.local_device_id,
        });
    };
    if target == plane.local_device_id {
        return Ok(JobPlacement::Local {
            device_id: plane.local_device_id,
        });
    }
    if let Some(dev) = plane.devices.iter().find(|d| d.device_id == target) {
        if dev.is_local {
            return Ok(JobPlacement::Local {
                device_id: plane.local_device_id,
            });
        }
        return Ok(JobPlacement::RemoteRegistered {
            device_id: dev.device_id.clone(),
            label: dev.label.clone(),
        });
    }
    Ok(JobPlacement::Unknown {
        device_id: target.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::DeviceContext;
    use std::sync::Mutex;

    // app_meta_dir is process-global via env; serialise tests that mutate it.
    static META_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn placement_defaults_to_local_shape() {
        let p = JobPlacement::Local {
            device_id: "did:q42:device:aa".into(),
        };
        assert!(matches!(p, JobPlacement::Local { .. }));
        let r = JobPlacement::RemoteRegistered {
            device_id: "did:q42:device:bb".into(),
            label: "laptop".into(),
        };
        assert!(matches!(r, JobPlacement::RemoteRegistered { .. }));
        let _ = DeviceContext::default();
    }

    #[test]
    fn ensure_local_separates_person_from_device_and_os() {
        let _g = META_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("QUALIA_APP_META_DIR", dir.path());
        let plane = ensure_local_apparatus(Some(DeviceContext {
            ownership: "owned_by_me".into(),
            machine_fleet: "one_of_several".into(),
            user_scope: "just_me".into(),
            ..DeviceContext::default()
        }))
        .expect("apparatus");
        assert!(plane.person.person_id.starts_with("did:q42:person:"));
        assert!(plane.local_device_id.starts_with("did:q42:device:"));
        assert_ne!(plane.person.person_id, plane.local_device_id);
        assert!(plane.os_account_is_not_principal);
        let os = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_default();
        if !os.is_empty() {
            assert!(!plane.person.person_id.contains(&os));
        }
        assert_eq!(plane.devices.len(), 1);
        assert!(plane.devices[0].is_local);
        assert_eq!(
            plane.devices[0].device_context.ownership,
            "owned_by_me"
        );

        // Second ensure is stable.
        let again = ensure_local_apparatus(None).unwrap();
        assert_eq!(again.person.person_id, plane.person.person_id);
        assert_eq!(again.local_device_id, plane.local_device_id);

        // Remote peer under same person.
        let mut peer = plane.devices[0].clone();
        peer.device_id = "did:q42:device:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into();
        peer.identity_pubkey_hex =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into();
        peer.is_local = false;
        peer.label = "laptop".into();
        let with_peer = register_remote_device(peer).unwrap();
        assert_eq!(with_peer.devices.len(), 2);
        let place = resolve_job_placement(Some(
            "did:q42:device:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        ))
        .unwrap();
        assert!(matches!(place, JobPlacement::RemoteRegistered { .. }));
        std::env::remove_var("QUALIA_APP_META_DIR");
    }
}
