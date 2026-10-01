//! SMTP send + IMAP fetch transport for the semantic mail client.
//!
//! This module is the **network edge** for mail: it puts bytes on the wire (SMTP over
//! STARTTLS) and pulls unseen messages off it (IMAP over implicit TLS). It deliberately does
//! *not* make delivery decisions — those belong to the pure rules layer in
//! [`crate::mail_rules`]. The bridge between the two is [`build_inbound`], a pure function that
//! constructs the [`crate::mail_rules::InboundMessage`] envelope the rules engine consumes; the
//! IMAP path uses it to turn each fetched message into something [`crate::mail_rules::evaluate`]
//! can rule on.
//!
//! Network functions are gated `#[cfg(not(target_arch = "wasm32"))]` — there is no raw-socket
//! SMTP/IMAP on the `wasm32` target, so only the pure surface ([`SmtpConfig`], [`ImapConfig`],
//! [`OutgoingMail`], [`build_inbound`]) compiles there.
//!
//! All fallible network functions map their underlying errors to `String` so callers get a flat,
//! transport-agnostic error surface.

use serde::{Deserialize, Serialize};

/// Connection + credentials for an outbound SMTP submission server (STARTTLS on the submission port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// The SMTP server hostname (e.g. `smtp.example.org`).
    pub host: String,
    /// The submission port (commonly `587` for STARTTLS).
    pub port: u16,
    /// The SMTP username (usually the full mailbox address).
    pub username: String,
    /// The SMTP password / app-password / token.
    pub password: String,
}

/// Connection + credentials for an inbound IMAP server (implicit TLS on the IMAPS port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImapConfig {
    /// The IMAP server hostname (e.g. `imap.example.org`).
    pub host: String,
    /// The IMAPS port (commonly `993` for implicit TLS).
    pub port: u16,
    /// The IMAP username (usually the full mailbox address).
    pub username: String,
    /// The IMAP password / app-password / token.
    pub password: String,
}

/// Connection + credentials for a legacy POP3S service. POP3 is retrieval
/// only; SMTP remains the outbound transport. Implicit TLS is required so the
/// account password is never sent on a plaintext POP3 connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pop3Config {
    /// The POP3 server hostname (for example `pop.example.org`).
    pub host: String,
    /// POP3S is commonly served on port 995.
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// A message fetched through POP3S before semantic delivery rules decide its
/// final local mailbox and quarantine state.
#[derive(Debug, Clone)]
pub struct Pop3FetchedMessage {
    pub uid: String,
    pub inbound: crate::mail_rules::InboundMessage,
    pub body: String,
}

/// A message to be sent via [`send`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMail {
    /// The sender address (`from`), e.g. `me@example.org`.
    pub from: String,
    /// The recipient address (`to`), e.g. `alice@example.org`.
    pub to: String,
    /// The subject line.
    pub subject: String,
    /// The plain-text body.
    pub body: String,
}

/// Construct an [`InboundMessage`](crate::mail_rules::InboundMessage) envelope for the rules engine.
///
/// This is **pure**: it only populates the struct from its arguments — no I/O, no clock. It is the
/// single place both real IMAP delivery and tests build the envelope the rules engine consumes, so
/// the mapping from wire data to the rules' view lives in exactly one spot.
pub fn build_inbound(
    from: &str,
    to: &str,
    subject: &str,
    size: usize,
    sender_verified: bool,
    sender_did: Option<String>,
) -> crate::mail_rules::InboundMessage {
    crate::mail_rules::InboundMessage {
        from_address: from.to_string(),
        to_address: to.to_string(),
        sender_did,
        sender_verified,
        subject: subject.to_string(),
        size_bytes: size,
    }
}

/// Send `mail` through the SMTP submission server described by `cfg` (STARTTLS).
///
/// Builds a plain-text [`lettre::Message`], opens a pooled STARTTLS relay to `cfg.host:cfg.port`
/// authenticating with `cfg.username` / `cfg.password`, and submits the message. All underlying
/// errors (address parse, transport build, send) are flattened to `String`.
#[cfg(not(target_arch = "wasm32"))]
pub fn send(cfg: &SmtpConfig, mail: &OutgoingMail) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let from = mail
        .from
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| format!("invalid from address {:?}: {}", mail.from, e))?;
    let to = mail
        .to
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| format!("invalid to address {:?}: {}", mail.to, e))?;

    let msg: Message = Message::builder()
        .from(from)
        .to(to)
        .subject(mail.subject.clone())
        .body(mail.body.clone())
        .map_err(|e| format!("failed to build message: {}", e))?;

    let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());

    let transport = SmtpTransport::starttls_relay(&cfg.host)
        .map_err(|e| format!("failed to build STARTTLS relay for {:?}: {}", cfg.host, e))?
        .port(cfg.port)
        .credentials(creds)
        .build();

    transport
        .send(&msg)
        .map(|_| ())
        .map_err(|e| format!("SMTP send failed: {}", e))
}

