//! Domain licence instruments — signed grants binding licence terms to domain names.
//!
//! Implements the methodology in
//! `docs/work-in-progress/DOMAIN_LICENCE_METHODOLOGY_WIP.md`: a `DomainLicence`
//! is an Ed25519-signed JSON document (`.qlic.json`) that names subject domains,
//! a licence class, a capability scope, a validity window, optional payment
//! evidence, and optional chain anchors carrying the canonical digest.
//!
//! Verification is fail-closed: unknown formats, classes, signatures, domains,
//! or windows all deny. `chain_anchors` are deliberately **outside** the signed
//! canonical bytes (WIP §7-8) so an anchor can be added post-issuance without
//! re-issuing the licence.

mod codec;
mod model;
mod registry;
mod verify;

pub use codec::{
    amendment_from_json, amendment_to_json, anchor_op_return, canonical_bytes, canonical_digest,
    canonical_digest_tag, from_json, to_json, LICENCE_FORMAT,
};
pub use model::{
    AgentKind, ChainAnchor, Delegation, DomainLicence, LicenceAmendment, LicenceClass,
    PaymentEvidence,
};
pub use registry::{
    LicenceRegistry, RegistryEntry, RegistrySource, COMPILED_IN_LICENSED_DOMAINS,
};
pub use verify::{
    delegated_members, issue, issue_amendment, licence_status, normalize_domain, verify,
    verify_amendment, IssueParams, LicenceError, LicenceStatus,
};

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn base_params() -> IssueParams {
        IssueParams {
            issuer_did: "did:q42:issuer:test".into(),
            subject_domains: vec!["Example.ORG".into()],
            licence_class: LicenceClass::Community,
            capability_scope: vec!["wasm-ontology".into()],
            terms_uri: "https://webcivics.example/terms/community-1".into(),
            ..Default::default()
        }
    }

    fn pinned(lic: &DomainLicence) -> String {
        lic.issuer_pubkey_hex.clone()
    }

    #[test]
    fn issued_licence_verifies_for_subject_domain() {
        let lic = issue(base_params(), &key()).unwrap();
        assert!(lic.licence_id.starts_with("did:q42:licence:"));
        assert!(verify(&lic, 100, "example.org", "wasm-ontology", Some(&pinned(&lic)), &[]).is_ok());
        // Case-insensitive, trailing dot tolerated.
        assert!(verify(&lic, 100, "EXAMPLE.org.", "wasm-ontology", Some(&pinned(&lic)), &[]).is_ok());
    }

    #[test]
    fn tampered_signature_denies() {
        let mut lic = issue(base_params(), &key()).unwrap();
        lic.licensee_label = "forged".into();
        assert_eq!(
            verify(&lic, 100, "example.org", "wasm-ontology", Some(&pinned(&lic)), &[]),
            Err(LicenceError::BadSignature)
        );
    }

    #[test]
    fn wrong_domain_denies() {
        let lic = issue(base_params(), &key()).unwrap();
        assert_eq!(
            verify(&lic, 100, "other.org", "wasm-ontology", Some(&pinned(&lic)), &[]),
            Err(LicenceError::DomainNotCovered)
        );
    }

    #[test]
    fn expired_and_not_yet_valid_deny() {
        let mut p = base_params();
        p.not_before_unix = 50;
        p.expires_unix = 200;
        let lic = issue(p, &key()).unwrap();
        assert_eq!(
            verify(&lic, 10, "example.org", "", Some(&pinned(&lic)), &[]),
            Err(LicenceError::NotYetValid)
        );
        assert_eq!(
            verify(&lic, 200, "example.org", "", Some(&pinned(&lic)), &[]),
            Err(LicenceError::Expired)
        );
    }

    #[test]
    fn scope_not_covered_denies() {
        let lic = issue(base_params(), &key()).unwrap();
        assert_eq!(
            verify(&lic, 100, "example.org", "portal", Some(&pinned(&lic)), &[]),
            Err(LicenceError::ScopeNotCovered)
        );
    }

    #[test]
    fn unpinned_issuer_denies_when_pinned() {
        let lic = issue(base_params(), &key()).unwrap();
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let wrong = hex::encode(other.verifying_key().to_bytes());
        assert_eq!(
            verify(&lic, 100, "example.org", "", Some(&wrong), &[]),
            Err(LicenceError::UntrustedIssuer)
        );
    }

    #[test]
    fn unknown_class_and_format_fail_closed() {
        let mut lic = issue(base_params(), &key()).unwrap();
        lic.licence_class = "platinum".into();
        assert_eq!(
            verify(&lic, 100, "example.org", "", Some(&pinned(&lic)), &[]),
            Err(LicenceError::UnknownClass)
        );
        let mut lic2 = issue(base_params(), &key()).unwrap();
        lic2.format = "something-else".into();
        assert_eq!(
            verify(&lic2, 100, "example.org", "", Some(&pinned(&lic2)), &[]),
            Err(LicenceError::UnknownFormat)
        );
    }

    #[test]
    fn commercial_requires_payment_evidence_at_issue() {
        let mut p = base_params();
        p.licence_class = LicenceClass::Commercial;
        assert_eq!(issue(p.clone(), &key()), Err(LicenceError::PaymentEvidenceMissing));
        p.payment_evidence = Some(PaymentEvidence {
            rail: "xec".into(),
            txid: "ab".repeat(32),
            amount_sats: 50_000,
        });
        assert!(issue(p, &key()).is_ok());
    }

    #[test]
    fn delegation_only_for_steward_classes() {
        let mut p = base_params();
        p.delegation = Some(Delegation {
            covers_subdomains: false,
            members: vec!["bakery.example.org".into()],
            steward_agreement: String::new(),
        });
        // community does not permit delegation — refuse to issue.
        assert_eq!(issue(p.clone(), &key()), Err(LicenceError::DelegationNotPermitted));
        p.licence_class = LicenceClass::CommunitySteward;
        let lic = issue(p, &key()).unwrap();
        // member subdomain covered; non-member denied.
        assert!(verify(&lic, 100, "bakery.example.org", "", Some(&pinned(&lic)), &[]).is_ok());
        assert_eq!(
            verify(&lic, 100, "grocer.example.org", "", Some(&pinned(&lic)), &[]),
            Err(LicenceError::DomainNotCovered)
        );
    }

    #[test]
    fn subdomain_wildcard_via_covers_subdomains() {
        let mut p = base_params();
        p.licence_class = LicenceClass::CommunitySteward;
        p.delegation = Some(Delegation {
            covers_subdomains: true,
            members: vec![],
            steward_agreement: String::new(),
        });
        let lic = issue(p, &key()).unwrap();
        assert!(verify(&lic, 100, "anything.example.org", "", Some(&pinned(&lic)), &[]).is_ok());
        // The parent itself and unrelated domains stay denied.
        assert!(verify(&lic, 100, "example.org", "", Some(&pinned(&lic)), &[]).is_ok());
        assert_eq!(
            verify(&lic, 100, "notexample.org", "", Some(&pinned(&lic)), &[]),
            Err(LicenceError::DomainNotCovered)
        );
    }

    #[test]
    fn amendment_chain_join_leave() {
        let mut p = base_params();
        p.licence_class = LicenceClass::CommunitySteward;
        p.delegation = Some(Delegation {
            covers_subdomains: false,
            members: vec!["bakery.example.org".into()],
            steward_agreement: String::new(),
        });
        let lic = issue(p, &key()).unwrap();
        let add = issue_amendment(
            &lic.licence_id,
            1,
            vec!["grocer.example.org".into()],
            vec![],
            100,
            &key(),
        )
        .unwrap();
        let members = delegated_members(&lic, &[add.clone()]).unwrap();
        assert!(members.contains(&"grocer.example.org".to_string()));
        assert!(verify(&lic, 100, "grocer.example.org", "", Some(&pinned(&lic)), &[add.clone()]).is_ok());
        let remove = issue_amendment(
            &lic.licence_id,
            2,
            vec![],
            vec!["bakery.example.org".into()],
            200,
            &key(),
        )
        .unwrap();
        let members = delegated_members(&lic, &[add.clone(), remove.clone()]).unwrap();
        assert!(!members.contains(&"bakery.example.org".to_string()));
        assert_eq!(
            verify(&lic, 100, "bakery.example.org", "", Some(&pinned(&lic)), &[add, remove]),
            Err(LicenceError::DomainNotCovered)
        );
    }

    #[test]
    fn amendment_signed_by_non_issuer_denies() {
        let mut p = base_params();
        p.licence_class = LicenceClass::CommunitySteward;
        p.delegation = Some(Delegation {
            covers_subdomains: false,
            members: vec![],
            steward_agreement: String::new(),
        });
        let lic = issue(p, &key()).unwrap();
        let stranger = SigningKey::from_bytes(&[3u8; 32]);
        let forged = issue_amendment(
            &lic.licence_id,
            1,
            vec!["mall.example.org".into()],
            vec![],
            100,
            &stranger,
        )
        .unwrap();
        assert_eq!(
            delegated_members(&lic, &[forged]),
            Err(LicenceError::BadSignature)
        );
    }

    #[test]
    fn json_round_trip_preserves_licence() {
        let lic = issue(base_params(), &key()).unwrap();
        let json = to_json(&lic).unwrap();
        let back = from_json(&json).unwrap();
        assert_eq!(back, lic);
        assert_eq!(canonical_digest(&back), canonical_digest(&lic));
        assert!(canonical_digest_tag(&lic).starts_with("sha256:"));
    }

    #[test]
    fn anchors_outside_signed_bytes() {
        let mut lic = issue(base_params(), &key()).unwrap();
        let digest = canonical_digest_tag(&lic);
        lic.chain_anchors.push(ChainAnchor {
            chain: "xec".into(),
            txid: "ab".repeat(32),
            op_return_hex: hex::encode(anchor_op_return(&lic)),
            digest: digest.clone(),
        });
        // Adding an anchor does not invalidate the signature.
        assert!(verify(&lic, 100, "example.org", "", Some(&pinned(&lic)), &[]).is_ok());
        assert_eq!(canonical_digest_tag(&lic), digest);
        assert!(anchor_op_return(&lic).starts_with(b"QUALIA-LIC "));
    }

    #[test]
    fn status_reports_reason_for_unlicensed() {
        let s = licence_status(None, 0, "example.org", "", None, &[]);
        assert!(!s.licensed);
        assert_eq!(s.reason, "no licence");
        let lic = issue(base_params(), &key()).unwrap();
        let s = licence_status(Some(&lic), 100, "evil.example", "", Some(&pinned(&lic)), &[]);
        assert!(!s.licensed);
        assert_eq!(s.reason, "domain is not covered by this licence");
    }
}

