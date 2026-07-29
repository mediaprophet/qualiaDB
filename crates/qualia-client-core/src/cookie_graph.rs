//! Cookie / personal-data transparency graph (v0).
//!
//! Honest scope:
//! - Model: site origin → cookie name → attributes → purpose hypothesis (heuristic).
//! - Persist under `{storage}/webizen/cookie_graph.json`.
//! - Observe: v0 accepts agent-fetched `Set-Cookie` headers and/or explicit observations
//!   from the host. Full WebView2 cookie-jar parity is **not** claimed.
//! - No MITM proxy.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const COOKIE_GRAPH_FILE: &str = "webizen/cookie_graph.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CookiePurposeHypothesis {
    Session,
    Analytics,
    Tracker,
    Preference,
    Auth,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieNode {
    pub origin: String,
    pub name: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub same_site: String,
    pub expiry: Option<String>,
    pub purpose: CookiePurposeHypothesis,
    pub third_party: bool,
    /// How we learned about this cookie (agent_set_cookie | host_observe | manual).
    pub source: String,
    pub observed_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CookieGraph {
    pub version: u32,
    pub nodes: Vec<CookieNode>,
    /// Honest coverage label for UI.
    pub coverage_note: String,
}

impl CookieGraph {
    pub fn new() -> Self {
        Self {
            version: 1,
            nodes: Vec::new(),
            coverage_note: "v1: WebView jar (cookies_for_url) + agent Set-Cookie observe — not complete Chromium parity.".into(),
        }
    }

    pub fn path(storage_root: &Path) -> PathBuf {
        storage_root.join(COOKIE_GRAPH_FILE)
    }

    pub fn load(storage_root: &Path) -> Self {
        let p = Self::path(storage_root);
        match fs::read_to_string(&p) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self, storage_root: &Path) -> Result<(), String> {
        let p = Self::path(storage_root);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = p.with_extension("json.tmp");
        fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &p).map_err(|e| e.to_string())
    }

    pub fn upsert(&mut self, node: CookieNode) {
        if let Some(existing) = self
            .nodes
            .iter_mut()
            .find(|n| n.origin == node.origin && n.name == node.name && n.domain == node.domain)
        {
            *existing = node;
        } else {
            self.nodes.push(node);
        }
    }

    pub fn for_origin(&self, origin: &str) -> Vec<&CookieNode> {
        let o = origin.trim().trim_end_matches('/');
        self.nodes
            .iter()
            .filter(|n| n.origin.trim_end_matches('/') == o)
            .collect()
    }

    pub fn third_parties_for_host(&self, host: &str) -> Vec<String> {
        let host = host.trim().to_ascii_lowercase();
        let mut out = Vec::new();
        for n in &self.nodes {
            if !n.third_party {
                continue;
            }
            let page_host = host_of(&n.origin);
            if page_host == host || n.origin.contains(&host) {
                let d = n.domain.trim_start_matches('.').to_ascii_lowercase();
                if !d.is_empty() && d != host && !out.contains(&d) {
                    out.push(d);
                }
            }
        }
        out.sort();
        out
    }

    /// Remove all graph nodes for an origin (exact match, trailing slash normalized).
    pub fn clear_origin(&mut self, origin: &str) -> usize {
        let o = origin.trim().trim_end_matches('/');
        let before = self.nodes.len();
        self.nodes.retain(|n| n.origin.trim_end_matches('/') != o);
        before.saturating_sub(self.nodes.len())
    }

    /// Remove nodes whose page host matches (origin host or domain).
    pub fn clear_host(&mut self, host: &str) -> usize {
        let host = host.trim().to_ascii_lowercase();
        let before = self.nodes.len();
        self.nodes.retain(|n| {
            let page = host_of(&n.origin);
            let d = n.domain.trim_start_matches('.').to_ascii_lowercase();
            page != host && d != host
        });
        before.saturating_sub(self.nodes.len())
    }

    /// Clear entire graph (principal request).
    pub fn clear_all(&mut self) -> usize {
        let n = self.nodes.len();
        self.nodes.clear();
        n
    }
}

