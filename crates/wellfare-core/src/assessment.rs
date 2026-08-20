//! Self-report **wellbeing assessment instruments** (T2.2) — a data-driven scoring engine plus the
//! two canonical, freely-usable instruments: **PHQ-9** (depression) and **GAD-7** (anxiety), both by
//! Spitzer, Kroenke & Williams (Pfizer), which are free to reproduce and use without permission.
//!
//! **This is a self-monitoring aid, not a diagnostic instrument.** A score never diagnoses a
//! condition — only a qualified clinician can. The engine is deliberately data-driven so that adding
//! an instrument is pure data; the numeric scoring, severity bands, and safety flags are what must be
//! correct, and they are what the tests pin down.
//!
//! ⚑ **Curation-grade, Timothy's call (out of scope for the agent to decide):** which validated
//! instruments to ship, sign-off on the interpretation copy for clinical use, and licensing for
//! copyrighted instruments (e.g. **BDI-II** is Pearson-licensed; **DASS-21** and **K10** are free and
//! can be added as data here once you approve their inclusion + wording).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// One selectable response on an instrument's ordinal scale (e.g. `0 = "Not at all"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseOption {
    pub value: u8,
    pub label: &'static str,
}

/// A severity band `[min, max]` (inclusive) over the total score, with a non-diagnostic
/// interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeverityBand {
    pub min: u32,
    pub max: u32,
    pub label: &'static str,
    pub interpretation: &'static str,
}

/// A safety flag: if the response to `item_index` (0-based) is `>= min_value`, surface `message`
/// regardless of the total score (e.g. PHQ-9 item 9, self-harm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagRule {
    pub item_index: usize,
    pub min_value: u8,
    pub message: &'static str,
}

/// A self-report instrument definition (static data).
#[derive(Debug, Clone, Copy)]
pub struct Instrument {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub attribution: &'static str,
    pub prompt: &'static str,
    pub items: &'static [&'static str],
    pub options: &'static [ResponseOption],
    pub bands: &'static [SeverityBand],
    pub flags: &'static [FlagRule],
    pub disclaimer: &'static str,
}

impl Instrument {
    /// The maximum attainable total: the largest option value × item count.
    pub fn max_score(&self) -> u32 {
        let max_opt = self
            .options
            .iter()
            .map(|o| o.value as u32)
            .max()
            .unwrap_or(0);
        max_opt * self.items.len() as u32
    }
}

/// The shared 0–3 frequency scale used by PHQ-9 and GAD-7.
const FREQ_0_3: &[ResponseOption] = &[
    ResponseOption {
        value: 0,
        label: "Not at all",
    },
    ResponseOption {
        value: 1,
        label: "Several days",
    },
    ResponseOption {
        value: 2,
        label: "More than half the days",
    },
    ResponseOption {
        value: 3,
        label: "Nearly every day",
    },
];

const DISCLAIMER: &str = "This is a self-monitoring aid, not a diagnosis — a score can't diagnose a \
condition, only a qualified clinician can. If you're in crisis or thinking of harming yourself, \
contact a local crisis line or emergency services now.";

/// PHQ-9 (Patient Health Questionnaire-9) — depression. Free to use (Spitzer, Kroenke & Williams).
pub static PHQ9: Instrument = Instrument {
    id: "phq9",
    name: "PHQ-9 (Patient Health Questionnaire-9)",
    domain: "depression",
    attribution: "Developed by Drs Spitzer, Kroenke & Williams (Pfizer). Free to reproduce; no permission required.",
    prompt: "Over the last 2 weeks, how often have you been bothered by any of the following problems?",
    items: &[
        "Little interest or pleasure in doing things",
        "Feeling down, depressed, or hopeless",
        "Trouble falling or staying asleep, or sleeping too much",
        "Feeling tired or having little energy",
        "Poor appetite or overeating",
        "Feeling bad about yourself — or that you are a failure or have let yourself or your family down",
        "Trouble concentrating on things, such as reading the newspaper or watching television",
        "Moving or speaking so slowly that other people could have noticed — or the opposite, being so fidgety or restless that you have been moving around a lot more than usual",
        "Thoughts that you would be better off dead, or of hurting yourself in some way",
    ],
    options: FREQ_0_3,
    bands: &[
        SeverityBand {
            min: 0,
            max: 4,
            label: "Minimal",
            interpretation: "Your responses suggest minimal depressive symptoms right now.",
        },
        SeverityBand {
            min: 5,
            max: 9,
            label: "Mild",
            interpretation: "Mild depressive symptoms. It can help to keep an eye on how you're doing and re-check in a couple of weeks.",
        },
        SeverityBand {
            min: 10,
            max: 14,
            label: "Moderate",
            interpretation: "Moderate symptoms. A conversation with a clinician or counsellor may help.",
        },
        SeverityBand {
            min: 15,
            max: 19,
            label: "Moderately severe",
            interpretation: "Moderately severe symptoms. Support from a clinician is recommended.",
        },
        SeverityBand {
            min: 20,
            max: 27,
            label: "Severe",
            interpretation: "Severe symptoms. Please consider reaching out to a clinician or someone you trust soon.",
        },
    ],
    flags: &[FlagRule {
        item_index: 8,
        min_value: 1,
        message: "You indicated some thoughts of being better off dead or of hurting yourself. You deserve support — please reach out to someone you trust or a crisis line. If you're in immediate danger, contact emergency services.",
    }],
    disclaimer: DISCLAIMER,
};

