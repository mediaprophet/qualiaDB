//! Solid **consumer** (relying party / personal agent side).
//!
//! Fetches LDP resources from a remote (or local) Solid pod and converts Turtle
//! into NQuins for Qualia import. Also PUTs Turtle to a pod (egress deposit /
//! sync-to-pod).

use qualia_core_db::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub quin_count: usize,
}

#[derive(Debug)]
pub enum ConsumerError {
    Http(String),
    Status(u16, String),
    Empty,
}

impl std::fmt::Display for ConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsumerError::Http(e) => write!(f, "http: {e}"),
            ConsumerError::Status(c, b) => write!(f, "status {c}: {b}"),
            ConsumerError::Empty => write!(f, "empty body"),
        }
    }
}

impl std::error::Error for ConsumerError {}

/// GET a Solid resource (Turtle preferred). Optional Bearer access token.
pub async fn fetch_resource(
    url: &str,
    bearer: Option<&str>,
) -> Result<FetchResult, ConsumerError> {
    let client = reqwest::Client::builder()
        .user_agent("QualiaSolidBridge/0.0.23 (Webizen consumer agent)")
        .build()
        .map_err(|e| ConsumerError::Http(e.to_string()))?;

    let mut req = client
        .get(url)
        .header(
            "Accept",
            "text/turtle, application/ld+json;q=0.9, application/rdf+xml;q=0.8, */*;q=0.1",
        );
    if let Some(tok) = bearer {
        req = req.bearer_auth(tok);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| ConsumerError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let body = resp
        .text()
        .await
        .map_err(|e| ConsumerError::Http(e.to_string()))?;

    if !(200..300).contains(&status) {
        return Err(ConsumerError::Status(
            status,
            body.chars().take(400).collect(),
        ));
    }
    if body.trim().is_empty() {
        return Err(ConsumerError::Empty);
    }

    let quins = turtle_to_quins(&body);
    Ok(FetchResult {
        url: url.to_string(),
        status,
        content_type,
        body,
        quin_count: quins.len(),
    })
}

/// PUT Turtle (or other RDF) to a Solid resource URL.
pub async fn put_resource(
    url: &str,
    body: &[u8],
    content_type: &str,
    bearer: Option<&str>,
) -> Result<u16, ConsumerError> {
    let client = reqwest::Client::builder()
        .user_agent("QualiaSolidBridge/0.0.23 (Webizen consumer agent)")
        .build()
        .map_err(|e| ConsumerError::Http(e.to_string()))?;

    let mut req = client
        .put(url)
        .header("Content-Type", content_type)
        .header("Link", r#"<http://www.w3.org/ns/ldp#Resource>; rel="type""#)
        .body(body.to_vec());
    if let Some(tok) = bearer {
        req = req.bearer_auth(tok);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| ConsumerError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let t = resp.text().await.unwrap_or_default();
        return Err(ConsumerError::Status(
            status,
            t.chars().take(400).collect(),
        ));
    }
    Ok(status)
}

/// POST into an LDP container (create resource). Returns Location if present.
pub async fn post_to_container(
    container_url: &str,
    body: &[u8],
    content_type: &str,
    slug: Option<&str>,
    bearer: Option<&str>,
) -> Result<(u16, Option<String>), ConsumerError> {
    let client = reqwest::Client::builder()
        .user_agent("QualiaSolidBridge/0.0.23 (Webizen consumer agent)")
        .build()
        .map_err(|e| ConsumerError::Http(e.to_string()))?;

    let mut req = client
        .post(container_url)
        .header("Content-Type", content_type)
        .header(
            "Link",
            r#"<http://www.w3.org/ns/ldp#Resource>; rel="type""#,
        )
        .body(body.to_vec());
    if let Some(s) = slug {
        req = req.header("Slug", s);
    }
    if let Some(tok) = bearer {
        req = req.bearer_auth(tok);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| ConsumerError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if !(200..300).contains(&status) {
        let t = resp.text().await.unwrap_or_default();
        return Err(ConsumerError::Status(
            status,
            t.chars().take(400).collect(),
        ));
    }
    Ok((status, location))
}

/// Best-effort Turtle statements → NQuins (allocation firewall boundary).
///
/// Accumulates multi-line statements until `.`, then parses terms of the form
/// `<s> <p> <o>` / `s p o` (ignores `@prefix`). Full RDF parsers remain cold-path;
/// this covers Qualia export round-trip and simple VC-in-Turtle deposits.
pub fn turtle_to_quins(turtle: &str) -> Vec<NQuin> {
    let mut out = Vec::new();
    let mut stmt = String::new();
    for line in turtle.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('@') || t.starts_with('#') {
            continue;
        }
        // Strip trailing comment outside strings (best-effort)
        let t = t.split('#').next().unwrap_or(t).trim();
        if !stmt.is_empty() {
            stmt.push(' ');
        }
        stmt.push_str(t);
        if !stmt.ends_with('.') {
            continue;
        }
        let body = stmt.trim_end_matches('.').trim().to_string();
        stmt.clear();
        // Expand `;` predicate lists: subject p1 o1 ; p2 o2
        let subject_and_rest = split_turtle_terms(&body);
        if subject_and_rest.len() < 3 {
            continue;
        }
        // Re-scan body for `;` separated predicate-object pairs after first subject
        for triple in expand_turtle_statement(&body) {
            if triple.len() < 3 {
                continue;
            }
            let subject = hash_term(&triple[0]);
            let predicate = hash_term(&triple[1]);
            let object = hash_term(&triple[2]);
            let context = q_hash("solid:import");
            let metadata = 0x4000_0000_0000_0000u64;
            let mut q = NQuin {
                subject,
                predicate,
                object,
                context,
                metadata,
                parity: 0,
            };
            q.recalculate_parity();
            out.push(q);
        }
    }
    out
}

/// Expand one Turtle statement body (no trailing `.`) into SPO triples.
fn expand_turtle_statement(body: &str) -> Vec<Vec<String>> {
    let terms = split_turtle_terms(body);
    if terms.len() < 3 {
        return Vec::new();
    }
    let mut triples = Vec::new();
    let subject = terms[0].clone();
    // Walk: p o [; p o]*
    let mut i = 1;
    while i + 1 < terms.len() {
        if terms[i] == ";" {
            i += 1;
            continue;
        }
        let pred = terms[i].clone();
        let obj = terms[i + 1].clone();
        triples.push(vec![subject.clone(), pred, obj]);
        i += 2;
        if i < terms.len() && terms[i] == ";" {
            i += 1;
        }
    }
    if triples.is_empty() && terms.len() >= 3 {
        triples.push(vec![terms[0].clone(), terms[1].clone(), terms[2].clone()]);
    }
    triples
}

fn hash_term(term: &str) -> u64 {
    let t = term.trim();
    if t.starts_with('<') && t.ends_with('>') {
        let iri = &t[1..t.len() - 1];
        return q_hash(iri);
    }
    if t.starts_with('"') {
        // literal — hash full token
        return q_hash(t);
    }
    // prefixed name or bare
    q_hash(t)
}

/// Split a triple line into terms, respecting angle brackets and quotes.
fn split_turtle_terms(line: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' | '\r'
                if !cur.is_empty() && !cur.starts_with('<') && !cur.starts_with('"') =>
            {
                terms.push(std::mem::take(&mut cur));
            }
            ' ' | '\t' | '\n' | '\r' if cur.is_empty() => {}
            ';' if cur.is_empty() => terms.push(";".into()),
            '<' => {
                if !cur.is_empty() {
                    terms.push(std::mem::take(&mut cur));
                }
                cur.push('<');
                for c2 in chars.by_ref() {
                    cur.push(c2);
                    if c2 == '>' {
                        break;
                    }
                }
                terms.push(std::mem::take(&mut cur));
            }
            '"' => {
                if !cur.is_empty() {
                    terms.push(std::mem::take(&mut cur));
                }
                cur.push('"');
                for c2 in chars.by_ref() {
                    cur.push(c2);
                    if c2 == '"' {
                        break;
                    }
                }
                terms.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        terms.push(cur);
    }
    terms
}

/// Import fetch result into a caller-owned quin buffer (cold path; returns count).
pub fn fetch_to_quin_buffer(result: &FetchResult, out: &mut [NQuin]) -> usize {
    let quins = turtle_to_quins(&result.body);
    let n = quins.len().min(out.len());
    out[..n].copy_from_slice(&quins[..n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_qualia_export_style_triples() {
        let ttl = r#"
@prefix qualia: <urn:qualia:schema:> .
<urn:qualia:node:1> <urn:qualia:pred:2> <urn:qualia:node:3> . # Context: 9
<urn:qualia:node:4> <urn:qualia:pred:5> <urn:qualia:node:6> .
"#;
        let quins = turtle_to_quins(ttl);
        assert_eq!(quins.len(), 2);
        assert!(quins[0].verify_ecc_parity());
        assert_eq!(quins[0].subject, q_hash("urn:qualia:node:1"));
    }

    #[test]
    fn parses_multiline_semicolon_statement() {
        let ttl = r#"
<#deposit> a <https://www.w3.org/2018/credentials#VerifiableCredential> ;
  <http://purl.org/dc/terms/title> "Hackathon smoke deposit" ;
  <http://purl.org/dc/terms/creator> <urn:institution:demo> .
"#;
        let quins = turtle_to_quins(ttl);
        assert!(quins.len() >= 2, "got {}", quins.len());
        assert!(quins.iter().all(|q| q.verify_ecc_parity()));
    }

    #[test]
    fn ignores_prefix_only_lines() {
        let quins = turtle_to_quins("@prefix foaf: <http://xmlns.com/foaf/0.1/> .\n");
        assert!(quins.is_empty());
    }
}