fn host_of(url_or_origin: &str) -> String {
    let u = url_or_origin.trim();
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .unwrap_or(u);
    rest.split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Heuristic purpose from cookie name (not ground truth).
pub fn hypothesize_purpose(name: &str) -> CookiePurposeHypothesis {
    let n = name.to_ascii_lowercase();
    if n.contains("session") || n == "sid" || n.starts_with("jsession") {
        return CookiePurposeHypothesis::Session;
    }
    if n.contains("auth") || n.contains("token") || n.contains("login") || n == "jwt" {
        return CookiePurposeHypothesis::Auth;
    }
    if n.contains("ga")
        || n.contains("_utm")
        || n.contains("analytics")
        || n.starts_with("_gid")
        || n.starts_with("_gat")
    {
        return CookiePurposeHypothesis::Analytics;
    }
    if n.contains("fbp")
        || n.contains("fbc")
        || n.contains("doubleclick")
        || n.contains("track")
        || n.contains("ads")
    {
        return CookiePurposeHypothesis::Tracker;
    }
    if n.contains("pref") || n.contains("theme") || n.contains("lang") || n.contains("consent") {
        return CookiePurposeHypothesis::Preference;
    }
    CookiePurposeHypothesis::Unknown
}

/// Parse a single Set-Cookie header line into a CookieNode (best-effort).
pub fn parse_set_cookie(
    page_url: &str,
    set_cookie: &str,
    now: u64,
    source: &str,
) -> Option<CookieNode> {
    let line = set_cookie.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split(';');
    let nv = parts.next()?.trim();
    let (name, _value) = nv.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let page_host = host_of(page_url);
    let origin = if page_url.starts_with("http") {
        // scheme://host
        let scheme = if page_url.starts_with("https") {
            "https"
        } else {
            "http"
        };
        format!("{scheme}://{page_host}")
    } else {
        page_url.to_string()
    };

    let mut domain = page_host.clone();
    let mut path = "/".to_string();
    let mut secure = false;
    let mut same_site = "Lax".to_string();
    let mut expiry = None;
    for p in parts {
        let p = p.trim();
        let (k, v) = match p.split_once('=') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (p, ""),
        };
        let kl = k.to_ascii_lowercase();
        match kl.as_str() {
            "domain" => domain = v.trim_start_matches('.').to_ascii_lowercase(),
            "path" => path = if v.is_empty() { "/".into() } else { v.into() },
            "secure" => secure = true,
            "samesite" => same_site = v.to_string(),
            "expires" | "max-age" => expiry = Some(v.to_string()),
            _ => {}
        }
    }
    let third_party = {
        let d = domain.trim_start_matches('.');
        !d.is_empty() && d != page_host && !page_host.ends_with(&format!(".{d}"))
    };
    Some(CookieNode {
        origin,
        name: name.into(),
        domain,
        path,
        secure,
        same_site,
        expiry,
        purpose: hypothesize_purpose(name),
        third_party,
        source: source.into(),
        observed_unix: now,
    })
}

/// Observe many Set-Cookie lines for a page URL; persist graph.
pub fn observe_set_cookies(
    storage_root: &Path,
    page_url: &str,
    set_cookies: &[String],
    now: u64,
) -> Result<CookieGraph, String> {
    let mut g = CookieGraph::load(storage_root);
    for sc in set_cookies {
        if let Some(node) = parse_set_cookie(page_url, sc, now, "agent_set_cookie") {
            g.upsert(node);
        }
    }
    g.save(storage_root)?;
    Ok(g)
}

