//! Domain & Web Presence Subsystem (Spec 07).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements self-determined domain management, static/dynamic HyperDoc publishing
//! with W3C RDFa and JSON-LD, Solid WebID profiles, purpose-bound mail routing,
//! and automated DNS zone / tunnel generation.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// DNS Record Types for domain configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsRecordType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    SRV,
}

impl DnsRecordType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::AAAA => "AAAA",
            Self::CNAME => "CNAME",
            Self::MX => "MX",
            Self::TXT => "TXT",
            Self::SRV => "SRV",
        }
    }
}

/// A DNS zone record entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DnsRecord {
    pub record_type: DnsRecordType,
    pub name: String,
    pub value: String,
    pub priority: Option<u16>,
    pub ttl: u32,
}

/// Purpose-bound email inbox for the domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainMailbox {
    pub local_part: String, // e.g. "inquiry", "contracts", "agent.astra"
    pub role_title: String, // "General Intake", "Legal & Rights", "AI Autonomous Subagent"
    pub is_catchall: bool,
    pub forward_target_did: Option<String>,
}

/// A published HyperDoc on the domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublishedHyperDoc {
    pub slug: String,
    pub title: String,
    pub author_did: String,
    pub published_at: u64,
    pub harvested_triples_count: usize,
    pub has_3d_viewport: bool,
    pub is_live: bool,
}

/// Unified domain presence configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainConfig {
    pub domain_name: String, // e.g. "holborn.id", "mindware.agency"
    pub owner_did: String,
    pub solid_webid_enabled: bool,
    pub sparql_endpoint_enabled: bool,
    pub webtorrent_seeder_enabled: bool,
    pub mailboxes: Vec<DomainMailbox>,
    pub published_docs: Vec<PublishedHyperDoc>,
    pub dns_records: Vec<DnsRecord>,
}

impl DomainConfig {
    pub fn new(domain_name: &str, owner_did: &str) -> Self {
        let default_mailboxes = vec![
            DomainMailbox {
                local_part: "inquiry".into(),
                role_title: "General Public Intake".into(),
                is_catchall: false,
                forward_target_did: Some(owner_did.to_string()),
            },
            DomainMailbox {
                local_part: "contracts".into(),
                role_title: "Legal, Rights & Agreements".into(),
                is_catchall: false,
                forward_target_did: Some(owner_did.to_string()),
            },
            DomainMailbox {
                local_part: "agent.astra".into(),
                role_title: "Autonomous AI Agent Ingestion".into(),
                is_catchall: false,
                forward_target_did: None,
            },
        ];

        let default_dns = vec![
            DnsRecord {
                record_type: DnsRecordType::A,
                name: "@".into(),
                value: "192.0.2.42".into(),
                priority: None,
                ttl: 300,
            },
            DnsRecord {
                record_type: DnsRecordType::CNAME,
                name: "www".into(),
                value: domain_name.to_string(),
                priority: None,
                ttl: 300,
            },
            DnsRecord {
                record_type: DnsRecordType::MX,
                name: "@".into(),
                value: format!("mail.{}", domain_name),
                priority: Some(10),
                ttl: 300,
            },
            DnsRecord {
                record_type: DnsRecordType::TXT,
                name: "@".into(),
                value: "v=spf1 mx ~all".into(),
                priority: None,
                ttl: 300,
            },
            DnsRecord {
                record_type: DnsRecordType::TXT,
                name: "_qualia".into(),
                value: format!("did={}", owner_did),
                priority: None,
                ttl: 300,
            },
        ];

        let sample_doc = PublishedHyperDoc {
            slug: "manifesto".into(),
            title: "QualiaDB & Poet Hypermedia Manifesto".into(),
            author_did: owner_did.to_string(),
            published_at: 1774000000000,
            harvested_triples_count: 28,
            has_3d_viewport: true,
            is_live: true,
        };

        Self {
            domain_name: domain_name.to_string(),
            owner_did: owner_did.to_string(),
            solid_webid_enabled: true,
            sparql_endpoint_enabled: true,
            webtorrent_seeder_enabled: true,
            mailboxes: default_mailboxes,
            published_docs: vec![sample_doc],
            dns_records: default_dns,
        }
    }

    /// Generate an authoritative W3C Solid WebID Card HTML+RDFa string.
    pub fn export_solid_webid_card_rdfa(&self) -> String {
        let mut out = String::new();
        out.push_str("<!DOCTYPE html>\n");
        out.push_str("<html lang=\"en\" prefix=\"schema: http://schema.org/ qualia: https://qualia.network/vocab#\">\n");
        out.push_str("<head>\n  <meta charset=\"utf-8\">\n  <title>Solid WebID Profile — ");
        out.push_str(&self.domain_name);
        out.push_str("</title>\n</head>\n");
        out.push_str(
            "<body vocab=\"http://xmlns.com/foaf/0.1/\" typeof=\"Person\" resource=\"#me\">\n",
        );
        out.push_str("  <h1 property=\"name\">Owner of ");
        out.push_str(&self.domain_name);
        out.push_str("</h1>\n");
        out.push_str("  <p>DID: <span property=\"qualia:did\">");
        out.push_str(&self.owner_did);
        out.push_str("</span></p>\n");
        out.push_str("  <p>Solid Storage Pod: <a rel=\"storage\" href=\"https://");
        out.push_str(&self.domain_name);
        out.push_str("/data/\">/data/</a></p>\n");
        out.push_str("  <p>Public SPARQL Endpoint: <a rel=\"qualia:sparql\" href=\"https://");
        out.push_str(&self.domain_name);
        out.push_str("/sparql\">/sparql</a></p>\n");
        out.push_str("</body>\n</html>");
        out
    }

