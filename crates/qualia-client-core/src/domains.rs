//! **Domains & mail addresses** — the foundation of the domain + semantic-mail/address stack
//! (`docs/plans/social-network-plan.md` §0.5). A domain/subdomain acts as an **agent** (QDP — Timothy's
//! `draft-webcivics-QDP`: a domain publishes an RDF agent profile at `/.well-known/QDP`). A person runs
//! several **context-domains** (personal/work/projects), each with its own front-door DID(s); subdomains
//! serve families/children. A domain may be **single-owner** or (deferred placeholder) **group-owned via an
//! M:N agreement** — modelled now so group domains slot in later without a refactor.
//!
//! Addresses on a domain are **rule-bearing mailboxes** — **purpose inboxes** (`frontdoor@`/`junkmail@`/
//! `mygov@`) or **per-relationship** (`bob@alice.example`). This module is the data model + presets + pure
//! helpers; the QDP profile generation, the semantic mail client (SMTP/IMAP), and the rules-execution engine
//! are follow-on slices.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::state::app_meta_dir;

/// QDP agent type (`draft-webcivics-QDP` §1) — what kind of agent a domain represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    NaturalPerson,
    Organization,
    AiAgent,
    HumanitarianService,
    ContentProvider,
    /// A group/collective (project, cooperative, household) — see `DomainOwner::Group`.
    Group,
}

/// Who owns/controls a domain. `Personal` today; **`Group` is the deferred placeholder** (governed by an
/// M:N agreement) — present so the model does not hard-assume single-owner domains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainOwner {
    /// A single person — `did` controls the domain.
    Personal { did: String },
    /// PLACEHOLDER (deferred): group/agreement-owned; `agreement_ref` points to the (future) M:N agreement
    /// that defines membership/roles/permissions. Not yet implemented — just not precluded.
    Group { agreement_ref: String },
}

/// A domain (or subdomain) acting as an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Domain {
    /// The name, e.g. `personal.me`, `kid.family.me`, `project-x.coop`.
    pub name: String,
    pub agent_type: AgentType,
    pub owner: DomainOwner,
    /// Front-door DID for this domain (the QDP agent id).
    pub front_door_did: String,
    /// Additional DIDs scoped to this context (pairwise per relationship, etc.).
    #[serde(default)]
    pub dids: Vec<String>,
    /// Parent domain, for subdomains (families/children).
    #[serde(default)]
    pub parent: Option<String>,
    /// Human label for the context ("Personal", "Work", "Project X").
    #[serde(default)]
    pub label: String,
    pub created_at: u64,
}

/// The kind of a mail address on a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressKind {
    /// A purpose inbox: `frontdoor` / `junkmail` / `mygov` / `newsletters` / …
    Purpose,
    /// A per-relationship (pairwise) address bound to one relationship.
    Relationship,
}

/// Rules governing an address (a mailbox) — procedural + semantic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MailRules {
    /// Quarantine incoming by default (the `junkmail@` burner).
    #[serde(default)]
    pub quarantine: bool,
    /// Only accept mail from a verified/known sender (DID-signed / an established relationship).
    #[serde(default)]
    pub require_verified_sender: bool,
    /// Priority hint (0 = normal; higher = more important, e.g. `mygov@`).
    #[serde(default)]
    pub priority: i8,
    /// Retention in days (0 = keep indefinitely).
    #[serde(default)]
    pub retention_days: u32,
    /// Notify on receipt.
    #[serde(default)]
    pub notify: bool,
    /// Optional **semantic** routing/handling rule — an agreement / credential / values-credential id the
    /// mail client evaluates (the hook into the rights model).
    #[serde(default)]
    pub semantic_route: Option<String>,
}

/// A mail address = a rule-bearing mailbox on a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAddress {
    /// The full address, `local@domain`.
    pub address: String,
    pub local_part: String,
    pub domain: String,
    pub kind: AddressKind,
    /// For a `Relationship` address: the peer/relationship DID it is bound to.
    #[serde(default)]
    pub relationship_did: Option<String>,
    /// Optional PGP public key (armored) for this address.
    #[serde(default)]
    pub pgp_pubkey: Option<String>,
    pub rules: MailRules,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub created_at: u64,
}

fn default_true() -> bool {
    true
}

/// A common purpose-inbox preset (a sensible name + rules a user can accept or tweak).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurposePreset {
    pub local: String,
    pub label: String,
    pub rules: MailRules,
}

