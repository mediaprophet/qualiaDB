//! ConnectRequest codec and `connect_from_wire`.

use super::{
    disclosure_byte, disclosure_from, finish, walk_tlvs, write_tlv, CscpError, MAGIC, MAX_BODY,
    MSG_CONNECT_REQUEST, TAG_AUTHORITY, TAG_BUDGET, TAG_CRITICAL, TAG_DEADLINE, TAG_PEER,
    TAG_PROTECTION, TAG_PURPOSE, VERSION,
};
use crate::net::peer::connectivity::policy::Disclosure;
use crate::net::peer::fabric::connect::{connect, ConnectHandle, Fabric};
use crate::net::peer::fabric::intent::{
    ConnectionIntent, ProtectionPolicy, Purpose, PurposeClass,
};
use crate::net::peer::fabric::kernel::FabricError;
use crate::net::peer::runtime::ResourceBudget;

fn known(tag: u8) -> bool {
    matches!(
        tag,
        TAG_PEER | TAG_PURPOSE | TAG_PROTECTION | TAG_BUDGET | TAG_DEADLINE | TAG_AUTHORITY
    )
}

pub fn encode_connect_request(
    intent: &ConnectionIntent,
    out: &mut [u8],
) -> Result<usize, CscpError> {
    let mut body = [0u8; 256];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], TAG_PEER | TAG_CRITICAL, &intent.peer)?;
    let mut purpose = [0u8; 9];
    purpose[..8].copy_from_slice(&intent.purpose.iri_hash.to_be_bytes());
    purpose[8] = intent.purpose.class as u8;
    n += write_tlv(&mut body[n..], TAG_PURPOSE | TAG_CRITICAL, &purpose)?;
    let prot = [
        disclosure_byte(intent.protection.disclosure),
        u8::from(intent.protection.require_e2e_session),
        u8::from(intent.protection.public_dht),
    ];
    n += write_tlv(&mut body[n..], TAG_PROTECTION | TAG_CRITICAL, &prot)?;
    let mut budget = [0u8; 24];
    budget[..8].copy_from_slice(&intent.budget.bytes.to_be_bytes());
    budget[8..16].copy_from_slice(&intent.budget.work.to_be_bytes());
    budget[16..24].copy_from_slice(&intent.budget.io.to_be_bytes());
    n += write_tlv(&mut body[n..], TAG_BUDGET | TAG_CRITICAL, &budget)?;
    let mut dl = [0u8; 16];
    dl[..8].copy_from_slice(&intent.created_ms.to_be_bytes());
    dl[8..16].copy_from_slice(&intent.deadline_ms.to_be_bytes());
    n += write_tlv(&mut body[n..], TAG_DEADLINE | TAG_CRITICAL, &dl)?;
    n += write_tlv(
        &mut body[n..],
        TAG_AUTHORITY,
        &intent.authority.to_be_bytes(),
    )?;
    finish(out, MSG_CONNECT_REQUEST, &body[..n])
}

pub fn decode_connect_request(bytes: &[u8], now_ms: u64) -> Result<ConnectionIntent, CscpError> {
    let body = super::parse_header(bytes, MSG_CONNECT_REQUEST)?;
    let mut peer = [0u8; 32];
    let mut purpose_hash = 0u64;
    let mut purpose_class = PurposeClass::Ordinary;
    let mut disclosure = Disclosure::Isolated;
    let mut require_e2e = true;
    let mut public_dht = false;
    let mut budget = ResourceBudget::ZERO;
    let mut created = 0u64;
    let mut deadline = 0u64;
    let mut authority = 0u64;
    walk_tlvs(body, known, |tag, val| {
        match tag {
            TAG_PEER if val.len() == 32 => peer.copy_from_slice(val),
            TAG_PURPOSE if val.len() == 9 => {
                purpose_hash = super::u64_be(&val[..8])?;
                purpose_class = match val[8] {
                    1 => PurposeClass::Ordinary,
                    2 => PurposeClass::Clinical,
                    3 => PurposeClass::Infrastructure,
                    _ => return Err(CscpError::Policy),
                };
            }
            TAG_PROTECTION if val.len() == 3 => {
                disclosure = disclosure_from(val[0])?;
                require_e2e = val[1] != 0;
                public_dht = val[2] != 0;
            }
            TAG_BUDGET if val.len() == 24 => {
                budget.bytes = super::u64_be(&val[..8])?;
                budget.work = super::u64_be(&val[8..16])?;
                budget.io = super::u64_be(&val[16..24])?;
            }
            TAG_DEADLINE if val.len() == 16 => {
                created = super::u64_be(&val[..8])?;
                deadline = super::u64_be(&val[8..16])?;
            }
            TAG_AUTHORITY if val.len() == 8 => authority = super::u64_be(val)?,
            _ => {}
        }
        Ok(())
    })?;
    if now_ms >= deadline {
        return Err(CscpError::Policy);
    }
    let mut intent = ConnectionIntent::new(
        peer,
        Purpose {
            iri_hash: purpose_hash,
            class: purpose_class,
        },
        ProtectionPolicy {
            disclosure,
            require_e2e_session: require_e2e,
            public_dht,
        },
        budget,
        created,
        deadline,
    );
    intent.authority = authority;
    Ok(intent)
}

pub fn connect_from_wire(
    fabric: &mut Fabric,
    bytes: &[u8],
    now_ms: u64,
) -> Result<ConnectHandle, FabricError> {
    let intent = decode_connect_request(bytes, now_ms)?;
    connect(
        fabric,
        intent.peer,
        intent.purpose,
        intent.protection,
        intent.budget,
        now_ms,
        intent.deadline_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::kernel::FabricState;
    use crate::net::peer::fabric::intent::ProtectionPolicy;

    fn sample() -> ConnectionIntent {
        ConnectionIntent::new(
            [0x42u8; 32],
            Purpose::clinical(),
            ProtectionPolicy::RELAY_ONLY,
            ResourceBudget {
                bytes: 4096,
                work: 2,
                io: 2,
            },
            10,
            9_000,
        )
    }

    #[test]
    fn connect_request_round_trip() {
        let intent = sample();
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        let back = decode_connect_request(&buf[..n], 20).unwrap();
        assert_eq!(back.peer, intent.peer);
        assert_eq!(back.protection.disclosure, Disclosure::ApprovedRelaysOnly);
        assert!(!back.protection.public_dht);
    }

    #[test]
    fn unknown_critical_tlv_is_rejected() {
        let intent = sample();
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        let extra = super::super::write_tlv(&mut buf[n..], 0x7f | TAG_CRITICAL, &[9]).unwrap();
        let blen = u16::from_be_bytes([buf[8], buf[9]]) + extra as u16;
        buf[8..10].copy_from_slice(&blen.to_be_bytes());
        assert_eq!(
            decode_connect_request(&buf[..n + extra], 20),
            Err(CscpError::UnknownCritical)
        );
        let _ = (MAGIC, VERSION, MAX_BODY);
    }

    #[test]
    fn wire_connect_isolated_queues() {
        let mut intent = sample();
        intent.protection = ProtectionPolicy::ISOLATED;
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        let mut f = Fabric::new();
        let h = connect_from_wire(&mut f, &buf[..n], 20).unwrap();
        assert_eq!(h.state, FabricState::OfflineQueued);
        assert!(f.session().is_none());
    }
}