/// GAD-7 (Generalized Anxiety Disorder-7) — anxiety. Free to use (Spitzer, Kroenke & Williams).
pub static GAD7: Instrument = Instrument {
    id: "gad7",
    name: "GAD-7 (Generalized Anxiety Disorder-7)",
    domain: "anxiety",
    attribution: "Developed by Drs Spitzer, Kroenke & Williams (Pfizer). Free to reproduce; no permission required.",
    prompt: "Over the last 2 weeks, how often have you been bothered by the following problems?",
    items: &[
        "Feeling nervous, anxious, or on edge",
        "Not being able to stop or control worrying",
        "Worrying too much about different things",
        "Trouble relaxing",
        "Being so restless that it is hard to sit still",
        "Becoming easily annoyed or irritable",
        "Feeling afraid, as if something awful might happen",
    ],
    options: FREQ_0_3,
    bands: &[
        SeverityBand {
            min: 0,
            max: 4,
            label: "Minimal",
            interpretation: "Your responses suggest minimal anxiety symptoms right now.",
        },
        SeverityBand {
            min: 5,
            max: 9,
            label: "Mild",
            interpretation: "Mild anxiety symptoms. Keeping an eye on how you're doing can help.",
        },
        SeverityBand {
            min: 10,
            max: 14,
            label: "Moderate",
            interpretation: "Moderate symptoms. A conversation with a clinician or counsellor may help.",
        },
        SeverityBand {
            min: 15,
            max: 21,
            label: "Severe",
            interpretation: "Severe symptoms. Support from a clinician is recommended.",
        },
    ],
    flags: &[],
    disclaimer: DISCLAIMER,
};

/// All instruments this build ships.
pub fn instruments() -> [&'static Instrument; 2] {
    [&PHQ9, &GAD7]
}

/// Look up an instrument by id (`"phq9" | "gad7"`).
pub fn instrument(id: &str) -> Option<&'static Instrument> {
    match id {
        "phq9" => Some(&PHQ9),
        "gad7" => Some(&GAD7),
        _ => None,
    }
}

