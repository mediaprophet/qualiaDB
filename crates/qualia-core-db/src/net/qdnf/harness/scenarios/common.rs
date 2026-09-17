//! Shared synthetic identifiers for observational fixtures. Not production authority.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{
    encode_label_into, verify_label, Confidentiality, LabelFields, VerifiedLabel,
};
use crate::net::qdnf::types::StrongDigest;

pub fn digest(tag: u8) -> StrongDigest {
    let mut d = StrongDigest::ZERO;
    d.0[0] = tag;
    d.0[47] = 0x5A;
    d
}

pub fn verified(conf: Confidentiality, iss: StrongDigest) -> Result<VerifiedLabel, QdnfError> {
    let mut fields = LabelFields::request(conf, iss);
    if conf.requires_audience() {
        fields.audience = iss;
    }
    let mut buf = [0u8; 256];
    let n = encode_label_into(&fields, &mut buf)?;
    verify_label(fields, &buf[..n])
}

pub fn expect_err(got: Result<(), QdnfError>, want: QdnfError) -> Result<(), QdnfError> {
    match got {
        Err(e) if e == want => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Err(want),
    }
}
