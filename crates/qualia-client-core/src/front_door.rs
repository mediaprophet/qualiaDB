//! **Front-door record** — how a domain is discovered as an agent (QDP: `draft-webcivics-QDP`).
//!
//! Two forms, per the plan (§0.5) — **DNS is the primary, no-hosting anchor**:
//! - **DNS TXT at `_qdp.<domain>`** (QDP §3.6): the **Front Door DID** (a contextually-isolated, per-domain
//!   DID) + compact peering material. A domain owner adds one record at their registrar/Cloudflare — **no
//!   server needed**. This is the minimum viable front-door.
//! - **`/.well-known/QDP` profile** (QDP §3.1): the *rich* agent profile in **Turtle + JSON-LD + CBOR-LD**
//!   (Solid-compatible) — the **optional** enhancement for those who have hosting (web server / Cloudflare
//!   Worker+R2 / Solid POD). The DNS record may point to it (`qdp:profile`).
//!
//! Private keys are NEVER placed in either form (QDP §5). CBOR-LD term-dictionary compaction (the q42 vocab)
//! is deferred — `to_cbor_ld` is CBOR of the JSON-LD document (linked data in CBOR), which is lossless.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domains::AgentType;

const NS_QDP: &str = "https://webcivics.github.io/QDP/ontdev/QDP#";

/// A service a domain-agent exposes (QDP §3.4): eCash address, Solid POD, SPARQL endpoint, …
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QdpService {
    /// `"ecash" | "solidpod" | "sparql" | "endpoint" | "webid"`.
    pub kind: String,
    pub value: String,
}

/// The front-door record for a domain acting as a QDP agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontDoorRecord {
    pub domain: String,
    pub agent_type: AgentType,
    /// The contextually-isolated **Front Door DID** for this domain (QDP §3.6).
    pub front_door_did: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub webid: Option<String>,
    /// Services (eCash address, Solid POD, SPARQL…). Only `ecash` is carried in the compact DNS form.
    #[serde(default)]
    pub services: Vec<QdpService>,
    // --- compact peering material (carried in DNS, no hosting) ---
    #[serde(default)]
    pub identity_pubkey_hex: Option<String>,
    #[serde(default)]
    pub wireguard_pubkey_hex: Option<String>,
    #[serde(default)]
    pub overlay_addr: Option<String>,
    /// Optional pointer to the rich hosted profile (`/.well-known/QDP` or a Solid POD).
    #[serde(default)]
    pub profile_url: Option<String>,
}

/// The DNS record name the front-door TXT lives at (QDP §3.6).
pub fn dns_record_name(domain: &str) -> String {
    format!("_qdp.{domain}")
}

fn agent_type_token(t: &AgentType) -> &'static str {
    match t {
        AgentType::NaturalPerson => "person",
        AgentType::Organization => "org",
        AgentType::AiAgent => "ai",
        AgentType::HumanitarianService => "service",
        AgentType::ContentProvider => "content",
        AgentType::Group => "group",
    }
}
fn agent_type_from_token(s: &str) -> AgentType {
    match s {
        "org" => AgentType::Organization,
        "ai" => AgentType::AiAgent,
        "service" => AgentType::HumanitarianService,
        "content" => AgentType::ContentProvider,
        "group" => AgentType::Group,
        _ => AgentType::NaturalPerson,
    }
}
/// The QDP `rdf:type` for an agent (QDP §3.3 controlled vocabulary).
fn agent_type_class(t: &AgentType) -> &'static str {
    match t {
        AgentType::NaturalPerson => "foaf:Person",
        AgentType::Organization | AgentType::Group => "schema:Organization",
        AgentType::AiAgent => "QDP:AIAgent",
        AgentType::HumanitarianService => "QDP:EssentialService",
        AgentType::ContentProvider => "QDP:ContentProvider",
    }
}

fn ecash(rec: &FrontDoorRecord) -> Option<&str> {
    rec.services
        .iter()
        .find(|s| s.kind == "ecash")
        .map(|s| s.value.as_str())
}

impl FrontDoorRecord {
    /// The **DNS TXT value** for `_qdp.<domain>` — compact, no hosting. An RDF snippet (QDP §3.6 shows
    /// `qdp:signer <did>`), extended with the peering material. Keep it small (DNS strings are 255 bytes).
    pub fn to_dns_txt(&self) -> String {
        let mut parts = vec![
            format!("qdp:signer <{}>", self.front_door_did),
            format!("qdp:agentType \"{}\"", agent_type_token(&self.agent_type)),
        ];
        if let Some(k) = &self.identity_pubkey_hex {
            parts.push(format!("qdp:identityKey \"{k}\""));
        }
        if let Some(w) = &self.wireguard_pubkey_hex {
            parts.push(format!("qdp:wireguard \"{w}\""));
        }
        if let Some(o) = &self.overlay_addr {
            parts.push(format!("qdp:overlay \"{o}\""));
        }
        if let Some(e) = ecash(self) {
            parts.push(format!("qdp:ecash \"{e}\""));
        }
        if let Some(u) = &self.profile_url {
            parts.push(format!("qdp:profile <{u}>"));
        }
        parts.join(" ; ")
    }

