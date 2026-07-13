//! Personal directory — an **"Active Directory"-like** service for a person's relationships.
//!
//! It hosts the **addressbook** (Parties) organised into **categories** (organisational units / groups —
//! a Party may be in several, like AD groups), and is the home for the **agreements** governing each
//! relationship. It unifies the two pre-existing stores — directory [`Actor`](crate::state::Actor)s and
//! chat [`ChatContact`](crate::social_connect::ChatContact)s — into ONE categorised view joined by pairwise
//! DID, **without a destructive migration**: it reads both and persists only the additive parts (custom
//! categories + per-entry category assignments) in their own files under [`app_meta_dir`].
//!
//! Agreement links are surfaced per entry from the first-class agreement store ([`crate::agreements`]) —
//! see `docs/plans/rights-aware-peer-agreement-addressbook.md`. This module is the **P0** foundation of
//! that plan (the directory that hosts the addressbook); the agreement store is **P1**, and
//! [`build_view_core`] joins the two by DID so each Party carries the ids of the Agreements governing its
//! relationship.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agreements::Agreement;
use crate::social_connect::ChatContact;
use crate::state::{app_meta_dir, Actor};

/// A directory category — an organisational unit / group (AD-style). Built-in ones are always present;
/// users can add their own.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectoryCategory {
    pub id: String,
    pub label: String,
    /// A grouping/iconography hint (not a hard type).
    pub kind: String,
    pub builtin: bool,
}

/// The built-in categories. `people` is the catch-all every entry falls into.
pub fn builtin_categories() -> Vec<DirectoryCategory> {
    let mk = |id: &str, label: &str, kind: &str| DirectoryCategory {
        id: id.into(),
        label: label.into(),
        kind: kind.into(),
        builtin: true,
    };
    vec![
        mk("people", "People", "people"),
        mk("health", "Health practitioners", "health"),
        mk("cooperative", "Cooperative", "cooperative"),
        mk("organizations", "Organizations", "organization"),
        mk("agents", "Agents", "agent"),
        mk("family-friends", "Family & friends", "personal"),
    ]
}

/// One unified addressbook entry (a Party), joined across the directory-actor + chat-contact stores by DID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectoryEntry {
    /// Pairwise DID — the join key and the party's identifier.
    pub did: String,
    pub display_name: String,
    /// actor_type / roles / contact tags — descriptive labels, not a hard type.
    pub kinds: Vec<String>,
    pub organization: Option<String>,
    pub verification_status: String,
    pub front_door_did: Option<String>,
    /// Which store(s) this entry was found in ("directory-actor", "contact").
    pub sources: Vec<String>,
    /// Category ids this entry belongs to (an entry may be in several).
    pub categories: Vec<String>,
    /// Agreements governing this relationship (ids). Empty until the agreement store lands (plan P1).
    pub agreement_ids: Vec<String>,
}

/// The whole categorised directory returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectoryView {
    pub categories: Vec<DirectoryCategory>,
    pub entries: Vec<DirectoryEntry>,
}

fn categories_path() -> PathBuf {
    app_meta_dir().join("directory_categories.json")
}
fn assignments_path() -> PathBuf {
    app_meta_dir().join("directory_assignments.json")
}