/// Fetch the unseen messages from `mailbox` on the IMAP server described by `cfg` (implicit TLS).
///
/// Connects over implicit TLS to `cfg.host:cfg.port`, logs in, selects `mailbox`, searches for
/// `UNSEEN` messages, and for each fetches `RFC822.SIZE ENVELOPE`. Each fetched message is turned
/// into an [`InboundMessage`](crate::mail_rules::InboundMessage) via [`build_inbound`] using the
/// envelope's `from` (first sender address, reconstructed as `local@host`) and `subject`, its
/// reported size, and `mailbox` as the recipient. Because IMAP does not itself attest sender
/// identity, `sender_verified` is `false` and `sender_did` is `None`; verification is a higher-layer
/// concern. The session is logged out before returning. This is written defensively — a message with
/// no envelope or no size is skipped rather than failing the whole fetch — and all underlying errors
/// are flattened to `String`.
#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_unseen(
    cfg: &ImapConfig,
    mailbox: &str,
) -> Result<Vec<crate::mail_rules::InboundMessage>, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("failed to build TLS connector: {}", e))?;

    let client = imap::connect((cfg.host.as_str(), cfg.port), &cfg.host, &tls)
        .map_err(|e| format!("IMAP connect to {:?}:{} failed: {}", cfg.host, cfg.port, e))?;

    let mut session = client
        .login(&cfg.username, &cfg.password)
        // On login failure imap returns (Error, Client); keep only the error text.
        .map_err(|e| format!("IMAP login failed: {}", e.0))?;

    // Ensure we log out even if a later step fails.
    let result = (|| -> Result<Vec<crate::mail_rules::InboundMessage>, String> {
        session
            .select(mailbox)
            .map_err(|e| format!("IMAP SELECT {:?} failed: {}", mailbox, e))?;

        let unseen = session
            .search("UNSEEN")
            .map_err(|e| format!("IMAP SEARCH UNSEEN failed: {}", e))?;

        let mut out: Vec<crate::mail_rules::InboundMessage> = Vec::with_capacity(unseen.len());

        for uid in unseen {
            // Fetch by sequence number; ask only for the envelope + size.
            let fetches = match session.fetch(uid.to_string(), "RFC822.SIZE ENVELOPE") {
                Ok(f) => f,
                // Be defensive: skip a message that can't be fetched rather than aborting all.
                Err(_) => continue,
            };

            for fetch in fetches.iter() {
                let envelope = match fetch.envelope() {
                    Some(e) => e,
                    None => continue,
                };

                // Subject — envelope fields are raw bytes; decode lossily.
                let subject = envelope
                    .subject
                    .as_ref()
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .unwrap_or_default();

                // Reconstruct `local@host` from raw IMAP mailbox/host byte fields.
                let parts_to_string =
                    |mailbox_bytes: Option<&[u8]>, host_bytes: Option<&[u8]>| -> String {
                        let mailbox_part = mailbox_bytes
                            .map(|m| String::from_utf8_lossy(m).into_owned())
                            .unwrap_or_default();
                        let host_part = host_bytes
                            .map(|h| String::from_utf8_lossy(h).into_owned())
                            .unwrap_or_default();
                        if host_part.is_empty() {
                            mailbox_part
                        } else {
                            format!("{mailbox_part}@{host_part}")
                        }
                    };

                // From — first sender address, if present.
                let from = envelope
                    .from
                    .as_ref()
                    .and_then(|addrs| addrs.first())
                    .map(|addr| {
                        parts_to_string(
                            addr.mailbox.as_ref().map(|m| m.as_ref()),
                            addr.host.as_ref().map(|h| h.as_ref()),
                        )
                    })
                    .unwrap_or_default();

                // To — first envelope recipient when it looks like an address; else IMAP folder
                // name (often "INBOX"). mail_fetch falls back to IMAP username for non-@ targets.
                let to = envelope
                    .to
                    .as_ref()
                    .and_then(|addrs| addrs.first())
                    .map(|addr| {
                        parts_to_string(
                            addr.mailbox.as_ref().map(|m| m.as_ref()),
                            addr.host.as_ref().map(|h| h.as_ref()),
                        )
                    })
                    .filter(|s| s.contains('@'))
                    .unwrap_or_else(|| mailbox.to_string());

                // Size — `RFC822.SIZE` populates `fetch.size`; default to 0 when absent.
                let size = fetch.size.unwrap_or(0) as usize;

                out.push(build_inbound(
                    &from, &to, &subject, size,
                    false, // sender_verified — IMAP does not attest identity
                    None,  // sender_did — verification is a higher-layer concern
                ));
            }
        }

        Ok(out)
    })();

    // Best-effort logout; the fetch result takes precedence.
    let _ = session.logout();

    result
}

