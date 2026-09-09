//! Bounded protection labels. Separate from NQuin sensitivity-byte cache.

pub mod decode;
pub mod flow;
pub mod join;
pub mod projection;
pub mod release;
pub mod types;
pub mod verify;

pub use decode::{decode_label_into, encode_label_into};
pub use flow::{protected_read, JobLabelContext};
pub use join::{join_labels_into, join_one};
pub use projection::{
    project_sensitivity, project_sink, require_labelled, require_labelled_content, DerivedSink,
};
pub use release::release_derivation;
pub use types::{
    Confidentiality, LabelFields, VerifiedLabel, MAX_COMPARTMENTS, MAX_JOIN_INPUTS, MAX_LABEL_BYTES,
    MAX_PURPOSES, NO_BIOMETRIC_REUSE, NO_EXTERNAL_AI, NO_PUBLIC_INDEX, NO_REDISTRIBUTE, NO_TRAINING,
};
pub use verify::{verify_label, verify_label_digest};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::types::StrongDigest;

    fn issuer(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 1;
        d
    }

    fn verified(conf: Confidentiality, iss: StrongDigest, restrictions: u16) -> VerifiedLabel {
        let mut fields = LabelFields::request(conf, iss);
        fields.restriction_bits = restrictions;
        if conf.requires_audience() {
            fields.audience = iss;
        }
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        verify_label(fields, &buf[..n]).unwrap()
    }

    #[test]
    fn join_c1_and_c2_is_c2() {
        let req = LabelFields::request(Confidentiality::C1Private, issuer(1));
        let dep = verified(Confidentiality::C2Sensitive, issuer(2), 0);
        let mut acc = req;
        join_one(&mut acc, dep.fields()).unwrap();
        assert_eq!(acc.confidentiality, Confidentiality::C2Sensitive);
    }

    #[test]
    fn disjoint_purposes_conflict() {
        let mut a = LabelFields::request(Confidentiality::C1Private, issuer(1));
        a.purpose_count = 1;
        a.purposes[0] = issuer(9);
        let mut b = LabelFields::request(Confidentiality::C1Private, issuer(2));
        b.purpose_count = 1;
        b.purposes[0] = issuer(8);
        let mut acc = a;
        assert_eq!(join_one(&mut acc, &b), Err(QdnfError::Conflict));
    }

    #[test]
    fn protected_read_joins_c2_even_if_request_was_c0() {
        let req = LabelFields::request(Confidentiality::C0Public, issuer(1));
        let mut ctx = JobLabelContext::new(req).unwrap();
        let object = verified(Confidentiality::C2Sensitive, issuer(3), NO_TRAINING);
        let src = b"patient";
        let mut dst = [0u8; 8];
        let n = protected_read(&mut ctx, &object, src, &mut dst).unwrap();
        assert_eq!(n, 7);
        assert_eq!(&dst[..7], src);
        assert_eq!(
            ctx.current().confidentiality,
            Confidentiality::C2Sensitive
        );
        assert_eq!(ctx.current().restriction_bits & NO_TRAINING, NO_TRAINING);
    }

    #[test]
    fn unknown_confidentiality_does_not_become_public() {
        assert_eq!(
            JobLabelContext::new(LabelFields::blank()).err(),
            Some(QdnfError::Conflict)
        );
        assert_ne!(Confidentiality::Unknown, Confidentiality::C0Public);
    }

    #[test]
    fn restrictions_union() {
        let mut a = LabelFields::request(Confidentiality::C1Private, issuer(1));
        a.restriction_bits = NO_TRAINING;
        let mut b = LabelFields::request(Confidentiality::C1Private, issuer(2));
        b.restriction_bits = NO_EXTERNAL_AI;
        join_one(&mut a, &b).unwrap();
        assert_eq!(a.restriction_bits, NO_TRAINING | NO_EXTERNAL_AI);
    }

    #[test]
    fn foreign_issuer_cannot_relax_confidentiality() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let mut proposed = *original.fields();
        proposed.confidentiality = Confidentiality::C0Public;
        proposed.restriction_bits = 0;
        assert_eq!(
            release_derivation(&original, &proposed, issuer(2)),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn matching_issuer_cannot_drop_restrictions() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let mut proposed = *original.fields();
        proposed.confidentiality = Confidentiality::C1Private;
        proposed.restriction_bits = 0;
        assert_eq!(
            release_derivation(&original, &proposed, issuer(1)),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn failed_join_copies_zero_bytes() {
        let mut object_fields = LabelFields::request(Confidentiality::C1Private, issuer(2));
        object_fields.purpose_count = 1;
        object_fields.purposes[0] = issuer(9);
        let mut buf = [0u8; 256];
        let n = encode_label_into(&object_fields, &mut buf).unwrap();
        let object = verify_label(object_fields, &buf[..n]).unwrap();
        let mut request = LabelFields::request(Confidentiality::C1Private, issuer(1));
        request.purpose_count = 1;
        request.purposes[0] = issuer(8);
        let mut ctx = JobLabelContext::new(request).unwrap();
        let src = b"secret";
        let mut dst = [0xFFu8; 8];
        assert_eq!(
            protected_read(&mut ctx, &object, src, &mut dst),
            Err(QdnfError::Conflict)
        );
        assert_eq!(dst, [0xFFu8; 8]);
    }

    #[test]
    fn missing_label_on_protected_content_fails_closed() {
        assert_eq!(
            require_labelled_content(true, None),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            require_labelled_content(false, None).unwrap(),
            Confidentiality::C0Public
        );
    }

    #[test]
    fn sinks_inherit_and_cannot_lower() {
        let req = LabelFields::request(Confidentiality::C2Sensitive, issuer(1));
        let mut fields = req;
        fields.audience = issuer(1);
        let ctx = JobLabelContext::new(fields).unwrap();
        for sink in [
            DerivedSink::Reply,
            DerivedSink::Summary,
            DerivedSink::Translation,
            DerivedSink::Embedding,
            DerivedSink::ModelContext,
            DerivedSink::QueryResult,
            DerivedSink::Export,
            DerivedSink::Log,
            DerivedSink::Receipt,
            DerivedSink::Backup,
        ] {
            let projected = project_sink(&ctx, sink).unwrap();
            assert_eq!(projected.confidentiality, Confidentiality::C2Sensitive);
        }
    }

    #[test]
    fn joined_distinct_issuers_cannot_release() {
        let req = LabelFields::request(Confidentiality::C1Private, issuer(1));
        let dep = verified(Confidentiality::C2Sensitive, issuer(2), NO_TRAINING);
        let mut acc = req;
        join_one(&mut acc, dep.fields()).unwrap();
        assert!(acc.issuer.is_zero());
        let joined = VerifiedLabel::from_verified(acc, issuer(9));
        let mut proposed = acc;
        proposed.confidentiality = Confidentiality::C0Public;
        assert_eq!(
            release_derivation(&joined, &proposed, issuer(1)),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            release_derivation(&joined, &proposed, issuer(2)),
            Err(QdnfError::Unauthorized)
        );
    }
}
