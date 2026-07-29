//! Local mail message store — where delivered mail actually lives.
//!
//! Inbound mail (local SMTP receiver, inject, or optional IMAP import) is resolved and ruled,
//! then persisted under `app_meta_dir()/mail_messages.json`. This is the product inbox — not a
//! pointer at a paid provider.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::state::app_meta_dir;

/// A delivered (or quarantined) message in the local inbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMail {
    pub id: String,
    pub received_at: u64,
    pub from_address: String,
    pub to_address: String,
    /// Mailbox address that accepted delivery (exact or catchall surface).
    pub mailbox: String,
    pub subject: String,
    pub body: String,
    /// `exact` | `catchall`
    pub via: String,
    pub quarantined: bool,
    pub priority: i8,
    pub read: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
    /// Size in bytes of the original body (for UI).
    #[serde(default)]
    pub size_bytes: usize,
}

fn messages_path() -> PathBuf {
    app_meta_dir().join("mail_messages.json")
}

fn load_all() -> Vec<StoredMail> {
    fs::read_to_string(messages_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_all(msgs: &[StoredMail]) -> Result<(), String> {
    let path = messages_path();
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(msgs).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn new_id() -> String {
    // Compact unique-enough id without pulling uuid into hot API paths unnecessarily.
    let n = now_unix();
    let r: u32 = rand::random();
    format!("m-{n:x}-{r:08x}")
}

/// Append a delivered message. Returns the stored record.
pub fn append(msg: StoredMail) -> Result<StoredMail, String> {
    let mut all = load_all();
    all.insert(0, msg.clone()); // newest first
                                // Soft cap — keep last 5_000 messages so the file stays bounded.
    if all.len() > 5_000 {
        all.truncate(5_000);
    }
    save_all(&all)?;
    Ok(msg)
}

/// Build + append from delivery fields.
pub fn store_delivery(
    from: &str,
    to: &str,
    mailbox: &str,
    subject: &str,
    body: &str,
    via: &str,
    quarantined: bool,
    priority: i8,
    reasons: Vec<String>,
) -> Result<StoredMail, String> {
    let body = body.to_string();
    let size_bytes = body.len();
    let msg = StoredMail {
        id: new_id(),
        received_at: now_unix(),
        from_address: from.to_string(),
        to_address: to.to_string(),
        mailbox: mailbox.to_string(),
        subject: subject.to_string(),
        body,
        via: via.to_string(),
        quarantined,
        priority,
        read: false,
        reasons,
        size_bytes,
    };
    append(msg)
}

/// List messages, newest first. `mailbox` filters by accepting mailbox when set.
/// `include_quarantine` includes quarantined messages (default true for full inbox; UI can filter).
pub fn list(mailbox: Option<&str>, include_quarantine: bool) -> Vec<StoredMail> {
    load_all()
        .into_iter()
        .filter(|m| {
            if let Some(mb) = mailbox {
                if !m.mailbox.eq_ignore_ascii_case(mb) && !m.to_address.eq_ignore_ascii_case(mb) {
                    return false;
                }
            }
            if !include_quarantine && m.quarantined {
                return false;
            }
            true
        })
        .collect()
}

pub fn get(id: &str) -> Option<StoredMail> {
    load_all().into_iter().find(|m| m.id == id)
}

pub fn set_read(id: &str, read: bool) -> Result<StoredMail, String> {
    let mut all = load_all();
    let msg = all
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("unknown message '{id}'"))?;
    msg.read = read;
    let out = msg.clone();
    save_all(&all)?;
    Ok(out)
}

pub fn delete(id: &str) -> Result<(), String> {
    let mut all = load_all();
    let before = all.len();
    all.retain(|m| m.id != id);
    if all.len() == before {
        return Err(format!("unknown message '{id}'"));
    }
    save_all(&all)
}

/// Counts for the UI badge.
pub fn counts() -> (usize, usize, usize) {
    let all = load_all();
    let total = all.len();
    let unread = all.iter().filter(|m| !m.read).count();
    let quarantine = all.iter().filter(|m| m.quarantined).count();
    (total, unread, quarantine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_list_read_delete_roundtrip() {
        // Isolate: only exercises pure helpers that don't need a special dir if app_meta works;
        // use unique subject markers and clean up.
        let marker = format!("test-subject-{}", now_unix());
        let stored = store_delivery(
            "a@b.example",
            "frontdoor@c.example",
            "frontdoor@c.example",
            &marker,
            "hello body",
            "exact",
            false,
            1,
            vec!["test".into()],
        )
        .expect("store");
        assert!(!stored.id.is_empty());
        assert!(list(None, true).iter().any(|m| m.id == stored.id));
        let got = get(&stored.id).expect("get");
        assert_eq!(got.subject, marker);
        set_read(&stored.id, true).expect("read");
        assert!(get(&stored.id).unwrap().read);
        delete(&stored.id).expect("delete");
        assert!(get(&stored.id).is_none());
    }
}
