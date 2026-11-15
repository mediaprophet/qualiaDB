//! In-process QBDL provider/client over the existing `ipc_pair` fixture.
//! Wire Grant is a protocol ack. LocalWholeSourceGrant remains the authority.
mod client;
mod describe;
mod provider;
mod session;
mod protected;
mod mux;

pub use client::{LeafTicket, OpenTicket, PlainClient};
pub use describe::{
    answer_describe_pages, DescribeClient, DescribeTicket, ServedDescriptor,
};
pub use provider::{refuse_non_plain, OpenedTransfer, PlainProvider, ServedLeaf};
pub use session::{ServiceIo, Session, SessionOwner, ROLE_CLIENT, ROLE_PROVIDER};
pub use mux::{transfer_scheduled, BoundEntry, MuxReport, MuxTransfer};
pub use protected::ProtectedSession;
pub use session::encode_open_body;

use super::carrier::CarrierError;
use super::provider::OutboardError;
use super::wire::{self, Limits, PREFIX_BYTES, SERVICE_IRI};
use crate::bundle::stream::policy::LocalWholeSourceGrant;
use crate::bundle::stream::policy::PolicyError;
use crate::bundle::stream::proof::ProofError;
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::errors::QdnfError;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod two_process;

pub use wire::{PREFIX_BYTES as SERVICE_PREFIX_BYTES, SERVICE_IRI as STREAM_SERVICE_IRI};

const _: () = assert!(PREFIX_BYTES == 208);
const _: () = assert!(SERVICE_IRI.len() == 16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Malformed,
    Unsupported,
    Limit,
    Capacity,
    Overflow,
    Closed,
    Cancelled,
    State,
    Identity,
    Authority,
    Expired,
    StaleGeneration,
    Credit,
    Proof,
    Transport,
    WouldBlock,
    Association,
    Order,
    Admission,
}

impl From<wire::Error> for Error {
    fn from(e: wire::Error) -> Self {
        match e {
            wire::Error::Malformed => Self::Malformed,
            wire::Error::Unsupported => Self::Unsupported,
            wire::Error::Limit => Self::Limit,
            wire::Error::Capacity => Self::Capacity,
            wire::Error::Overflow => Self::Overflow,
        }
    }
}

pub(crate) fn from_carrier(e: CarrierError) -> Error {
    match e {
        CarrierError::Wire(w) => w.into(),
        CarrierError::Closed => Error::Closed,
        CarrierError::Association => Error::Association,
        CarrierError::Order => Error::Order,
        CarrierError::Capacity => Error::Capacity,
        CarrierError::Admission => Error::Admission,
    }
}

pub(crate) fn to_carrier(e: Error) -> CarrierError {
    match e {
        Error::Malformed => CarrierError::Wire(wire::Error::Malformed),
        Error::Unsupported => CarrierError::Wire(wire::Error::Unsupported),
        Error::Limit => CarrierError::Wire(wire::Error::Limit),
        Error::Capacity | Error::Overflow => CarrierError::Capacity,
        Error::Closed | Error::Cancelled => CarrierError::Closed,
        Error::Association => CarrierError::Association,
        Error::Order => CarrierError::Order,
        _ => CarrierError::Admission,
    }
}

pub(crate) fn map_policy(e: PolicyError) -> Error {
    match e {
        PolicyError::Spent => Error::Authority,
        PolicyError::Limit => Error::Limit,
        PolicyError::Capacity => Error::Capacity,
        PolicyError::Authority(QdnfError::Expired) => Error::Expired,
        PolicyError::Authority(QdnfError::Cancelled) => Error::Cancelled,
        PolicyError::Authority(QdnfError::StaleGeneration) => Error::StaleGeneration,
        PolicyError::Authority(QdnfError::WouldBlock) => Error::WouldBlock,
        PolicyError::Authority(_) | _ => Error::Authority,
    }
}

pub(crate) fn from_outboard(e: OutboardError) -> Error {
    match e {
        OutboardError::Identity | OutboardError::MissingNode => Error::Identity,
        OutboardError::Bounds | OutboardError::Limit => Error::Limit,
        OutboardError::Capacity => Error::Capacity,
        OutboardError::Proof(_) => Error::Proof,
        OutboardError::DuplicateNode | OutboardError::UnsortedNodes => Error::Identity,
    }
}

pub(crate) fn from_proof(e: ProofError) -> Error {
    match e {
        ProofError::Capacity => Error::Capacity,
        ProofError::Limit | ProofError::Index | ProofError::Length => Error::Limit,
        _ => Error::Proof,
    }
}

pub(crate) fn from_qdnf(e: QdnfError) -> Error {
    match e {
        QdnfError::WouldBlock => Error::WouldBlock,
        QdnfError::Closed => Error::Closed,
        QdnfError::Capacity => Error::Capacity,
        QdnfError::Malformed | QdnfError::Truncated => Error::Malformed,
        QdnfError::Unsupported | QdnfError::UnknownProfile | QdnfError::PlatformUnsupported => {
            Error::Unsupported
        }
        QdnfError::Cancelled => Error::Cancelled,
        QdnfError::Expired => Error::Expired,
        QdnfError::StaleGeneration => Error::StaleGeneration,
        QdnfError::Unauthorized | QdnfError::Denied | QdnfError::Revoked => Error::Authority,
        QdnfError::Replay => Error::Credit,
        _ => Error::Transport,
    }
}

pub(crate) fn profile_limits() -> Limits {
    Limits {
        record: 65776,
        page: 65536,
        cose: 4608,
        index: 8388608,
        metadata: 8388608,
        closure: 1048576,
        entries: 4096,
        inflight: 30,
        window: 33554432,
        cold_rows: 64,
        extent: 1 << 40,
        lifetime_ms: 30000,
    }
}

/// One selected entry, one leaf, over any [`ServiceIo`] pair.
/// Wire Grant is only a protocol ack; disclosure stays inside the local grant.
#[allow(clippy::too_many_arguments)]
pub fn transfer_one_leaf<PS: ServiceIo, CS: ServiceIo>(
    provider: &PlainProvider<'_, '_, '_>,
    client: &PlainClient<'_>,
    provider_session: &mut PS,
    client_session: &mut CS,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    leaf: u64,
    payload_out: &mut [u8],
) -> Result<ServedLeaf, Error> {
    match grant.accept_local_scoped(pins, clock, |authority| {
        Ok(run_one_leaf(
            provider,
            client,
            provider_session,
            client_session,
            authority,
            leaf,
            payload_out,
        ))
    }) {
        Ok(inner) => inner,
        Err(e) => Err(map_policy(e)),
    }
}

fn run_one_leaf<PS: ServiceIo, CS: ServiceIo>(
    provider: &PlainProvider<'_, '_, '_>,
    client: &PlainClient<'_>,
    provider_session: &mut PS,
    client_session: &mut CS,
    authority: &mut crate::bundle::stream::policy::LocalEffectControl<'_, '_>,
    leaf: u64,
    payload_out: &mut [u8],
) -> Result<ServedLeaf, Error> {
    let open = client.send_open(client_session)?;
    let opened = provider.answer_open(provider_session, authority)?;
    let ticket = client.recv_grant_send_credit_request(client_session, &open, leaf)?;
    let served = provider.answer_leaf(provider_session, authority, &opened)?;
    let n = client.recv_leaf(client_session, &ticket, payload_out)?;
    if n != served.length || served.leaf != leaf || served.entry != opened.entry {
        return Err(Error::Identity);
    }
    Ok(served)
}
