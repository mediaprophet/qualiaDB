//! Ethnicity and genetic ancestry as **knowledge context**, not appearance and not a body type.
//!
//! Self-identified ethnicity is biomedical *and* personal. It can change which prevalence,
//! pharmacology, and screening edges are *relevant* (always [`EpistemicStatus::Hypothesis`]).
//! It must not pick a stereotyped mesh, invent skin/hair/eyes, or stand in for karyotype.
//! Cosmetic appearance stays on explicit eye / hair / skin declarations.
//!
//! There is **no closed racial taxonomy** in the engine. Tokens are the person's words (or a
//! jurisdiction pack IRI). Genetic ancestry is a **separate** record and only exists if they
//! imported one they chose — never inferred from a photo or from ethnicity.
//!
//! The shipped [`illustrative_context_pack`] is machinery + a few well-cited public examples.
//! It is not a guideline set. Timothy / a clinician pack replaces it.

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

use super::factor::EvidenceTier;
use super::observations::{BodyObservation, RepresentationBind, bind_for_code};
use super::slugify;

/// One self-identified affiliation. Repeatable. Open vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthnicityAffiliation {
    /// Slug or pack IRI used for matching. Not a race enum.
    pub token: String,
    /// What they wrote (or the pack's display label).
    pub label: String,
}

impl EthnicityAffiliation {
    pub fn declared(label: &str) -> Option<Self> {
        let label = label.trim();
        if label.is_empty() {
            return None;
        }
        Some(Self {
            token: slugify(label),
            label: label.to_string(),
        })
    }
}

/// Why an ancestry record was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AncestryInvalid {
    /// Ancestry is import-only. Typing ethnicity, a photo, or an empty source is not enough.
    NotImported,
    EmptyLabel,
}

/// Genetic-ancestry context. Exists only when the person imported a record they chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryRecord {
    pub token: String,
    pub label: String,
    /// Provenance of the imported record (lab, file, pack id). Required.
    pub imported_from: String,
}

impl AncestryRecord {
    pub fn from_import(label: &str, imported_from: &str) -> Result<Self, AncestryInvalid> {
        let label = label.trim();
        let imported_from = imported_from.trim();
        if label.is_empty() {
            return Err(AncestryInvalid::EmptyLabel);
        }
        if imported_from.is_empty() {
            return Err(AncestryInvalid::NotImported);
        }
        Ok(Self {
            token: slugify(label),
            label: label.to_string(),
            imported_from: imported_from.to_string(),
        })
    }
}

/// The person's declared / imported knowledge context. Never a fit input.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectKnowledgeContext {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ethnicities: Vec<EthnicityAffiliation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genetic_ancestry: Vec<AncestryRecord>,
}

impl SubjectKnowledgeContext {
    pub fn is_empty(&self) -> bool {
        self.ethnicities.is_empty() && self.genetic_ancestry.is_empty()
    }

    /// Slugs used to match a context pack (ethnicity ∪ imported ancestry).
    pub fn match_tokens(&self) -> Vec<String> {
        let mut out = Vec::new();
        for e in &self.ethnicities {
            push_token(&mut out, &e.token);
        }
        for a in &self.genetic_ancestry {
            push_token(&mut out, &a.token);
        }
        out
    }
}

/// Kind of biomedical edge a context token can activate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextEdgeKind {
    Prevalence,
    Pharmacology,
    Screening,
}

/// One curated (or illustrative) context → topic edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeContextEdge {
    pub id: String,
    pub kind: ContextEdgeKind,
    /// Open tokens that activate this edge (slugs). Not a closed race list.
    pub context_tokens: Vec<String>,
    /// Topic key (`g6pd`, `carbamazepine-hla-b1502`, a MONDO / drug slug).
    pub topic: String,
    pub note: String,
    pub evidence: EvidenceTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    /// Shipped example, not a curated guideline pack.
    #[serde(default)]
    pub illustrative: bool,
}

/// A hypothesis produced when the person's context matches a pack edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeConsideration {
    pub kind: ContextEdgeKind,
    pub topic: String,
    pub note: String,
    pub matched_tokens: Vec<String>,
    pub evidence: EvidenceTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    pub epistemic_status: EpistemicStatus,
    pub illustrative: bool,
}

/// Collect declared ethnicity observations. Genetic-ancestry codes are **not** turned into
/// affiliations here — ancestry requires [`AncestryRecord::from_import`].
pub fn affiliations_from_observations(obs: &[BodyObservation]) -> Vec<EthnicityAffiliation> {
    let mut out = Vec::new();
    for o in obs {
        if o.code.trim() != "q42:ethnicity" {
            continue;
        }
        if bind_for_code(&o.code) != RepresentationBind::KnowledgeContext {
            continue;
        }
        if let Some(a) = o.named.as_deref().and_then(EthnicityAffiliation::declared) {
            if !out
                .iter()
                .any(|x: &EthnicityAffiliation| x.token == a.token)
            {
                out.push(a);
            }
        }
    }
    out
}