    /// Parse a `_qdp.<domain>` TXT value back into a record (`domain` comes from the record name).
    pub fn from_dns_txt(domain: &str, txt: &str) -> Result<Self, String> {
        let mut rec = FrontDoorRecord {
            domain: domain.to_string(),
            agent_type: AgentType::NaturalPerson,
            front_door_did: String::new(),
            name: None,
            webid: None,
            services: vec![],
            identity_pubkey_hex: None,
            wireguard_pubkey_hex: None,
            overlay_addr: None,
            profile_url: None,
        };
        for clause in txt.split(';') {
            let clause = clause.trim();
            let Some((pred, obj)) = clause.split_once(char::is_whitespace) else {
                continue;
            };
            let obj = obj.trim();
            let uri = || {
                obj.trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            };
            let lit = || obj.trim_matches('"').to_string();
            match pred.trim() {
                "qdp:signer" => rec.front_door_did = uri(),
                "qdp:agentType" => rec.agent_type = agent_type_from_token(&lit()),
                "qdp:identityKey" => rec.identity_pubkey_hex = Some(lit()),
                "qdp:wireguard" => rec.wireguard_pubkey_hex = Some(lit()),
                "qdp:overlay" => rec.overlay_addr = Some(lit()),
                "qdp:ecash" => rec.services.push(QdpService {
                    kind: "ecash".into(),
                    value: lit(),
                }),
                "qdp:profile" => rec.profile_url = Some(uri()),
                _ => {}
            }
        }
        if rec.front_door_did.is_empty() {
            return Err("front-door record has no qdp:signer".into());
        }
        Ok(rec)
    }

    /// The rich `/.well-known/QDP` profile in **Turtle** (QDP §3.2/§4). Needs hosting.
    pub fn to_turtle(&self) -> String {
        use std::fmt::Write as _;
        let mut t = String::new();
        let _ = writeln!(t, "@prefix QDP: <{NS_QDP}> .");
        let _ = writeln!(
            t,
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> ."
        );
        let _ = writeln!(t, "@prefix foaf: <http://xmlns.com/foaf/0.1/> .");
        let _ = writeln!(t, "@prefix schema: <https://schema.org/> .");
        let _ = writeln!(t);
        let _ = writeln!(t, "<#this> a {} ;", agent_type_class(&self.agent_type));
        let _ = writeln!(t, "    schema:domain \"{}\" ;", self.domain);
        let _ = writeln!(t, "    QDP:signer <{}> ;", self.front_door_did);
        if let Some(w) = &self.webid {
            let _ = writeln!(t, "    QDP:webid <{w}> ;");
        }
        if let Some(n) = &self.name {
            let _ = writeln!(t, "    foaf:name \"{n}\" ;");
        }
        for s in &self.services {
            match s.kind.as_str() {
                "ecash" => {
                    let _ = writeln!(t, "    QDP:hasEcashAccount \"{}\" ;", s.value);
                }
                "solidpod" => {
                    let _ = writeln!(t, "    QDP:hasSolidPod <{}> ;", s.value);
                }
                "sparql" => {
                    let _ = writeln!(t, "    QDP:sparqlEndpoint <{}> ;", s.value);
                }
                _ => {
                    let _ = writeln!(t, "    QDP:serviceEndpoint <{}> ;", s.value);
                }
            }
        }
        // close with a metadata node (satisfies QDP:hasMetadata SHACL minCount 1).
        let _ = writeln!(t, "    QDP:hasMetadata [ QDP:metadataType \"profile\" ] .");
        t
    }

    /// The rich profile as **JSON-LD** (Solid-native; QDP §3.2). Needs hosting.
    pub fn to_json_ld(&self) -> serde_json::Value {
        let mut node = json!({
            "@context": {
                "QDP": NS_QDP,
                "foaf": "http://xmlns.com/foaf/0.1/",
                "schema": "https://schema.org/",
            },
            "@id": "#this",
            "@type": agent_type_class(&self.agent_type),
            "schema:domain": self.domain,
            "QDP:signer": { "@id": self.front_door_did },
        });
        let obj = node.as_object_mut().unwrap();
        if let Some(w) = &self.webid {
            obj.insert("QDP:webid".into(), json!({ "@id": w }));
        }
        if let Some(n) = &self.name {
            obj.insert("foaf:name".into(), json!(n));
        }
        if let Some(e) = ecash(self) {
            obj.insert("QDP:hasEcashAccount".into(), json!(e));
        }
        if let Some(s) = self.services.iter().find(|s| s.kind == "solidpod") {
            obj.insert("QDP:hasSolidPod".into(), json!({ "@id": s.value }));
        }
        obj.insert(
            "QDP:hasMetadata".into(),
            json!({ "QDP:metadataType": "profile" }),
        );
        node
    }