fn load_custom_categories() -> Vec<DirectoryCategory> {
    fs::read_to_string(categories_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_custom_categories(cats: &[DirectoryCategory]) -> Result<(), String> {
    let path = categories_path();
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(cats).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn load_assignments() -> BTreeMap<String, Vec<String>> {
    fs::read_to_string(assignments_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_assignments(a: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let path = assignments_path();
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(a).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// All categories: built-in + user-created.
pub fn list_categories() -> Vec<DirectoryCategory> {
    let mut cats = builtin_categories();
    for c in load_custom_categories() {
        if !cats.iter().any(|e| e.id == c.id) {
            cats.push(c);
        }
    }
    cats
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Create a custom category. Idempotent by slug; errors if the slug already exists.
pub fn create_category(label: &str) -> Result<DirectoryCategory, String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("category label is empty".into());
    }
    let id = slugify(label);
    if id.is_empty() {
        return Err("category label has no usable characters".into());
    }
    if list_categories().iter().any(|c| c.id == id) {
        return Err(format!("a category '{label}' already exists"));
    }
    let cat = DirectoryCategory {
        id,
        label: label.to_string(),
        kind: "custom".into(),
        builtin: false,
    };
    let mut custom = load_custom_categories();
    custom.push(cat.clone());
    save_custom_categories(&custom)?;
    Ok(cat)
}

/// Set the categories an entry (by DID) belongs to. Unknown category ids are dropped; an empty set clears
/// the explicit assignment (the entry falls back to inferred categories).
pub fn set_entry_categories(did: &str, categories: Vec<String>) -> Result<(), String> {
    let valid: std::collections::HashSet<String> =
        list_categories().into_iter().map(|c| c.id).collect();
    let cleaned: Vec<String> = categories
        .into_iter()
        .map(|c| c.trim().to_string())
        .filter(|c| valid.contains(c))
        .collect();
    let mut a = load_assignments();
    if cleaned.is_empty() {
        a.remove(did);
    } else {
        a.insert(did.to_string(), cleaned);
    }
    save_assignments(&a)
}

/// Default category inference from an entry's kinds/organisation (used when no explicit assignment exists).
fn infer_categories(kinds: &[String], organization: &Option<String>) -> Vec<String> {
    let hay = kinds.join(" ").to_lowercase();
    let has = |needle: &str| hay.contains(needle);
    let mut cats = vec!["people".to_string()];
    if has("agent") {
        cats.push("agents".into());
    }
    if has("clinician") || has("practitioner") || has("doctor") || has("health") || has("nurse")
        || has("therapist")
    {
        cats.push("health".into());
    }
    if has("cooperative") || has("coop") {
        cats.push("cooperative".into());
    }
    if has("friend") || has("family") {
        cats.push("family-friends".into());
    }
    if organization.is_some() || has("organization") || has("org") {
        cats.push("organizations".into());
    }
    cats.sort();
    cats.dedup();
    cats
}

fn merge_kinds(dst: &mut Vec<String>, src: Vec<String>) {
    for s in src {
        let s = s.trim().to_string();
        if !s.is_empty() && !dst.iter().any(|d| d.eq_ignore_ascii_case(&s)) {
            dst.push(s);
        }
    }
}

fn push_unique(dst: &mut Vec<String>, v: &str) {
    if !dst.iter().any(|d| d == v) {
        dst.push(v.to_string());
    }
}

/// Pure core: merge directory actors + chat contacts into one categorised view, given the persisted
/// category assignments and the category list. Kept pure (no filesystem) so it is deterministically
/// testable; [`build_view`] wires it to the persisted stores.
pub fn build_view_core(
    actors: &[Actor],
    contacts: &[ChatContact],
    assignments: &BTreeMap<String, Vec<String>>,
    categories: Vec<DirectoryCategory>,
    agreements: &[Agreement],
) -> DirectoryView {
    let mut by_did: BTreeMap<String, DirectoryEntry> = BTreeMap::new();

    for a in actors {
        let did = if a.pairwise_did.is_empty() {
            a.id.clone()
        } else {
            a.pairwise_did.clone()
        };
        let entry = by_did.entry(did.clone()).or_insert_with(|| DirectoryEntry {
            did: did.clone(),
            display_name: a.name.clone(),
            kinds: vec![],
            organization: a.organization.clone(),
            verification_status: a.verification_status.clone(),
            front_door_did: a.root_did_uri.clone(),
            sources: vec![],
            categories: vec![],
            agreement_ids: vec![],
        });
        if entry.display_name.is_empty() {
            entry.display_name = a.name.clone();
        }
        if entry.organization.is_none() {
            entry.organization = a.organization.clone();
        }
        let mut kinds = vec![a.actor_type.clone()];
        kinds.extend(a.roles.iter().cloned());
        merge_kinds(&mut entry.kinds, kinds);
        push_unique(&mut entry.sources, "directory-actor");
    }

    for c in contacts {
        let entry = by_did.entry(c.did.clone()).or_insert_with(|| DirectoryEntry {
            did: c.did.clone(),
            display_name: c.display_name.clone(),
            kinds: vec![],
            organization: None,
            verification_status: "INVITE_ACCEPTED".into(),
            front_door_did: None,
            sources: vec![],
            categories: vec![],
            agreement_ids: vec![],
        });
        if entry.display_name.is_empty() {
            entry.display_name = c.display_name.clone();
        }
        merge_kinds(&mut entry.kinds, c.categories.clone());
        push_unique(&mut entry.sources, "contact");
    }

    let mut entries: Vec<DirectoryEntry> = by_did
        .into_values()
        .map(|mut e| {
            e.categories = match assignments.get(&e.did) {
                Some(cats) if !cats.is_empty() => cats.clone(),
                _ => infer_categories(&e.kinds, &e.organization),
            };
            // Join P1: the agreements governing this relationship — the entry is a party to them, or the
            // agreement names this DID as the relationship it governs.
            e.agreement_ids = agreements
                .iter()
                .filter(|a| a.relationship_did == e.did || a.parties.iter().any(|p| *p == e.did))
                .map(|a| a.id.clone())
                .collect();
            e
        })
        .collect();
    entries.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));

    DirectoryView { categories, entries }
}

/// Build the unified, categorised directory view over the persisted directory-actor + chat-contact stores,
/// joined against the persisted agreement store so each entry carries its governing agreement ids.
pub fn build_view(actors: &[Actor], contacts: &[ChatContact]) -> DirectoryView {
    build_view_core(
        actors,
        contacts,
        &load_assignments(),
        list_categories(),
        &crate::agreements::list_agreements(),
    )
}

// ===========================================================================
// Faceted + concept-aware search — for a directory that grows large over time.
// ===========================================================================

/// One selectable value within a facet, with its count in the current result and whether it's selected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacetValue {
    pub value: String,
    pub label: String,
    pub count: usize,
    pub selected: bool,
}

/// A facet (a filterable dimension of the directory) and its values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Facet {
    pub id: String,
    pub label: String,
    pub values: Vec<FacetValue>,
}

/// A search result: the ranked matching entries, the facet counts (drill-down), and the categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectorySearchResult {
    pub categories: Vec<DirectoryCategory>,
    pub facets: Vec<Facet>,
    pub entries: Vec<DirectoryEntry>,
    pub total: usize,
    pub query: String,
}

