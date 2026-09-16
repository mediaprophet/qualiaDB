//! Optional Open Badges v3 nested JSON export for SI-04.
//!
//! Native attestation is the source of truth (owner 2026-09-15). This adapter
//! does **not** replace `identity::credentials::codecs::OpenBadgeCodec`, which
//! only stamps context/type onto a flat `HashMap<String, String>` subject.

use serde_json::{json, Value};

use super::kinds::{AttestationKind, InstrumentAttestation};
use crate::semantic_instruments::errors::InstrumentError;

/// W3C VC 1.1 context required on every export.
pub const VC_V1_CONTEXT: &str = "https://www.w3.org/2018/credentials/v1";
/// Open Badges v3 context URL (same string as `OpenBadgeCodec`; not that codec).
pub const OB_V3_CONTEXT: &str =
    "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json";

/// Accessible badge image bound to a digest. Not proof of the claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeImage {
    pub locator: String,
    pub media_type: String,
    pub digest: String,
    pub accessible_text: String,
}

/// Flattened export handle. [`InstrumentBadgeExport::to_json`] nests achievement/image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentBadgeExport {
    pub context: Vec<String>,
    pub types: Vec<String>,
    pub id: String,
    pub issuer: String,
    pub credential_subject_id: String,
    pub achievement_id: String,
    pub achievement_name: String,
    pub assessment_instrument: String,
    pub image: BadgeImage,
    pub origin_is_not_truth: bool,
    pub content_digest: String,
}

fn require_text(value: &str, field: &'static str) -> Result<(), InstrumentError> {
    if value.trim().is_empty() {
        Err(InstrumentError::MissingField(field))
    } else {
        Ok(())
    }
}

fn require_sha256(value: &str, field: &'static str) -> Result<(), InstrumentError> {
    if value.starts_with("sha256:") && value.len() > "sha256:".len() {
        Ok(())
    } else {
        Err(InstrumentError::MissingField(field))
    }
}

fn person_is_instrument(person: &str, instrument: &str) -> bool {
    person == instrument
}

/// Build an optional Open Badge export from a validated native attestation.
pub fn badge_export(
    attestation: &InstrumentAttestation,
    image: BadgeImage,
    achievement_name: &str,
) -> Result<InstrumentBadgeExport, InstrumentError> {
    attestation.validate()?;
    require_sha256(&image.digest, "digest")?;
    require_text(&image.accessible_text, "accessible_text")?;
    require_text(&image.locator, "locator")?;
    require_text(achievement_name, "achievement_name")?;

    let (credential_subject_id, assessment_instrument) = match attestation.kind {
        AttestationKind::CapabilityAward => (
            attestation.award_subject.clone(),
            attestation.assessment_release_id.clone(),
        ),
        _ => (attestation.actor.clone(), attestation.release_id.clone()),
    };

    if person_is_instrument(&credential_subject_id, &assessment_instrument)
        || person_is_instrument(&credential_subject_id, &attestation.release_id)
    {
        return Err(match attestation.kind {
            AttestationKind::CapabilityAward => InstrumentError::AwardIsInstrument,
            _ => InstrumentError::RoleCollision,
        });
    }

    Ok(InstrumentBadgeExport {
        context: vec![VC_V1_CONTEXT.to_string(), OB_V3_CONTEXT.to_string()],
        types: vec![
            "VerifiableCredential".into(),
            "OpenBadgeCredential".into(),
            "InstrumentAttestation".into(),
        ],
        id: format!(
            "{}#si-open-badge/{}",
            attestation.release_id, attestation.issued_at
        ),
        issuer: attestation.actor.clone(),
        credential_subject_id,
        achievement_id: attestation.release_id.clone(),
        achievement_name: achievement_name.to_string(),
        assessment_instrument,
        image,
        origin_is_not_truth: true,
        content_digest: attestation.content_digest.clone(),
    })
}

impl InstrumentBadgeExport {
    fn nested_value(&self) -> Result<Value, InstrumentError> {
        if !self.origin_is_not_truth {
            return Err(InstrumentError::SignatureAsTruth);
        }
        if person_is_instrument(&self.credential_subject_id, &self.assessment_instrument) {
            return Err(InstrumentError::AwardIsInstrument);
        }
        Ok(json!({
            "@context": self.context,
            "type": self.types,
            "id": self.id,
            "issuer": self.issuer,
            "credentialSubject": {
                "id": self.credential_subject_id,
                "achievement": {
                    "id": self.achievement_id,
                    "name": self.achievement_name,
                    "image": {
                        "id": self.image.locator,
                        "digest": self.image.digest,
                        "mediaType": self.image.media_type,
                        "accessibleText": self.image.accessible_text,
                    }
                },
                "assessmentInstrument": self.assessment_instrument,
            },
            "originIsNotTruth": true,
            "contentDigest": self.content_digest,
        }))
    }

