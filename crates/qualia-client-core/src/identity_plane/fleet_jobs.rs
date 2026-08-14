//! Multi-apparatus job delivery: sign, POST, accept, outbox.
//!
//! Local jobs stay on the local queue. Jobs aimed at a registered remote
//! apparatus are signed by the person principal and delivered to that device's
//! `control_base_url` (`POST /api/fleet/jobs`). Failures land in a durable
//! outbox for retry. Unknown targets still fail closed.

use super::fleet::{ensure_local_apparatus, resolve_job_placement, JobPlacement};
use super::person::PersonPrincipal;
use crate::local_job_scheduler::{LocalJob, LocalJobKind, LocalJobScheduler};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const FLEET_JOB_FORMAT: &str = "qualia.fleet.job.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetJobEnvelope {
    pub format: String,
    pub person_id: String,
    pub person_verifying_key_hex: String,
    pub source_device_id: String,
    pub target_device_id: String,
    pub kind: LocalJobKind,
    pub created_at_unix: u64,
    /// Hex-encoded Ed25519 signature over [`signing_payload`].
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteOutboxEntry {
    pub id: String,
    pub envelope: FleetJobEnvelope,
    pub target_url: String,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub last_attempt_unix: u64,
    pub created_at_unix: u64,
    pub delivered: bool,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn outbox_path() -> PathBuf {
    crate::state::app_meta_dir().join("remote_job_outbox.json")
}

fn load_outbox() -> Vec<RemoteOutboxEntry> {
    let path = outbox_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_outbox(entries: &[RemoteOutboxEntry]) -> Result<(), String> {
    let path = outbox_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Canonical bytes signed by the person principal.
pub fn signing_payload(
    person_id: &str,
    source_device_id: &str,
    target_device_id: &str,
    kind: &LocalJobKind,
    created_at_unix: u64,
) -> Result<Vec<u8>, String> {
    let kind_json = serde_json::to_string(kind).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    h.update(FLEET_JOB_FORMAT.as_bytes());
    h.update(b"|");
    h.update(person_id.as_bytes());
    h.update(b"|");
    h.update(source_device_id.as_bytes());
    h.update(b"|");
    h.update(target_device_id.as_bytes());
    h.update(b"|");
    h.update(created_at_unix.to_le_bytes());
    h.update(b"|");
    h.update(kind_json.as_bytes());
    Ok(h.finalize().to_vec())
}

impl FleetJobEnvelope {
    pub fn build(
        person: &PersonPrincipal,
        source_device_id: &str,
        target_device_id: &str,
        kind: LocalJobKind,
    ) -> Result<Self, String> {
        let created_at_unix = now_unix();
        let payload = signing_payload(
            &person.person_id,
            source_device_id,
            target_device_id,
            &kind,
            created_at_unix,
        )?;
        let sig = person.sign_message(&payload);
        Ok(Self {
            format: FLEET_JOB_FORMAT.to_string(),
            person_id: person.person_id.clone(),
            person_verifying_key_hex: person.verifying_key_hex(),
            source_device_id: source_device_id.to_string(),
            target_device_id: target_device_id.to_string(),
            kind,
            created_at_unix,
            signature_hex: hex::encode(sig),
        })
    }

    pub fn verify(&self) -> Result<(), String> {
        if self.format != FLEET_JOB_FORMAT {
            return Err(format!("unsupported fleet job format: {}", self.format));
        }
        let payload = signing_payload(
            &self.person_id,
            &self.source_device_id,
            &self.target_device_id,
            &self.kind,
            self.created_at_unix,
        )?;
        let sig_bytes = hex::decode(&self.signature_hex).map_err(|e| e.to_string())?;
        if sig_bytes.len() != 64 {
            return Err("signature must be 64 bytes".into());
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&sig_bytes);
        PersonPrincipal::verify_message(
            &self.person_id,
            &self.person_verifying_key_hex,
            &payload,
            &sig,
        )
    }
}

/// Deliver a job to a remote apparatus, or queue to outbox on failure.
pub fn deliver_or_queue_remote_job(
    kind: LocalJobKind,
    target_device_id: &str,
) -> Result<RemoteOutboxEntry, String> {
    let plane = ensure_local_apparatus(None)?;
    let placement = resolve_job_placement(Some(target_device_id))?;
    let (device_id, label) = match placement {
        JobPlacement::RemoteRegistered { device_id, label } => (device_id, label),
        JobPlacement::Local { .. } => {
            return Err("target is the local apparatus — use the local job queue".into());
        }
        JobPlacement::Unknown { device_id } => {
            return Err(format!("unknown device {device_id}"));
        }
    };
    let peer = plane
        .devices
        .iter()
        .find(|d| d.device_id == device_id)
        .ok_or("peer missing from fleet")?;
    if peer.control_base_url.trim().is_empty() {
        return Err(format!(
            "Peer '{label}' has no control_base_url. On that machine, set a LAN URL (Settings → Person & devices) so jobs can be delivered."
        ));
    }
    let person = PersonPrincipal::load_or_create(None)?;
    let envelope = FleetJobEnvelope::build(&person, &plane.local_device_id, &device_id, kind)?;
    let url = format!(
        "{}/api/fleet/jobs",
        peer.control_base_url.trim_end_matches('/')
    );
    let mut entry = RemoteOutboxEntry {
        id: uuid::Uuid::new_v4().to_string(),
        envelope: envelope.clone(),
        target_url: url.clone(),
        attempts: 0,
        last_error: None,
        last_attempt_unix: 0,
        created_at_unix: now_unix(),
        delivered: false,
    };

    match try_deliver(&url, &envelope) {
        Ok(()) => {
            entry.delivered = true;
            entry.attempts = 1;
            entry.last_attempt_unix = now_unix();
        }
        Err(e) => {
            entry.attempts = 1;
            entry.last_error = Some(e);
            entry.last_attempt_unix = now_unix();
            let mut box_ = load_outbox();
            box_.push(entry.clone());
            // Cap outbox
            if box_.len() > 64 {
                box_.retain(|e| !e.delivered);
                if box_.len() > 64 {
                    box_.drain(0..box_.len() - 64);
                }
            }
            save_outbox(&box_)?;
        }
    }
    if entry.delivered {
        // Keep a short delivered audit trail
        let mut box_ = load_outbox();
        box_.push(entry.clone());
        if box_.len() > 64 {
            box_.drain(0..box_.len() - 64);
        }
        let _ = save_outbox(&box_);
    }
    Ok(entry)
}

fn try_deliver(url: &str, envelope: &FleetJobEnvelope) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(url)
        .json(envelope)
        .send()
        .map_err(|e| format!("fleet deliver: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("fleet deliver HTTP {status}: {body}"));
    }
    Ok(())
}

/// Accept a signed fleet job on this apparatus (HTTP handler body).
pub fn accept_fleet_job_envelope(envelope: FleetJobEnvelope) -> Result<LocalJob, String> {
    envelope.verify()?;
    let plane = ensure_local_apparatus(None)?;
    if envelope.target_device_id != plane.local_device_id {
        return Err(format!(
            "job targets {} but this apparatus is {}",
            envelope.target_device_id, plane.local_device_id
        ));
    }
    // Same person principal required (imported on both machines).
    if envelope.person_id != plane.person.person_id {
        return Err(
            "fleet job person_id does not match this install's person principal — import the same person transfer bundle on both machines"
                .into(),
        );
    }
    // Enqueue as local work; placement already verified as this apparatus.
    let mut job = LocalJobScheduler::global()
        .enqueue_for_device(envelope.kind, Some(plane.local_device_id.clone()))?;
    job.originating_device_id = Some(envelope.source_device_id.clone());
    job.person_id = Some(envelope.person_id.clone());
    job.message = format!("Accepted from fleet peer {}", envelope.source_device_id);
    LocalJobScheduler::global().update_job_meta(&job)?;
    Ok(job)
}

pub fn list_remote_outbox() -> Result<Vec<RemoteOutboxEntry>, String> {
    Ok(load_outbox())
}

/// Retry undelivered outbox entries (best-effort).
pub fn retry_remote_outbox() -> Result<usize, String> {
    let mut box_ = load_outbox();
    let mut delivered = 0usize;
    for entry in box_.iter_mut() {
        if entry.delivered {
            continue;
        }
        entry.attempts = entry.attempts.saturating_add(1);
        entry.last_attempt_unix = now_unix();
        match try_deliver(&entry.target_url, &entry.envelope) {
            Ok(()) => {
                entry.delivered = true;
                entry.last_error = None;
                delivered += 1;
            }
            Err(e) => entry.last_error = Some(e),
        }
    }
    save_outbox(&box_)?;
    Ok(delivered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_verifies() {
        let person = PersonPrincipal::generate("t").unwrap();
        let kind = LocalJobKind::DaemonGraphReload;
        let env = FleetJobEnvelope::build(&person, "did:q42:device:aa", "did:q42:device:bb", kind)
            .unwrap();
        env.verify().unwrap();
        let mut bad = env.clone();
        bad.signature_hex = "00".repeat(64);
        assert!(bad.verify().is_err());
    }
}