/// The built-in purpose inboxes. `frontdoor@` is public-but-**governed**; `junkmail@` is the burner;
/// `mygov@` is verified-sender-only + high-priority + long retention.
pub fn purpose_presets() -> Vec<PurposePreset> {
    vec![
        PurposePreset {
            local: "frontdoor".into(),
            label: "Front door (public intake)".into(),
            rules: MailRules { notify: true, ..Default::default() },
        },
        PurposePreset {
            local: "junkmail".into(),
            label: "Junk (untrusted sign-ups)".into(),
            rules: MailRules { quarantine: true, notify: false, retention_days: 30, ..Default::default() },
        },
        PurposePreset {
            local: "mygov".into(),
            label: "Government / official".into(),
            rules: MailRules {
                require_verified_sender: true,
                priority: 5,
                retention_days: 3650,
                notify: true,
                ..Default::default()
            },
        },
        PurposePreset {
            local: "newsletters".into(),
            label: "Newsletters".into(),
            rules: MailRules { notify: false, retention_days: 90, ..Default::default() },
        },
    ]
}

/// A local-part is a lowercase RFC-ish token: alphanumeric + `.` `-` `_`, 1..=64 chars, no leading/trailing
/// separator. (Deliberately stricter than RFC 5321 for legibility + safety.)
pub fn is_valid_local_part(local: &str) -> bool {
    let l = local;
    if l.is_empty() || l.len() > 64 {
        return false;
    }
    if l.starts_with(['.', '-', '_']) || l.ends_with(['.', '-', '_']) {
        return false;
    }
    l.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// Build a `Domain` value (does not persist).
#[allow(clippy::too_many_arguments)]
pub fn make_domain(
    name: &str,
    agent_type: AgentType,
    owner: DomainOwner,
    front_door_did: &str,
    label: &str,
    parent: Option<String>,
    now: u64,
) -> Result<Domain, String> {
    let name = name.trim().to_lowercase();
    if name.is_empty() || !name.contains('.') {
        return Err("a domain must be a dotted name (e.g. personal.me)".into());
    }
    Ok(Domain {
        name,
        agent_type,
        owner,
        front_door_did: front_door_did.to_string(),
        dids: vec![],
        parent,
        label: label.to_string(),
        created_at: now,
    })
}

/// Build a purpose-inbox address (does not persist).
pub fn make_purpose_address(
    domain: &str,
    local: &str,
    rules: MailRules,
    now: u64,
) -> Result<MailAddress, String> {
    let local = local.trim().to_lowercase();
    if !is_valid_local_part(&local) {
        return Err(format!("invalid local part '{local}'"));
    }
    Ok(MailAddress {
        address: format!("{local}@{domain}"),
        local_part: local,
        domain: domain.to_string(),
        kind: AddressKind::Purpose,
        relationship_did: None,
        pgp_pubkey: None,
        rules,
        enabled: true,
        created_at: now,
    })
}

/// Build a per-relationship (pairwise) address bound to a relationship DID (does not persist).
pub fn make_relationship_address(
    domain: &str,
    local: &str,
    relationship_did: &str,
    now: u64,
) -> Result<MailAddress, String> {
    let mut a = make_purpose_address(domain, local, MailRules::default(), now)?;
    a.kind = AddressKind::Relationship;
    a.relationship_did = Some(relationship_did.to_string());
    // A relationship address defaults to verified-sender-only (it's for one known peer).
    a.rules.require_verified_sender = true;
    Ok(a)
}

/// Resolve a full address (case-insensitive) against a set of addresses.
pub fn resolve<'a>(addresses: &'a [MailAddress], address: &str) -> Option<&'a MailAddress> {
    let want = address.trim().to_lowercase();
    addresses.iter().find(|a| a.address.eq_ignore_ascii_case(&want))
}

// --- Thin persistence (additive JSON under app_meta_dir; mirrors directory.rs) ---

fn domains_path() -> PathBuf {
    app_meta_dir().join("mail_domains.json")
}
fn addresses_path() -> PathBuf {
    app_meta_dir().join("mail_addresses.json")
}

fn load_json<T: for<'de> Deserialize<'de> + Default>(path: PathBuf) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}
fn save_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

pub fn list_domains() -> Vec<Domain> {
    load_json(domains_path())
}

/// Persist a domain (upsert by name).
pub fn upsert_domain(domain: Domain) -> Result<(), String> {
    let mut all = list_domains();
    all.retain(|d| d.name != domain.name);
    all.push(domain);
    save_json(domains_path(), &all)
}

pub fn list_addresses(domain: Option<&str>) -> Vec<MailAddress> {
    let all: Vec<MailAddress> = load_json(addresses_path());
    match domain {
        Some(d) => all.into_iter().filter(|a| a.domain == d).collect(),
        None => all,
    }
}

/// Persist an address (upsert by full address). Errors if the domain is unknown.
pub fn upsert_address(address: MailAddress) -> Result<(), String> {
    if !list_domains().iter().any(|d| d.name == address.domain) {
        return Err(format!("unknown domain '{}'", address.domain));
    }
    let mut all: Vec<MailAddress> = load_json(addresses_path());
    all.retain(|a| !a.address.eq_ignore_ascii_case(&address.address));
    all.push(address);
    save_json(addresses_path(), &all)
}

