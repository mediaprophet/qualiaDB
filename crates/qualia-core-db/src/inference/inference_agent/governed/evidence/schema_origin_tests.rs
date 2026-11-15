use super::*;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

#[test]
fn signed_origin_permission_varies_independently_of_identical_media_and_bytes() {
    let k = key();
    let mut d = document();
    d.observations.allowed_sources = CandidateSource::TextReplacement.bit();
    d.observations.allowed_artifact_origins = ArtifactOrigin::RenderedCompletion.bit();
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    let verified = signed.verify(&trust(&k), 20).unwrap();
    let mut p = proposal(&d, b"same bytes");
    p.source = CandidateSource::TextReplacement;
    p.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Text,
        bytes: b"same bytes",
    };
    p.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
    let rendered = BoundProposal::bind(p, 32).unwrap();
    assert_eq!(
        evaluate_constraints(&verified, &rendered, actor(), 20),
        Ok(Decision::Allow)
    );
    p.artifact_origin = Some(ArtifactOrigin::GroundedReplacement);
    let grounded = BoundProposal::bind(p, 32).unwrap();
    assert_ne!(rendered.digest(), grounded.digest());
    assert_zero_alloc("signed artifact origin refusal", || {
        assert_eq!(
            evaluate_constraints(&verified, &grounded, actor(), 20),
            Err(PolicyError::Scope)
        );
    });
    let mut tampered = signed.clone();
    tampered.document.observations.allowed_artifact_origins |=
        ArtifactOrigin::GroundedReplacement.bit();
    assert!(matches!(
        tampered.verify(&trust(&k), 20),
        Err(PolicyError::InvalidSignature)
    ));
    let changed = tampered.document;
    let signed = SignedEvidencePolicy::sign(changed.clone(), &k).unwrap();
    let verified = signed.verify(&trust(&k), 20).unwrap();
    p.identity.policy_id = policy_digest_v3(&changed);
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 32).unwrap(), actor(), 20),
        Ok(Decision::Allow)
    );
}

#[test]
fn origin_presence_and_media_compatibility_are_explicit() {
    let d = document();
    let mut p = proposal(&d, b"text");
    for origin in [
        ArtifactOrigin::RenderedCompletion,
        ArtifactOrigin::GroundedReplacement,
        ArtifactOrigin::SemanticProjection,
    ] {
        p.artifact_origin = Some(origin);
        assert!(BoundProposal::bind(p, 64).is_err());
    }
    for (source, media) in [
        (CandidateSource::TextReplacement, ArtifactMedia::Utf8Text),
        (CandidateSource::HtmlReplacement, ArtifactMedia::Utf8Html),
    ] {
        p.source = source;
        p.payload = Payload::Artifact {
            media,
            bytes: b"text",
        };
        p.artifact_origin = None;
        assert!(BoundProposal::bind(p, 64).is_err());
        for origin in [
            ArtifactOrigin::RenderedCompletion,
            ArtifactOrigin::GroundedReplacement,
        ] {
            p.artifact_origin = Some(origin);
            assert!(BoundProposal::bind(p, 64).is_ok());
        }
        p.artifact_origin = Some(ArtifactOrigin::SemanticProjection);
        assert!(BoundProposal::bind(p, 64).is_err());
    }
    let quin = [0u8; 48];
    p.source = CandidateSource::SemanticArtifact;
    p.payload = Payload::Artifact {
        media: ArtifactMedia::CanonicalQuins,
        bytes: &quin,
    };
    assert!(BoundProposal::bind(p, 64).is_ok());
    for origin in [
        None,
        Some(ArtifactOrigin::RenderedCompletion),
        Some(ArtifactOrigin::GroundedReplacement),
    ] {
        p.artifact_origin = origin;
        assert!(BoundProposal::bind(p, 64).is_err());
    }
}

#[test]
fn signed_origin_masks_reject_missing_unknown_and_unrepresentable_permissions() {
    let k = key();
    for (sources, origins, valid) in [
        (0x01, 0, true),
        (0x01, 1, false),
        (0x10, 0, false),
        (0x10, 1, true),
        (0x20, 2, true),
        (0x30, 3, true),
        (0x10, 4, false),
        (0x40, 3, false),
        (0x40, 4, true),
        (0x70, 7, true),
        (0x70, 8, false),
        (0x70, 0xff, false),
    ] {
        let mut d = document();
        d.observations.allowed_sources = sources;
        d.observations.allowed_artifact_origins = origins;
        assert_eq!(SignedEvidencePolicy::sign(d, &k).is_ok(), valid);
    }
    let mut old = serde_json::to_value(document()).unwrap();
    old["observations"]
        .as_object_mut()
        .unwrap()
        .remove("allowed_artifact_origins");
    assert!(serde_json::from_value::<EvidencePolicy>(old).is_err());
}