/// The scored outcome of one assessment sitting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentResult {
    pub id: String,
    pub instrument_id: String,
    pub responses: Vec<u8>,
    pub total: u32,
    pub band_label: String,
    pub interpretation: String,
    /// Triggered safety-flag messages (e.g. self-harm), independent of the total.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
    pub taken_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `taken_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub taken_at_instant: Option<InstantBridge>,
}

impl AssessmentResult {
    /// Resolve the taken-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn taken_at(&self) -> InstantBridge {
        self.taken_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.taken_at_unix))
    }
}

/// Score a set of responses against an instrument. Fail-closed: wrong response count or an
/// out-of-range value is an error, not a silently mis-scored result.
pub fn score(
    instrument: &Instrument,
    responses: &[u8],
    taken_at_unix: u32,
) -> Result<AssessmentResult, String> {
    if responses.len() != instrument.items.len() {
        return Err(format!(
            "{} expects {} responses, got {}",
            instrument.id,
            instrument.items.len(),
            responses.len()
        ));
    }
    let max_val = instrument
        .options
        .iter()
        .map(|o| o.value)
        .max()
        .unwrap_or(0);
    for (i, &r) in responses.iter().enumerate() {
        if r > max_val {
            return Err(format!(
                "response {r} for item {} exceeds the maximum {max_val}",
                i + 1
            ));
        }
    }
    let total: u32 = responses.iter().map(|&r| r as u32).sum();
    let band = instrument
        .bands
        .iter()
        .find(|b| total >= b.min && total <= b.max)
        .ok_or_else(|| format!("no severity band covers total {total}"))?;
    let flags: Vec<String> = instrument
        .flags
        .iter()
        .filter(|f| {
            responses
                .get(f.item_index)
                .is_some_and(|&r| r >= f.min_value)
        })
        .map(|f| f.message.to_string())
        .collect();
    Ok(AssessmentResult {
        id: Uuid::new_v4().to_string(),
        instrument_id: instrument.id.to_string(),
        responses: responses.to_vec(),
        total,
        band_label: band.label.to_string(),
        interpretation: band.interpretation.to_string(),
        flags,
        taken_at_unix,
        taken_at_instant: Some(InstantBridge::from_coarse(taken_at_unix)),
    })
}

// --- Persistence (same signed-journal pattern as the other records) ---

pub fn assessment_record_id(uuid: &str) -> String {
    format!("urn:wellfair:wellbeing_assessment:{uuid}")
}

pub fn build_assessment_envelope(
    result: &AssessmentResult,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    RecordEnvelope {
        id: assessment_record_id(&result.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(result.taken_at_unix),
        valid_time_start_instant: result.taken_at_instant,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    }
}

/// Lossless JSON of a result — stored as the journal summary so it reconstructs on read.
pub fn assessment_summary(result: &AssessmentResult) -> String {
    serde_json::to_string(result).unwrap_or_default()
}

/// Reconstruct a result from its stored JSON.
pub fn parse_assessment(json: &str) -> Option<AssessmentResult> {
    serde_json::from_str(json).ok()
}

// --- DTO for the host/UI (owned, serializable — the static `Instrument` is not `Serialize`) ---

/// A serializable snapshot of an instrument for the UI (items, options, bands, disclaimer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub attribution: String,
    pub prompt: String,
    pub items: Vec<String>,
    /// (value, label) pairs for the ordinal scale.
    pub options: Vec<(u8, String)>,
    /// (min, max, label, interpretation) severity bands.
    pub bands: Vec<(u32, u32, String, String)>,
    pub max_score: u32,
    pub disclaimer: String,
}

pub fn instrument_dto(inst: &Instrument) -> InstrumentDto {
    InstrumentDto {
        id: inst.id.to_string(),
        name: inst.name.to_string(),
        domain: inst.domain.to_string(),
        attribution: inst.attribution.to_string(),
        prompt: inst.prompt.to_string(),
        items: inst.items.iter().map(|s| s.to_string()).collect(),
        options: inst
            .options
            .iter()
            .map(|o| (o.value, o.label.to_string()))
            .collect(),
        bands: inst
            .bands
            .iter()
            .map(|b| {
                (
                    b.min,
                    b.max,
                    b.label.to_string(),
                    b.interpretation.to_string(),
                )
            })
            .collect(),
        max_score: inst.max_score(),
        disclaimer: inst.disclaimer.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ships_phq9_and_gad7_with_correct_shape() {
        assert_eq!(instruments().len(), 2);
        assert_eq!(PHQ9.items.len(), 9);
        assert_eq!(GAD7.items.len(), 7);
        assert_eq!(PHQ9.max_score(), 27);
        assert_eq!(GAD7.max_score(), 21);
        assert!(instrument("phq9").is_some());
        assert!(instrument("gad7").is_some());
        assert!(instrument("bdi2").is_none());
    }

    #[test]
    fn phq9_bands_cover_every_possible_total_without_gaps() {
        // Every total 0..=27 must fall in exactly one band.
        for total in 0..=PHQ9.max_score() {
            let hits: Vec<_> = PHQ9
                .bands
                .iter()
                .filter(|b| total >= b.min && total <= b.max)
                .collect();
            assert_eq!(hits.len(), 1, "total {total} must map to exactly one band");
        }
    }

    #[test]
    fn gad7_bands_cover_every_possible_total_without_gaps() {
        for total in 0..=GAD7.max_score() {
            let hits: Vec<_> = GAD7
                .bands
                .iter()
                .filter(|b| total >= b.min && total <= b.max)
                .collect();
            assert_eq!(hits.len(), 1, "total {total} must map to exactly one band");
        }
    }

    #[test]
    fn phq9_scoring_matches_known_bands() {
        // All zero → minimal, no flags.
        let r = score(&PHQ9, &[0; 9], 100).unwrap();
        assert_eq!(r.total, 0);
        assert_eq!(r.band_label, "Minimal");
        assert!(r.flags.is_empty());

        // All threes → 27, severe, and the self-harm flag fires (item 9).
        let r = score(&PHQ9, &[3; 9], 100).unwrap();
        assert_eq!(r.total, 27);
        assert_eq!(r.band_label, "Severe");
        assert_eq!(r.flags.len(), 1);

        // Boundary: total 10 → Moderate; total 14 → Moderate; total 15 → Moderately severe.
        assert_eq!(
            score(&PHQ9, &[2, 2, 2, 2, 2, 0, 0, 0, 0], 1).unwrap().total,
            10
        );
        assert_eq!(
            score(&PHQ9, &[2, 2, 2, 2, 2, 0, 0, 0, 0], 1)
                .unwrap()
                .band_label,
            "Moderate"
        );
        assert_eq!(
            score(&PHQ9, &[3, 3, 3, 3, 2, 0, 0, 0, 0], 1).unwrap().total,
            14
        );
        assert_eq!(
            score(&PHQ9, &[3, 3, 3, 3, 2, 0, 0, 0, 0], 1)
                .unwrap()
                .band_label,
            "Moderate"
        );
        assert_eq!(
            score(&PHQ9, &[3, 3, 3, 3, 3, 0, 0, 0, 0], 1).unwrap().total,
            15
        );
        assert_eq!(
            score(&PHQ9, &[3, 3, 3, 3, 3, 0, 0, 0, 0], 1)
                .unwrap()
                .band_label,
            "Moderately severe"
        );
    }

    #[test]
    fn phq9_self_harm_flag_fires_even_at_a_low_total() {
        // Only item 9 endorsed, everything else zero → total 1 (Minimal) but the flag MUST fire.
        let mut resp = [0u8; 9];
        resp[8] = 1;
        let r = score(&PHQ9, &resp, 1).unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.band_label, "Minimal");
        assert_eq!(
            r.flags.len(),
            1,
            "self-harm flag must fire regardless of total"
        );
    }

    #[test]
    fn gad7_scoring_matches_known_bands() {
        assert_eq!(score(&GAD7, &[0; 7], 1).unwrap().band_label, "Minimal");
        assert_eq!(score(&GAD7, &[3; 7], 1).unwrap().band_label, "Severe");
        assert_eq!(score(&GAD7, &[2, 2, 2, 2, 2, 0, 0], 1).unwrap().total, 10);
        assert_eq!(
            score(&GAD7, &[2, 2, 2, 2, 2, 0, 0], 1).unwrap().band_label,
            "Moderate"
        );
    }

    #[test]
    fn scoring_is_fail_closed_on_bad_input() {
        // Wrong response count.
        assert!(score(&PHQ9, &[0; 8], 1).is_err());
        // Out-of-range value.
        assert!(score(&GAD7, &[0, 0, 0, 0, 0, 0, 4], 1).is_err());
    }

    #[test]
    fn result_summary_round_trips_and_envelope_kind() {
        let r = score(&PHQ9, &[1; 9], 42).unwrap();
        let summary = assessment_summary(&r);
        let back = parse_assessment(&summary).expect("reconstructs");
        assert_eq!(r, back);
        let env = build_assessment_envelope(&r, "did:o", "did:a", 42);
        assert!(env.id.contains(":wellbeing_assessment:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    }

    #[test]
    fn instrument_dto_carries_items_options_and_bands() {
        let dto = instrument_dto(&PHQ9);
        assert_eq!(dto.items.len(), 9);
        assert_eq!(dto.options.len(), 4);
        assert_eq!(dto.max_score, 27);
        assert!(dto.bands.iter().any(|(_, _, label, _)| label == "Severe"));
        assert!(!dto.disclaimer.is_empty());
    }
}
