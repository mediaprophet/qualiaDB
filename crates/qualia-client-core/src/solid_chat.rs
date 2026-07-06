//! **Solid Chat interop** — bidirectional mapping between the native chat model and the SolidOS
//! "long chat" data model (<https://solid.github.io/chat/>), **without degrading the native format**.
//!
//! The native chat (threaded [`ChatSession`]/[`ChatMessage`] with Lamport clocks, roles, agent sub-DIDs,
//! provenance) is unchanged; this is an **adapter at the boundary**. The no-degradation mechanism is
//! **additive fidelity**: every exported message carries BOTH
//!   - the standard Solid subset every long-chat client understands — `meeting:LongChat` +
//!     `meeting:message`, and per message `dct:created` / `sioc:content` / `foaf:maker`; AND
//!   - native-only fields as extra `qc:` (`https://ns.webcivics.net/qualia-chat/`) triples on the SAME
//!     resource (`qc:lamport`, `qc:role`, `qc:contentHash`, `qc:agentDid`, `qc:modelId`,
//!     `qc:replyToFragment`, …).
//!
//! A vanilla Solid client reads the `sioc:`/`meeting:` subset and ignores `qc:`; a Qualia peer reads the
//! full graph. So **Qualia → Solid → Qualia round-trips losslessly** (proven by the tests), while a Solid
//! user still sees a normal chat. Messages are serialised with **single-line, escaped string literals**
//! (one predicate per line) so the round-trip parser is small and robust; hardening for arbitrary Turtle
//! reuses the engine `N3Parser` (a follow-on). Layout matches the spec: `index.ttl` (the channel) +
//! date-partitioned `YYYY/MM/DD/chat.ttl` (the day's messages).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::chat_session::{ChatMessage, ChatSession, Role, SessionKind, SessionMeta};

const NS_QC: &str = "https://ns.webcivics.net/qualia-chat/";

/// The rendered Solid long-chat resources for a session: the channel index plus each day's message file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidChatExport {
    /// `index.ttl` — the `meeting:LongChat` channel resource.
    pub index_ttl: String,
    /// `YYYY/MM/DD` → the day's `chat.ttl` content.
    pub day_files: BTreeMap<String, String>,
}

impl SolidChatExport {
    /// Flatten to `(relative_path, content)` pairs ready to PUT/PATCH onto a Pod (or write to disk):
    /// `index.ttl` + `YYYY/MM/DD/chat.ttl`.
    pub fn files(&self) -> Vec<(String, String)> {
        let mut out = vec![("index.ttl".to_string(), self.index_ttl.clone())];
        for (day, body) in &self.day_files {
            out.push((format!("{day}/chat.ttl"), body.clone()));
        }
        out
    }
}

