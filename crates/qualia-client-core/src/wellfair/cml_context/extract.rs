//! Deterministic signal extractors over plain text (legislation, policy, general prose).
//!
//! These are **heuristic proposals** for the CML layer — not legal advice and not attested.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Proposed deontic class for a provision / paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeonticClass {
    Obligation,
    Permission,
    Prohibition,
    Right,
    /// Neutral / descriptive / machinery — no clear duty.
    Undertaking,
}

impl DeonticClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Obligation => "obligation",
            Self::Permission => "permission",
            Self::Prohibition => "prohibition",
            Self::Right => "right",
            Self::Undertaking => "undertaking",
        }
    }

    pub fn cml_type(self) -> &'static str {
        match self {
            Self::Obligation => "values:Obligation",
            Self::Permission => "values:Permission",
            Self::Prohibition => "values:Prohibition",
            Self::Right => "values:Right",
            Self::Undertaking => "values:Undertaking",
        }
    }
}

/// A named signal hit (privacy family, rights, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalHit {
    pub family: String,
    pub signal: String,
    pub confidence: u8,
}

/// Privacy / data-protection family (GDPR-like and cognates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacySignal {
    PersonalData,
    SpecialCategory,
    DataSubject,
    Controller,
    Processor,
    Consent,
    LawfulBasis,
    PurposeLimitation,
    DataMinimisation,
    StorageLimitation,
    IntegrityConfidentiality,
    Accountability,
    Erasure,
    AccessRight,
    Portability,
    Objection,
    Rectification,
    Restriction,
    AutomatedDecision,
    Dpia,
    CrossBorderTransfer,
    BreachNotification,
    Children,
    Surveillance,
}

impl PrivacySignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PersonalData => "personal-data",
            Self::SpecialCategory => "special-category-data",
            Self::DataSubject => "data-subject",
            Self::Controller => "controller",
            Self::Processor => "processor",
            Self::Consent => "consent",
            Self::LawfulBasis => "lawful-basis",
            Self::PurposeLimitation => "purpose-limitation",
            Self::DataMinimisation => "data-minimisation",
            Self::StorageLimitation => "storage-limitation",
            Self::IntegrityConfidentiality => "integrity-confidentiality",
            Self::Accountability => "accountability",
            Self::Erasure => "erasure-right-to-be-forgotten",
            Self::AccessRight => "access-right",
            Self::Portability => "data-portability",
            Self::Objection => "right-to-object",
            Self::Rectification => "rectification",
            Self::Restriction => "restriction-of-processing",
            Self::AutomatedDecision => "automated-decision-making",
            Self::Dpia => "dpia",
            Self::CrossBorderTransfer => "cross-border-transfer",
            Self::BreachNotification => "breach-notification",
            Self::Children => "children-data",
            Self::Surveillance => "surveillance",
        }
    }
}

/// Classify deontic force from English legislative / policy phrasing.
pub fn classify_deontic(text: &str) -> (DeonticClass, u8) {
    let t = text.to_ascii_lowercase();
    // Order matters: stronger / more specific first.
    let forbid = [
        "must not",
        "shall not",
        "may not",
        "is prohibited",
        "are prohibited",
        "it is an offence",
        "commits an offence",
        "is guilty of an offence",
        "must never",
        "shall never",
        "is forbidden",
        "not permitted to",
        "unlawful to",
    ];
    if forbid.iter().any(|p| t.contains(p)) {
        return (DeonticClass::Prohibition, 88);
    }
    let right = [
        "has a right to",
        "have a right to",
        "is entitled to",
        "are entitled to",
        "right of access",
        "right to erasure",
        "right to be forgotten",
        "right to object",
        "right to data portability",
        "right to rectification",
        "right to restriction",
        "fundamental right",
        "human right",
    ];
    if right.iter().any(|p| t.contains(p)) {
        return (DeonticClass::Right, 86);
    }
    let oblig = [
        " must ",
        " shall ",
        "is required to",
        "are required to",
        "is obliged to",
        "duty to",
        "obligation to",
        "must ensure",
        "shall ensure",
        "must provide",
        "shall provide",
        "must notify",
        "shall notify",
    ];
    // Leading "Must" / "Shall" at sentence start
    if t.starts_with("must ") || t.starts_with("shall ") {
        return (DeonticClass::Obligation, 84);
    }
    if oblig.iter().any(|p| t.contains(p)) {
        return (DeonticClass::Obligation, 82);
    }
    let permit = [
        " may ",
        "is permitted to",
        "are permitted to",
        "is authorised to",
        "is authorized to",
        "has power to",
        "may elect",
        "at the discretion",
    ];
    if t.starts_with("may ") {
        return (DeonticClass::Permission, 78);
    }
    if permit.iter().any(|p| t.contains(p)) {
        return (DeonticClass::Permission, 76);
    }
    (DeonticClass::Undertaking, 40)
}

