//! Inbound mail acceptance — the path that makes domain mail **work**.
//!
//! Flow:
//! 1. Envelope `RCPT TO` / inject target → [`crate::domains::resolve_delivery`]
//! 2. Body + headers → [`crate::mail_rules::evaluate`]
//! 3. If deliver → [`crate::mail_store::store_delivery`]
//!
//! Also runs a **local SMTP receiver** (default `127.0.0.1:2525`) so mail can land without a paid
//! mailbox product. Public internet delivery still needs DNS MX + a tunnel/VPS to your host
//! (port 25 is often blocked on residential links) — but the product inbox is yours, local, and
//! rule-bearing.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domains::{self, DeliveryResolution, ResolutionVia};
use crate::mail_rules::{self, InboundMessage};
use crate::mail_store::{self, StoredMail};

/// Default bind for the personal SMTP edge (submission-style port; avoid privileged 25).
pub const DEFAULT_SMTP_BIND: &str = "127.0.0.1:2525";

static SMTP_RUNNING: AtomicBool = AtomicBool::new(false);
static SMTP_PORT: AtomicU16 = AtomicU16::new(0);
static SMTP_STOP: AtomicBool = AtomicBool::new(false);
static SMTP_BIND_DISPLAY: OnceLock<Mutex<String>> = OnceLock::new();

fn bind_display() -> &'static Mutex<String> {
    SMTP_BIND_DISPLAY.get_or_init(|| Mutex::new(String::new()))
}

/// Outcome of accepting one message (for API / UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptResult {
    pub accepted: bool,
    pub rejected: Option<String>,
    pub stored: Option<StoredMail>,
}

/// Accept a message for a registered domain mailbox — pure product path (no network).
pub fn accept_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    sender_verified: bool,
    sender_did: Option<String>,
) -> AcceptResult {
    let addresses = domains::list_addresses(None);
    match domains::resolve_delivery(&addresses, to) {
        DeliveryResolution::Reject { reason } => AcceptResult {
            accepted: false,
            rejected: Some(reason),
            stored: None,
        },
        DeliveryResolution::Deliver { address, via } => {
            let msg = InboundMessage {
                from_address: from.to_string(),
                to_address: to.to_string(),
                sender_did,
                sender_verified,
                subject: subject.to_string(),
                size_bytes: body.len(),
            };
            let mut verdict = mail_rules::evaluate(&address.rules, &msg);
            let via_str = match via {
                ResolutionVia::Exact => "exact",
                ResolutionVia::Catchall => "catchall",
                ResolutionVia::Unsolicited => "unsolicited",
            };
            if matches!(via, ResolutionVia::Catchall) && !verdict.quarantined {
                verdict.quarantined = true;
                verdict.reasons.push("catchall intake — quarantined".into());
            }
            if !verdict.deliver {
                return AcceptResult {
                    accepted: false,
                    rejected: verdict
                        .rejected
                        .or_else(|| Some("rejected by rules".into())),
                    stored: None,
                };
            }
            match mail_store::store_delivery(
                from,
                to,
                &address.address,
                subject,
                body,
                via_str,
                verdict.quarantined,
                verdict.priority,
                verdict.reasons,
            ) {
                Ok(stored) => AcceptResult {
                    accepted: true,
                    rejected: None,
                    stored: Some(stored),
                },
                Err(e) => AcceptResult {
                    accepted: false,
                    rejected: Some(e),
                    stored: None,
                },
            }
        }
    }
}