/// Summary for chrome UI.
pub fn summary_for_url(storage_root: &Path, url: &str) -> serde_json::Value {
    let g = CookieGraph::load(storage_root);
    let host = host_of(url);
    let origin = if url.starts_with("https") {
        format!("https://{host}")
    } else if url.starts_with("http") {
        format!("http://{host}")
    } else {
        url.to_string()
    };
    let nodes = g.for_origin(&origin);
    let third = g.third_parties_for_host(&host);
    serde_json::json!({
        "url": url,
        "origin": origin,
        "cookie_count": nodes.len(),
        "cookies": nodes,
        "third_parties": third,
        "coverage_note": g.coverage_note,
        "honesty": "view + graph coverage is best-effort; not complete Chromium jar parity",
    })
}

/// Clear cookie graph for origin and optionally log intent for site-data clear.
/// Does **not** by itself wipe the WebView jar — pair with platform clear.
pub fn clear_graph_for_origin(
    storage_root: &Path,
    origin_or_url: &str,
) -> Result<serde_json::Value, String> {
    let host = host_of(origin_or_url);
    let origin = if origin_or_url.starts_with("https") {
        format!("https://{host}")
    } else if origin_or_url.starts_with("http") {
        format!("http://{host}")
    } else if origin_or_url.contains("://") {
        origin_or_url.trim().trim_end_matches('/').to_string()
    } else {
        format!("https://{host}")
    };
    let mut g = CookieGraph::load(storage_root);
    let removed = g.clear_origin(&origin);
    // Also drop host-matching third-party rows observed under this page host
    let removed2 = g.clear_host(&host);
    g.save(storage_root)?;
    append_clear_audit(storage_root, &origin, removed + removed2, "origin")?;
    Ok(serde_json::json!({
        "origin": origin,
        "host": host,
        "removed_graph_nodes": removed + removed2,
        "coverage_note": g.coverage_note,
        "note": "Graph rows cleared. WebView jar clear is platform-side (see browser_clear_site_data).",
    }))
}

pub fn clear_graph_all(storage_root: &Path) -> Result<serde_json::Value, String> {
    let mut g = CookieGraph::load(storage_root);
    let n = g.clear_all();
    g.save(storage_root)?;
    append_clear_audit(storage_root, "*", n, "all")?;
    Ok(serde_json::json!({
        "removed_graph_nodes": n,
        "note": "Entire cookie graph cleared. WebView jar may still hold cookies until platform clear.",
    }))
}

fn append_clear_audit(
    storage_root: &Path,
    scope: &str,
    removed: usize,
    kind: &str,
) -> Result<(), String> {
    let path = storage_root.join("webizen/cookie_clear_audit.jsonl");
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = serde_json::json!({
        "unix": unix,
        "scope": scope,
        "kind": kind,
        "removed": removed,
    });
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(
        f,
        "{}",
        serde_json::to_string(&line).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let sc = "SID=abc; Domain=example.org; Path=/; Secure; SameSite=None".to_string();
        let g = observe_set_cookies(dir.path(), "https://example.org/page", &[sc], 1).unwrap();
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].name, "SID");
        assert_eq!(g.nodes[0].purpose, CookiePurposeHypothesis::Session);
        let loaded = CookieGraph::load(dir.path());
        assert_eq!(loaded.nodes.len(), 1);
        let sum = summary_for_url(dir.path(), "https://example.org/x");
        assert_eq!(sum["cookie_count"], 1);
    }

    #[test]
    fn ga_is_analytics() {
        assert_eq!(
            hypothesize_purpose("_ga"),
            CookiePurposeHypothesis::Analytics
        );
    }

    #[test]
    fn clear_origin_removes_rows() {
        let dir = tempfile::tempdir().unwrap();
        let sc = "SID=abc; Domain=example.org; Path=/".to_string();
        observe_set_cookies(dir.path(), "https://example.org/page", &[sc], 1).unwrap();
        let r = clear_graph_for_origin(dir.path(), "https://example.org/x").unwrap();
        assert!(r["removed_graph_nodes"].as_u64().unwrap() >= 1);
        let g = CookieGraph::load(dir.path());
        assert!(g.nodes.is_empty());
    }
}