/// Extract GDPR-like / privacy-family signals.
pub fn extract_privacy_signals(text: &str) -> Vec<SignalHit> {
    let t = text.to_ascii_lowercase();
    let mut hits = Vec::new();
    let mut push = |sig: PrivacySignal, conf: u8, needles: &[&str]| {
        if needles.iter().any(|n| t.contains(n)) {
            hits.push(SignalHit {
                family: "privacy".into(),
                signal: sig.as_str().into(),
                confidence: conf,
            });
        }
    };
    push(
        PrivacySignal::PersonalData,
        90,
        &[
            "personal data",
            "personal information",
            "personally identifiable",
            "pii",
            "information about an individual",
            "identifiable natural person",
        ],
    );
    push(
        PrivacySignal::SpecialCategory,
        92,
        &[
            "special category",
            "sensitive personal",
            "racial or ethnic",
            "political opinion",
            "religious belief",
            "trade union membership",
            "genetic data",
            "biometric data",
            "health data",
            "sex life",
            "sexual orientation",
        ],
    );
    push(
        PrivacySignal::DataSubject,
        88,
        &["data subject", "individual concerned", "person to whom"],
    );
    push(
        PrivacySignal::Controller,
        88,
        &[
            "data controller",
            "controller shall",
            "controller must",
            "as controller",
        ],
    );
    push(
        PrivacySignal::Processor,
        88,
        &[
            "data processor",
            "processor shall",
            "processor must",
            "as processor",
        ],
    );
    push(
        PrivacySignal::Consent,
        85,
        &[
            "consent",
            "freely given",
            "informed consent",
            "withdraw consent",
        ],
    );
    push(
        PrivacySignal::LawfulBasis,
        87,
        &[
            "lawful basis",
            "lawful ground",
            "legitimate interest",
            "legal obligation",
            "vital interest",
            "public task",
            "contractual necessity",
        ],
    );
    push(
        PrivacySignal::PurposeLimitation,
        84,
        &[
            "purpose limitation",
            "specified purpose",
            "compatible purpose",
            "further processing",
        ],
    );
    push(
        PrivacySignal::DataMinimisation,
        84,
        &[
            "data minimisation",
            "data minimization",
            "not excessive",
            "adequate, relevant",
        ],
    );
    push(
        PrivacySignal::StorageLimitation,
        84,
        &[
            "storage limitation",
            "no longer than necessary",
            "retention period",
            "kept no longer",
        ],
    );
    push(
        PrivacySignal::IntegrityConfidentiality,
        83,
        &[
            "integrity and confidentiality",
            "appropriate security",
            "technical and organisational",
            "technical and organizational",
            "security of processing",
            "encryption",
            "pseudonymisation",
            "pseudonymization",
        ],
    );
    push(
        PrivacySignal::Accountability,
        82,
        &[
            "accountability",
            "demonstrate compliance",
            "records of processing",
        ],
    );
    push(
        PrivacySignal::Erasure,
        90,
        &[
            "right to erasure",
            "right to be forgotten",
            "erase personal data",
            "delete the personal",
            "destruction of personal",
        ],
    );
    push(
        PrivacySignal::AccessRight,
        88,
        &[
            "right of access",
            "subject access",
            "access to personal data",
            "copy of the personal",
        ],
    );
    push(
        PrivacySignal::Portability,
        88,
        &[
            "data portability",
            "structured, commonly used",
            "machine-readable format",
        ],
    );
    push(
        PrivacySignal::Objection,
        86,
        &["right to object", "object to processing", "opt out"],
    );
    push(
        PrivacySignal::Rectification,
        86,
        &[
            "right to rectification",
            "rectify",
            "inaccurate personal data",
        ],
    );
    push(
        PrivacySignal::Restriction,
        85,
        &["restriction of processing", "restrict processing"],
    );
    push(
        PrivacySignal::AutomatedDecision,
        90,
        &[
            "automated decision",
            "automated processing",
            "profiling",
            "solely automated",
            "algorithmic decision",
        ],
    );
    push(
        PrivacySignal::Dpia,
        91,
        &[
            "data protection impact assessment",
            "dpia",
            "privacy impact assessment",
            "pia ",
        ],
    );
    push(
        PrivacySignal::CrossBorderTransfer,
        89,
        &[
            "transfer to a third country",
            "cross-border",
            "cross border",
            "adequacy decision",
            "standard contractual clauses",
            "binding corporate rules",
            "overseas disclosure",
            "overseas recipient",
        ],
    );
    push(
        PrivacySignal::BreachNotification,
        90,
        &[
            "personal data breach",
            "data breach",
            "notify the supervisory",
            "notify the commissioner",
            "breach notification",
            "security incident",
        ],
    );
    push(
        PrivacySignal::Children,
        87,
        &[
            "child's personal data",
            "children's data",
            "under the age of 16",
            "parental consent",
        ],
    );
    push(
        PrivacySignal::Surveillance,
        80,
        &[
            "surveillance",
            "intercept",
            "tracking",
            "cctv",
            "location data",
            "metadata retention",
        ],
    );
    // Dedup by signal name (keep highest conf).
    hits.sort_by(|a, b| a.signal.cmp(&b.signal));
    hits.dedup_by(|a, b| {
        if a.signal == b.signal {
            if a.confidence < b.confidence {
                *a = b.clone();
            }
            true
        } else {
            false
        }
    });
    hits
}