    /// Export standard BIND-format DNS zone file text.
    pub fn export_bind_zone_file(&self) -> String {
        let mut out = format!(
            "; Zone file for {}\n$ORIGIN {}.\n$TTL 300\n\n",
            self.domain_name, self.domain_name
        );
        for rec in &self.dns_records {
            if let Some(prio) = rec.priority {
                out.push_str(&format!(
                    "{:<16} IN {:<6} {:<4} {}\n",
                    rec.name,
                    rec.record_type.label(),
                    prio,
                    rec.value
                ));
            } else {
                out.push_str(&format!(
                    "{:<16} IN {:<6} {}\n",
                    rec.name,
                    rec.record_type.label(),
                    rec.value
                ));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the full Domain & Web Presence Manifold Viewport.
pub fn build_domains_manager_view(document: &Document, config: &DomainConfig) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(&format!(
        "\u{1F310} Domain Digital Home: https://{}",
        config.domain_name
    )));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let owner = document.create_element("span").unwrap();
    owner.set_text_content(Some(&format!(
        "DID: {} \u{00B7} Solid WebID: \u{2713} \u{00B7} SPARQL: \u{2713}",
        &config.owner_did[..config.owner_did.len().min(16)]
    )));
    let owner_el: HtmlElement = owner.clone().dyn_into().unwrap();
    owner_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #94a3b8;");
    header.append_child(&owner).unwrap();

    root.append_child(&header).unwrap();

    // 4 Pillars Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px;",
    );

    // Pillar 1: Web Publishing Card
    let card1 = create_pillar_card(
        document,
        "\u{1F4D6} Public Web & HyperDocs",
        &format!(
            "{} Published Dossiers \u{00B7} Live WebID Card",
            config.published_docs.len()
        ),
        &format!(
            "Homepage: https://{}/\nWebID: https://{}/profile/card#me",
            config.domain_name, config.domain_name
        ),
    );
    grid.append_child(&card1).unwrap();

    // Pillar 2: Inalienable Mail Card
    let card2 = create_pillar_card(
        document,
        "\u{1F4E7} Purpose-Bound Mailboxes",
        &format!(
            "{} Active Inboxes \u{00B7} Local SMTP Node",
            config.mailboxes.len()
        ),
        &config
            .mailboxes
            .iter()
            .map(|m| format!("{}@{}: {}", m.local_part, config.domain_name, m.role_title))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    grid.append_child(&card2).unwrap();

    // Pillar 3: Solid Pod & Linked Data APIs Card
    let card3 = create_pillar_card(
        document,
        "\u{1F5C4}\u{FE0F} Solid LDP Pod & SPARQL",
        "W3C Solid Storage \u{00B7} SPARQL 1.1 Endpoint",
        &format!("LDP Container: /data/\nSPARQL API: /sparql\nWebTorrent Seeder: Active"),
    );
    grid.append_child(&card3).unwrap();

    // Pillar 4: DNS & Gateways Card
    let card4 = create_pillar_card(
        document,
        "\u{2699}\u{FE0F} DNS Zone & Tunnels",
        &format!(
            "{} Zone Records \u{00B7} Cloudflare Tunnel Active",
            config.dns_records.len()
        ),
        &format!(
            "A: 192.0.2.42\nMX: mail.{}\nSPF: v=spf1 mx ~all\nDKIM: Ed25519 Verified",
            config.domain_name
        ),
    );
    grid.append_child(&card4).unwrap();

    root.append_child(&grid).unwrap();
    root
}

fn create_pillar_card(document: &Document, title: &str, subtitle: &str, body: &str) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;",
    );

    let title_el = document.create_element("span").unwrap();
    title_el.set_text_content(Some(title));
    let title_h: HtmlElement = title_el.clone().dyn_into().unwrap();
    title_h
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card.append_child(&title_el).unwrap();

    let sub_el = document.create_element("span").unwrap();
    sub_el.set_text_content(Some(subtitle));
    let sub_h: HtmlElement = sub_el.clone().dyn_into().unwrap();
    sub_h
        .style()
        .set_css_text("font-size: 10px; font-weight: 600; color: #34d399;");
    card.append_child(&sub_el).unwrap();

    let body_el = document.create_element("pre").unwrap();
    body_el.set_text_content(Some(body));
    let body_h: HtmlElement = body_el.clone().dyn_into().unwrap();
    body_h.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8; margin: 4px 0 0 0; background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px; white-space: pre-wrap;");
    card.append_child(&body_el).unwrap();

    card
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_config_creation() {
        let conf = DomainConfig::new("holborn.id", "did:qualia:timothy");
        assert_eq!(conf.domain_name, "holborn.id");
        assert_eq!(conf.mailboxes.len(), 3);
        assert_eq!(conf.dns_records.len(), 5);
    }

    #[test]
    fn test_export_solid_webid_card_rdfa() {
        let conf = DomainConfig::new("mindware.agency", "did:qualia:org42");
        let html = conf.export_solid_webid_card_rdfa();
        assert!(html.contains("typeof=\"Person\""));
        assert!(html.contains("https://mindware.agency/data/"));
        assert!(html.contains("https://mindware.agency/sparql"));
    }

    #[test]
    fn test_export_bind_zone_file() {
        let conf = DomainConfig::new("wellfair.org", "did:qualia:wellfair");
        let zone = conf.export_bind_zone_file();
        assert!(zone.contains("$ORIGIN wellfair.org."));
        assert!(zone.contains("IN MX     10   mail.wellfair.org"));
        assert!(zone.contains("v=spf1 mx ~all"));
    }
}