/// The facet dimensions, in display order.
const FACET_IDS: [&str; 5] = ["category", "kind", "source", "verification", "agreements"];

/// Concept clusters for **meaning-aware search**: a query token that hits any member of a cluster matches
/// the whole cluster, so "doctor" finds a "clinician". This is honest concept expansion over the
/// role/category space — NOT embedding similarity. (The scale path — entries as quins queried through the
/// semantic graph engine / embeddings — is noted in the plan.)
fn concept_clusters() -> &'static [&'static [&'static str]] {
    &[
        &[
            "doctor", "clinician", "physician", "gp", "practitioner", "medic", "medical", "health",
            "nurse", "therapist", "care", "psychiatrist", "psychologist", "counsellor",
        ],
        &["cooperative", "coop", "co-op", "member", "collective", "union"],
        &["friend", "family", "personal", "kin", "mate"],
        &["organization", "organisation", "org", "company", "institution", "business", "ngo"],
        &["agent", "ai", "bot", "assistant", "subagent", "sub-agent"],
    ]
}

/// Expand a single query token to itself plus any concept cluster it belongs to.
fn expand_token(tok: &str) -> Vec<String> {
    let t = tok.trim().to_lowercase();
    let mut out = vec![t.clone()];
    if !t.is_empty() {
        for cluster in concept_clusters() {
            if cluster.iter().any(|w| *w == t) {
                out.extend(cluster.iter().map(|w| w.to_string()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn facet_label(id: &str) -> &'static str {
    match id {
        "category" => "Category",
        "kind" => "Kind",
        "source" => "Source",
        "verification" => "Verification",
        "agreements" => "Agreements",
        _ => "",
    }
}

fn entry_facet_values(e: &DirectoryEntry, facet: &str) -> Vec<String> {
    match facet {
        "category" => e.categories.clone(),
        "kind" => e.kinds.clone(),
        "source" => e.sources.clone(),
        "verification" => vec![e.verification_status.clone()],
        "agreements" => vec![if e.agreement_ids.is_empty() {
            "without".to_string()
        } else {
            "with".to_string()
        }],
        _ => vec![],
    }
}

fn value_label(fid: &str, value: &str, categories: &[DirectoryCategory]) -> String {
    match fid {
        "category" => categories
            .iter()
            .find(|c| c.id == value)
            .map(|c| c.label.clone())
            .unwrap_or_else(|| value.to_string()),
        "agreements" => {
            if value == "with" {
                "With agreements".to_string()
            } else {
                "Without agreements".to_string()
            }
        }
        _ => value.to_string(),
    }
}

fn searchable_text(e: &DirectoryEntry, categories: &[DirectoryCategory]) -> String {
    let mut parts = vec![
        e.display_name.clone(),
        e.did.clone(),
        e.verification_status.clone(),
    ];
    if let Some(o) = &e.organization {
        parts.push(o.clone());
    }
    parts.extend(e.kinds.iter().cloned());
    for c in &e.categories {
        parts.push(c.clone());
        if let Some(cat) = categories.iter().find(|x| &x.id == c) {
            parts.push(cat.label.clone());
        }
    }
    parts.join(" ").to_lowercase()
}

/// Score an entry against the expanded query tokens. `None` = no match (AND across tokens); higher = better
/// (name-field hits weighted heavier). An empty query matches everything with score 0.
fn query_score(text: &str, name: &str, expanded: &[Vec<String>]) -> Option<i32> {
    if expanded.is_empty() {
        return Some(0);
    }
    let name_l = name.to_lowercase();
    let mut score = 0i32;
    for token_exp in expanded {
        let mut hit = false;
        for w in token_exp {
            if !w.is_empty() && text.contains(w.as_str()) {
                hit = true;
                score += if name_l.contains(w.as_str()) { 3 } else { 1 };
            }
        }
        if !hit {
            return None; // every query token must match (recall comes from concept expansion)
        }
    }
    Some(score)
}

/// Does the entry pass the selected facets (AND across facet groups, OR within a group)? `except` skips one
/// group — used to compute drill-down counts for that group.
fn passes_facets(
    e: &DirectoryEntry,
    selected: &BTreeMap<String, Vec<String>>,
    except: Option<&str>,
) -> bool {
    for (fid, vals) in selected {
        if vals.is_empty() || Some(fid.as_str()) == except {
            continue;
        }
        let ev = entry_facet_values(e, fid);
        if !vals.iter().any(|v| ev.iter().any(|x| x == v)) {
            return false;
        }
    }
    true
}

/// Pure search core: rank entries by a concept-expanded query and narrow by selected facets, returning the
/// entries plus drill-down facet counts. Kept pure (no filesystem) for deterministic testing.
pub fn search_core(
    entries: Vec<DirectoryEntry>,
    categories: Vec<DirectoryCategory>,
    query: &str,
    selected: &BTreeMap<String, Vec<String>>,
) -> DirectorySearchResult {
    let expanded: Vec<Vec<String>> = query.split_whitespace().map(expand_token).collect();

    // Entries matching the text query (facets not yet applied), with scores.
    let scored: Vec<(DirectoryEntry, i32)> = entries
        .into_iter()
        .filter_map(|e| {
            let text = searchable_text(&e, &categories);
            query_score(&text, &e.display_name, &expanded).map(|s| (e, s))
        })
        .collect();

    // Facet counts: for each group, count over (query-matched ∩ every OTHER selected group) — the standard
    // drill-down count so selecting a value shows how many you'd get.
    let mut facets = Vec::new();
    for fid in FACET_IDS {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for (e, _) in scored.iter().filter(|(e, _)| passes_facets(e, selected, Some(fid))) {
            for v in entry_facet_values(e, fid) {
                *counts.entry(v).or_insert(0) += 1;
            }
        }
        let sel = selected.get(fid).cloned().unwrap_or_default();
        let mut values: Vec<FacetValue> = counts
            .into_iter()
            .map(|(value, count)| {
                let label = value_label(fid, &value, &categories);
                let is_sel = sel.iter().any(|x| x == &value);
                FacetValue { value, label, count, selected: is_sel }
            })
            .collect();
        values.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then(a.label.to_lowercase().cmp(&b.label.to_lowercase()))
        });
        if !values.is_empty() {
            facets.push(Facet {
                id: fid.to_string(),
                label: facet_label(fid).to_string(),
                values,
            });
        }
    }

    // Narrow by all selected facets, then rank (score desc, then name).
    let mut narrowed: Vec<(DirectoryEntry, i32)> = scored
        .into_iter()
        .filter(|(e, _)| passes_facets(e, selected, None))
        .collect();
    narrowed.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.display_name.to_lowercase().cmp(&b.0.display_name.to_lowercase()))
    });
    let entries: Vec<DirectoryEntry> = narrowed.into_iter().map(|(e, _)| e).collect();
    let total = entries.len();

    DirectorySearchResult {
        categories,
        facets,
        entries,
        total,
        query: query.to_string(),
    }
}