/// Retrieve messages from a traditional POP3S mailbox. POP3 has no portable
/// `UNSEEN` flag, so this returns the mailbox contents; the caller is expected
/// to persist accepted UIDLs before the next poll. Only implicit TLS is
/// supported—plaintext POP3 would expose the account password.
#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_pop3(cfg: &Pop3Config, mailbox: &str) -> Result<Vec<Pop3FetchedMessage>, String> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    fn response_line<S: std::io::Read>(reader: &mut BufReader<S>) -> Result<String, String> {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| format!("POP3 read failed: {e}"))?;
        if !line.starts_with("+OK") { return Err(format!("POP3 rejected command: {}", line.trim())); }
        Ok(line)
    }
    fn command<S: std::io::Read + Write>(reader: &mut BufReader<S>, command: &str) -> Result<(), String> {
        reader.get_mut().write_all(command.as_bytes()).map_err(|e| format!("POP3 write failed: {e}"))?;
        reader.get_mut().flush().map_err(|e| format!("POP3 flush failed: {e}"))?;
        response_line(reader).map(|_| ())
    }
    fn multiline<S: std::io::Read>(reader: &mut BufReader<S>) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| format!("POP3 read failed: {e}"))?;
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if line == "." { break; }
            lines.push(line.strip_prefix("..").unwrap_or(&line).to_string());
        }
        Ok(lines)
    }
    fn headers_and_body(lines: &[String]) -> (String, String, String, String) {
        let mut from = String::new(); let mut to = String::new(); let mut subject = String::new(); let mut body_at = lines.len();
        for (index, line) in lines.iter().enumerate() {
            if line.is_empty() { body_at = index + 1; break; }
            if let Some(value) = line.strip_prefix("From:").or_else(|| line.strip_prefix("from:")) { from = value.trim().to_string(); }
            if let Some(value) = line.strip_prefix("To:").or_else(|| line.strip_prefix("to:")) { to = value.trim().to_string(); }
            if let Some(value) = line.strip_prefix("Subject:").or_else(|| line.strip_prefix("subject:")) { subject = value.trim().to_string(); }
        }
        (from, to, subject, lines[body_at..].join("\n"))
    }

    let tcp = TcpStream::connect((cfg.host.as_str(), cfg.port)).map_err(|e| format!("POP3S connect to {}:{} failed: {e}", cfg.host, cfg.port))?;
    let tls = native_tls::TlsConnector::builder().build().map_err(|e| format!("POP3S TLS setup failed: {e}"))?;
    let stream = tls.connect(&cfg.host, tcp).map_err(|e| format!("POP3S TLS connect failed: {e}"))?;
    let mut reader = BufReader::new(stream);
    response_line(&mut reader)?;
    command(&mut reader, &format!("USER {}\r\n", cfg.username))?;
    command(&mut reader, &format!("PASS {}\r\n", cfg.password))?;
    command(&mut reader, "UIDL\r\n")?;
    let uids = multiline(&mut reader)?;
    let mut messages = Vec::new();
    for entry in uids {
        let mut parts = entry.split_whitespace();
        let Some(number) = parts.next() else { continue; };
        let Some(uid) = parts.next() else { continue; };
        command(&mut reader, &format!("RETR {number}\r\n"))?;
        let lines = multiline(&mut reader)?;
        let (from, mut to, subject, body) = headers_and_body(&lines);
        if !to.contains('@') { to = mailbox.to_string(); }
        let inbound = build_inbound(&from, &to, &subject, body.len(), false, None);
        messages.push(Pop3FetchedMessage { uid: uid.to_string(), inbound, body });
    }
    let _ = command(&mut reader, "QUIT\r\n");
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_inbound_populates_fields() {
        let msg = build_inbound(
            "alice@example.org",
            "me@example.org",
            "Hello there",
            2048,
            true,
            Some("did:example:alice".to_string()),
        );

        assert_eq!(msg.from_address, "alice@example.org");
        assert_eq!(msg.to_address, "me@example.org");
        assert_eq!(msg.subject, "Hello there");
        assert_eq!(msg.size_bytes, 2048);
        assert!(msg.sender_verified);
        assert_eq!(msg.sender_did.as_deref(), Some("did:example:alice"));
    }

    #[test]
    fn build_inbound_defaults_unverified_no_did() {
        let msg = build_inbound("spam@nowhere.test", "junk@example.org", "", 0, false, None);

        assert_eq!(msg.from_address, "spam@nowhere.test");
        assert_eq!(msg.to_address, "junk@example.org");
        assert_eq!(msg.subject, "");
        assert_eq!(msg.size_bytes, 0);
        assert!(!msg.sender_verified);
        assert!(msg.sender_did.is_none());
    }
}
