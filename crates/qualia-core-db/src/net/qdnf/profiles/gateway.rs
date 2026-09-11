//! Compartment gateways and removable-media transfer (E16.5).
//!
//! This module is a transfer gate, not a router: it does not call
//! `plan_routes` and does not import `crate::net::qdnf::route`.
//!
//! Labels are [`VerifiedLabel`] handles. Join/release failures map to
//! [`QdnfError::Denied`]. Removable media is [`GatewayMedia::Removable`] and
//! is admitted only for profiles that list [`ControlPredicate::IsolatedBearer`]
//! (P4). Counting boolean flags is not an approval instrument.
//!
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure (E16.7).

use super::catalog::{catalog_entry, ControlPredicate, ProtectionProfile};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{join_one, release_derivation, LabelFields, VerifiedLabel};
use crate::net::qdnf::types::StrongDigest;

/// Transfer media. Removable media is IsolatedBearer (P4) only.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayMedia {
    Network = 0,
    Removable = 1,
}

fn map_denied<T>(r: Result<T, QdnfError>) -> Result<T, QdnfError> {
    r.map_err(|_| QdnfError::Denied)
}

/// Verify labels and releasability at every transfer.
///
/// `reviewer` is the release-issuer digest checked by
/// [`release_derivation`]. Failed join or release is Denied.
pub fn gateway_transfer(
    source: &VerifiedLabel,
    destination: &VerifiedLabel,
    proposed: &LabelFields,
    reviewer: StrongDigest,
    media: GatewayMedia,
    profile: ProtectionProfile,
) -> Result<(), QdnfError> {
    if media == GatewayMedia::Removable
        && !catalog_entry(profile).contains_predicate(ControlPredicate::IsolatedBearer)
    {
        return Err(QdnfError::Denied);
    }
    let mut joined = *source.fields();
    map_denied(join_one(&mut joined, destination.fields()))?;
    map_denied(release_derivation(source, proposed, reviewer))?;
    Ok(())
}

/// Negotiated floor cannot be lowered at a gateway.
pub fn refuse_negotiated_downgrade(
    negotiated: ProtectionProfile,
    proposed: ProtectionProfile,
) -> Result<(), QdnfError> {
    if proposed.floor_rank() < negotiated.floor_rank() {
        Err(QdnfError::Downgrade)
    } else {
        Ok(())
    }
}

/// Transfer using a profile that must not sit below the negotiated floor.
pub fn gateway_transfer_negotiated(
    source: &VerifiedLabel,
    destination: &VerifiedLabel,
    proposed: &LabelFields,
    reviewer: StrongDigest,
    media: GatewayMedia,
    negotiated: ProtectionProfile,
    transfer_profile: ProtectionProfile,
) -> Result<(), QdnfError> {
    refuse_negotiated_downgrade(negotiated, transfer_profile)?;
    gateway_transfer(
        source,
        destination,
        proposed,
        reviewer,
        media,
        transfer_profile,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::policy_labels::{
        encode_label_into, verify_label, Confidentiality, NO_TRAINING,
    };

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
    fn matching_issuer_same_label_transfers() {
        let src = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        let dst = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        gateway_transfer(
            &src,
            &dst,
            src.fields(),
            issuer(1),
            GatewayMedia::Network,
            ProtectionProfile::P3,
        )
        .unwrap();
    }

    #[test]
    fn failed_join_is_denied() {
        let mut a_fields = LabelFields::request(Confidentiality::C2Sensitive, issuer(1));
        a_fields.audience = issuer(1);
        a_fields.purpose_count = 1;
        a_fields.purposes[0] = issuer(9);
        let mut buf = [0u8; 256];
        let n = encode_label_into(&a_fields, &mut buf).unwrap();
        let src = verify_label(a_fields, &buf[..n]).unwrap();
        let mut b_fields = LabelFields::request(Confidentiality::C2Sensitive, issuer(2));
        b_fields.audience = issuer(2);
        b_fields.purpose_count = 1;
        b_fields.purposes[0] = issuer(8);
        let n = encode_label_into(&b_fields, &mut buf).unwrap();
        let dst = verify_label(b_fields, &buf[..n]).unwrap();
        assert_eq!(
            gateway_transfer(
                &src,
                &dst,
                src.fields(),
                issuer(1),
                GatewayMedia::Network,
                ProtectionProfile::P3,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn failed_release_is_denied() {
        let src = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let dst = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let mut proposed = *src.fields();
        proposed.restriction_bits = 0;
        proposed.confidentiality = Confidentiality::C0Public;
        assert_eq!(
            gateway_transfer(
                &src,
                &dst,
                &proposed,
                issuer(1),
                GatewayMedia::Network,
                ProtectionProfile::P3,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn removable_media_requires_isolated_bearer() {
        let src = verified(Confidentiality::C3Compartmented, issuer(1), 0);
        let dst = verified(Confidentiality::C3Compartmented, issuer(1), 0);
        assert_eq!(
            gateway_transfer(
                &src,
                &dst,
                src.fields(),
                issuer(1),
                GatewayMedia::Removable,
                ProtectionProfile::P3,
            ),
            Err(QdnfError::Denied)
        );
        gateway_transfer(
            &src,
            &dst,
            src.fields(),
            issuer(1),
            GatewayMedia::Removable,
            ProtectionProfile::P4,
        )
        .unwrap();
    }

    #[test]
    fn foreign_reviewer_denied() {
        let src = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        let dst = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        assert_eq!(
            gateway_transfer(
                &src,
                &dst,
                src.fields(),
                issuer(2),
                GatewayMedia::Network,
                ProtectionProfile::P2,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn cannot_downgrade_negotiated_profile() {
        let src = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        let dst = verified(Confidentiality::C2Sensitive, issuer(1), 0);
        assert_eq!(
            refuse_negotiated_downgrade(ProtectionProfile::P3, ProtectionProfile::P1),
            Err(QdnfError::Downgrade)
        );
        assert_eq!(
            gateway_transfer_negotiated(
                &src,
                &dst,
                src.fields(),
                issuer(1),
                GatewayMedia::Network,
                ProtectionProfile::P3,
                ProtectionProfile::P1,
            ),
            Err(QdnfError::Downgrade)
        );
        gateway_transfer_negotiated(
            &src,
            &dst,
            src.fields(),
            issuer(1),
            GatewayMedia::Network,
            ProtectionProfile::P2,
            ProtectionProfile::P2,
        )
        .unwrap();
    }
}