/// Human-rights / civil-rights cues (broader than privacy).
pub fn extract_rights_signals(text: &str) -> Vec<SignalHit> {
    let t = text.to_ascii_lowercase();
    let mut hits = Vec::new();
    let pairs: &[(&str, &str, u8)] = &[
        ("human-rights", "human rights", 88),
        ("human-rights", "charter of rights", 90),
        ("human-rights", "bill of rights", 88),
        ("non-discrimination", "discrimination", 80),
        ("non-discrimination", "equal treatment", 82),
        ("due-process", "due process", 85),
        ("due-process", "natural justice", 84),
        ("freedom-of-expression", "freedom of expression", 88),
        ("freedom-of-expression", "freedom of speech", 86),
        ("privacy-as-right", "right to privacy", 90),
        ("privacy-as-right", "respect for private life", 88),
        ("liberty", "personal liberty", 82),
        ("fair-trial", "fair trial", 88),
        ("refugees", "non-refoulement", 90),
        ("indigenous", "indigenous", 75),
        ("indigenous", "aboriginal", 75),
        ("disability", "disability rights", 85),
        ("labour", "workplace right", 80),
        ("labour", "industrial relations", 78),
    ];
    for (sig, needle, conf) in pairs {
        if t.contains(needle) {
            hits.push(SignalHit {
                family: "rights".into(),
                signal: (*sig).into(),
                confidence: *conf,
            });
        }
    }
    hits.sort_by(|a, b| a.signal.cmp(&b.signal));
    hits.dedup_by(|a, b| a.signal == b.signal);
    hits
}