/// DNS records a human pastes so the **internet** can find this receiver.
/// Host is optional public hostname (MX target); when empty, placeholders are used.
pub fn mail_dns_forms(domain: &str, mx_host: Option<&str>) -> serde_json::Value {
    let domain = domain.trim().to_lowercase();
    let mx = mx_host
        .map(|h| h.trim().to_lowercase())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| format!("mail.{domain}"));
    let port = SMTP_PORT.load(Ordering::Relaxed);
    let port_note = if port == 0 {
        DEFAULT_SMTP_BIND.to_string()
    } else {
        format!("listening on port {port}")
    };
    serde_json::json!({
        "domain": domain,
        "mx_host": mx,
        "records": [
            {
                "type": "MX",
                "name": "@",
                "priority": 10,
                "value": format!("{mx}."),
                "hint": "Points senders at your mail host (tunnel, VPS, or home public hostname)."
            },
            {
                "type": "A_or_AAAA",
                "name": mx.strip_suffix(&format!(".{domain}")).unwrap_or("mail"),
                "value": "<your public IP or tunnel target>",
                "hint": "Where the MX name resolves. Cloudflare Tunnel can map hostname → this app."
            },
            {
                "type": "TXT",
                "name": "@",
                "value": "v=spf1 mx a -all",
                "hint": "SPF: only MX/A may send as this domain (tighten later with DKIM)."
            },
        ],
        "local_receiver": {
            "default_bind": DEFAULT_SMTP_BIND,
            "status": if SMTP_RUNNING.load(Ordering::Relaxed) { "running" } else { "stopped" },
            "bind": bind_display().lock().map(|g| g.clone()).unwrap_or_default(),
            "port_note": port_note,
            "how": "Start the local SMTP receiver in Talk → Mail. For public mail, forward TCP 25 (or 2525) to that port via tunnel/router. Same machine can use localhost:2525 without DNS."
        },
        "plaintext_block": format!(
            "MX  @  10  {mx}.\n\
             A/AAAA  {mx}  →  <public IP or tunnel>\n\
             TXT  @  v=spf1 mx a -all\n\
             # Local receiver: {port_note}\n\
             # QDP front-door TXT is separate (_qdp) — identity, not MX."
        ),
    })
}

/// Status of the local SMTP receiver.
pub fn receiver_status() -> serde_json::Value {
    serde_json::json!({
        "running": SMTP_RUNNING.load(Ordering::Relaxed),
        "port": SMTP_PORT.load(Ordering::Relaxed),
        "bind": bind_display().lock().map(|g| g.clone()).unwrap_or_default(),
        "default_bind": DEFAULT_SMTP_BIND,
    })
}

