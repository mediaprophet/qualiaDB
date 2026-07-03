//! S3 — the **factor-knowledge base**: content-addressed, provenance-tagged, versioned templates that
//! turn a *named* thing (a condition, a herb, a food, a nutrient, a medication) into a [`Factor`] (for
//! the accumulation layer) or a [`FactorEvent`] (for the temporal layer).
//!
//! This is the reusable library S4 consults to convert a person's WellFair records / diet log into
//! factors. The engine (S1/S2) is authored by the agent; the **authoritative corpus is Timothy's to
//! supply** (trusted nutrition / traditional-medicine / clinical sources). What lives here is the
//! *machinery*: the schema, an integrity hash, a source-trust registry that structurally caps how
//! strong a claim a source may make, import adapters for the bundled knowledge, and a small, honestly
//! sourced **seed set** clearly marked as illustrative and meant to be replaced/extended.
//!
//! **Honesty properties:**
//! - Every entry is **content-addressed** (`content_hash`, SHA-256 over its canonical fields) so a
//!   tampered/edited datum is detectable, and **versioned**.
//! - Every entry carries **[`Provenance`]** naming its source; nothing is fetched live — datums are
//!   imported offline.
//! - A [`KnowledgeSource`] declares the *highest* [`EvidenceTier`] it may assert; [`KnowledgeBase`]
//!   **caps** each entry's evidence to its source's ceiling, so a community "hot take" source can never
//!   masquerade as clinical evidence and traditional knowledge is preserved at its own honest tier.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::factor::{Effect, EvidenceTier, Factor, FactorKind, FactorTarget};
use super::systems::body_system_by_label;
use super::temporal::{FactorEvent, Kinetics};

/// Where a knowledge datum came from. Imported offline; never a live fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Stable id of the source in the [`KnowledgeBase`] source registry (e.g. `"who-monographs"`).
    pub source_id: String,
    /// Human title of the source.
    pub source_title: String,
    /// Citation (URL / DOI / page) if known — left `None` rather than fabricated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    /// ISO date the datum was imported (offline), if recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_at: Option<String>,
}

/// One (system, effect, evidence, weight, kinetics) mapping in a knowledge template. Mirrors a
/// [`FactorTarget`] plus the default [`Kinetics`] the temporal layer uses when this becomes an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTarget {
    pub system_id: String,
    pub effect: Effect,
    pub evidence: EvidenceTier,
    pub weight_milli: u32,
    pub kinetics: Kinetics,
}

/// A reusable, provenance-tagged, versioned knowledge template for one named factor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorKnowledge {
    /// Stable lookup key, e.g. `"cond:hypertension"`, `"herb:milk-thistle"`, `"food:beer"`.
    pub key: String,
    pub kind: FactorKind,
    pub label: String,
    pub targets: Vec<KnowledgeTarget>,
    pub provenance: Provenance,
    pub version: u32,
    /// SHA-256 (hex) over the canonical fields above — set by [`FactorKnowledge::sealed`].
    #[serde(default)]
    pub content_hash: String,
}

impl FactorKnowledge {
    pub fn new(
        key: impl Into<String>,
        kind: FactorKind,
        label: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            key: key.into(),
            kind,
            label: label.into(),
            targets: Vec::new(),
            provenance,
            version: 1,
            content_hash: String::new(),
        }
    }

    pub fn targeting(
        mut self,
        system_id: impl Into<String>,
        effect: Effect,
        evidence: EvidenceTier,
        weight_milli: u32,
        kinetics: Kinetics,
    ) -> Self {
        self.targets.push(KnowledgeTarget {
            system_id: system_id.into(),
            effect,
            evidence,
            weight_milli: weight_milli.min(1000),
            kinetics,
        });
        self
    }

    /// Compute the content hash over the canonical fields (excludes `content_hash` itself).
    pub fn compute_content_hash(&self) -> String {
        // A deterministic tuple of the addressed fields — serde field/element order is stable.
        let canonical = serde_json::to_vec(&(
            &self.key,
            &self.kind,
            &self.label,
            &self.targets,
            &self.provenance,
            self.version,
        ))
        .expect("knowledge entry serializes");
        let digest = Sha256::digest(&canonical);
        hex::encode(digest)
    }

    /// Finalize the entry by stamping its content hash. Call after building the targets.
    pub fn sealed(mut self) -> Self {
        self.content_hash = self.compute_content_hash();
        self
    }

    /// Whether the stamped `content_hash` matches the current fields (integrity check).
    pub fn integrity_ok(&self) -> bool {
        !self.content_hash.is_empty() && self.content_hash == self.compute_content_hash()
    }

    /// Instantiate a non-temporal [`Factor`] for a specific occurrence (`instance_id`).
    pub fn to_factor(&self, instance_id: impl Into<String>) -> Factor {
        let mut f = Factor::new(instance_id, self.kind.clone(), self.label.clone())
            .from_source(self.provenance.source_id.clone());
        f.targets = self
            .targets
            .iter()
            .map(|t| FactorTarget {
                system_id: t.system_id.clone(),
                effect: t.effect,
                evidence: t.evidence,
                weight_milli: t.weight_milli,
            })
            .collect();
        f
    }

    /// Instantiate a temporal [`FactorEvent`] at `at_minute` with a dose scaler, wiring each target's
    /// knowledge kinetics onto the event so different systems clear on their own clocks.
    pub fn to_event(
        &self,
        instance_id: impl Into<String>,
        at_minute: i64,
        dose_scale_pct: u32,
    ) -> FactorEvent {
        let factor = self.to_factor(instance_id);
        let mut ev = FactorEvent::new(factor, at_minute).with_dose_pct(dose_scale_pct);
        for t in &self.targets {
            ev = ev.with_system_kinetics(t.system_id.clone(), t.kinetics);
        }
        ev
    }
}