/// Search the persisted directory: build the unified view, then run [`search_core`].
pub fn search(
    actors: &[Actor],
    contacts: &[ChatContact],
    query: &str,
    selected: &BTreeMap<String, Vec<String>>,
) -> DirectorySearchResult {
    let view = build_view(actors, contacts);
    search_core(view.entries, view.categories, query, selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: &str, name: &str, did: &str, ty: &str, roles: &[&str]) -> Actor {
        Actor {
            id: id.into(),
            actor_type: ty.into(),
            name: name.into(),
            organization: None,
            qualifications: vec![],
            roles: roles.iter().map(|s| s.to_string()).collect(),
            verification_status: "VERIFIED".into(),
            pairwise_did: did.into(),
            root_did_uri: None,
            routing_hints: vec![],
        }
    }

    fn contact(name: &str, did: &str, categories: &[&str]) -> ChatContact {
        ChatContact {
            actor_id: format!("contact-{did}"),
            display_name: name.into(),
            did: did.into(),
            source: "connect".into(),
            added_at: 0,
            relay_endpoint: None,
            categories: categories.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn slugify_makes_stable_ids() {
        assert_eq!(slugify("Health Practitioners"), "health-practitioners");
        assert_eq!(slugify("  My Co-op!! "), "my-co-op");
        assert_eq!(slugify("A/B\\C"), "a-b-c");
    }

    #[test]
    fn inference_routes_by_kind_and_org() {
        assert!(infer_categories(&["clinician".into()], &None).contains(&"health".to_string()));
        assert!(infer_categories(&["AGENT".into()], &None).contains(&"agents".to_string()));
        assert!(infer_categories(&["FRIEND".into()], &None).contains(&"family-friends".to_string()));
        assert!(infer_categories(&[], &Some("Acme".into())).contains(&"organizations".to_string()));
        // Everyone is in People.
        assert!(infer_categories(&[], &None).contains(&"people".to_string()));
    }

    #[test]
    fn same_did_in_both_stores_merges_to_one_entry() {
        let actors = vec![actor("a1", "Dr Smith", "did:wf:smith", "PRACTITIONER", &["clinician"])];
        let contacts = vec![contact("Dr Smith", "did:wf:smith", &["health"])];
        let view = build_view_core(&actors, &contacts, &BTreeMap::new(), builtin_categories(), &[]);
        assert_eq!(view.entries.len(), 1, "one DID → one entry across both stores");
        let e = &view.entries[0];
        assert!(e.sources.contains(&"directory-actor".to_string()));
        assert!(e.sources.contains(&"contact".to_string()));
        assert!(e.categories.contains(&"health".to_string()));
    }

    #[test]
    fn explicit_assignment_overrides_inference() {
        let actors = vec![actor("a1", "Bob", "did:wf:bob", "FRIEND", &[])];
        let mut assignments = BTreeMap::new();
        assignments.insert("did:wf:bob".to_string(), vec!["cooperative".to_string()]);
        let view = build_view_core(&actors, &[], &assignments, builtin_categories(), &[]);
        assert_eq!(view.entries[0].categories, vec!["cooperative".to_string()]);
    }

    #[test]
    fn distinct_dids_stay_separate_and_sorted() {
        let actors = vec![
            actor("a1", "Zed", "did:wf:z", "FRIEND", &[]),
            actor("a2", "Ann", "did:wf:a", "FRIEND", &[]),
        ];
        let view = build_view_core(&actors, &[], &BTreeMap::new(), builtin_categories(), &[]);
        assert_eq!(view.entries.len(), 2);
        assert_eq!(view.entries[0].display_name, "Ann"); // sorted case-insensitively
        assert_eq!(view.entries[1].display_name, "Zed");
    }

    fn entries_of(actors: &[Actor], contacts: &[ChatContact]) -> Vec<DirectoryEntry> {
        build_view_core(actors, contacts, &BTreeMap::new(), builtin_categories(), &[]).entries
    }

    fn agreement(id: &str, relationship_did: &str, parties: &[&str]) -> Agreement {
        Agreement {
            id: id.into(),
            title: "Care relationship".into(),
            relationship_did: relationship_did.into(),
            parties: parties.iter().map(|s| s.to_string()).collect(),
            values_anchors: vec!["urn:qualia:values:udhr".into()],
            undertakings: vec![],
            consents: vec![],
            stage: crate::agreements::FormationStage::Draft,
            jurisdiction: None,
            intents: Vec::new(),
            artifact_context: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn agreements_join_onto_the_governed_party() {
        let actors = vec![
            actor("a1", "Dr Smith", "did:wf:smith", "PRACTITIONER", &["clinician"]),
            actor("a2", "Bob", "did:wf:bob", "FRIEND", &[]),
        ];
        // One agreement governs the relationship with Dr Smith (as relationship_did AND as a party);
        // none touches Bob.
        let ags = vec![agreement("ag-1", "did:wf:smith", &["did:wf:me", "did:wf:smith"])];
        let view = build_view_core(&actors, &[], &BTreeMap::new(), builtin_categories(), &ags);

        let smith = view.entries.iter().find(|e| e.did == "did:wf:smith").unwrap();
        assert_eq!(smith.agreement_ids, vec!["ag-1".to_string()], "agreement joins onto its party");
        let bob = view.entries.iter().find(|e| e.did == "did:wf:bob").unwrap();
        assert!(bob.agreement_ids.is_empty(), "unrelated party has no agreements");

        // The 'agreements' facet now distinguishes with/without.
        let r = search_core(view.entries, builtin_categories(), "", &BTreeMap::new());
        let facet = r.facets.iter().find(|f| f.id == "agreements").expect("agreements facet");
        assert!(facet.values.iter().any(|v| v.value == "with" && v.count == 1));
        assert!(facet.values.iter().any(|v| v.value == "without" && v.count == 1));
    }

    #[test]
    fn concept_search_matches_by_meaning() {
        let actors = vec![actor("a1", "Dr Smith", "did:wf:smith", "PRACTITIONER", &["clinician"])];
        let entries = entries_of(&actors, &[]);
        // "doctor" is not a literal token on the entry, but it shares a concept cluster with "clinician".
        let hit = search_core(entries.clone(), builtin_categories(), "doctor", &BTreeMap::new());
        assert_eq!(hit.total, 1, "concept expansion: 'doctor' finds a 'clinician'");
        // An unrelated concept does not match.
        let miss = search_core(entries, builtin_categories(), "cooperative", &BTreeMap::new());
        assert_eq!(miss.total, 0);
    }

    #[test]
    fn query_tokens_are_anded() {
        let actors = vec![
            actor("a1", "Dr Smith", "did:wf:smith", "FRIEND", &[]),
            actor("a2", "Dr Jones", "did:wf:jones", "FRIEND", &[]),
        ];
        let entries = entries_of(&actors, &[]);
        let r = search_core(entries, builtin_categories(), "dr smith", &BTreeMap::new());
        assert_eq!(r.total, 1);
        assert_eq!(r.entries[0].display_name, "Dr Smith");
    }

    #[test]
    fn facets_count_and_narrow() {
        let actors = vec![
            actor("a1", "Dr Smith", "did:wf:smith", "PRACTITIONER", &["clinician"]), // → health
            actor("a2", "Bob", "did:wf:bob", "FRIEND", &[]),                          // → family-friends
        ];
        let entries = entries_of(&actors, &[]);
        let all = search_core(entries.clone(), builtin_categories(), "", &BTreeMap::new());
        let cat_facet = all.facets.iter().find(|f| f.id == "category").expect("category facet");
        let health = cat_facet.values.iter().find(|v| v.value == "health").expect("health value");
        assert_eq!(health.count, 1);

        // Selecting the health category narrows to just the clinician.
        let mut sel = BTreeMap::new();
        sel.insert("category".to_string(), vec!["health".to_string()]);
        let narrowed = search_core(entries, builtin_categories(), "", &sel);
        assert_eq!(narrowed.total, 1);
        assert_eq!(narrowed.entries[0].display_name, "Dr Smith");
        // The category facet still reports drill-down counts (health selected, count preserved).
        let cat_facet = narrowed.facets.iter().find(|f| f.id == "category").unwrap();
        assert!(cat_facet.values.iter().any(|v| v.value == "health" && v.selected));
    }
}