/// Match the person's context against a pack. Empty pack or no matching tokens → no considerations
/// (honest empty, not a guess). Every hit is a [`EpistemicStatus::Hypothesis`].
///
/// `live_topics` filters to issues already on the graph (MONDO / drug / UniProt slugs). Empty
/// means "show every matching edge" (education / any-person study).
pub fn considerations_for_context(
    subject: &SubjectKnowledgeContext,
    pack: &[KnowledgeContextEdge],
    live_topics: &[String],
) -> Vec<KnowledgeConsideration> {
    let tokens = subject.match_tokens();
    if tokens.is_empty() {
        return Vec::new();
    }
    let live: Vec<String> = live_topics.iter().map(|t| slugify(t)).collect();
    let mut out = Vec::new();
    for edge in pack {
        let matched: Vec<String> = edge
            .context_tokens
            .iter()
            .map(|t| slugify(t))
            .filter(|et| tokens.iter().any(|st| tokens_match(st, et)))
            .collect();
        if matched.is_empty() {
            continue;
        }
        if !live.is_empty() {
            let topic = slugify(&edge.topic);
            if !live.iter().any(|t| t == &topic || tokens_match(t, &topic)) {
                continue;
            }
        }
        out.push(KnowledgeConsideration {
            kind: edge.kind,
            topic: edge.topic.clone(),
            note: edge.note.clone(),
            matched_tokens: matched,
            evidence: edge.evidence,
            citation: edge.citation.clone(),
            epistemic_status: EpistemicStatus::Hypothesis,
            illustrative: edge.illustrative,
        });
    }
    out.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.topic.cmp(&b.topic))
            .then(a.note.cmp(&b.note))
    });
    out
}

/// Small public-knowledge examples so the machinery is exercisable. Not a guideline set,
/// not a racial typology, not appearance.
pub fn illustrative_context_pack() -> Vec<KnowledgeContextEdge> {
    vec![
        KnowledgeContextEdge {
            id: "illust-g6pd-pharm".into(),
            kind: ContextEdgeKind::Pharmacology,
            context_tokens: vec![
                "mediterranean".into(),
                "african".into(),
                "west-african".into(),
                "middle-eastern".into(),
                "southeast-asian".into(),
                "kurdish".into(),
            ],
            topic: "g6pd".into(),
            note: "Some populations in which this affiliation is often recorded have a higher \
published prevalence of G6PD deficiency. That can be relevant to oxidant drugs \
(e.g. primaquine) and fava beans — a pharmacology hypothesis to discuss, not a diagnosis \
and not a type of human."
                .into(),
            evidence: EvidenceTier::ClinicalEvidence,
            citation: Some("WHO G6PD deficiency (public monograph)".into()),
            illustrative: true,
        },
        KnowledgeContextEdge {
            id: "illust-hla-b1502".into(),
            kind: ContextEdgeKind::Pharmacology,
            context_tokens: vec![
                "han-chinese".into(),
                "thai".into(),
                "malay".into(),
                "filipino".into(),
                "east-asian".into(),
                "southeast-asian".into(),
            ],
            topic: "carbamazepine-hla-b1502".into(),
            note: "HLA-B*1502 is more frequent in some East and South-East Asian groups and is \
linked to severe cutaneous reactions with carbamazepine. A labelled pharmacology \
hypothesis (FDA boxed warning), not a prescription decision and not a facial type."
                .into(),
            evidence: EvidenceTier::ClinicalEvidence,
            citation: Some("FDA carbamazepine boxed warning — HLA-B*1502 / SJS-TEN".into()),
            illustrative: true,
        },
        KnowledgeContextEdge {
            id: "illust-sickle-screen".into(),
            kind: ContextEdgeKind::Screening,
            context_tokens: vec![
                "west-african".into(),
                "central-african".into(),
                "african".into(),
                "mediterranean".into(),
                "middle-eastern".into(),
                "south-asian".into(),
            ],
            topic: "sickle-cell".into(),
            note: "Sickle-cell trait and disease have higher published prevalence in several \
African, Mediterranean, Middle-Eastern, and South-Asian populations. A screening \
hypothesis if relevant to the person — never inferred from appearance."
                .into(),
            evidence: EvidenceTier::ClinicalEvidence,
            citation: Some("CDC sickle cell disease (public fact sheet)".into()),
            illustrative: true,
        },
        KnowledgeContextEdge {
            id: "illust-taysachs-screen".into(),
            kind: ContextEdgeKind::Screening,
            context_tokens: vec![
                "ashkenazi".into(),
                "ashkenazi-jewish".into(),
                "french-canadian".into(),
                "cajun".into(),
            ],
            topic: "tay-sachs".into(),
            note:
                "Tay-Sachs carrier frequency is higher in some Ashkenazi Jewish, French-Canadian, \
and Cajun communities. A carrier-screening hypothesis, not a diagnosis and not a \
cosmetic cue."
                    .into(),
            evidence: EvidenceTier::ClinicalEvidence,
            citation: Some("ACMG / ACOG carrier-screening statements (public)".into()),
            illustrative: true,
        },
    ]
}

fn push_token(out: &mut Vec<String>, token: &str) {
    let t = slugify(token);
    if t.is_empty() {
        return;
    }
    if !out.iter().any(|x| x == &t) {
        out.push(t);
    }
}