    /// Nested JSON-LD. Compact UTF-8. Origin is never serialised as truth.
    pub fn to_json(&self) -> Result<Vec<u8>, InstrumentError> {
        serde_json::to_vec(&self.nested_value()?)
            .map_err(|e| InstrumentError::Canonical(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::demo::demo_catalog;
    use crate::semantic_instruments::visual::VISUAL_MEDIA_TYPE;

    const DIGEST: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ACTOR: &str = "did:webizen:agent:demo-reviewer";
    const LEARNER: &str = "did:webizen:agent:learner";

    fn unit_release() -> &'static str {
        demo_catalog()
            .iter()
            .find(|s| s.slug == "unit-convert")
            .expect("unit-convert demo seed")
            .release_id
    }

    fn image() -> BadgeImage {
        BadgeImage {
            locator: "https://ns.webizen.org/demo/unit-convert/badge.10d".into(),
            media_type: VISUAL_MEDIA_TYPE.into(),
            digest: DIGEST.into(),
            accessible_text: "Unit conversion demo badge".into(),
        }
    }

    fn authorship() -> InstrumentAttestation {
        InstrumentAttestation {
            kind: AttestationKind::Authorship,
            release_id: unit_release().into(),
            content_digest: DIGEST.into(),
            actor: ACTOR.into(),
            issued_at: 1_726_358_400,
            valid_until: 0,
            origin_is_not_truth: true,
            award_subject: String::new(),
            assessment_release_id: String::new(),
        }
    }

    fn award() -> InstrumentAttestation {
        InstrumentAttestation {
            kind: AttestationKind::CapabilityAward,
            release_id: unit_release().into(),
            content_digest: DIGEST.into(),
            actor: ACTOR.into(),
            issued_at: 1_726_358_400,
            valid_until: 0,
            origin_is_not_truth: true,
            award_subject: LEARNER.into(),
            assessment_release_id: unit_release().into(),
        }
    }

    #[test]
    fn nested_json_contains_achievement_and_image_digest() {
        let export = badge_export(&authorship(), image(), "Unit conversion (demo)").unwrap();
        let bytes = export.to_json().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"achievement\""));
        assert!(text.contains("\"digest\""));
        assert!(text.contains(DIGEST));
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            v["credentialSubject"]["achievement"]["image"]["digest"].as_str(),
            Some(DIGEST)
        );
        assert_eq!(
            v["credentialSubject"]["achievement"]["id"].as_str(),
            Some(unit_release())
        );
        assert_eq!(v["originIsNotTruth"].as_bool(), Some(true));
        assert!(export.origin_is_not_truth);
        assert!(v["@context"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.as_str() == Some(VC_V1_CONTEXT)));
        assert!(v["@context"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.as_str() == Some(OB_V3_CONTEXT)));
        let types = v["type"].as_array().unwrap();
        for required in ["VerifiableCredential", "OpenBadgeCredential", "InstrumentAttestation"]
        {
            assert!(types.iter().any(|t| t.as_str() == Some(required)));
        }
    }

    #[test]
    fn capability_award_subject_differs_from_assessment_instrument() {
        let export = badge_export(&award(), image(), "Unit conversion (demo)").unwrap();
        assert_eq!(export.credential_subject_id, LEARNER);
        assert_eq!(export.assessment_instrument, unit_release());
        assert_ne!(export.credential_subject_id, export.assessment_instrument);
        let v: Value = serde_json::from_slice(&export.to_json().unwrap()).unwrap();
        let subject = v["credentialSubject"]["id"].as_str().unwrap();
        let instrument = v["credentialSubject"]["assessmentInstrument"]
            .as_str()
            .unwrap();
        assert_eq!(subject, LEARNER);
        assert_eq!(instrument, unit_release());
        assert_ne!(subject, instrument);
    }

    #[test]
    fn missing_accessible_text_fails() {
        let mut img = image();
        img.accessible_text.clear();
        assert_eq!(
            badge_export(&authorship(), img, "Unit conversion (demo)").unwrap_err(),
            InstrumentError::MissingField("accessible_text")
        );
    }

    #[test]
    fn actor_equal_to_release_is_role_collision() {
        let mut claim = authorship();
        claim.actor = unit_release().into();
        assert_eq!(
            badge_export(&claim, image(), "Unit conversion (demo)").unwrap_err(),
            InstrumentError::RoleCollision
        );
    }

    #[test]
    fn award_subject_equal_to_assessment_is_rejected() {
        let mut claim = award();
        claim.award_subject = unit_release().into();
        assert_eq!(
            badge_export(&claim, image(), "Unit conversion (demo)").unwrap_err(),
            InstrumentError::AwardIsInstrument
        );
    }

    #[test]
    fn missing_image_digest_prefix_fails() {
        let mut img = image();
        img.digest = "md5:not-sha256".into();
        assert_eq!(
            badge_export(&authorship(), img, "Unit conversion (demo)").unwrap_err(),
            InstrumentError::MissingField("digest")
        );
    }
}
