//! **Browser agent (P2 / B4)** — page-aware helper for the Webizen browser.
//!
//! Scope (honest):
//! - Structured intents: `summarise` | `trust` | `privacy` | `navigate_help` | `general`.
//! - Always return provenance + CML signals + trust verdict.
//! - Optionally ingest page topics into the hypermedia library.
//! - Deterministic grounded answers (no unbounded tools; 20s fetch timeout).
//! - Local LLM path is optional and not required for acceptance.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::webizen_trust::{self, TrustStore, TrustVerdict};
use crate::wellfair::cml_context::{
    build_document_context, units_from_headings, ContextUnit,
};
use crate::wellfair::hypermedia_store::{
    CommonsVisibility, HypermediaStore, LibraryEntry, LibrarySection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAgentIntent {
    Summarise,
    Trust,
    Privacy,
    NavigateHelp,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAgentRequest {
    pub url: String,
    pub question: String,
    /// When true, write a library entry under Work for page topics.
    #[serde(default)]
    pub ingest_to_library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAgentResponse {
    pub url: String,
    pub answer: String,
    pub intent: String,
    pub trust: TrustVerdict,
    pub cml_signals: Vec<String>,
    pub topics: Vec<String>,
    pub deontic_norms: usize,
    pub privacy_hits: usize,
    pub page_excerpt: String,
    pub provenance: Vec<String>,
    pub library_asset_uri: Option<String>,
    pub curation: String,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Classify the user question into a bounded intent.
pub fn classify_intent(question: &str) -> BrowserAgentIntent {
    let q = question.trim().to_ascii_lowercase();
    if q.is_empty() {
        return BrowserAgentIntent::Summarise;
    }
    if q.contains("trust")
        || q.contains("certificate")
        || q.contains("secure")
        || q.contains("trusted")
        || q.contains("is this safe")
    {
        return BrowserAgentIntent::Trust;
    }
    if q.contains("privacy")
        || q.contains("gdpr")
        || q.contains("personal data")
        || q.contains("cookie")
        || q.contains("tracker")
    {
        return BrowserAgentIntent::Privacy;
    }
    if q.contains("how do i")
        || q.contains("navigate")
        || q.contains("where is")
        || q.contains("open ")
        || q.contains("bookmark")
    {
        return BrowserAgentIntent::NavigateHelp;
    }
    if q.contains("about")
        || q.contains("summar")
        || q.contains("what is")
        || q.contains("page")
        || q.contains("tell me")
    {
        return BrowserAgentIntent::Summarise;
    }
    BrowserAgentIntent::General
}

/// Fetch page body (best-effort HTML → text). Uses system TLS; custom PEM roots
/// are noted in the answer for agent awareness when present.
pub async fn fetch_page_text(url: &str) -> Result<String, String> {
    let u = url.trim();
    if u.starts_with("qualia://") || u.starts_with("webizen://") {
        return Ok(format!(
            "Local Webizen resource: {u}\n(Content is served by the desktop protocol handler, not HTTP.)"
        ));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("WebizenBrowserAgent/0.0.25 (+https://ns.webcivics.net)")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(u).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(html_to_text(&html))
}

fn html_to_text(html: &str) -> String {
    let mut text = String::new();
    let doc = scraper::Html::parse_document(html);
    let sel = scraper::Selector::parse("body").ok();
    if let Some(sel) = sel {
        if let Some(body) = doc.select(&sel).next() {
            text = body.text().collect::<Vec<_>>().join(" ");
        }
    }
    if text.trim().is_empty() {
        let mut out = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => out.push(c),
                _ => {}
            }
        }
        text = out;
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_answer(
    intent: BrowserAgentIntent,
    question: &str,
    url: &str,
    page: &str,
    trust: &TrustVerdict,
    signals: &[String],
    topics: &[String],
) -> String {
    let excerpt: String = page.chars().take(600).collect();
    let signal_join = |cap: usize| {
        if signals.is_empty() && topics.is_empty() {
            "(none extracted)".into()
        } else {
            signals
                .iter()
                .chain(topics.iter())
                .take(cap)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        }
    };

    match intent {
        BrowserAgentIntent::Trust => format!(
            "Trust verdict for {url}: **{}** — {}\n\
             Matching anchors: {}\n\
             Notes: {}\n\
             (WebView TLS still uses the OS store unless the platform cert-override is active; \
             this verdict is Webizen policy + store.)",
            trust.level,
            trust.summary,
            if trust.matching_anchors.is_empty() {
                "none".into()
            } else {
                trust.matching_anchors.join(", ")
            },
            trust.notes.join(" · ")
        ),
        BrowserAgentIntent::Summarise => format!(
            "Page: {url}\n\
             Trust: {} ({})\n\
             Topics/signals: {}\n\
             Excerpt (grounded in fetched text):\n{excerpt}{}",
            trust.level,
            trust.summary,
            signal_join(16),
            if page.len() > 600 { "…" } else { "" }
        ),
        BrowserAgentIntent::Privacy => {
            let privacy_sigs: Vec<_> = signals
                .iter()
                .filter(|s| s.starts_with("privacy:"))
                .cloned()
                .collect();
            format!(
                "Privacy signals on {url}: {}\n\
                 Trust: {}.\n\
                 Grounded excerpt: {excerpt}{}",
                if privacy_sigs.is_empty() {
                    "none detected by deterministic extractors (privacy:* tags)".into()
                } else {
                    privacy_sigs.join(", ")
                },
                trust.level,
                if page.len() > 600 { "…" } else { "" }
            )
        }
        BrowserAgentIntent::NavigateHelp => format!(
            "Webizen Browser help for {url}:\n\
             · Omnibox Go / back / forward / reload in chrome\n\
             · 🔖 saves a bookmark (qlinks + Library purpose=bookmark)\n\
             · Trust panel manages your DID/PEM store (agent policy; OS TLS separate)\n\
             · Agent answers summarise / trust / privacy about the current page\n\
             Question was: {question}\n\
             Current trust: {} — {}",
            trust.level, trust.summary
        ),
        BrowserAgentIntent::General => format!(
            "Question: {question}\n\
             URL: {url}\n\
             Trust: {} — {}\n\
             Detected: {}\n\
             Grounded excerpt:\n{excerpt}{}\n\
             Provenance: fetched page text + CML context extractors + Webizen trust store (cml:Proposed).",
            trust.level,
            trust.summary,
            signal_join(12),
            if page.len() > 600 { "…" } else { "" }
        ),
    }
}

/// Run the browser agent against the current page.
pub async fn run_browser_agent(
    storage_root: &Path,
    req: BrowserAgentRequest,
) -> Result<BrowserAgentResponse, String> {
    let intent = classify_intent(&req.question);
    let store = TrustStore::load(storage_root);
    let trust = webizen_trust::evaluate_url(&store, &req.url);
    let page = fetch_page_text(&req.url)
        .await
        .unwrap_or_else(|e| format!("(fetch failed: {e})"));

    let units = if page.len() > 80 {
        units_from_headings(&page)
    } else {
        vec![ContextUnit {
            frag: "page".into(),
            kind: "document".into(),
            label: req.url.clone(),
            text: page.clone(),
            page: None,
            parent: None,
        }]
    };
    let g = build_document_context(&req.url, &req.url, &units);

    let answer = build_answer(
        intent,
        &req.question,
        &req.url,
        &page,
        &trust,
        &g.signal_tags,
        &g.topics,
    );

    let mut library_asset_uri = None;
    if req.ingest_to_library && page.len() > 40 {
        let store_lib = HypermediaStore::open(storage_root).map_err(|e| e.to_string())?;
        let uri = format!("urn:webizen:browser-page:{}", short_id(req.url.as_bytes()));
        let mut entry = LibraryEntry {
            asset_uri: uri.clone(),
            primary_subject: fnv60(uri.as_bytes()),
            media_type: "text/html".into(),
            quins: g.quins.clone(),
            topics: g.topics.clone(),
            projects: vec!["browser".into()],
            purposes: {
                let mut p = g.purposes.clone();
                p.push("browser".into());
                p.push("research".into());
                p
            },
            place: None,
            occurred_at: None,
            lat: None,
            lon: None,
            flags: Vec::new(),
            ingested_unix: now_unix(),
            excerpt: page.chars().take(400).collect(),
            sensitivity: "public".into(),
            section: LibrarySection::Work.as_str().into(),
            commons_visibility: CommonsVisibility::None,
            cml_signals: g.signal_tags.clone(),
            cml_concept_count: g.concepts.len() as u32,
            cml_n3: g.n3.chars().take(24_000).collect(),
            cof_html: String::new(),
            cof_segment_count: 0,
            cof_segment_index: 0,
            cof_profile: String::new(),
        };
        entry.recompute_section();
        store_lib.add(entry).map_err(|e| e.to_string())?;
        library_asset_uri = Some(uri);
    }

    let intent_str = match intent {
        BrowserAgentIntent::Summarise => "summarise",
        BrowserAgentIntent::Trust => "trust",
        BrowserAgentIntent::Privacy => "privacy",
        BrowserAgentIntent::NavigateHelp => "navigate_help",
        BrowserAgentIntent::General => "general",
    };

    Ok(BrowserAgentResponse {
        url: req.url,
        answer,
        intent: intent_str.into(),
        trust,
        cml_signals: g.signal_tags,
        topics: g.topics,
        deontic_norms: g.deontic_norms,
        privacy_hits: g.privacy_hits,
        page_excerpt: page.chars().take(800).collect(),
        provenance: vec![
            "page-fetch".into(),
            "cml_context".into(),
            "webizen_trust".into(),
            "cml:Proposed".into(),
            format!("intent:{intent_str}"),
        ],
        library_asset_uri,
        curation: "cml:Proposed".into(),
    })
}

fn short_id(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(&h.finalize()[..10])
}

fn fnv60(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_classify() {
        assert_eq!(classify_intent("Is this trusted?"), BrowserAgentIntent::Trust);
        assert_eq!(
            classify_intent("Privacy signals?"),
            BrowserAgentIntent::Privacy
        );
        assert_eq!(
            classify_intent("What is this page about?"),
            BrowserAgentIntent::Summarise
        );
        assert_eq!(
            classify_intent("How do I bookmark?"),
            BrowserAgentIntent::NavigateHelp
        );
    }
}