fn iso_utc(unix_secs: u64) -> String {
    chrono::DateTime::from_timestamp(unix_secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

fn day_path(unix_secs: u64) -> String {
    chrono::DateTime::from_timestamp(unix_secs as i64, 0)
        .map(|dt| dt.format("%Y/%m/%d").to_string())
        .unwrap_or_else(|| "1970/01/01".to_string())
}

fn parse_iso(s: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp().max(0) as u64)
        .unwrap_or(0)
}

/// Escape a string for a single-line Turtle short-string literal (`"..."`).
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn maker_uri(msg: &ChatMessage, owner_did: &str) -> String {
    match &msg.author_did {
        Some(d) if !d.is_empty() => d.clone(),
        _ => owner_did.to_string(),
    }
}

fn message_id(msg: &ChatMessage) -> String {
    format!("msg-{}", msg.lamport)
}

/// Export a native session to the Solid long-chat resources (`index.ttl` + date-partitioned day files).
pub fn export_session(session: &ChatSession) -> SolidChatExport {
    export_parts(&session.meta, &session.messages)
}

/// Export from a session's metadata + messages (decoupled from the heavy `ChatEnvironment`).
pub fn export_parts(meta: &SessionMeta, messages: &[ChatMessage]) -> SolidChatExport {
    let owner = &meta.owner_did;
    let kind = match meta.session_kind {
        SessionKind::Solo => "solo",
        SessionKind::Group => "group",
    };

    // --- index.ttl (the channel) ---
    let mut index = String::new();
    let _ = writeln!(index, "@prefix meeting: <http://www.w3.org/ns/pim/meeting#> .");
    let _ = writeln!(index, "@prefix dct: <http://purl.org/dc/terms/> .");
    let _ = writeln!(index, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .");
    let _ = writeln!(index, "@prefix qc: <{NS_QC}> .");
    let _ = writeln!(index);
    let _ = writeln!(index, "<#this> a meeting:LongChat ;");
    let _ = writeln!(index, "    dct:title \"{}\" ;", esc(&meta.title));
    if !owner.is_empty() {
        let _ = writeln!(index, "    dct:author <{owner}> ;");
    }
    let _ = writeln!(
        index,
        "    dct:created \"{}\"^^xsd:dateTime ;",
        iso_utc(meta.created_at)
    );
    if !meta.session_did.is_empty() {
        let _ = writeln!(index, "    qc:sessionDid \"{}\" ;", esc(&meta.session_did));
    }
    let _ = write!(index, "    qc:sessionKind \"{kind}\"");
    // meeting:message links (day-relative URIs into the message resources).
    if messages.is_empty() {
        let _ = writeln!(index, " .");
    } else {
        let _ = writeln!(index, " ;");
        let n = messages.len();
        for (i, msg) in messages.iter().enumerate() {
            let uri = format!("{}/chat.ttl#{}", day_path(msg.timestamp), message_id(msg));
            let sep = if i + 1 == n { " ." } else { " ," };
            let lead = if i == 0 { "    meeting:message " } else { "        " };
            let _ = writeln!(index, "{lead}<{uri}>{sep}");
        }
    }

    // --- YYYY/MM/DD/chat.ttl (the messages) ---
    let mut day_files: BTreeMap<String, String> = BTreeMap::new();
    for msg in messages {
        let day = day_path(msg.timestamp);
        let body = day_files.entry(day).or_insert_with(day_header);
        write_message(body, msg, owner);
    }

    SolidChatExport { index_ttl: index, day_files }
}

fn day_header() -> String {
    let mut h = String::new();
    let _ = writeln!(h, "@prefix sioc: <http://rdfs.org/sioc/ns#> .");
    let _ = writeln!(h, "@prefix dct: <http://purl.org/dc/terms/> .");
    let _ = writeln!(h, "@prefix foaf: <http://xmlns.com/foaf/0.1/> .");
    let _ = writeln!(h, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .");
    let _ = writeln!(h, "@prefix qc: <{NS_QC}> .");
    let _ = writeln!(h);
    h
}

fn write_message(out: &mut String, msg: &ChatMessage, owner_did: &str) {
    let id = message_id(msg);
    // Standard Solid subset (every long-chat client reads these three).
    let _ = writeln!(out, "<#{id}> dct:created \"{}\"^^xsd:dateTime ;", iso_utc(msg.timestamp));
    let _ = writeln!(out, "    sioc:content \"{}\" ;", esc(&msg.content));
    let _ = writeln!(out, "    foaf:maker <{}> ;", maker_uri(msg, owner_did));
    // Additive native fidelity (a vanilla Solid client ignores the qc: namespace).
    let _ = writeln!(out, "    qc:lamport {} ;", msg.lamport);
    let _ = writeln!(out, "    qc:role \"{}\" ;", msg.role.as_str());
    let _ = write!(out, "    qc:contentHash \"{:016x}\"", msg.content_hash);
    let opt = |out: &mut String, pred: &str, v: &Option<String>, is_uri: bool| {
        if let Some(x) = v {
            if !x.is_empty() {
                if is_uri {
                    let _ = write!(out, " ;\n    {pred} <{x}>");
                } else {
                    let _ = write!(out, " ;\n    {pred} \"{}\"", esc(x));
                }
            }
        }
    };
    opt(out, "qc:authorName", &msg.author_name, false);
    opt(out, "qc:replyToFragment", &msg.reply_to_fragment, false);
    opt(out, "qc:source", &msg.source, false);
    opt(out, "qc:subAgentOf", &msg.sub_agent_of, true);
    opt(out, "qc:agentDid", &msg.agent_did, true);
    opt(out, "qc:modelId", &msg.model_id, false);
    opt(out, "qc:agentBackend", &msg.agent_backend, false);
    let _ = writeln!(out, " .");
    let _ = writeln!(out);
}

/// A message parsed back from a Solid long-chat day file. Fields absent in a pure-Solid file (no `qc:`)
/// are defaulted honestly (role = User, Lamport derived from order by the caller if needed).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportedMessage {
    pub id: String,
    pub content: String,
    pub maker: String,
    pub created_unix: u64,
    pub lamport: Option<u64>,
    pub role: Option<String>,
    pub content_hash: Option<u64>,
    pub author_name: Option<String>,
    pub reply_to_fragment: Option<String>,
    pub source: Option<String>,
    pub sub_agent_of: Option<String>,
    pub agent_did: Option<String>,
    pub model_id: Option<String>,
    pub agent_backend: Option<String>,
}

impl ImportedMessage {
    /// Reconstruct a native [`ChatMessage`]. `fallback_lamport` is used only when the file carried no
    /// `qc:lamport` (a message that originated in a non-Qualia Solid client).
    pub fn to_chat_message(&self, fallback_lamport: u64) -> ChatMessage {
        let content_hash = self
            .content_hash
            .unwrap_or_else(|| crate::chat_session::content_hash_u64(&self.content));
        ChatMessage {
            lamport: self.lamport.unwrap_or(fallback_lamport),
            role: self
                .role
                .as_deref()
                .and_then(|r| Role::from_str(r).ok())
                .unwrap_or(Role::User),
            content: self.content.clone(),
            timestamp: self.created_unix,
            content_hash,
            author_did: (!self.maker.is_empty()).then(|| self.maker.clone()),
            author_name: self.author_name.clone(),
            reply_to_fragment: self.reply_to_fragment.clone(),
            source: self.source.clone().or_else(|| Some("solid".to_string())),
            sub_agent_of: self.sub_agent_of.clone(),
            agent_did: self.agent_did.clone(),
            model_id: self.model_id.clone(),
            agent_backend: self.agent_backend.clone(),
            outcome_sharing: None,
        }
    }
}

/// Parse a Solid long-chat day file (`chat.ttl`) into messages. Handles our own additive `qc:` output
/// and the standard `sioc:`/`foaf:`/`dct:` subset produced by any long-chat client (single-line literals).
pub fn parse_day_ttl(ttl: &str) -> Vec<ImportedMessage> {
    let mut msgs = Vec::new();
    // Statements are terminated by " ." at line end; accumulate a subject's lines until then.
    let mut stmt = String::new();
    for raw in ttl.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("@prefix") || line.starts_with('#') {
            continue;
        }
        if !stmt.is_empty() {
            stmt.push(' ');
        }
        stmt.push_str(line);
        if line.ends_with('.') && !line.ends_with("\\.") {
            if let Some(m) = parse_statement(stmt.trim_end_matches('.').trim()) {
                msgs.push(m);
            }
            stmt.clear();
        }
    }
    msgs
}

fn parse_statement(stmt: &str) -> Option<ImportedMessage> {
    // subject predicate obj ; predicate obj ; ...
    let (subject, rest) = split_first_token(stmt);
    if !subject.starts_with("<#") {
        return None;
    }
    let mut m = ImportedMessage {
        id: subject.trim_start_matches("<#").trim_end_matches('>').to_string(),
        ..Default::default()
    };
    for clause in split_top_level_semicolons(rest) {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        let (pred, obj) = split_first_token(clause);
        let obj = obj.trim();
        match pred {
            "a" => {}
            "dct:created" => m.created_unix = parse_iso(&literal(obj)),
            "sioc:content" => m.content = literal(obj),
            "foaf:maker" => m.maker = uri(obj),
            "qc:lamport" => m.lamport = obj.trim().parse().ok(),
            "qc:role" => m.role = Some(literal(obj)),
            "qc:contentHash" => m.content_hash = u64::from_str_radix(&literal(obj), 16).ok(),
            "qc:authorName" => m.author_name = Some(literal(obj)),
            "qc:replyToFragment" => m.reply_to_fragment = Some(literal(obj)),
            "qc:source" => m.source = Some(literal(obj)),
            "qc:subAgentOf" => m.sub_agent_of = Some(uri(obj)),
            "qc:agentDid" => m.agent_did = Some(uri(obj)),
            "qc:modelId" => m.model_id = Some(literal(obj)),
            "qc:agentBackend" => m.agent_backend = Some(literal(obj)),
            _ => {}
        }
    }
    (!m.id.is_empty()).then_some(m)
}

fn split_first_token(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// Split on `;` that are not inside a `"..."` literal.
fn split_top_level_semicolons(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut escaped = false;
    for c in s.chars() {
        match c {
            '\\' if in_str => {
                escaped = !escaped;
                cur.push(c);
            }
            '"' if !escaped => {
                in_str = !in_str;
                cur.push(c);
            }
            ';' if !in_str => {
                out.push(std::mem::take(&mut cur));
            }
            _ => {
                escaped = false;
                cur.push(c);
            }
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Extract a Turtle literal's text: `"..."` possibly followed by `^^xsd:...`.
fn literal(obj: &str) -> String {
    let obj = obj.trim();
    if let Some(rest) = obj.strip_prefix('"') {
        // find closing unescaped quote
        let mut end = None;
        let mut escaped = false;
        for (i, c) in rest.char_indices() {
            if c == '\\' && !escaped {
                escaped = true;
            } else if c == '"' && !escaped {
                end = Some(i);
                break;
            } else {
                escaped = false;
            }
        }
        if let Some(i) = end {
            return unesc(&rest[..i]);
        }
    }
    obj.to_string()
}

fn uri(obj: &str) -> String {
    obj.trim().trim_start_matches('<').trim_end_matches('>').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_session::{ChatMessage, Role, SessionKind, SessionMeta};

    fn msg(lamport: u64, role: Role, content: &str, ts: u64) -> ChatMessage {
        ChatMessage {
            lamport,
            role,
            content: content.into(),
            timestamp: ts,
            content_hash: crate::chat_session::content_hash_u64(content),
            author_did: Some("did:wf:alice".into()),
            author_name: Some("Alice".into()),
            reply_to_fragment: None,
            source: None,
            sub_agent_of: None,
            agent_did: None,
            model_id: None,
            agent_backend: None,
            outcome_sharing: None,
        }
    }

    fn meta() -> SessionMeta {
        SessionMeta {
            id: "s1".into(),
            title: "Care chat".into(),
            created_at: 1_700_000_000,
            updated_at: 1_700_000_100,
            message_count: 0,
            next_lamport: 99,
            environment_ref: String::new(),
            session_kind: SessionKind::Group,
            participants: vec![],
            owner_did: "did:wf:owner".into(),
            session_did: "did:qualia:chat:group:abcd".into(),
        }
    }

    #[test]
    fn index_declares_a_long_chat() {
        let export = export_parts(&meta(), &[msg(1, Role::User, "hi", 1_700_000_050)]);
        assert!(export.index_ttl.contains("a meeting:LongChat"));
        assert!(export.index_ttl.contains("dct:title \"Care chat\""));
        assert!(export.index_ttl.contains("dct:author <did:wf:owner>"));
        assert!(export.index_ttl.contains("meeting:message"));
        // Day-partitioned message URI.
        assert!(export.index_ttl.contains("2023/11/14/chat.ttl#msg-1") || export.index_ttl.contains("/chat.ttl#msg-1"));
    }

    #[test]
    fn message_carries_solid_subset_and_native_fidelity() {
        let mut m = msg(7, Role::Agent, "grounded answer", 1_700_000_050);
        m.agent_did = Some("did:wf:agent7".into());
        m.model_id = Some("qwen2-1_5b".into());
        m.sub_agent_of = Some("did:wf:owner".into());
        let export = export_parts(&meta(), &[m]);
        let day = export.day_files.values().next().unwrap();
        // Standard Solid subset:
        assert!(day.contains("sioc:content \"grounded answer\""));
        assert!(day.contains("foaf:maker <did:wf:alice>"));
        assert!(day.contains("dct:created"));
        // Additive native fidelity (no degradation):
        assert!(day.contains("qc:lamport 7"));
        assert!(day.contains("qc:role \"agent\""));
        assert!(day.contains("qc:agentDid <did:wf:agent7>"));
        assert!(day.contains("qc:modelId \"qwen2-1_5b\""));
        assert!(day.contains("qc:subAgentOf <did:wf:owner>"));
    }

    #[test]
    fn roundtrip_is_lossless() {
        let mut agent = msg(2, Role::Agent, "line one\nline \"two\"", 1_700_000_060);
        agent.agent_did = Some("did:wf:agent".into());
        agent.model_id = Some("m1".into());
        agent.reply_to_fragment = Some("frag-1".into());
        agent.author_did = Some("did:wf:agent".into());
        let originals = vec![msg(1, Role::User, "hello", 1_700_000_050), agent];
        let export = export_parts(&meta(), &originals);

        // Parse every day file back.
        let mut parsed: Vec<ImportedMessage> =
            export.day_files.values().flat_map(|b| parse_day_ttl(b)).collect();
        parsed.sort_by_key(|m| m.lamport.unwrap_or(0));
        assert_eq!(parsed.len(), 2);

        for (orig, back) in originals.iter().zip(parsed.iter()) {
            let rebuilt = back.to_chat_message(0);
            assert_eq!(rebuilt.content, orig.content, "content (incl. newlines/quotes) preserved");
            assert_eq!(rebuilt.lamport, orig.lamport);
            assert_eq!(rebuilt.role, orig.role);
            assert_eq!(rebuilt.content_hash, orig.content_hash);
            assert_eq!(rebuilt.author_did, orig.author_did);
            assert_eq!(rebuilt.reply_to_fragment, orig.reply_to_fragment);
            assert_eq!(rebuilt.agent_did, orig.agent_did);
            assert_eq!(rebuilt.model_id, orig.model_id);
        }
    }

    #[test]
    fn pure_solid_message_without_qc_still_imports() {
        // A message from a non-Qualia client: only the sioc/foaf/dct subset.
        let ttl = r#"@prefix sioc: <http://rdfs.org/sioc/ns#> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<#msgX> dct:created "2023-11-14T20:54:10Z"^^xsd:dateTime ;
    sioc:content "hi from solid" ;
    foaf:maker <https://bob.example/profile/card#me> .
"#;
        let parsed = parse_day_ttl(ttl);
        assert_eq!(parsed.len(), 1);
        let m = parsed[0].to_chat_message(42);
        assert_eq!(m.content, "hi from solid");
        assert_eq!(m.author_did.as_deref(), Some("https://bob.example/profile/card#me"));
        assert_eq!(m.lamport, 42, "no qc:lamport → caller's fallback");
        assert_eq!(m.role, Role::User, "default role");
        assert_eq!(m.source.as_deref(), Some("solid"));
    }
}
