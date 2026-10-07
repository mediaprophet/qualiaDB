//! Local mail product (inbox + SMTP receiver)

#![allow(non_snake_case)]

/// List all locally configured interoperable mail accounts. Account labels and
/// addresses provide provenance when their mail is harmonised into one inbox.
pub fn mail_accounts() -> Result<serde_json::Value, String> {
    let accounts: Vec<serde_json::Value> = crate::mail_accounts::list()
        .into_iter()
        .map(|account| {
            serde_json::json!({
                "id": account.id,
                "label": account.label,
                "address": account.address,
                "smtp_configured": account.smtp.is_some(),
                "imap_configured": account.imap.is_some(),
                "pop3_configured": account.pop3.is_some(),
            })
        })
        .collect();
    Ok(serde_json::Value::Array(accounts))
}

/// Add or replace one account. This is intentionally local configuration; it
/// makes no network request and does not fetch mail until the user asks to.
pub fn save_mail_account(
    account: crate::mail_accounts::MailAccount,
) -> Result<serde_json::Value, String> {
    crate::mail_accounts::save(account)?;
    mail_accounts()
}

/// Send through one specifically selected, locally configured SMTP account.
/// Credentials never leave the account store or appear in the response.
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_send_account(
    account_id: String,
    to: String,
    subject: String,
    body: String,
) -> Result<serde_json::Value, String> {
    let account = crate::mail_accounts::find(&account_id)
        .ok_or_else(|| "The configured mail account was not found.".to_string())?;
    let smtp = account
        .smtp
        .as_ref()
        .ok_or_else(|| "This account has no SMTP submission settings.".to_string())?;
    let recipient = to.trim();
    if recipient.is_empty() || subject.trim().is_empty() || body.trim().is_empty() {
        return Err("Recipient, subject, and message are required.".to_string());
    }
    let message = crate::mail_transport::OutgoingMail {
        from: account.address.clone(),
        to: recipient.to_string(),
        subject: subject.trim().to_string(),
        body: body.trim().to_string(),
    };
    crate::mail_transport::send(smtp, &message)?;
    Ok(serde_json::json!({
        "account_id": account.id,
        "account_label": account.label,
        "from": account.address,
        "to": message.to,
        "subject": message.subject,
        "sent": true,
    }))
}

/// Fetch one configured account into the governed local inbox. The account
/// label is recorded on every accepted import so a unified inbox never loses
/// where a message came from. SMTP-only accounts have nothing to retrieve.
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_fetch_account(id: String) -> Result<serde_json::Value, String> {
    let account = crate::mail_accounts::find(&id)
        .ok_or_else(|| "The configured mail account was not found.".to_string())?;
    let mut imported = 0usize;
    let mut rejected = 0usize;
    if let Some(imap) = account.imap.as_ref() {
        for message in crate::mail_transport::fetch_unseen(imap, "INBOX")? {
            let to = if message.to_address.contains('@') {
                message.to_address.clone()
            } else {
                account.address.clone()
            };
            let result = crate::mail_inbound::accept_message(
                &message.from_address,
                &to,
                &message.subject,
                &format!("(imported via IMAP; size {} bytes)", message.size_bytes),
                false,
                None,
            );
            if let Some(stored) = result.stored {
                crate::mail_store::set_source_account(&stored.id, &account.label)?;
                imported += 1;
            } else {
                rejected += 1;
            }
        }
    } else if let Some(pop3) = account.pop3.as_ref() {
        for message in crate::mail_transport::fetch_pop3(pop3, &account.address)? {
            if crate::mail_accounts::has_pop3_receipt(&account.id, &message.uid) {
                continue;
            }
            let result = crate::mail_inbound::accept_message(
                &message.inbound.from_address,
                &message.inbound.to_address,
                &message.inbound.subject,
                &message.body,
                false,
                None,
            );
            if let Some(stored) = result.stored {
                crate::mail_store::set_source_account(&stored.id, &account.label)?;
                imported += 1;
            } else {
                rejected += 1;
            }
            // Record both accepted and rejected deliveries. Re-evaluating a
            // rejected POP3 message on every poll is neither useful nor fair.
            crate::mail_accounts::record_pop3_receipt(&account.id, &message.uid)?;
        }
    } else {
        return Err(
            "This account has SMTP only; configure IMAP or POP3S to retrieve mail.".to_string(),
        );
    }
    Ok(
        serde_json::json!({ "account_id": account.id, "account_label": account.label, "imported": imported, "rejected": rejected }),
    )
}

/// Fetch all accounts explicitly configured for incoming mail. Failures are
/// reported per account so one provider outage cannot hide other accounts.
#[cfg(not(target_arch = "wasm32"))]
pub fn mail_fetch_all_accounts() -> Result<serde_json::Value, String> {
    let accounts = crate::mail_accounts::list();
    let mut results = Vec::with_capacity(accounts.len());
    for account in accounts {
        if account.imap.is_none() && account.pop3.is_none() {
            continue;
        }
        let id = account.id.clone();
        match mail_fetch_account(id) { Ok(result) => results.push(result), Err(error) => results.push(serde_json::json!({ "account_id": account.id, "account_label": account.label, "error": error })) }
    }
    Ok(serde_json::json!({ "results": results }))
}

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
