//! Local mail product (inbox + SMTP receiver)

#![allow(non_snake_case)]

/// Accept a message into the local inbox (same path as SMTP DATA) — for tests and mesh inject.
pub fn mail_accept(
    from: String,
    to: String,
    subject: String,
    body: String,
    sender_verified: bool,
) -> Result<serde_json::Value, String> {
    let r = crate::mail_inbound::accept_message(&from, &to, &subject, &body, sender_verified, None);
    serde_json::to_value(r).map_err(|e| e.to_string())
}

/// List local inbox messages (newest first).
pub fn mail_list(
    mailbox: Option<String>,
    include_quarantine: Option<bool>,
) -> Result<serde_json::Value, String> {
    let inc = include_quarantine.unwrap_or(true);
    let list = crate::mail_store::list(mailbox.as_deref(), inc);
    let (total, unread, quarantine) = crate::mail_store::counts();
    Ok(serde_json::json!({
        "messages": list,
        "counts": { "total": total, "unread": unread, "quarantine": quarantine },
    }))
}

pub fn mail_get(id: String) -> Result<serde_json::Value, String> {
    let m = crate::mail_store::get(&id).ok_or_else(|| format!("unknown message '{id}'"))?;
    serde_json::to_value(m).map_err(|e| e.to_string())
}

pub fn mail_set_read(id: String, read: bool) -> Result<serde_json::Value, String> {
    let m = crate::mail_store::set_read(&id, read)?;
    serde_json::to_value(m).map_err(|e| e.to_string())
}

pub fn mail_delete(id: String) -> Result<serde_json::Value, String> {
    crate::mail_store::delete(&id)?;
    Ok(serde_json::json!({ "deleted": id }))
}

/// MX/SPF paste block + local receiver status for a domain.
pub fn mail_dns_forms(
    domain: String,
    mx_host: Option<String>,
) -> Result<serde_json::Value, String> {
    Ok(crate::mail_inbound::mail_dns_forms(
        &domain,
        mx_host.as_deref(),
    ))
}

pub fn mail_receiver_status() -> Result<serde_json::Value, String> {
    Ok(crate::mail_inbound::receiver_status())
}

/// Start local SMTP receiver (default `127.0.0.1:2525`). Use `0.0.0.0:2525` for LAN/tunnel.
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_receiver_start(bind: Option<String>) -> Result<serde_json::Value, String> {
    let b = bind.unwrap_or_default();
    crate::mail_inbound::start_receiver(&b)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn mail_receiver_stop() -> Result<serde_json::Value, String> {
    crate::mail_inbound::stop_receiver()
}
