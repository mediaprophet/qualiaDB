//! Confidentiality lattice properties over [`join_one`].

use super::join::{join_labels_into, join_one};
use super::types::{Confidentiality, LabelFields};
use crate::net::qdnf::errors::QdnfError;

/// Lattice join of two confidentiality ranks. Unknown cannot join.
pub fn join_confidentiality(
    a: Confidentiality,
    b: Confidentiality,
) -> Result<Confidentiality, QdnfError> {
    let ra = a.lattice_rank()?;
    let rb = b.lattice_rank()?;
    if rb > ra {
        Ok(b)
    } else {
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::policy_labels::decode::encode_label_into;
    use crate::net::qdnf::policy_labels::egress::{
        admit_cache_insert, admit_caller_egress, EgressCaller,
    };
    use crate::net::qdnf::policy_labels::flow::JobLabelContext;
    use crate::net::qdnf::policy_labels::projection::{project_sink, DerivedSink};
    use crate::net::qdnf::policy_labels::release::release_derivation;
    use crate::net::qdnf::policy_labels::types::{VerifiedLabel, NO_TRAINING};
    use crate::net::qdnf::policy_labels::verify::verify_label;
    use crate::net::qdnf::types::StrongDigest;

    fn issuer(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 1;
        d
    }

    fn fields(conf: Confidentiality, iss: u8) -> LabelFields {
        LabelFields::request(conf, issuer(iss))
    }

    fn verified(conf: Confidentiality, iss: StrongDigest, restrictions: u16) -> VerifiedLabel {
        let mut f = LabelFields::request(conf, iss);
        f.restriction_bits = restrictions;
        if conf.requires_audience() {
            f.audience = iss;
        }
        let mut buf = [0u8; 256];
        let n = encode_label_into(&f, &mut buf).unwrap();
        verify_label(f, &buf[..n]).unwrap()
    }

    #[test]
    fn join_is_idempotent() {
        for conf in [
            Confidentiality::C0Public,
            Confidentiality::C1Private,
            Confidentiality::C2Sensitive,
            Confidentiality::C3Compartmented,
        ] {
            assert_eq!(join_confidentiality(conf, conf).unwrap(), conf);
            let mut acc = fields(conf, 1);
            if conf.requires_audience() {
                acc.audience = issuer(1);
            }
            let dep = acc;
            join_one(&mut acc, &dep).unwrap();
            assert_eq!(acc.confidentiality, conf);
        }
    }

    #[test]
    fn join_is_commutative() {
        let pairs = [
            (Confidentiality::C0Public, Confidentiality::C1Private),
            (Confidentiality::C1Private, Confidentiality::C2Sensitive),
            (
                Confidentiality::C2Sensitive,
                Confidentiality::C3Compartmented,
            ),
            (Confidentiality::C0Public, Confidentiality::C3Compartmented),
        ];
        for (a, b) in pairs {
            assert_eq!(
                join_confidentiality(a, b).unwrap(),
                join_confidentiality(b, a).unwrap()
            );
            let mut left = fields(a, 1);
            let mut right = fields(b, 1);
            if a.requires_audience() || b.requires_audience() {
                left.audience = issuer(1);
                right.audience = issuer(1);
            }
            let mut ab = left;
            join_one(&mut ab, &right).unwrap();
            let mut ba = right;
            join_one(&mut ba, &left).unwrap();
            assert_eq!(ab.confidentiality, ba.confidentiality);
        }
    }

    #[test]
    fn join_is_monotonic_max_confidentiality() {
        let mut acc = fields(Confidentiality::C0Public, 1);
        acc.audience = issuer(1);
        let mut dep = fields(Confidentiality::C2Sensitive, 1);
        dep.audience = issuer(1);
        let before = acc.confidentiality.lattice_rank().unwrap();
        join_one(&mut acc, &dep).unwrap();
        let after = acc.confidentiality.lattice_rank().unwrap();
        assert!(after >= before);
        assert_eq!(acc.confidentiality, Confidentiality::C2Sensitive);
        assert_eq!(
            join_confidentiality(Confidentiality::C1Private, Confidentiality::C3Compartmented)
                .unwrap(),
            Confidentiality::C3Compartmented
        );
    }

    #[test]
    fn multi_input_disjoint_purposes_conflict() {
        let mut a = fields(Confidentiality::C1Private, 1);
        a.purpose_count = 1;
        a.purposes[0] = issuer(9);
        let mut b = fields(Confidentiality::C1Private, 2);
        b.purpose_count = 1;
        b.purposes[0] = issuer(8);
        let mut out = LabelFields::blank();
        assert_eq!(
            join_labels_into(&a, &[b], &mut out),
            Err(QdnfError::Conflict)
        );
        let mut c = fields(Confidentiality::C1Private, 3);
        c.purpose_count = 1;
        c.purposes[0] = issuer(7);
        assert_eq!(
            join_labels_into(&a, &[b, c], &mut out),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn covert_metadata_output_inherits() {
        let mut req = fields(Confidentiality::C2Sensitive, 1);
        req.audience = issuer(1);
        let ctx = JobLabelContext::new(req).unwrap();
        for sink in [DerivedSink::Log, DerivedSink::Receipt, DerivedSink::Backup] {
            let projected = project_sink(&ctx, sink).unwrap();
            assert_eq!(projected.confidentiality, Confidentiality::C2Sensitive);
        }
    }

    #[test]
    fn forged_release_is_denied() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let mut proposed = *original.fields();
        proposed.confidentiality = Confidentiality::C0Public;
        proposed.restriction_bits = 0;
        assert_eq!(
            release_derivation(&original, &proposed, issuer(2)),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            release_derivation(&original, &proposed, issuer(1)),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn cache_contamination_c2_under_c0_denied() {
        assert_eq!(
            admit_cache_insert(Confidentiality::C0Public, Confidentiality::C2Sensitive),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn delegated_tool_egress_uses_admit_egress() {
        let mut req = fields(Confidentiality::C2Sensitive, 1);
        req.audience = issuer(1);
        req.purpose_count = 1;
        req.purposes[0] = issuer(5);
        req.compartment_count = 1;
        req.compartments[0] = issuer(4);
        let ctx = JobLabelContext::new(req).unwrap();
        assert!(admit_caller_egress(
            &ctx,
            EgressCaller::DelegatedTool,
            DerivedSink::Export,
            issuer(1),
            issuer(4),
            issuer(5),
            2,
        )
        .is_ok());
        assert_eq!(
            admit_caller_egress(
                &ctx,
                EgressCaller::DelegatedTool,
                DerivedSink::Export,
                issuer(1),
                issuer(4),
                issuer(5),
                0,
            ),
            Err(QdnfError::Denied)
        );
    }
}