/// Enable/disable an address (the surgical per-relationship revoke).
pub fn set_address_enabled(address: &str, enabled: bool) -> Result<(), String> {
    let mut all: Vec<MailAddress> = load_json(addresses_path());
    let a = all
        .iter_mut()
        .find(|a| a.address.eq_ignore_ascii_case(address))
        .ok_or_else(|| format!("unknown address '{address}'"))?;
    a.enabled = enabled;
    save_json(addresses_path(), &all)
}

/// Summary counts for the UI.
pub fn address_counts_by_domain() -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for a in list_addresses(None) {
        *m.entry(a.domain).or_insert(0) += 1;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_part_validation() {
        assert!(is_valid_local_part("frontdoor"));
        assert!(is_valid_local_part("bob.smith"));
        assert!(is_valid_local_part("my-gov_2"));
        assert!(!is_valid_local_part(""));
        assert!(!is_valid_local_part(".bad"));
        assert!(!is_valid_local_part("bad."));
        assert!(!is_valid_local_part("Has Space"));
        assert!(!is_valid_local_part("UPPER"));
        assert!(!is_valid_local_part("bad@char"));
    }

    #[test]
    fn presets_carry_the_right_rules() {
        let p = purpose_presets();
        let junk = p.iter().find(|x| x.local == "junkmail").unwrap();
        assert!(junk.rules.quarantine && !junk.rules.notify);
        let gov = p.iter().find(|x| x.local == "mygov").unwrap();
        assert!(gov.rules.require_verified_sender && gov.rules.priority > 0);
        let front = p.iter().find(|x| x.local == "frontdoor").unwrap();
        assert!(!front.rules.quarantine, "front door is public but governed, not quarantined");
    }

    #[test]
    fn purpose_address_is_built_correctly() {
        let a = make_purpose_address("personal.me", "FrontDoor", MailRules::default(), 100).unwrap();
        assert_eq!(a.address, "frontdoor@personal.me");
        assert_eq!(a.kind, AddressKind::Purpose);
        assert!(a.relationship_did.is_none());
        assert!(make_purpose_address("personal.me", "bad space", MailRules::default(), 100).is_err());
    }

    #[test]
    fn relationship_address_binds_a_did_and_defaults_to_verified() {
        let a = make_relationship_address("alice.example", "bob", "did:qualia:bob", 100).unwrap();
        assert_eq!(a.address, "bob@alice.example");
        assert_eq!(a.kind, AddressKind::Relationship);
        assert_eq!(a.relationship_did.as_deref(), Some("did:qualia:bob"));
        assert!(a.rules.require_verified_sender);
    }

    #[test]
    fn group_ownership_is_modelled_not_precluded() {
        let personal = make_domain(
            "personal.me",
            AgentType::NaturalPerson,
            DomainOwner::Personal { did: "did:qualia:me".into() },
            "did:qualia:me",
            "Personal",
            None,
            1,
        )
        .unwrap();
        assert!(matches!(personal.owner, DomainOwner::Personal { .. }));
        let group = make_domain(
            "project-x.coop",
            AgentType::Group,
            DomainOwner::Group { agreement_ref: "agr:project-x".into() },
            "did:qualia:project-x",
            "Project X",
            None,
            1,
        )
        .unwrap();
        assert!(matches!(group.owner, DomainOwner::Group { .. }), "group domains slot in without a refactor");
        // A subdomain (child under a household).
        let kid = make_domain(
            "kid.family.me",
            AgentType::NaturalPerson,
            DomainOwner::Personal { did: "did:qualia:kid".into() },
            "did:qualia:kid",
            "Kid",
            Some("family.me".into()),
            1,
        )
        .unwrap();
        assert_eq!(kid.parent.as_deref(), Some("family.me"));
    }

    #[test]
    fn make_domain_requires_a_dotted_name() {
        assert!(make_domain("nodots", AgentType::NaturalPerson, DomainOwner::Personal { did: "d".into() }, "d", "", None, 1).is_err());
    }

    #[test]
    fn resolve_is_case_insensitive() {
        let addrs = vec![
            make_purpose_address("personal.me", "frontdoor", MailRules::default(), 1).unwrap(),
            make_relationship_address("personal.me", "bob", "did:x", 1).unwrap(),
        ];
        assert!(resolve(&addrs, "FrontDoor@Personal.ME").is_some());
        assert_eq!(resolve(&addrs, "bob@personal.me").unwrap().kind, AddressKind::Relationship);
        assert!(resolve(&addrs, "nope@personal.me").is_none());
    }
}