    /// The rich profile as **CBOR-LD** — CBOR of the JSON-LD document (linked data in CBOR, lossless).
    /// Full term-dictionary compaction (the q42 vocab) is a follow-on.
    pub fn to_cbor_ld(&self) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        ciborium::into_writer(&self.to_json_ld(), &mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }

    /// Decode a CBOR-LD profile back to the JSON-LD document.
    pub fn from_cbor_ld(bytes: &[u8]) -> Result<serde_json::Value, String> {
        ciborium::from_reader(bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FrontDoorRecord {
        FrontDoorRecord {
            domain: "alice.example".into(),
            agent_type: AgentType::NaturalPerson,
            front_door_did: "did:qdp:alice-frontdoor".into(),
            name: Some("Alice".into()),
            webid: Some("https://alice.example/profile#me".into()),
            services: vec![
                QdpService {
                    kind: "ecash".into(),
                    value: "ecash:qq123".into(),
                },
                QdpService {
                    kind: "solidpod".into(),
                    value: "https://alice.example/pod/".into(),
                },
            ],
            identity_pubkey_hex: Some("aa".repeat(32)),
            wireguard_pubkey_hex: Some("bb".repeat(32)),
            overlay_addr: Some("fd00::1".into()),
            profile_url: Some("https://alice.example/.well-known/QDP".into()),
        }
    }

    #[test]
    fn dns_record_name_is_underscore_qdp() {
        assert_eq!(dns_record_name("alice.example"), "_qdp.alice.example");
    }

    #[test]
    fn dns_txt_roundtrips_the_no_hosting_anchor() {
        let rec = sample();
        let txt = rec.to_dns_txt();
        assert!(txt.contains("qdp:signer <did:qdp:alice-frontdoor>"));
        assert!(txt.contains("qdp:wireguard"));
        let back = FrontDoorRecord::from_dns_txt("alice.example", &txt).unwrap();
        // The DNS-carried subset round-trips.
        assert_eq!(back.front_door_did, rec.front_door_did);
        assert_eq!(back.agent_type, rec.agent_type);
        assert_eq!(back.wireguard_pubkey_hex, rec.wireguard_pubkey_hex);
        assert_eq!(back.overlay_addr, rec.overlay_addr);
        assert_eq!(back.profile_url, rec.profile_url);
        assert_eq!(ecash(&back), Some("ecash:qq123"));
    }

    #[test]
    fn from_dns_txt_requires_a_signer() {
        assert!(FrontDoorRecord::from_dns_txt("x.example", "qdp:agentType \"person\"").is_err());
    }

    #[test]
    fn turtle_has_qdp_mandatory_fields() {
        let t = sample().to_turtle();
        assert!(t.contains("@prefix QDP:"));
        assert!(t.contains("a foaf:Person"));
        assert!(t.contains("schema:domain \"alice.example\""));
        assert!(t.contains("QDP:signer <did:qdp:alice-frontdoor>"));
        assert!(t.contains("QDP:hasEcashAccount \"ecash:qq123\""));
        assert!(t.contains("QDP:hasMetadata"));
    }

    #[test]
    fn jsonld_has_type_domain_and_ecash() {
        let j = sample().to_json_ld();
        assert_eq!(j["@type"], "foaf:Person");
        assert_eq!(j["schema:domain"], "alice.example");
        assert_eq!(j["QDP:hasEcashAccount"], "ecash:qq123");
        assert_eq!(j["QDP:signer"]["@id"], "did:qdp:alice-frontdoor");
    }

    #[test]
    fn cbor_ld_roundtrips_the_jsonld() {
        let rec = sample();
        let bytes = rec.to_cbor_ld().unwrap();
        let back = FrontDoorRecord::from_cbor_ld(&bytes).unwrap();
        assert_eq!(
            back,
            rec.to_json_ld(),
            "CBOR-LD is a lossless encoding of the JSON-LD"
        );
    }

    #[test]
    fn org_and_ai_map_to_qdp_types() {
        let mut r = sample();
        r.agent_type = AgentType::Organization;
        assert!(r.to_turtle().contains("a schema:Organization"));
        r.agent_type = AgentType::AiAgent;
        assert!(r.to_turtle().contains("a QDP:AIAgent"));
    }
}
