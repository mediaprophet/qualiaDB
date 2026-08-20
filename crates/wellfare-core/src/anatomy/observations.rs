//! Extensible **body observations** — measurements and attributes from the person
//! or from instruments (scale, tape, watch, clinical device, camera).
//!
//! New instruments add a **code** (LOINC / house IRI), not a new struct field.
//! Drawing on the anatomy model requires a curated [`RepresentationBind`]. An
//! unknown code is stored and listed; it is not painted.

use serde::{Deserialize, Serialize};

/// Where a value came from. Extensible via [`InstrumentKind::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    /// The person typed or declared it.
    Declared,
    Scale,
    Tape,
    Watch,
    ClinicalDevice,
    Camera,
    Other(String),
}

/// How (if at all) this observation may affect the rendered body.
/// Closed on purpose: a new watch metric does not get a new shader until
/// a bind is curated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationBind {
    /// Stored only — the default for unknown codes.
    None,
    /// Drives [`super::constitution::BodyFit`] stature.
    FitStature,
    /// Reserved for a future weight morph (not implemented).
    FitWeight,
    /// Tints eye meshes if present.
    AppearanceEye,
    /// Tints hair meshes if present.
    AppearanceHair,
    /// Tints skin envelope if present (declared; not a race code).
    AppearanceSkin,
    /// Pulse / rate overlay on the circulatory system.
    CirculatoryRate,
    /// Informs graph considerations (prevalence, pharmacology, screening).
    /// **Never** the mesh shape, karyotype, or a skin/hair tint.
    KnowledgeContext,
    /// Generic overlay on a named body-system id.
    OverlayOnSystem { system_id: String },
}

/// One coded observation about the subject. Integer value + UCUM unit
/// (no float health arithmetic in the record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyObservation {
    /// LOINC, UCUM-backed house IRI, or `q42:` token.
    pub code: String,
    pub value_milli: i64,
    /// UCUM or a short token (`mm`, `g`, `bpm`, `{named}`).
    pub unit: String,
    pub instrument: InstrumentKind,
    pub at_unix: u32,
    /// Optional human caption (“left wrist watch”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Named value for `{named}` codes (ethnicity, eye/hair/skin). Integer `value_milli` stays 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
}

/// A seed code the environment already knows how to bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownObservationCode {
    pub code: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub bind: RepresentationBindSeed,
}

/// Copy-friendly bind for the seed table (`OverlayOnSystem` is not in the seed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationBindSeed {
    None,
    FitStature,
    FitWeight,
    AppearanceEye,
    AppearanceHair,
    AppearanceSkin,
    CirculatoryRate,
    KnowledgeContext,
}

impl RepresentationBindSeed {
    pub fn into_bind(self) -> RepresentationBind {
        match self {
            Self::None => RepresentationBind::None,
            Self::FitStature => RepresentationBind::FitStature,
            Self::FitWeight => RepresentationBind::FitWeight,
            Self::AppearanceEye => RepresentationBind::AppearanceEye,
            Self::AppearanceHair => RepresentationBind::AppearanceHair,
            Self::AppearanceSkin => RepresentationBind::AppearanceSkin,
            Self::CirculatoryRate => RepresentationBind::CirculatoryRate,
            Self::KnowledgeContext => RepresentationBind::KnowledgeContext,
        }
    }
}

/// Seed binds. New instruments: add a row here or register via the graph (W16).
pub const KNOWN_OBSERVATION_CODES: &[KnownObservationCode] = &[
    KnownObservationCode {
        code: "8302-2",
        label: "body height",
        unit: "mm",
        bind: RepresentationBindSeed::FitStature,
    },
    KnownObservationCode {
        code: "29463-7",
        label: "body weight",
        unit: "g",
        bind: RepresentationBindSeed::FitWeight,
    },
    KnownObservationCode {
        code: "8867-4",
        label: "heart rate",
        unit: "/min",
        bind: RepresentationBindSeed::CirculatoryRate,
    },
    KnownObservationCode {
        code: "q42:eye-colour",
        label: "eye colour (declared)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceEye,
    },
    KnownObservationCode {
        code: "q42:hair-colour",
        label: "hair colour (declared)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceHair,
    },
    KnownObservationCode {
        code: "q42:skin-tone",
        label: "skin tone (declared; not inferred from ethnicity)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceSkin,
    },
    KnownObservationCode {
        code: "q42:ethnicity",
        label: "self-identified ethnicity (declared, repeatable)",
        unit: "{named}",
        bind: RepresentationBindSeed::KnowledgeContext,
    },
    KnownObservationCode {
        code: "q42:genetic-ancestry",
        label: "genetic ancestry context (only if the person imported a record)",
        unit: "{named}",
        bind: RepresentationBindSeed::KnowledgeContext,
    },
];

/// Bind for a code. Unknown → [`RepresentationBind::None`] (extensible store).
pub fn bind_for_code(code: &str) -> RepresentationBind {
    KNOWN_OBSERVATION_CODES
        .iter()
        .find(|k| k.code == code.trim())
        .map(|k| k.bind.into_bind())
        .unwrap_or(RepresentationBind::None)
}

pub fn is_known_code(code: &str) -> bool {
    KNOWN_OBSERVATION_CODES
        .iter()
        .any(|k| k.code == code.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_instrument_code_is_stored_not_drawn() {
        let o = BodyObservation {
            code: "99999-9".into(),
            value_milli: 72_000,
            unit: "/min".into(),
            instrument: InstrumentKind::Watch,
            at_unix: 1,
            note: Some("new watch metric".into()),
            named: None,
        };
        assert!(!is_known_code(&o.code));
        assert_eq!(bind_for_code(&o.code), RepresentationBind::None);
    }

    #[test]
    fn pulse_binds_to_circulatory_rate_not_skeleton() {
        assert_eq!(bind_for_code("8867-4"), RepresentationBind::CirculatoryRate);
        assert_ne!(
            bind_for_code("8867-4"),
            RepresentationBind::OverlayOnSystem {
                system_id: "skeletal".into()
            }
        );
    }

    #[test]
    fn ethnicity_is_knowledge_context_never_appearance_or_fit() {
        let b = bind_for_code("q42:ethnicity");
        assert_eq!(b, RepresentationBind::KnowledgeContext);
        assert_ne!(b, RepresentationBind::AppearanceSkin);
        assert_ne!(b, RepresentationBind::AppearanceHair);
        assert_ne!(b, RepresentationBind::AppearanceEye);
        assert_ne!(b, RepresentationBind::FitStature);
        assert_eq!(
            bind_for_code("q42:genetic-ancestry"),
            RepresentationBind::KnowledgeContext
        );
    }

    #[test]
    fn stature_and_weight_and_appearance_are_seeded() {
        assert_eq!(bind_for_code("8302-2"), RepresentationBind::FitStature);
        assert_eq!(bind_for_code("29463-7"), RepresentationBind::FitWeight);
        assert_eq!(
            bind_for_code("q42:eye-colour"),
            RepresentationBind::AppearanceEye
        );
        assert_eq!(
            bind_for_code("q42:hair-colour"),
            RepresentationBind::AppearanceHair
        );
        assert_eq!(
            bind_for_code("q42:skin-tone"),
            RepresentationBind::AppearanceSkin
        );
    }
}