/// Start the local SMTP receiver on `bind` (e.g. `127.0.0.1:2525` or `0.0.0.0:2525`).
/// Idempotent if already running on the same bind.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_receiver(bind: &str) -> Result<serde_json::Value, String> {
    let bind = if bind.trim().is_empty() {
        DEFAULT_SMTP_BIND.to_string()
    } else {
        bind.trim().to_string()
    };
    if SMTP_RUNNING.load(Ordering::Relaxed) {
        return Ok(serde_json::json!({
            "already_running": true,
            "bind": bind_display().lock().map(|g| g.clone()).unwrap_or_default(),
            "port": SMTP_PORT.load(Ordering::Relaxed),
        }));
    }
    let listener = TcpListener::bind(&bind)
        .map_err(|e| format!("bind {bind} failed: {e} (is another process using the port?)"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("set_nonblocking: {e}"))?;
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    SMTP_STOP.store(false, Ordering::Relaxed);
    SMTP_PORT.store(port, Ordering::Relaxed);
    if let Ok(mut g) = bind_display().lock() {
        *g = bind.clone();
    }
    SMTP_RUNNING.store(true, Ordering::Relaxed);

    thread::Builder::new()
        .name("webizen-smtp".into())
        .spawn(move || {
            while !SMTP_STOP.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
                        // One connection per thread — personal MTA volume is small.
                        thread::spawn(move || {
                            if let Err(e) = handle_smtp_session(stream) {
                                log::debug!("smtp session end: {e}");
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
            SMTP_RUNNING.store(false, Ordering::Relaxed);
            SMTP_PORT.store(0, Ordering::Relaxed);
        })
        .map_err(|e| format!("spawn smtp thread: {e}"))?;

    Ok(serde_json::json!({
        "started": true,
        "bind": bind,
        "port": port,
        "message": format!("Local SMTP receiver on {bind} — point MX/tunnel here, or send to localhost for tests."),
    }))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stop_receiver() -> Result<serde_json::Value, String> {
    SMTP_STOP.store(true, Ordering::Relaxed);
    // Give the accept loop a moment to notice.
    thread::sleep(Duration::from_millis(80));
    Ok(serde_json::json!({
        "stopped": true,
        "running": SMTP_RUNNING.load(Ordering::Relaxed),
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn smtp_write(stream: &mut TcpStream, line: &str) -> Result<(), String> {
    stream
        .write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

/// Extract address from `MAIL FROM:<a@b>` / `RCPT TO:<a@b>` / bare forms.
fn parse_smtp_path(arg: &str) -> String {
    let s = arg.trim();
    if let Some(start) = s.find('<') {
        if let Some(end) = s[start + 1..].find('>') {
            return s[start + 1..start + 1 + end].trim().to_string();
        }
    }
    s.trim_matches(|c| c == '<' || c == '>').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_smtp_session(mut stream: TcpStream) -> Result<(), String> {
    // Blocking reads after accept — switch socket back to blocking for BufReader.
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    smtp_write(
        &mut stream,
        "220 webizen.local ESMTP Qualia semantic mail ready\r\n",
    )?;

    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut mail_from = String::new();
    let mut rcpt_to: Vec<String> = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let cmd = line.trim_end_matches(['\r', '\n']);
        let upper = cmd.to_ascii_uppercase();

        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            smtp_write(
                &mut stream,
                "250-webizen.local\r\n250-PIPELINING\r\n250 8BITMIME\r\n",
            )?;
        } else if upper.starts_with("MAIL FROM:") {
            mail_from = parse_smtp_path(&cmd[10..]);
            smtp_write(&mut stream, "250 OK\r\n")?;
        } else if upper.starts_with("RCPT TO:") {
            let to = parse_smtp_path(&cmd[8..]);
            let addresses = domains::list_addresses(None);
            match domains::resolve_delivery(&addresses, &to) {
                DeliveryResolution::Deliver { .. } => {
                    rcpt_to.push(to);
                    smtp_write(&mut stream, "250 OK\r\n")?;
                }
                DeliveryResolution::Reject { reason } => {
                    smtp_write(&mut stream, &format!("550 5.1.1 {reason}\r\n"))?;
                }
            }
        } else if upper == "DATA" {
            if rcpt_to.is_empty() {
                smtp_write(&mut stream, "503 5.5.1 No valid recipients\r\n")?;
                continue;
            }
            smtp_write(&mut stream, "354 End data with <CR><LF>.<CR><LF>\r\n")?;
            let mut data = String::new();
            loop {
                line.clear();
                let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                // End of data: line is ".\r\n" or "."
                let trimmed = line.trim_end_matches(['\r', '\n']);
                if trimmed == "." {
                    break;
                }
                // RFC 5321 dot-stuffing: lines starting with ".." → strip one dot.
                if let Some(rest) = line.strip_prefix("..") {
                    data.push_str(rest);
                } else {
                    data.push_str(&line);
                }
            }
            let (subject, body) = split_headers_body(&data);
            let from = if mail_from.is_empty() {
                "unknown@invalid".to_string()
            } else {
                mail_from.clone()
            };
            let mut any_ok = false;
            let mut last_err = String::new();
            for to in &rcpt_to {
                let r = accept_message(&from, to, &subject, &body, false, None);
                if r.accepted {
                    any_ok = true;
                } else if let Some(e) = r.rejected {
                    last_err = e;
                }
            }
            if any_ok {
                smtp_write(&mut stream, "250 OK queued\r\n")?;
            } else {
                smtp_write(&mut stream, &format!("550 5.7.1 rejected: {last_err}\r\n"))?;
            }
            // Reset transaction
            mail_from.clear();
            rcpt_to.clear();
        } else if upper == "RSET" {
            mail_from.clear();
            rcpt_to.clear();
            smtp_write(&mut stream, "250 OK\r\n")?;
        } else if upper == "NOOP" {
            smtp_write(&mut stream, "250 OK\r\n")?;
        } else if upper == "QUIT" {
            smtp_write(&mut stream, "221 webizen.local closing\r\n")?;
            break;
        } else if upper.starts_with("VRFY") || upper.starts_with("EXPN") {
            smtp_write(&mut stream, "252 Not verified\r\n")?;
        } else {
            smtp_write(&mut stream, "502 5.5.2 Command not implemented\r\n")?;
        }
    }
    Ok(())
}

/// Split RFC822-ish message into subject + body (body includes remaining headers stripped of Subject).
fn split_headers_body(data: &str) -> (String, String) {
    let normalized = data.replace("\r\n", "\n");
    let (head, body) = if let Some(i) = normalized.find("\n\n") {
        (&normalized[..i], normalized[i + 2..].to_string())
    } else {
        return (String::new(), normalized);
    };
    let mut subject = String::new();
    for line in head.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("subject:") {
            // Preserve original casing of value from `line`
            subject = line[line.len() - rest.len()..].trim().to_string();
            break;
        }
    }
    // Prefer full original as body for reading; subject extracted for list UI.
    let full_body = if body.is_empty() {
        data.to_string()
    } else {
        body
    };
    (subject, full_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::{
        make_domain, make_purpose_address, upsert_address, upsert_domain, AgentType, DomainOwner,
        MailRules,
    };

    #[test]
    fn parse_smtp_path_angles() {
        assert_eq!(parse_smtp_path("<bob@alice.example>"), "bob@alice.example");
        assert_eq!(parse_smtp_path(" bob@alice.example "), "bob@alice.example");
    }

    #[test]
    fn split_subject_from_data() {
        let (s, b) = split_headers_body("From: a@b\r\nSubject: Hello\r\n\r\nBody line\r\n");
        assert_eq!(s, "Hello");
        assert!(b.contains("Body line"));
    }

    #[test]
    fn accept_message_fail_closed_without_mailbox() {
        let r = accept_message(
            "x@y.example",
            "nobody@not-registered.invalid.example",
            "hi",
            "body",
            false,
            None,
        );
        assert!(!r.accepted);
        assert!(r.rejected.is_some());
    }

    #[test]
    fn accept_message_delivers_when_domain_onboarded() {
        let domain = format!("inbound-test-{}.example", std::process::id());
        let d = make_domain(
            &domain,
            AgentType::NaturalPerson,
            DomainOwner::Personal {
                did: "did:test".into(),
            },
            "did:test",
            "T",
            None,
            1,
        )
        .unwrap();
        upsert_domain(d).unwrap();
        let a = make_purpose_address(
            &domain,
            "frontdoor",
            MailRules {
                notify: true,
                ..Default::default()
            },
            1,
        )
        .unwrap();
        upsert_address(a).unwrap();
        let to = format!("frontdoor@{domain}");
        let r = accept_message(
            "peer@other.example",
            &to,
            "Ping",
            "hello world",
            false,
            None,
        );
        assert!(r.accepted, "{:?}", r.rejected);
        let stored = r.stored.expect("stored");
        assert_eq!(stored.mailbox, to);
        assert!(!stored.quarantined);
        // cleanup
        let _ = mail_store::delete(&stored.id);
    }

    #[test]
    fn mail_dns_forms_include_mx_and_spf() {
        let v = mail_dns_forms("alice.example", Some("mail.alice.example"));
        let block = v["plaintext_block"].as_str().unwrap_or("");
        assert!(block.contains("MX"));
        assert!(block.contains("spf1"));
    }
}