/// A trusted knowledge source and the *ceiling* evidence tier it may assert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeSource {
    pub id: String,
    pub title: String,
    /// The strongest tier a datum from this source may carry — entries are capped to it on insert.
    pub trust_ceiling: EvidenceTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl KnowledgeSource {
    pub fn new(id: impl Into<String>, title: impl Into<String>, trust_ceiling: EvidenceTier) -> Self {
        Self { id: id.into(), title: title.into(), trust_ceiling, url: None }
    }
}

/// The factor-knowledge base: entries keyed by their stable `key`, plus the source registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub sources: Vec<KnowledgeSource>,
    entries: BTreeMap<String, FactorKnowledge>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_source(&mut self, source: KnowledgeSource) {
        if let Some(existing) = self.sources.iter_mut().find(|s| s.id == source.id) {
            *existing = source;
        } else {
            self.sources.push(source);
        }
    }

    /// The trust ceiling for a source id (unknown sources cannot exceed the lowest tier).
    fn ceiling_for(&self, source_id: &str) -> EvidenceTier {
        self.sources
            .iter()
            .find(|s| s.id == source_id)
            .map(|s| s.trust_ceiling)
            .unwrap_or(EvidenceTier::CommunityAnecdotal)
    }

    /// Insert an entry, **capping** each target's evidence tier to the entry's source ceiling, then
    /// (re)sealing the content hash so the stored hash reflects the capped tiers. This is the
    /// structural guarantee that a source cannot overclaim.
    pub fn insert(&mut self, mut entry: FactorKnowledge) {
        let ceiling = self.ceiling_for(&entry.provenance.source_id);
        for t in &mut entry.targets {
            if t.evidence > ceiling {
                t.evidence = ceiling;
            }
        }
        let sealed = entry.sealed();
        self.entries.insert(sealed.key.clone(), sealed);
    }

    pub fn get(&self, key: &str) -> Option<&FactorKnowledge> {
        self.entries.get(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FactorKnowledge> {
        self.entries.values()
    }

    pub fn by_kind<'a>(&'a self, kind: &'a FactorKind) -> impl Iterator<Item = &'a FactorKnowledge> {
        self.entries.values().filter(move |e| &e.kind == kind)
    }

    /// Verify every stored entry's content hash. Returns the keys of any that fail (empty = all good).
    pub fn verify_integrity(&self) -> Vec<String> {
        self.entries.values().filter(|e| !e.integrity_ok()).map(|e| e.key.clone()).collect()
    }
}

// ---- Import adapters (offline; caller supplies the file contents) ---------------------------------

#[derive(Deserialize)]
struct ConditionMapFile {
    conditions: BTreeMap<String, ConditionEntry>,
}

#[derive(Deserialize)]
struct ConditionEntry {
    #[serde(rename = "primarySystem")]
    primary_system: String,
    #[serde(rename = "ontologyIri", default)]
    #[allow(dead_code)]
    ontology_iri: Option<String>,
}

/// The result of an import: the entries that resolved, plus honest warnings for any that didn't.
pub struct ImportResult {
    pub entries: Vec<FactorKnowledge>,
    pub warnings: Vec<String>,
}

/// Import the bundled `condition-map.json` (condition → primary system label). Each condition becomes a
/// chronic clinical [`FactorKnowledge`] targeting its primary system. Unresolvable system labels are
/// skipped with a warning rather than silently dropped.
pub fn import_condition_map(json: &str, provenance: Provenance) -> Result<ImportResult, String> {
    let file: ConditionMapFile =
        serde_json::from_str(json).map_err(|e| format!("condition-map.json parse error: {e}"))?;
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for (name, entry) in file.conditions {
        match body_system_by_label(&entry.primary_system) {
            Some(sys) => {
                let key = format!("cond:{}", slugify(&name));
                entries.push(
                    FactorKnowledge::new(key, FactorKind::Condition, name, provenance.clone())
                        .targeting(
                            sys.id,
                            Effect::Adverse,
                            EvidenceTier::ClinicalEvidence,
                            500,
                            Kinetics::CHRONIC, // a standing condition holds until resolved/managed
                        )
                        .sealed(),
                );
            }
            None => warnings.push(format!(
                "condition '{name}': primary system '{}' did not resolve to a known body system",
                entry.primary_system
            )),
        }
    }
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(ImportResult { entries, warnings })
}

/// Import a native knowledge JSON (an array of [`FactorKnowledge`]). The content hash of each entry is
/// **recomputed and verified** against the imported `content_hash` (if present); a mismatch is a
/// warning, not a silent accept.
pub fn import_entries(json: &str) -> Result<ImportResult, String> {
    let raw: Vec<FactorKnowledge> =
        serde_json::from_str(json).map_err(|e| format!("knowledge JSON parse error: {e}"))?;
    let mut warnings = Vec::new();
    let mut entries = Vec::new();
    for e in raw {
        let recomputed = e.compute_content_hash();
        if !e.content_hash.is_empty() && e.content_hash != recomputed {
            warnings.push(format!(
                "entry '{}': content hash mismatch (imported {}, computed {})",
                e.key, e.content_hash, recomputed
            ));
        }
        entries.push(e.sealed());
    }
    Ok(ImportResult { entries, warnings })
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

// ---- Honest seed set ------------------------------------------------------------------------------

/// A small, honestly sourced **illustrative seed** — enough to exercise every tier and the temporal
/// wiring end-to-end. It is **not** an authoritative corpus: entries carry conservative weights, no
/// fabricated citations, and general well-known reference *classes* as sources. Timothy supplies /
/// points at the real corpus (⚑ S3 curation datum); replacing or extending this is expected.
///
/// The source-trust ceilings mean the community entry cannot overclaim: even if its target asserted
/// clinical evidence, [`KnowledgeBase::insert`] caps it to `CommunityAnecdotal`.
pub fn seed_knowledge_base() -> KnowledgeBase {
    let mut kb = KnowledgeBase::new();

    kb.register_source(KnowledgeSource::new(
        "clinical-reference",
        "General clinical references (condition→primary-system, well established)",
        EvidenceTier::ClinicalEvidence,
    ));
    kb.register_source(KnowledgeSource::new(
        "who-monographs",
        "WHO monographs on selected medicinal plants (traditional-use class)",
        EvidenceTier::TraditionalUse,
    ));
    kb.register_source(KnowledgeSource::new(
        "nutrition-db",
        "Food-composition / nutrition database (nutritional-data class)",
        EvidenceTier::NutritionalData,
    ));
    kb.register_source(KnowledgeSource::new(
        "community-anecdotal",
        "Community / internet posts (unverified 'hot takes')",
        EvidenceTier::CommunityAnecdotal,
    ));

    let who = || Provenance {
        source_id: "who-monographs".into(),
        source_title: "WHO monographs on selected medicinal plants".into(),
        citation: None,
        imported_at: None,
    };
    let nutrition = || Provenance {
        source_id: "nutrition-db".into(),
        source_title: "Food-composition / nutrition database".into(),
        citation: None,
        imported_at: None,
    };
    let community = || Provenance {
        source_id: "community-anecdotal".into(),
        source_title: "Community / internet posts".into(),
        citation: None,
        imported_at: None,
    };

    // Traditional-use: milk thistle (Silybum marianum) — traditionally used for liver support.
    kb.insert(
        FactorKnowledge::new("herb:milk-thistle", FactorKind::Herb, "Milk thistle", who()).targeting(
            "digestive",
            Effect::Supportive,
            EvidenceTier::TraditionalUse,
            200,
            Kinetics::new(30, 12 * 60),
        ),
    );
    // Traditional-use: chamomile tea — traditionally used to calm / aid sleep (nervous system).
    kb.insert(
        FactorKnowledge::new("tea:chamomile", FactorKind::Tea, "Chamomile tea", who()).targeting(
            "nervous",
            Effect::Supportive,
            EvidenceTier::TraditionalUse,
            120,
            Kinetics::new(20, 3 * 60),
        ),
    );
    // Nutritional-data: beer — an alcohol-containing intake loading hepatic + renal systems.
    kb.insert(
        FactorKnowledge::new("food:beer", FactorKind::Food, "Beer (alcohol)", nutrition())
            .targeting("digestive", Effect::Adverse, EvidenceTier::NutritionalData, 300, Kinetics::new(30, 5 * 60))
            .targeting("urinary", Effect::Adverse, EvidenceTier::NutritionalData, 250, Kinetics::new(60, 3 * 60)),
    );
    // Nutritional-data: water + electrolytes — a rehydration intervention (renal support).
    kb.insert(
        FactorKnowledge::new(
            "food:water-electrolytes",
            FactorKind::WholeFood,
            "Water + electrolytes",
            nutrition(),
        )
        .targeting("urinary", Effect::Supportive, EvidenceTier::NutritionalData, 500, Kinetics::new(20, 4 * 60)),
    );
    // Community "hot take" — deliberately over-tagged as clinical to prove the cap forces it down.
    kb.insert(
        FactorKnowledge::new(
            "tea:detox-claim",
            FactorKind::Tea,
            "'Detox' tea (some people say it cleanses the liver)",
            community(),
        )
        .targeting(
            "digestive",
            Effect::Supportive,
            EvidenceTier::ClinicalEvidence, // will be capped to CommunityAnecdotal on insert
            80,
            Kinetics::new(30, 2 * 60),
        ),
    );

    kb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{accumulate, Timeline};

    #[test]
    fn content_hash_seals_and_detects_tampering() {
        let mut e = FactorKnowledge::new(
            "herb:x",
            FactorKind::Herb,
            "X",
            Provenance {
                source_id: "who-monographs".into(),
                source_title: "WHO".into(),
                citation: None,
                imported_at: None,
            },
        )
        .targeting("digestive", Effect::Supportive, EvidenceTier::TraditionalUse, 100, Kinetics::CHRONIC)
        .sealed();
        assert!(e.integrity_ok());
        // Tamper with a weight after sealing → integrity fails.
        e.targets[0].weight_milli = 999;
        assert!(!e.integrity_ok());
    }

    #[test]
    fn source_trust_ceiling_caps_overclaimed_evidence() {
        let kb = seed_knowledge_base();
        let hot_take = kb.get("tea:detox-claim").unwrap();
        // Authored as ClinicalEvidence but capped to the community source's ceiling.
        assert_eq!(hot_take.targets[0].evidence, EvidenceTier::CommunityAnecdotal);
        assert!(hot_take.integrity_ok(), "hash reflects the capped tier");
        // A traditional-use source keeps its own honest tier (not subordinated, not erased).
        assert_eq!(
            kb.get("herb:milk-thistle").unwrap().targets[0].evidence,
            EvidenceTier::TraditionalUse
        );
        assert!(kb.verify_integrity().is_empty());
    }

    #[test]
    fn import_condition_map_resolves_labels_and_warns_on_unknown() {
        let json = r#"{
            "conditions": {
                "Hypertension": { "primarySystem": "Circulatory (Cardiovascular) System" },
                "Made Up": { "primarySystem": "Imaginary System" }
            }
        }"#;
        let prov = Provenance {
            source_id: "clinical-reference".into(),
            source_title: "condition-map".into(),
            citation: None,
            imported_at: None,
        };
        let res = import_condition_map(json, prov).unwrap();
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].key, "cond:hypertension");
        assert_eq!(res.entries[0].targets[0].system_id, "circulatory");
        assert!(res.entries[0].integrity_ok());
        assert_eq!(res.warnings.len(), 1, "the imaginary system is reported, not silently dropped");
    }

    #[test]
    fn seed_entries_instantiate_and_flow_through_the_temporal_engine() {
        let kb = seed_knowledge_base();
        // The beer template → a dosed event; still loads hepatic well after onset.
        let beer = kb.get("food:beer").unwrap().to_event("intake:beer-1", 0, 300);
        let tl = Timeline::new().with_event(beer);
        let dig = tl.burden_at(60).into_iter().find(|b| b.system_id == "digestive").unwrap();
        assert!(dig.net_milli > 0, "hepatic load present after the beer event");

        // The non-temporal instantiation accumulates via slice-1 too.
        let f = kb.get("herb:milk-thistle").unwrap().to_factor("intake:mt-1");
        let burdens = accumulate(&[f]);
        assert!(burdens.iter().any(|b| b.system_id == "digestive" && b.supportive_milli > 0));
    }

    #[test]
    fn import_entries_flags_a_tampered_hash() {
        let good = seed_knowledge_base();
        let entry = good.get("tea:chamomile").unwrap().clone();
        let mut json_val = serde_json::to_value([&entry]).unwrap();
        // Corrupt the stored hash of the first entry.
        json_val[0]["content_hash"] = serde_json::Value::String("deadbeef".into());
        let res = import_entries(&json_val.to_string()).unwrap();
        assert_eq!(res.warnings.len(), 1);
        // But the re-sealed entry is internally consistent again.
        assert!(res.entries[0].integrity_ok());
    }
}