/// Temporal / LTL-ish cues (commencement, deadlines).
pub fn extract_temporal_signals(text: &str) -> Vec<SignalHit> {
    let t = text.to_ascii_lowercase();
    let mut hits = Vec::new();
    let pairs: &[(&str, &str, u8)] = &[
        ("commencement", "commences", 80),
        ("commencement", "comes into force", 85),
        ("commencement", "comes into operation", 85),
        ("royal-assent", "royal assent", 88),
        ("proclamation", "proclamation", 82),
        ("deadline-days", "within ", 70),
        ("deadline-days", " not later than ", 78),
        ("deadline-days", "no later than", 80),
        ("sunset", "ceases to have effect", 85),
        ("sunset", "sunsets", 80),
        ("retrospective", "is taken to have", 82),
        ("retrospective", "deemed to have commenced", 85),
    ];
    for (sig, needle, conf) in pairs {
        if t.contains(needle) {
            hits.push(SignalHit {
                family: "temporal".into(),
                signal: (*sig).into(),
                confidence: *conf,
            });
        }
    }
    // Numeric day windows: "within 30 days"
    if Regex::new(r"(?i)within\s+\d+\s+days")
        .unwrap()
        .is_match(text)
    {
        hits.push(SignalHit {
            family: "temporal".into(),
            signal: "within-n-days".into(),
            confidence: 86,
        });
    }
    hits.sort_by(|a, b| a.signal.cmp(&b.signal));
    hits.dedup_by(|a, b| a.signal == b.signal);
    hits
}

/// Cross-references to other provisions / instruments.
pub fn extract_cross_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let patterns = [
        r"(?i)\bsection\s+(\d+[A-Za-z]{0,2}(?:\(\d+[A-Za-z]?\))?)",
        r"(?i)\bsubsection\s+\((\d+[A-Za-z]?)\)",
        r"(?i)\barticle\s+(\d+[A-Za-z]?)",
        r"(?i)\bschedule\s+(\d+[A-Za-z]?)",
        r"(?i)\bpart\s+([0-9IVXLC]+[A-Za-z]?)",
        r"(?i)\bdivision\s+(\d+[A-Za-z]?)",
        r"(?i)\bregulation\s+(\d+[A-Za-z]?)",
    ];
    for pat in patterns {
        let re = Regex::new(pat).unwrap();
        for cap in re.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                let s = m.as_str().trim().to_string();
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
    }
    out.truncate(32);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deontic_prohibition_and_obligation() {
        assert_eq!(
            classify_deontic("A person must not disclose personal data.").0,
            DeonticClass::Prohibition
        );
        assert_eq!(
            classify_deontic("The controller shall ensure appropriate security.").0,
            DeonticClass::Obligation
        );
        assert_eq!(
            classify_deontic("The individual has a right to access their data.").0,
            DeonticClass::Right
        );
        assert_eq!(
            classify_deontic("The Commissioner may issue guidelines.").0,
            DeonticClass::Permission
        );
    }

    #[test]
    fn privacy_gdpr_family_hits() {
        let text = "The data controller must obtain consent before processing personal data \
                    and honour the right to erasure. A DPIA is required for profiling.";
        let hits = extract_privacy_signals(text);
        let sigs: Vec<_> = hits.iter().map(|h| h.signal.as_str()).collect();
        assert!(sigs.contains(&"controller"));
        assert!(sigs.contains(&"consent"));
        assert!(sigs.contains(&"personal-data"));
        assert!(sigs.contains(&"erasure-right-to-be-forgotten"));
        assert!(sigs.contains(&"dpia") || sigs.contains(&"automated-decision-making"));
    }

    #[test]
    fn cross_refs_captured() {
        let refs = extract_cross_refs("Subject to section 12(1) and Schedule 2, see Article 6.");
        assert!(refs
            .iter()
            .any(|r| r.to_ascii_lowercase().contains("section")));
        assert!(refs
            .iter()
            .any(|r| r.to_ascii_lowercase().contains("schedule")));
    }
}
