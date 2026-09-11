//! Economics, identity-independence and public-error fixtures.

use super::common::digest;
use crate::net::qdnf::contracts::identity::{refuse_alias_merge, ReferentKind};
use crate::net::qdnf::economics::{apply_payment_to_consent, ConsentBudget, PaidCredit};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::link::cookies::CookieJar;
use crate::net::qdnf::policy_labels::{
    join_one, Confidentiality, LabelFields, NO_TRAINING,
};

pub fn s24_payment_not_consent() -> Result<(), QdnfError> {
    let consent = ConsentBudget {
        allowed_bytes: 64,
        allowed_work: 8,
    };
    let paid = PaidCredit { milli_units: 10_000 };
    let after = apply_payment_to_consent(consent, paid);
    if after != consent {
        return Err(QdnfError::Denied);
    }
    let mut medical = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    medical.audience = digest(1);
    medical.restriction_bits = NO_TRAINING;
    let paid_label = LabelFields::request(Confidentiality::C0Public, digest(2));
    join_one(&mut medical, &paid_label)?;
    if medical.confidentiality != Confidentiality::C2Sensitive {
        return Err(QdnfError::Downgrade);
    }
    Ok(())
}

pub fn s26_independence() -> Result<(), QdnfError> {
    match refuse_alias_merge(
        ReferentKind::NaturalPerson,
        ReferentKind::NaturalPerson,
        digest(1),
        digest(1),
    ) {
        Err(QdnfError::Conflict) | Err(QdnfError::Unauthorized) => Ok(()),
        Err(e) => Err(e),
        Ok(()) => {
            // Same id is the same person; independence rejects two *approvals*
            // that share a controller by treating Handle as personhood.
            match refuse_alias_merge(
                ReferentKind::Handle,
                ReferentKind::NaturalPerson,
                digest(1),
                digest(1),
            ) {
                Err(QdnfError::Unauthorized) | Err(QdnfError::Conflict) => Ok(()),
                Err(e) => Err(e),
                Ok(()) => Err(QdnfError::Denied),
            }
        }
    }
}

pub fn s27_public_error() -> Result<(), QdnfError> {
    if CookieJar::grants_membership() || CookieJar::grants_application() {
        return Err(QdnfError::Denied);
    }
    Ok(())
}