fn tokens_match(subject: &str, edge: &str) -> bool {
    subject == edge || subject.starts_with(edge) || edge.starts_with(subject)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::constitution::{BodyConstitution, BodyFit};
    use crate::anatomy::observations::{InstrumentKind, bind_for_code};

    #[test]
    fn ethnicity_does_not_change_body_fit() {
        let mut with = BodyConstitution::default();
        with.knowledge
            .ethnicities
            .push(EthnicityAffiliation::declared("Ashkenazi Jewish").unwrap());
        with.knowledge
            .ethnicities
            .push(EthnicityAffiliation::declared("Greek").unwrap());
        let empty = BodyConstitution::default();
        let a = with.fit();
        let b = empty.fit();
        assert!(same_geometry(&a, &b));
        assert!(a.identity);
        assert!(
            a.honesty_notes
                .iter()
                .any(|n| n.contains("knowledge context"))
        );
        assert!(!a.used_fields.iter().any(|f| f.contains("ethnic")));
    }

    #[test]
    fn ancestry_requires_an_import_the_person_chose() {
        assert_eq!(
            AncestryRecord::from_import("Yoruba", ""),
            Err(AncestryInvalid::NotImported)
        );
        assert!(AncestryRecord::from_import("Yoruba", "imported:lab-report.vcf").is_ok());
    }

    #[test]
    fn ethnicity_observation_is_not_ancestry_and_not_appearance() {
        let obs = [BodyObservation {
            code: "q42:ethnicity".into(),
            value_milli: 0,
            unit: "{named}".into(),
            instrument: InstrumentKind::Declared,
            at_unix: 1,
            note: None,
            named: Some("Han Chinese".into()),
        }];
        let aff = affiliations_from_observations(&obs);
        assert_eq!(aff.len(), 1);
        assert_eq!(aff[0].token, "han-chinese");
        assert_eq!(
            bind_for_code("q42:ethnicity"),
            RepresentationBind::KnowledgeContext
        );
        assert_ne!(
            bind_for_code("q42:ethnicity"),
            RepresentationBind::AppearanceSkin
        );
        assert_eq!(
            bind_for_code("q42:skin-tone"),
            RepresentationBind::AppearanceSkin
        );
        // Ethnicity never becomes an ancestry record.
        assert!(AncestryRecord::from_import(&aff[0].label, "").is_err());
    }

    #[test]
    fn considerations_are_hypotheses_and_token_matched() {
        let mut subject = SubjectKnowledgeContext::default();
        subject
            .ethnicities
            .push(EthnicityAffiliation::declared("Ashkenazi").unwrap());
        let all = considerations_for_context(&subject, &illustrative_context_pack(), &[]);
        assert!(all.iter().any(|c| c.topic == "tay-sachs"));
        assert!(
            all.iter()
                .all(|c| c.epistemic_status == EpistemicStatus::Hypothesis)
        );
        assert!(all.iter().all(|c| c.illustrative));
        assert!(!all.iter().any(|c| c.topic == "g6pd"));

        let filtered = considerations_for_context(
            &subject,
            &illustrative_context_pack(),
            &["tay-sachs".into()],
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].kind, ContextEdgeKind::Screening);
    }

    #[test]
    fn empty_context_or_pack_is_honest_empty() {
        let subject = SubjectKnowledgeContext::default();
        assert!(considerations_for_context(&subject, &illustrative_context_pack(), &[]).is_empty());
        let mut declared = SubjectKnowledgeContext::default();
        declared
            .ethnicities
            .push(EthnicityAffiliation::declared("Sámi").unwrap());
        assert!(considerations_for_context(&declared, &[], &[]).is_empty());
        // Unmatched token: no invented consideration.
        assert!(
            considerations_for_context(&declared, &illustrative_context_pack(), &[]).is_empty()
        );
    }

    #[test]
    fn repeatable_affiliations_and_imported_ancestry_both_match() {
        let mut subject = SubjectKnowledgeContext::default();
        subject
            .ethnicities
            .push(EthnicityAffiliation::declared("Italian").unwrap());
        subject
            .genetic_ancestry
            .push(AncestryRecord::from_import("West African", "imported:ancestry.pack").unwrap());
        let hits = considerations_for_context(&subject, &illustrative_context_pack(), &[]);
        assert!(hits.iter().any(|c| c.topic == "sickle-cell"));
        assert!(hits.iter().any(|c| c.topic == "g6pd"));
    }

    fn same_geometry(a: &BodyFit, b: &BodyFit) -> bool {
        a.stature_scale == b.stature_scale
            && a.torso_scale_y == b.torso_scale_y
            && a.leg_scale_y == b.leg_scale_y
            && a.arm_span_scale_x == b.arm_span_scale_x
            && a.shoulder_scale_x == b.shoulder_scale_x
            && a.chest_radial == b.chest_radial
            && a.waist_radial == b.waist_radial
            && a.hip_radial == b.hip_radial
            && a.pregnancy_abdomen == b.pregnancy_abdomen
            && a.hidden_keys == b.hidden_keys
            && a.identity == b.identity
    }
}
