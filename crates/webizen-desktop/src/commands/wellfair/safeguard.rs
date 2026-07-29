#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

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
    use qualia_client_core::dead_mans_switch::{
        DeadMansSwitch, Disposition, Heartbeat, TriggerRule,
    };
    let commitment =
        qualia_client_core::accountability_store::parse_commitment_hex(&commitment_hex)?;
    let now = wellfair_now_unix() as u64;
    let disposition = match disposition.as_str() {
        "make_public" => Disposition::MakePublic,
        _ => Disposition::ReleaseTo {
            parties: disposition_parties,
        },
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
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.arm_dead_mans_switch(switch)?;
        Ok("{\"armed\":true}".into())
    })?
}

/// Touch the heartbeat / un-fire a dead-man switch (the "I'm alive" action).
#[command]
pub fn wellfair_dead_mans_alive(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let found = host.dead_mans_alive(&commitment_hex)?;
        serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
    })?
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
    let attestation = PartyAttestation {
        party_did,
        kind,
        time_unix: wellfair_now_unix() as u64,
    };
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let found = host.attest_dead_mans(&commitment_hex, attestation)?;
        serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
    })?
}

/// Enact a dead-man switch if triggerable — returns the disposition (or null).
#[command]
pub fn wellfair_enact_dead_mans(app: AppHandle, commitment_hex: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let disposition = host.enact_dead_mans(&commitment_hex)?;
        serde_json::to_string(&serde_json::json!({ "disposition": disposition }))
            .map_err(|e| e.to_string())
    })?
}

/// List armed dead-man switches (with attestations).
#[command]
pub fn wellfair_list_dead_mans_switches(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let list = host.list_dead_mans_switches()?;
        serde_json::to_string(&list).map_err(|e| e.to_string())
    })?
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
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let result = host.enact_dead_mans_release(&commitment_hex, party_keys)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })?
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
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let result = host.split_dek_recovery(&commitment_hex, threshold, parties)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })?
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
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let result = host.reconstruct_and_release(&commitment_hex, shares, party_keys)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })?
}

/// Publish a peer's envelope (X25519) public key into their peer record (remote-key distribution).
#[command]
pub fn wellfair_set_peer_envelope_key(
    app: AppHandle,
    did: String,
    pubkey_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.set_peer_envelope_key(&did, &pubkey_hex)?;
        Ok("{\"set\":true}".into())
    })?
}

/// Enact + release resolving the disposition parties' keys from the peer store. Returns
/// `{ result, missing_keys_for }`.
#[command]
pub fn wellfair_enact_dead_mans_release_via_peers(
    app: AppHandle,
    commitment_hex: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let result = host.enact_dead_mans_release_via_peers(&commitment_hex)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })?
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
    use qualia_client_core::incapacity_switch::{
        IncapacityKind, IncapacitySwitch, IncapacityTrigger,
    };
    let kind = match kind.as_str() {
        "involuntary_psychiatric" => IncapacityKind::InvoluntaryPsychiatric,
        "serious_injury" => IncapacityKind::SeriousInjury,
        other => IncapacityKind::Other(other.to_string()),
    };
    let switch = IncapacitySwitch {
        principal_did,
        kind,
        trigger: IncapacityTrigger {
            parties,
            attestation_threshold: threshold,
            require_official_instrument,
        },
        advocate_did,
        active_since_unix: None,
    };
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.arm_incapacity_switch(switch)?;
        Ok("{\"armed\":true}".into())
    })?
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
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let activated =
            host.activate_incapacity(&principal_did, attesting_parties, official_instrument)?;
        serde_json::to_string(&serde_json::json!({ "activated": activated }))
            .map_err(|e| e.to_string())
    })?
}

/// Regain capacity — the advocate stands down (reversibility).
#[command]
pub fn wellfair_regain_capacity(app: AppHandle, principal_did: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let found = host.regain_capacity(&principal_did)?;
        serde_json::to_string(&serde_json::json!({ "found": found })).map_err(|e| e.to_string())
    })?
}

/// List armed incapacity switches.
#[command]
pub fn wellfair_list_incapacity_switches(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let list = host.list_incapacity_switches()?;
        serde_json::to_string(&list).map_err(|e| e.to_string())
    })?
}
