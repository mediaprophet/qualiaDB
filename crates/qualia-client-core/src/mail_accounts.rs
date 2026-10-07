//! Local registry for multiple interoperable mail accounts.
//!
//! Account configuration is cold-path user configuration. Imported messages
//! always enter the single governed product inbox, while `account_id` and
//! `label` stay with the import receipt so provenance is not discarded.

use crate::mail_transport::{ImapConfig, Pop3Config, SmtpConfig};
use crate::state::app_meta_dir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;

const ACCOUNTS_FILE: &str = "mail_accounts.json";
const MAX_ACCOUNTS: usize = 64;
const POP3_RECEIPTS_FILE: &str = "mail_pop3_receipts.json";
const MAX_POP3_RECEIPTS: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAccount {
    pub id: String,
    pub label: String,
    pub address: String,
    #[serde(default)]
    pub smtp: Option<SmtpConfig>,
    #[serde(default)]
    pub imap: Option<ImapConfig>,
    #[serde(default)]
    pub pop3: Option<Pop3Config>,
}

fn path() -> std::path::PathBuf {
    app_meta_dir().join(ACCOUNTS_FILE)
}

pub fn list() -> Vec<MailAccount> {
    fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(account: MailAccount) -> Result<Vec<MailAccount>, String> {
    if account.id.trim().is_empty()
        || account.label.trim().is_empty()
        || !account.address.contains('@')
    {
        return Err("Each account needs an id, a label, and a mailbox address.".to_string());
    }
    if account.imap.is_none() && account.pop3.is_none() && account.smtp.is_none() {
        return Err("Configure at least SMTP, IMAP, or POP3S for this account.".to_string());
    }
    let mut accounts = list();
    if let Some(existing) = accounts.iter_mut().find(|item| item.id == account.id) {
        *existing = account;
    } else {
        if accounts.len() >= MAX_ACCOUNTS {
            return Err("The local account limit has been reached.".to_string());
        }
        accounts.push(account);
    }
    if let Some(parent) = path().parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path(),
        serde_json::to_string_pretty(&accounts).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(accounts)
}

pub fn find(id: &str) -> Option<MailAccount> {
    list().into_iter().find(|account| account.id == id)
}

fn pop3_receipts_path() -> std::path::PathBuf {
    app_meta_dir().join(POP3_RECEIPTS_FILE)
}

fn pop3_receipts() -> BTreeSet<String> {
    fs::read_to_string(pop3_receipts_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// POP3 has no standard unseen state. A UIDL receipt is therefore recorded
/// locally once a fetched message has passed through delivery evaluation.
pub fn has_pop3_receipt(account_id: &str, uid: &str) -> bool {
    pop3_receipts().contains(&format!("{account_id}:{uid}"))
}

pub fn record_pop3_receipt(account_id: &str, uid: &str) -> Result<(), String> {
    let mut receipts = pop3_receipts();
    receipts.insert(format!("{account_id}:{uid}"));
    while receipts.len() > MAX_POP3_RECEIPTS {
        let Some(oldest) = receipts.iter().next().cloned() else {
            break;
        };
        receipts.remove(&oldest);
    }
    if let Some(parent) = pop3_receipts_path().parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        pop3_receipts_path(),
        serde_json::to_string(&receipts).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
